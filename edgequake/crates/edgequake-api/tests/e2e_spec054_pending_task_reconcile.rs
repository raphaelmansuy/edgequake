//! SPEC-054 / GitHub #298 — falsifiable e2e for pending docs without tasks.
//!
//! Invariant under test:
//!   pending/queued document metadata + zero active tasks
//!   → reconcile / recover-stuck / force-reprocess MUST create a task
//!   → otherwise the pipeline stays idle forever (reporter symptom).
//!
//! These tests are designed to FAIL if enqueue is skipped or only metadata
//! is rewritten.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use edgequake_api::server::{Server, ServerConfig};
use edgequake_api::services::pending_doc_task_reconcile::{
    build_pdf_recovery_task_data, ensure_task_for_pending_document, has_active_task_for_document,
    reconcile_pending_documents_missing_tasks, EnsureTaskOutcome,
};
use edgequake_api::state::AppState;
use edgequake_tasks::storage::{Pagination, TaskFilter};
use edgequake_tasks::TaskStatus;
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

const TEST_TENANT_ID: &str = "11111111-1111-1111-1111-111111111111";
const TEST_WORKSPACE_ID: &str = "22222222-2222-2222-2222-222222222222";

fn test_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    }
}

fn scoped_pending_metadata(doc_id: &str, title: &str) -> serde_json::Value {
    json!({
        "id": doc_id,
        "title": title,
        "status": "pending",
        "current_stage": "pending",
        "stage_message": "Auto-recovered after server restart (was in 'extracting' stage). Resuming from checkpoint...",
        "tenant_id": TEST_TENANT_ID,
        "workspace_id": TEST_WORKSPACE_ID,
        "source_type": "text",
        "updated_at": "2020-01-01T00:00:00Z",
    })
}

async fn seed_orphan_pending_doc(state: &AppState, doc_id: &str, content: &str) {
    let metadata = scoped_pending_metadata(doc_id, "orphan-pending.md");
    state
        .storage
        .kv_storage
        .upsert(&[(format!("{doc_id}-metadata"), metadata)])
        .await
        .expect("seed metadata");
    state
        .storage
        .kv_storage
        .upsert(&[(format!("{doc_id}-content"), json!({ "content": content }))])
        .await
        .expect("seed content");
}

async fn count_pending_tasks(state: &AppState) -> usize {
    let list = state
        .tasks
        .storage
        .list_tasks(
            TaskFilter {
                workspace_id: Some(Uuid::parse_str(TEST_WORKSPACE_ID).unwrap()),
                status: Some(TaskStatus::Pending),
                ..Default::default()
            },
            Pagination {
                page: 1,
                page_size: 100,
                ..Default::default()
            },
        )
        .await
        .expect("list tasks");
    list.tasks.len()
}

#[tokio::test]
async fn spec054_298_reconcile_respects_max_documents_stampede_guard() {
    let state = AppState::test_state();
    for i in 0..5 {
        let doc_id = format!("spec054-298-stampede-{i}");
        seed_orphan_pending_doc(&state, &doc_id, &format!("content {i}")).await;
    }

    let report = reconcile_pending_documents_missing_tasks(&state, 2, "stampede_guard")
        .await
        .expect("reconcile");

    assert_eq!(
        report.enqueued, 2,
        "falsifiable: stampede guard must enqueue at most max_documents, got {report:?}"
    );
    assert!(
        count_pending_tasks(&state).await >= 2,
        "at least 2 tasks must exist"
    );
}

#[tokio::test]
async fn spec054_298_reconcile_enqueues_task_for_orphan_pending_doc() {
    let state = AppState::test_state();
    let doc_id = "spec054-298-orphan-pending-1";
    seed_orphan_pending_doc(&state, doc_id, "Hello orphan pending content").await;

    let ws = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();
    assert!(
        !has_active_task_for_document(state.tasks.storage.as_ref(), doc_id, Some(ws))
            .await
            .unwrap(),
        "precondition: no active task — otherwise test is invalid"
    );
    assert_eq!(count_pending_tasks(&state).await, 0);

    let report = reconcile_pending_documents_missing_tasks(&state, 100, "e2e_spec054_298")
        .await
        .expect("reconcile");

    assert!(
        report.enqueued >= 1,
        "falsifiable: reconcile MUST enqueue ≥1 task for orphan pending, got {report:?}"
    );
    assert!(
        report.document_ids.iter().any(|id| id == doc_id),
        "falsifiable: enqueued ids must include {doc_id}, got {:?}",
        report.document_ids
    );
    assert!(
        has_active_task_for_document(state.tasks.storage.as_ref(), doc_id, Some(ws))
            .await
            .unwrap(),
        "falsifiable: document must have an active task after reconcile"
    );
    assert!(
        count_pending_tasks(&state).await >= 1,
        "falsifiable: TaskStorage must show pending work"
    );

    // Idempotent: second pass must not double-enqueue.
    let again = reconcile_pending_documents_missing_tasks(&state, 100, "e2e_spec054_298_again")
        .await
        .expect("second reconcile");
    assert_eq!(
        again.enqueued, 0,
        "falsifiable: second reconcile must be idempotent (AlreadyScheduled), got {again:?}"
    );
    assert!(again.already_scheduled >= 1);
}

#[tokio::test]
async fn spec054_298_ensure_task_skips_when_content_missing() {
    let state = AppState::test_state();
    let doc_id = "spec054-298-no-content";
    let metadata = scoped_pending_metadata(doc_id, "empty.md");
    state
        .storage
        .kv_storage
        .upsert(&[(format!("{doc_id}-metadata"), metadata.clone())])
        .await
        .unwrap();

    let outcome = ensure_task_for_pending_document(
        &state,
        doc_id,
        &metadata,
        None,
        "batch-no-content",
        "e2e",
    )
    .await
    .unwrap();
    assert_eq!(outcome, EnsureTaskOutcome::SkippedNoContent);
    assert_eq!(count_pending_tasks(&state).await, 0);
}

#[tokio::test]
async fn spec054_298_recover_stuck_creates_task_for_aged_pending() {
    let state = AppState::test_state();
    let app = Server::new(test_config(), state.clone()).build_router();
    let doc_id = "spec054-298-recover-stuck-pending";
    seed_orphan_pending_doc(&state, doc_id, "Recover stuck must enqueue me").await;

    let ws = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();
    assert!(
        !has_active_task_for_document(state.tasks.storage.as_ref(), doc_id, Some(ws))
            .await
            .unwrap()
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/recover-stuck")
                .header("Content-Type", "application/json")
                .header("X-Tenant-ID", TEST_TENANT_ID)
                .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                .body(Body::from(
                    json!({
                        "max_documents": 10,
                        "stuck_threshold_minutes": 1
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_success(), "recover-stuck must succeed");
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(
        parsed["requeued"].as_u64().unwrap_or(0) >= 1,
        "falsifiable: recover-stuck must requeue orphan pending, body={parsed}"
    );
    assert!(
        has_active_task_for_document(state.tasks.storage.as_ref(), doc_id, Some(ws))
            .await
            .unwrap(),
        "falsifiable: task must exist after recover-stuck"
    );
}

#[tokio::test]
async fn spec054_298_reprocess_orphan_pending_without_force() {
    let state = AppState::test_state();
    let app = Server::new(test_config(), state.clone()).build_router();
    let doc_id = "spec054-298-reprocess-orphan";
    seed_orphan_pending_doc(&state, doc_id, "Reprocess without force must work").await;

    let ws = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();
    assert!(
        !has_active_task_for_document(state.tasks.storage.as_ref(), doc_id, Some(ws))
            .await
            .unwrap()
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/reprocess")
                .header("Content-Type", "application/json")
                .header("X-Tenant-ID", TEST_TENANT_ID)
                .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                .body(Body::from(
                    json!({
                        "document_id": doc_id,
                        "force": false,
                        "max_documents": 1
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status().is_success(),
        "reprocess must succeed, got {}",
        response.status()
    );
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let requeued = parsed["requeued"]
        .as_u64()
        .or_else(|| parsed["documents_requeued"].as_u64())
        .unwrap_or(0);
    assert!(
        requeued >= 1,
        "falsifiable: orphan pending must reprocess without force, body={parsed}"
    );

    // Progress SSOT: response.task_id is the server task key (not batch reprocess_*).
    let task_id = parsed["task_id"]
        .as_str()
        .expect("single-doc reprocess must return task_id");
    assert!(
        !task_id.starts_with("reprocess_"),
        "task_id must not be the batch id: {task_id}"
    );
    assert_eq!(
        parsed["document_task_ids"][0]["task_id"].as_str(),
        Some(task_id)
    );
    let meta = state
        .storage
        .kv_storage
        .get_by_id(&format!("{doc_id}-metadata"))
        .await
        .unwrap()
        .expect("metadata");
    assert_eq!(
        meta["track_id"].as_str(),
        Some(task_id),
        "document.track_id must equal progress task_id"
    );
}

#[tokio::test]
async fn spec054_298_reprocess_reports_skip_reasons_when_no_content() {
    let state = AppState::test_state();
    let app = Server::new(test_config(), state.clone()).build_router();
    let doc_id = "spec054-298-reprocess-skip-reasons";
    let metadata = scoped_pending_metadata(doc_id, "no-content.md");
    state
        .storage
        .kv_storage
        .upsert(&[(format!("{doc_id}-metadata"), metadata)])
        .await
        .unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/reprocess")
                .header("Content-Type", "application/json")
                .header("X-Tenant-ID", TEST_TENANT_ID)
                .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                .body(Body::from(
                    json!({
                        "document_id": doc_id,
                        "force": false,
                        "max_documents": 1
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response.status().is_success());
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(parsed["requeued"].as_u64().unwrap_or(1), 0);
    assert!(
        parsed["skip_reasons"]["no_content"].as_u64().unwrap_or(0) >= 1,
        "falsifiable: reprocess must report skip_reasons when orphan has no content, body={parsed}"
    );
    assert_eq!(parsed["skipped"].as_u64().unwrap_or(0), 1);
}

#[tokio::test]
async fn spec054_298_pipeline_activity_lists_queued_after_reconcile() {
    let state = AppState::test_state();
    let app = Server::new(test_config(), state.clone()).build_router();
    let doc_id = "spec054-298-activity-queued";
    seed_orphan_pending_doc(&state, doc_id, "Activity should see queued doc").await;

    // Before reconcile: activity may be idle with empty queues (no working tasks).
    let before = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/pipeline/activity")
                .header("X-Tenant-ID", TEST_TENANT_ID)
                .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(before.status(), StatusCode::OK);

    reconcile_pending_documents_missing_tasks(&state, 10, "activity_e2e")
        .await
        .unwrap();

    let after = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/pipeline/activity")
                .header("X-Tenant-ID", TEST_TENANT_ID)
                .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(after.status(), StatusCode::OK);
    let body = axum::body::to_bytes(after.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // After reconcile: either queued docs or processing tasks must be visible.
    let queued_len = parsed["queued"].as_array().map(|a| a.len()).unwrap_or(0);
    let tasks_len = parsed["tasks"].as_array().map(|a| a.len()).unwrap_or(0);
    let working_len = parsed["working"].as_array().map(|a| a.len()).unwrap_or(0);
    assert!(
        queued_len + tasks_len + working_len >= 1,
        "falsifiable: after reconcile, activity must show queued/working/tasks, body={parsed}"
    );
}

#[tokio::test]
async fn spec054_298_pdf_reconcile_preserves_metadata_vision_provider() {
    let state = AppState::test_state();
    let doc_id = "spec054-298-pdf-mistral";
    let pdf_id = Uuid::new_v4();
    let metadata = json!({
        "id": doc_id,
        "title": "mistral-paper.pdf",
        "status": "pending",
        "current_stage": "pending",
        "stage_message": "Auto-recovered after server restart",
        "tenant_id": TEST_TENANT_ID,
        "workspace_id": TEST_WORKSPACE_ID,
        "source_type": "pdf",
        "pdf_id": pdf_id.to_string(),
        "vision_provider": "mistral",
        "vision_model": "mistral-small-latest",
        "updated_at": "2020-01-01T00:00:00Z",
    });
    state
        .storage
        .kv_storage
        .upsert(&[(format!("{doc_id}-metadata"), metadata.clone())])
        .await
        .expect("seed metadata");

    let ws = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();
    let outcome = ensure_task_for_pending_document(
        &state,
        doc_id,
        &metadata,
        None,
        "batch-mistral",
        "e2e_pdf_vision",
    )
    .await
    .expect("ensure pdf task");
    let EnsureTaskOutcome::Enqueued { task_id } = &outcome else {
        panic!("expected Enqueued, got {outcome:?}");
    };
    assert!(
        state
            .tasks
            .pipeline_state
            .get_pdf_progress(task_id)
            .await
            .is_some(),
        "falsifiable: PDF reconcile must seed progress under task_id={task_id}"
    );
    assert!(
        state
            .tasks
            .pipeline_state
            .get_pdf_progress("batch-mistral")
            .await
            .is_none(),
        "falsifiable: must not seed progress under client/batch track_id"
    );

    let list = state
        .tasks
        .storage
        .list_tasks(
            TaskFilter {
                workspace_id: Some(ws),
                status: Some(TaskStatus::Pending),
                ..Default::default()
            },
            Pagination {
                page: 1,
                page_size: 10,
                ..Default::default()
            },
        )
        .await
        .expect("list tasks");
    let task = list
        .tasks
        .iter()
        .find(|t| {
            t.task_data
                .get("existing_document_id")
                .and_then(|v| v.as_str())
                == Some(doc_id)
        })
        .expect("pdf recovery task must exist");
    assert_eq!(
        task.task_data
            .get("vision_provider")
            .and_then(|v| v.as_str()),
        Some("mistral"),
        "falsifiable: recovered PDF must keep metadata vision_provider, not hardcode ollama"
    );
    assert_eq!(
        task.task_data.get("vision_model").and_then(|v| v.as_str()),
        Some("mistral-small-latest")
    );

    let built = build_pdf_recovery_task_data(
        &metadata,
        pdf_id,
        Uuid::parse_str(TEST_TENANT_ID).unwrap(),
        ws,
        doc_id,
    );
    assert_eq!(built.vision_provider, "mistral");
}

/// #304: Reprocess must enqueue work for docs marked "Interrupted by server restart"
/// even when metadata uses the legacy `"default"` workspace/tenant alias.
#[tokio::test]
async fn issue_304_reprocess_interrupted_doc_with_default_alias() {
    let state = AppState::test_state();
    let app = Server::new(test_config(), state.clone()).build_router();
    let doc_id = "issue-304-interrupted-default";

    let metadata = json!({
        "id": doc_id,
        "title": "interrupted.md",
        "status": "failed",
        "current_stage": "failed",
        "error_message": "Interrupted by server restart — use Reprocess to resume",
        "stage_message": "Interrupted during 'extracting' stage by server restart. Use Reprocess to resume from checkpoint.",
        "tenant_id": "default",
        "workspace_id": "default",
        "source_type": "text",
        "updated_at": "2020-01-01T00:00:00Z",
    });
    state
        .storage
        .kv_storage
        .upsert(&[
            (format!("{doc_id}-metadata"), metadata),
            (
                format!("{doc_id}-content"),
                json!({ "content": "Resume me after restart" }),
            ),
        ])
        .await
        .expect("seed interrupted doc");

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/reprocess")
                .header("Content-Type", "application/json")
                .header("X-Tenant-ID", "default")
                .header("X-Workspace-ID", "default")
                .body(Body::from(
                    json!({
                        "document_id": doc_id,
                        "force": true,
                        "max_documents": 1,
                        "mode": "entities"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status().is_success(),
        "reprocess must not 400 on default alias, got {}",
        response.status()
    );
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let requeued = parsed["requeued"].as_u64().unwrap_or(0);
    assert!(
        requeued >= 1,
        "falsifiable (#304): interrupted doc with default alias must requeue, body={parsed}"
    );
    assert!(
        parsed["task_id"].as_str().is_some(),
        "must return a task_id so UI can track progress"
    );
}
