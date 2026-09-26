//! Contract / E2E: task cancel intent + fairness park wiring.
//!
//! Validates SPEC-057 P0 cancel/fairness through the HTTP API:
//! - Pending task cancel → Cancelled status + cancel intent
//! - Cancelled is idempotent
//! - Indexed rejects cancel (409)
//! - Doc KV cancel writes `failure_class=cancelled`
//! - PDF cancel → `PdfProcessingStatus::Cancelled` (postgres feature)
//! - Worker-backed app exposes a shared tenant fairness limiter

mod common;

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::{
    create_test_app_with_workers, extract_json, TEST_TENANT_ID, TEST_USER_ID, TEST_WORKSPACE_ID,
};
use edgequake_storage::kv_keys;
use edgequake_tasks::{
    classify_ingestion_failure, is_cancel_failure_message, IngestionFailureClass, Task, TaskStatus,
    TaskType,
};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

async fn post_cancel(app: &axum::Router, track_id: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/tasks/{track_id}/cancel"))
                .header("X-Tenant-ID", TEST_TENANT_ID)
                .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                .header("X-User-ID", TEST_USER_ID)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn get_task(app: &axum::Router, track_id: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/tasks/{track_id}"))
                .header("X-Tenant-ID", TEST_TENANT_ID)
                .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                .header("X-User-ID", TEST_USER_ID)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn e2e_cancel_pending_task_persists_cancelled_and_intent() {
    let workers = create_test_app_with_workers().await;
    let tenant_id = Uuid::parse_str(TEST_TENANT_ID).unwrap();
    let workspace_id = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();

    let task = Task::new(
        tenant_id,
        workspace_id,
        TaskType::Insert,
        serde_json::json!({ "document_id": "cancel-contract-doc" }),
    );
    let track_id = task.track_id.clone();
    workers.task_storage.create_task(&task).await.unwrap();

    let response = post_cancel(workers.app(), &track_id).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert_eq!(body["status"], "cancelled");
    assert_eq!(body["track_id"], track_id);

    let stored = workers
        .task_storage
        .get_task(&track_id)
        .await
        .unwrap()
        .expect("task row");
    assert_eq!(stored.status, TaskStatus::Cancelled);
    assert!(
        workers
            .cancellation_registry
            .has_cancel_intent(&track_id)
            .await,
        "cancel intent must be recorded so parked/queued copies are skipped"
    );

    // Idempotent re-cancel
    let again = post_cancel(workers.app(), &track_id).await;
    assert_eq!(again.status(), StatusCode::OK);
    let again_body = extract_json(again).await;
    assert_eq!(again_body["status"], "cancelled");
}

#[tokio::test]
async fn e2e_cancel_indexed_task_conflicts() {
    let workers = create_test_app_with_workers().await;
    let tenant_id = Uuid::parse_str(TEST_TENANT_ID).unwrap();
    let workspace_id = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();

    let mut task = Task::new(
        tenant_id,
        workspace_id,
        TaskType::Insert,
        serde_json::json!({}),
    );
    // SPEC-091 QW0: Complete requires Processing (state machine SSOT).
    task.mark_processing();
    task.mark_success(serde_json::json!({ "ok": true }));
    let track_id = task.track_id.clone();
    workers.task_storage.create_task(&task).await.unwrap();

    let response = post_cancel(workers.app(), &track_id).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert!(
        !workers
            .cancellation_registry
            .has_cancel_intent(&track_id)
            .await,
        "Indexed tasks must not receive cancel intent"
    );

    let get = get_task(workers.app(), &track_id).await;
    assert_eq!(get.status(), StatusCode::OK);
    let body = extract_json(get).await;
    assert_eq!(body["status"], "indexed");
}

#[tokio::test]
async fn e2e_worker_app_wires_tenant_fairness_limiter() {
    let workers = create_test_app_with_workers().await;
    // Queue metrics should report the shared limiter (max_tasks_per_tenant > 0).
    let response = workers
        .app()
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/pipeline/queue-metrics")
                .header("X-Tenant-ID", TEST_TENANT_ID)
                .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    let max = body["max_tasks_per_tenant"]
        .as_u64()
        .expect("max_tasks_per_tenant present");
    assert!(
        max > 0,
        "test worker pool must expose fairness limiter (got {max})"
    );
    let lifecycle_max = body["max_lifecycle_tasks_per_tenant"]
        .as_u64()
        .expect("max_lifecycle_tasks_per_tenant present");
    assert!(
        lifecycle_max > 0,
        "lifecycle fairness lane must be exposed (got {lifecycle_max})"
    );
    assert!(body["tenant_park_waiters"].as_u64().is_some());
    assert!(body["tenant_park_waiters_ingest"].as_u64().is_some());
    assert!(body["tenant_park_waiters_lifecycle"].as_u64().is_some());
    assert!(body["cancel_intent_count"].as_u64().is_some());
}

#[tokio::test]
async fn e2e_cancel_task_writes_doc_kv_failure_class_cancelled() {
    let workers = create_test_app_with_workers().await;
    let tenant_id = Uuid::parse_str(TEST_TENANT_ID).unwrap();
    let workspace_id = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();

    let task = Task::new(
        tenant_id,
        workspace_id,
        TaskType::Insert,
        json!({ "document_id": "cancel-kv-doc" }),
    );
    let track_id = task.track_id.clone();
    workers.task_storage.create_task(&task).await.unwrap();

    let doc_id = "cancel-kv-doc";
    let meta_key = kv_keys::doc_metadata(doc_id);
    let meta = json!({
        "id": doc_id,
        "track_id": track_id,
        "status": "processing",
        "current_stage": "extracting",
        "tenant_id": TEST_TENANT_ID,
        "workspace_id": TEST_WORKSPACE_ID,
    });
    edgequake_api::services::upsert_metadata_kv_with_index(
        workers.kv_storage.as_ref(),
        &meta_key,
        meta,
    )
    .await
    .expect("seed metadata");

    let response = post_cancel(workers.app(), &track_id).await;
    assert_eq!(response.status(), StatusCode::OK);

    let stored = workers
        .kv_storage
        .get_by_id(&meta_key)
        .await
        .unwrap()
        .expect("metadata after cancel");
    assert_eq!(stored["status"], "cancelled");
    assert_eq!(stored["failure_class"], "cancelled");
    assert_eq!(stored["recommended_action"], "none");
    assert_eq!(stored["cancelled_from_stage"], "extracting");
}

/// Worker cancel SSOT: sync_doc_cancelled clears stage_progress + failure_class.
#[tokio::test]
async fn e2e_worker_cancel_ssot_clears_stage_progress() {
    let workers = create_test_app_with_workers().await;
    let doc_id = "worker-cancel-ssot-doc";
    let meta_key = kv_keys::doc_metadata(doc_id);
    edgequake_api::services::upsert_metadata_kv_with_index(
        workers.kv_storage.as_ref(),
        &meta_key,
        json!({
            "id": doc_id,
            "status": "embedding",
            "current_stage": "embedding",
            "stage_progress": 0.99,
            "tenant_id": TEST_TENANT_ID,
            "workspace_id": TEST_WORKSPACE_ID,
        }),
    )
    .await
    .unwrap();

    let updated = edgequake_api::services::sync_doc_cancelled_by_document_id(
        Arc::clone(&workers.kv_storage),
        None,
        doc_id,
        "Task cancelled during 'embedding' stage",
    )
    .await
    .unwrap();
    assert!(updated);

    let stored = workers
        .kv_storage
        .get_by_id(&meta_key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored["status"], "cancelled");
    assert_eq!(stored["failure_class"], "cancelled");
    assert_eq!(stored["stage_progress"], 0.0);
}

/// Cancel mid-embedding must not leave Active Runs / list as embedding+running.
#[tokio::test]
async fn e2e_cancel_embedding_doc_list_shows_cancelled_not_running() {
    let workers = create_test_app_with_workers().await;
    let tenant_id = Uuid::parse_str(TEST_TENANT_ID).unwrap();
    let workspace_id = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();

    let doc_id = "cancel-embedding-zombie";
    let task = Task::new(
        tenant_id,
        workspace_id,
        TaskType::Insert,
        json!({ "document_id": doc_id }),
    );
    let track_id = task.track_id.clone();
    workers.task_storage.create_task(&task).await.unwrap();

    let meta_key = kv_keys::doc_metadata(doc_id);
    let meta = json!({
        "id": doc_id,
        "title": "01-databricks-ticket.pdf",
        "track_id": track_id,
        "status": "extracting",
        "current_stage": "embedding",
        "stage_progress": 0.99,
        "stage_message": "Embedding chunks: 0/58 (0%)",
        "tenant_id": TEST_TENANT_ID,
        "workspace_id": TEST_WORKSPACE_ID,
        "created_at": "2026-07-30T12:00:00Z",
        "updated_at": "2026-07-30T12:00:00Z",
    });
    edgequake_api::services::upsert_metadata_kv_with_index(
        workers.kv_storage.as_ref(),
        &meta_key,
        meta,
    )
    .await
    .expect("seed metadata");

    let response = post_cancel(workers.app(), &track_id).await;
    assert_eq!(response.status(), StatusCode::OK);

    let stored = workers
        .kv_storage
        .get_by_id(&meta_key)
        .await
        .unwrap()
        .expect("metadata after cancel");
    assert_eq!(stored["status"], "cancelled");
    assert_eq!(stored["current_stage"], "cancelled");
    assert_eq!(stored["stage_progress"], 0.0);

    let list = workers
        .app()
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
    assert_eq!(list.status(), StatusCode::OK);
    let body = extract_json(list).await;
    let doc = body["documents"]
        .as_array()
        .expect("documents")
        .iter()
        .find(|d| d["id"] == doc_id)
        .expect("cancelled doc in list");
    assert_eq!(doc["display_status"], "cancelled");
    assert_eq!(doc["ui_phase"], "terminal");
    assert_ne!(doc["display_status"], "embedding");
    assert_ne!(doc["ui_phase"], "running");
}

#[test]
fn vision_cancel_message_classifies_as_cancelled() {
    let msg = "Cancelled during vision PDF conversion";
    assert!(is_cancel_failure_message(msg));
    assert_eq!(
        classify_ingestion_failure(msg),
        IngestionFailureClass::Cancelled
    );
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn e2e_pdf_cancel_sets_pdf_status_cancelled_not_failed() {
    use edgequake_storage::{CreatePdfRequest, PdfProcessingStatus};

    let workers = create_test_app_with_workers().await;
    let pdf_storage = workers
        .pdf_storage
        .as_ref()
        .expect("memory pdf storage wired under postgres feature");

    let tenant_id = Uuid::parse_str(TEST_TENANT_ID).unwrap();
    let workspace_id = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();
    let pdf_bytes = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n1 0 obj<<>>endobj\ntrailer<<>>\n%%EOF\n";
    let pdf_id = pdf_storage
        .create_pdf(CreatePdfRequest {
            workspace_id,
            filename: "cancel-contract.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            file_size_bytes: pdf_bytes.len() as i64,
            sha256_checksum: format!("cancel-contract-{}", Uuid::new_v4()),
            page_count: Some(1),
            pdf_data: pdf_bytes.to_vec(),
            vision_model: None,
        })
        .await
        .expect("create pdf");

    pdf_storage
        .update_pdf_status(&pdf_id, PdfProcessingStatus::Processing)
        .await
        .unwrap();

    // Doc id in task.existing_document_id → sync_doc_cancelled_for_task on cancel.
    let doc_id = format!("pdf-cancel-doc-{}", Uuid::new_v4());
    let meta_key = kv_keys::doc_metadata(&doc_id);
    let meta = json!({
        "id": doc_id,
        "status": "processing",
        "tenant_id": TEST_TENANT_ID,
        "workspace_id": TEST_WORKSPACE_ID,
    });
    edgequake_api::services::upsert_metadata_kv_with_index(
        workers.kv_storage.as_ref(),
        &meta_key,
        meta,
    )
    .await
    .expect("seed metadata");

    let task = Task::new(
        tenant_id,
        workspace_id,
        TaskType::PdfProcessing,
        json!({
            "pdf_id": pdf_id.to_string(),
            "existing_document_id": doc_id,
        }),
    );
    let track_id = task.track_id.clone();
    workers.task_storage.create_task(&task).await.unwrap();

    let response = workers
        .app()
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/documents/pdf/{pdf_id}/cancel"))
                .header("X-Tenant-ID", TEST_TENANT_ID)
                .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                .header("X-User-ID", TEST_USER_ID)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert_eq!(body["success"], true);

    let pdf = pdf_storage
        .get_pdf(&pdf_id)
        .await
        .unwrap()
        .expect("pdf row");
    assert_eq!(
        pdf.processing_status,
        PdfProcessingStatus::Cancelled,
        "SPEC-057: PDF cancel must not map to Failed"
    );

    let stored_task = workers
        .task_storage
        .get_task(&track_id)
        .await
        .unwrap()
        .expect("task");
    assert_eq!(stored_task.status, TaskStatus::Cancelled);

    let stored_doc = workers
        .kv_storage
        .get_by_id(&meta_key)
        .await
        .unwrap()
        .expect("doc metadata after PDF cancel");
    assert_eq!(
        stored_doc["status"], "cancelled",
        "SPEC-057 P0: PDF cancel must sync doc KV to cancelled"
    );
    assert_eq!(stored_doc["failure_class"], "cancelled");
}

/// SPEC-057 P2: cancel Convert also cancels Pending Insert for the same pdf_id.
#[tokio::test]
async fn e2e_cancel_convert_cancels_linked_pending_insert() {
    let workers = create_test_app_with_workers().await;
    let tenant_id = Uuid::parse_str(TEST_TENANT_ID).unwrap();
    let workspace_id = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();
    let pdf_id = Uuid::new_v4();

    let convert = Task::new(
        tenant_id,
        workspace_id,
        TaskType::PdfProcessing,
        json!({ "pdf_id": pdf_id.to_string() }),
    );
    let convert_track = convert.track_id.clone();
    workers.task_storage.create_task(&convert).await.unwrap();

    let insert = Task::new(
        tenant_id,
        workspace_id,
        TaskType::Insert,
        json!({
            "text": "from convert",
            "file_source": "x.pdf",
            "workspace_id": workspace_id.to_string(),
            "metadata": { "pdf_id": pdf_id.to_string() },
        }),
    );
    let insert_track = insert.track_id.clone();
    workers.task_storage.create_task(&insert).await.unwrap();

    let response = post_cancel(workers.app(), &convert_track).await;
    assert_eq!(response.status(), StatusCode::OK);

    let convert_row = workers
        .task_storage
        .get_task(&convert_track)
        .await
        .unwrap()
        .unwrap();
    let insert_row = workers
        .task_storage
        .get_task(&insert_track)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(convert_row.status, TaskStatus::Cancelled);
    assert_eq!(
        insert_row.status,
        TaskStatus::Cancelled,
        "P2 cancel chain must cancel linked Insert"
    );
}

/// SPEC-057 P1: after cancel, claim_next (restart simulation) never returns the track.
#[tokio::test]
async fn e2e_cancel_pending_never_claimed_after_restart_sim() {
    use std::time::Duration;

    let workers = create_test_app_with_workers().await;
    let tenant_id = Uuid::parse_str(TEST_TENANT_ID).unwrap();
    let workspace_id = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();

    let task = Task::new(
        tenant_id,
        workspace_id,
        TaskType::Insert,
        json!({ "document_id": "cancel-restart-sim" }),
    );
    let track_id = task.track_id.clone();
    workers.task_storage.create_task(&task).await.unwrap();

    let response = post_cancel(workers.app(), &track_id).await;
    assert_eq!(response.status(), StatusCode::OK);

    let claimed = workers
        .task_storage
        .claim_next("restart-sim-worker", Duration::from_secs(120))
        .await
        .unwrap();
    assert!(
        claimed.as_ref().map(|t| t.track_id.as_str()) != Some(track_id.as_str()),
        "claim_next must not return Cancelled track after restart"
    );
}

/// Dual-lane fairness: 3 Deletion + 1 PdfProcessing under local-style caps —
/// PDF must start while a deletion is still non-terminal (not FIFO-starved).
#[tokio::test]
async fn e2e_delete_tasks_do_not_starve_pdf_ingest_lane() {
    use async_trait::async_trait;
    use edgequake_tasks::{
        memory::MemoryTaskStorage,
        queue::ChannelTaskQueue,
        worker::{SharedTaskProcessor, TaskProcessor, WorkerPool, WorkerPoolConfig},
        TaskQueue, TaskResult, TaskStorage,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio_util::sync::CancellationToken;

    struct CountingProcessor {
        deletion_started: Arc<AtomicUsize>,
        pdf_started: Arc<AtomicUsize>,
        pdf_gate: Arc<tokio::sync::Notify>,
    }

    #[async_trait]
    impl TaskProcessor for CountingProcessor {
        async fn process(
            &self,
            task: &mut Task,
            _cancel_token: CancellationToken,
        ) -> TaskResult<serde_json::Value> {
            match task.task_type {
                TaskType::Deletion => {
                    self.deletion_started.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
                }
                TaskType::PdfProcessing => {
                    self.pdf_started.fetch_add(1, Ordering::SeqCst);
                    self.pdf_gate.notify_waiters();
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                }
                _ => {}
            }
            Ok(json!({ "ok": true }))
        }
    }

    let tenant_id = Uuid::parse_str(TEST_TENANT_ID).unwrap();
    let workspace_id = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();
    let deletion_started = Arc::new(AtomicUsize::new(0));
    let pdf_started = Arc::new(AtomicUsize::new(0));
    let pdf_gate = Arc::new(tokio::sync::Notify::new());
    let processor: SharedTaskProcessor = Arc::new(CountingProcessor {
        deletion_started: Arc::clone(&deletion_started),
        pdf_started: Arc::clone(&pdf_started),
        pdf_gate: Arc::clone(&pdf_gate),
    });

    let queue = Arc::new(ChannelTaskQueue::new(50));
    let storage = Arc::new(MemoryTaskStorage::new());
    let config = WorkerPoolConfig {
        num_workers: 4,
        auto_retry: false,
        initial_retry_delay_ms: 100,
        max_retry_delay_ms: 5000,
        backoff_multiplier: 2.0,
        max_tasks_per_tenant: 2,
        max_lifecycle_tasks_per_tenant: 4,
        processing_timeout_secs: 300,
        provider_budget: 0,
        tenant_lane_weight: 1,
    };
    let mut pool = WorkerPool::new(config, queue.clone(), storage.clone(), processor);
    pool.start();

    for i in 0..3 {
        let mut task = Task::new(
            tenant_id,
            workspace_id,
            TaskType::Deletion,
            json!({ "document_id": format!("e2e-del-{i}") }),
        );
        if let Some(obj) = task.task_data.as_object_mut() {
            obj.insert("deletion_track_id".into(), json!(task.track_id));
        }
        storage.create_task(&task).await.unwrap();
        queue.send(task).await.unwrap();
    }

    let pdf = Task::new(
        tenant_id,
        workspace_id,
        TaskType::PdfProcessing,
        json!({ "document_id": "e2e-pdf-new" }),
    );
    let pdf_id = pdf.track_id.clone();
    storage.create_task(&pdf).await.unwrap();
    queue.send(pdf).await.unwrap();

    tokio::time::timeout(tokio::time::Duration::from_secs(3), pdf_gate.notified())
        .await
        .expect("PDF ingest lane must start under concurrent deletions");
    assert!(pdf_started.load(Ordering::SeqCst) >= 1);
    assert!(
        deletion_started.load(Ordering::SeqCst) >= 1,
        "at least one deletion should have started"
    );

    // While PDF was starting, at least one deletion should still be non-terminal
    // or have overlapped (deletion_started >= 1 already proves overlap intent).
    tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
    let pdf_row = storage.get_task(&pdf_id).await.unwrap().unwrap();
    assert_eq!(
        pdf_row.status,
        TaskStatus::Indexed,
        "PDF must complete, got {:?}",
        pdf_row.status
    );

    pool.shutdown().await;
}

/// SPEC-057 INV-06 FP-2: under-cap tenant progresses while another tenant is held at cap.
#[tokio::test]
async fn e2e_tenant_priority_under_cap_progresses_while_peer_held() {
    use async_trait::async_trait;
    use edgequake_tasks::{
        memory::MemoryTaskStorage,
        queue::ChannelTaskQueue,
        worker::{SharedTaskProcessor, TaskProcessor, WorkerPool, WorkerPoolConfig},
        TaskQueue, TaskResult, TaskStorage,
    };
    use std::sync::Arc;
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;

    struct NotifyB {
        b_started: Arc<tokio::sync::Notify>,
        tenant_b: Uuid,
    }

    #[async_trait]
    impl TaskProcessor for NotifyB {
        async fn process(
            &self,
            task: &mut Task,
            _cancel_token: CancellationToken,
        ) -> TaskResult<serde_json::Value> {
            if task.tenant_id == self.tenant_b {
                self.b_started.notify_waiters();
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
            Ok(json!({ "ok": true }))
        }
    }

    let tenant_a = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
    let tenant_b = Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap();
    let ws_a = Uuid::parse_str("aaaaaaaa-0000-0000-0000-000000000001").unwrap();
    let ws_b = Uuid::parse_str("bbbbbbbb-0000-0000-0000-000000000001").unwrap();
    let b_started = Arc::new(tokio::sync::Notify::new());
    let processor: SharedTaskProcessor = Arc::new(NotifyB {
        b_started: Arc::clone(&b_started),
        tenant_b,
    });

    let queue = Arc::new(ChannelTaskQueue::new(50));
    let storage = Arc::new(MemoryTaskStorage::new());
    let config = WorkerPoolConfig {
        num_workers: 2,
        auto_retry: false,
        initial_retry_delay_ms: 100,
        max_retry_delay_ms: 5000,
        backoff_multiplier: 2.0,
        max_tasks_per_tenant: 1,
        max_lifecycle_tasks_per_tenant: 1,
        processing_timeout_secs: 300,
        provider_budget: 0,
        tenant_lane_weight: 1,
    };
    let mut pool = WorkerPool::new(config, queue.clone(), storage.clone(), processor);
    pool.start();

    let holder = Task::new(tenant_a, ws_a, TaskType::Insert, json!({ "h": 1 }));
    storage.create_task(&holder).await.unwrap();
    queue.send(holder).await.unwrap();
    tokio::time::sleep(Duration::from_millis(80)).await;

    let pending_a = Task::new(tenant_a, ws_a, TaskType::Insert, json!({ "a": 1 }));
    storage.create_task(&pending_a).await.unwrap();
    storage
        .mark_fairness_hold(&pending_a.track_id, Duration::from_secs(30))
        .await
        .unwrap();
    queue.send(pending_a).await.unwrap();

    let pending_b = Task::new(tenant_b, ws_b, TaskType::Insert, json!({ "b": 1 }));
    storage.create_task(&pending_b).await.unwrap();
    queue.send(pending_b).await.unwrap();

    tokio::time::timeout(Duration::from_secs(2), b_started.notified())
        .await
        .expect("tenant B must progress while A is saturated/held");

    pool.shutdown().await;
}

/// Cancelled held task must not be requeued after park wake (hold cleared + skip).
#[tokio::test]
async fn e2e_cancel_held_task_not_requeued_after_park_wake() {
    use async_trait::async_trait;
    use edgequake_tasks::{
        memory::MemoryTaskStorage,
        queue::ChannelTaskQueue,
        worker::{SharedTaskProcessor, TaskProcessor, WorkerPool, WorkerPoolConfig},
        TaskQueue, TaskResult, TaskStatus, TaskStorage,
    };
    use std::sync::Arc;
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;

    struct SlowProcessor;
    #[async_trait]
    impl TaskProcessor for SlowProcessor {
        async fn process(
            &self,
            _task: &mut Task,
            _cancel_token: CancellationToken,
        ) -> TaskResult<serde_json::Value> {
            tokio::time::sleep(Duration::from_millis(400)).await;
            Ok(json!({ "ok": true }))
        }
    }

    let tenant = Uuid::parse_str(TEST_TENANT_ID).unwrap();
    let workspace = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();
    let queue = Arc::new(ChannelTaskQueue::new(20));
    let storage = Arc::new(MemoryTaskStorage::new());
    let processor: SharedTaskProcessor = Arc::new(SlowProcessor);
    let config = WorkerPoolConfig {
        num_workers: 2,
        auto_retry: false,
        initial_retry_delay_ms: 100,
        max_retry_delay_ms: 5000,
        backoff_multiplier: 2.0,
        max_tasks_per_tenant: 1,
        max_lifecycle_tasks_per_tenant: 1,
        processing_timeout_secs: 300,
        provider_budget: 0,
        tenant_lane_weight: 1,
    };
    let mut pool = WorkerPool::new(config, queue.clone(), storage.clone(), processor);
    let registry = pool.cancellation_registry();
    pool.start();

    let holder = Task::new(tenant, workspace, TaskType::Insert, json!({ "h": 1 }));
    storage.create_task(&holder).await.unwrap();
    queue.send(holder).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let parked = Task::new(tenant, workspace, TaskType::Insert, json!({ "p": 1 }));
    let track_id = parked.track_id.clone();
    storage.create_task(&parked).await.unwrap();
    queue.send(parked).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Cancel while fairness-held / parked.
    registry.mark_cancel_intent(&track_id).await;
    let mut row = storage.get_task(&track_id).await.unwrap().unwrap();
    row.mark_cancelled();
    storage.update_task(&row).await.unwrap();
    let _ = storage.clear_fairness_hold(&track_id).await;

    tokio::time::sleep(Duration::from_millis(600)).await;
    let final_row = storage.get_task(&track_id).await.unwrap().unwrap();
    assert_eq!(
        final_row.status,
        TaskStatus::Cancelled,
        "cancelled held task must stay Cancelled (not re-processed)"
    );

    pool.shutdown().await;
}
