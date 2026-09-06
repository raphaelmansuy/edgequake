//! Contract: SPEC-057 P4 IngestionStatusMapper SSOT on document API.
//!
//! Seed KV document metadata (+ optional cancel intent), GET list/detail,
//! assert `display_status` / `ui_phase` for cancelled, in-flight, completed,
//! and stopping fixtures.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::{
    create_test_app_with_workers, extract_json, TEST_TENANT_ID, TEST_USER_ID, TEST_WORKSPACE_ID,
};
use edgequake_storage::kv_keys;
use serde_json::json;
use tower::ServiceExt;

async fn seed_doc(
    workers: &common::WorkerAppGuard,
    doc_id: &str,
    status: &str,
    current_stage: Option<&str>,
    failure_class: Option<&str>,
    track_id: Option<&str>,
) {
    let mut meta = json!({
        "id": doc_id,
        "title": format!("{doc_id}.md"),
        "file_name": format!("{doc_id}.md"),
        "status": status,
        "tenant_id": TEST_TENANT_ID,
        "workspace_id": TEST_WORKSPACE_ID,
        "chunk_count": 1,
        "created_at": "2026-07-17T00:00:00Z",
        "updated_at": "2026-07-17T00:00:00Z",
    });
    if let Some(stage) = current_stage {
        meta["current_stage"] = json!(stage);
    }
    if let Some(fc) = failure_class {
        meta["failure_class"] = json!(fc);
    }
    if let Some(tid) = track_id {
        meta["track_id"] = json!(tid);
    }
    let meta_key = kv_keys::doc_metadata(doc_id);
    edgequake_api::services::upsert_metadata_kv_with_index(
        workers.kv_storage.as_ref(),
        &meta_key,
        meta,
    )
    .await
    .expect("seed metadata");
}

async fn get_document(app: &axum::Router, doc_id: &str) -> serde_json::Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/documents/{doc_id}"))
                .header("X-Tenant-ID", TEST_TENANT_ID)
                .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                .header("X-User-ID", TEST_USER_ID)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK, "GET document {doc_id}");
    extract_json(response).await
}

async fn list_documents(app: &axum::Router) -> serde_json::Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents?page=1&page_size=50")
                .header("X-Tenant-ID", TEST_TENANT_ID)
                .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                .header("X-User-ID", TEST_USER_ID)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK, "list documents");
    extract_json(response).await
}

fn find_doc<'a>(list: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    list["documents"]
        .as_array()
        .expect("documents array")
        .iter()
        .find(|d| d["id"] == id)
        .unwrap_or_else(|| panic!("document {id} missing from list"))
}

#[tokio::test]
async fn contract_detail_cancelled_display_status() {
    let workers = create_test_app_with_workers().await;
    seed_doc(
        &workers,
        "p4-cancelled",
        "cancelled",
        Some("extracting"),
        Some("cancelled"),
        None,
    )
    .await;

    let body = get_document(workers.app(), "p4-cancelled").await;
    assert_eq!(body["status"], "cancelled");
    assert_eq!(body["display_status"], "cancelled");
    assert_eq!(body["ui_phase"], "terminal");
}

#[tokio::test]
async fn contract_detail_extracting_running() {
    let workers = create_test_app_with_workers().await;
    seed_doc(
        &workers,
        "p4-extracting",
        "processing",
        Some("extracting"),
        None,
        None,
    )
    .await;

    let body = get_document(workers.app(), "p4-extracting").await;
    assert_eq!(body["status"], "processing");
    assert_eq!(body["display_status"], "extracting");
    assert_eq!(body["ui_phase"], "running");
}

#[tokio::test]
async fn contract_detail_completed_terminal() {
    let workers = create_test_app_with_workers().await;
    seed_doc(
        &workers,
        "p4-completed",
        "completed",
        Some("extracting"),
        None,
        None,
    )
    .await;

    let body = get_document(workers.app(), "p4-completed").await;
    assert_eq!(body["display_status"], "completed");
    assert_eq!(body["ui_phase"], "terminal");
}

#[tokio::test]
async fn contract_list_enriches_display_status_and_ui_phase() {
    let workers = create_test_app_with_workers().await;
    seed_doc(
        &workers,
        "p4-list-cancelled",
        "cancelled",
        Some("converting"),
        Some("cancelled"),
        None,
    )
    .await;
    seed_doc(
        &workers,
        "p4-list-converting",
        "processing",
        Some("converting"),
        None,
        None,
    )
    .await;
    seed_doc(
        &workers,
        "p4-list-done",
        "completed",
        Some("completed"),
        None,
        None,
    )
    .await;

    let list = list_documents(workers.app()).await;
    let cancelled = find_doc(&list, "p4-list-cancelled");
    assert_eq!(cancelled["display_status"], "cancelled");
    assert_eq!(cancelled["ui_phase"], "terminal");

    let converting = find_doc(&list, "p4-list-converting");
    assert_eq!(converting["display_status"], "converting");
    assert_eq!(converting["ui_phase"], "running");

    let done = find_doc(&list, "p4-list-done");
    assert_eq!(done["display_status"], "completed");
    assert_eq!(done["ui_phase"], "terminal");
}

#[tokio::test]
async fn contract_list_stopping_when_cancel_intent_active() {
    let workers = create_test_app_with_workers().await;
    let track_id = "p4-stop-track-001";
    seed_doc(
        &workers,
        "p4-stopping",
        "processing",
        Some("extracting"),
        None,
        Some(track_id),
    )
    .await;

    workers.cancellation_registry.cancel(track_id).await;

    let list = list_documents(workers.app()).await;
    let doc = find_doc(&list, "p4-stopping");
    assert_eq!(
        doc["display_status"], "extracting",
        "stage preserved while stopping"
    );
    assert_eq!(doc["ui_phase"], "stopping");

    let detail = get_document(workers.app(), "p4-stopping").await;
    assert_eq!(detail["display_status"], "extracting");
    assert_eq!(detail["ui_phase"], "stopping");
}

/// Dual-SSOT zombie: relational cancelled wins over KV embedding 99% at list enrich.
#[tokio::test]
async fn contract_dual_ssot_merge_cancelled_over_kv_embedding() {
    use edgequake_api::document_read_model::merge_document_summaries;
    use edgequake_api::handlers::documents_types::DocumentSummary;
    use edgequake_api::services::enrich_document_summaries;

    let mut kv = DocumentSummary {
        id: "dual-ssot-zombie".into(),
        title: Some("01-databricks-ticket.pdf".into()),
        file_name: Some("01-databricks-ticket.pdf".into()),
        content_summary: None,
        content_length: None,
        chunk_count: 0,
        entity_count: Some(0),
        status: Some("extracting".into()),
        error_message: None,
        warning_message: None,
        track_id: Some("insert-cancelled".into()),
        created_at: Some("2026-07-30T12:00:00Z".into()),
        updated_at: Some("2026-07-30T12:07:20Z".into()),
        cost_usd: None,
        input_tokens: None,
        output_tokens: None,
        total_tokens: None,
        llm_model: None,
        embedding_model: None,
        source_type: Some("pdf".into()),
        current_stage: Some("embedding".into()),
        stage_progress: Some(0.99),
        stage_message: Some("Embedding chunks: 0/58 (0%)".into()),
        pdf_id: None,
        display_status: None,
        ui_phase: None,
        progress_counts: None,
        queue_position: None,
        eta_seconds: None,
        eta_basis: None,
        query_ready: None,
        cancelled_from_stage: None,
    classification: None,
    share_mode: None,
    security_status: None,
    owner_principal_id: None,
            export_control: None,
            pii: None,
            project_id: None,
    };
    let rel = DocumentSummary {
        status: Some("cancelled".into()),
        current_stage: Some("cancelled".into()),
        ..kv.clone()
    };
    // Simulate stale KV presentation fields vs terminal relational column.
    kv.status = Some("extracting".into());
    kv.current_stage = Some("embedding".into());
    kv.stage_progress = Some(0.99);

    let mut merged = merge_document_summaries(vec![kv], vec![rel]);
    enrich_document_summaries(&mut merged);
    assert_eq!(merged[0].status.as_deref(), Some("cancelled"));
    assert_eq!(merged[0].display_status.as_deref(), Some("cancelled"));
    assert_eq!(merged[0].ui_phase.as_deref(), Some("terminal"));
    assert_eq!(merged[0].stage_progress, Some(0.0));
}

/// Task Cancelled + KV embedding lag → list enrich uses task_status → cancelled.
#[tokio::test]
async fn contract_list_task_cancelled_beats_kv_embedding_lag() {
    use edgequake_tasks::{Task, TaskType};
    use uuid::Uuid;

    let workers = create_test_app_with_workers().await;
    let tenant_id = Uuid::parse_str(TEST_TENANT_ID).unwrap();
    let workspace_id = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();
    let doc_id = "p4-task-lag-embedding";

    let mut task = Task::new(
        tenant_id,
        workspace_id,
        TaskType::Insert,
        json!({ "document_id": doc_id }),
    );
    task.mark_cancelled();
    let track_id = task.track_id.clone();
    workers.task_storage.create_task(&task).await.unwrap();

    seed_doc(
        &workers,
        doc_id,
        "extracting",
        Some("embedding"),
        None,
        Some(&track_id),
    )
    .await;
    // Force residual progress after seed.
    let meta_key = kv_keys::doc_metadata(doc_id);
    let mut meta = workers
        .kv_storage
        .get_by_id(&meta_key)
        .await
        .unwrap()
        .unwrap();
    meta["stage_progress"] = json!(0.99);
    meta["stage_message"] = json!("Embedding chunks: 0/58 (0%)");
    edgequake_api::services::upsert_metadata_kv_with_index(
        workers.kv_storage.as_ref(),
        &meta_key,
        meta,
    )
    .await
    .unwrap();

    let list = list_documents(workers.app()).await;
    let doc = find_doc(&list, doc_id);
    assert_eq!(
        doc["display_status"], "cancelled",
        "task Cancelled must win over KV embedding lag"
    );
    assert_eq!(doc["ui_phase"], "terminal");
}

#[tokio::test]
async fn contract_cancel_message_maps_to_cancelled_not_failed() {
    let workers = create_test_app_with_workers().await;
    let meta_key = kv_keys::doc_metadata("p4-cancel-msg");
    let meta = json!({
        "id": "p4-cancel-msg",
        "title": "p4-cancel-msg.md",
        "status": "failed",
        "failure_class": "cancelled",
        "error_message": "Task cancelled by user",
        "tenant_id": TEST_TENANT_ID,
        "workspace_id": TEST_WORKSPACE_ID,
        "created_at": "2026-07-17T00:00:00Z",
        "updated_at": "2026-07-17T00:00:00Z",
    });
    edgequake_api::services::upsert_metadata_kv_with_index(
        workers.kv_storage.as_ref(),
        &meta_key,
        meta,
    )
    .await
    .expect("seed");

    let body = get_document(workers.app(), "p4-cancel-msg").await;
    assert_eq!(body["display_status"], "cancelled");
    assert_eq!(body["ui_phase"], "terminal");
}
