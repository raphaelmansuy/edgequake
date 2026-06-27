//! SPEC-021 P-G13/P-G14/P-G15 E2E contracts — ingest resilience & admission.
//!
//! Memory-mode CI-safe tests proving:
//! - `/live` liveness (P-G13)
//! - Queued PDF document shell idempotency (P-G14)
//! - Single-flight admission via TaskStorage + registry (P-G15)

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::services::{provision_queued_pdf_document_shell, QueuedPdfDocumentShell};
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_core::{CreateWorkspaceRequest, Tenant, TenantPlan};
use edgequake_tasks::{TaskStatus, TaskType};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

fn test_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: false,
    }
}

async fn setup_workspace(state: &AppState, suffix: &str) -> (Uuid, Uuid) {
    let tenant = Tenant::new(format!("Tenant-{}", suffix), format!("tenant-{}", suffix))
        .with_plan(TenantPlan::Pro);
    let tenant = state.workspace_service.create_tenant(tenant).await.unwrap();
    let tenant_id = tenant.tenant_id;
    let ws = state
        .workspace_service
        .create_workspace(
            tenant_id,
            CreateWorkspaceRequest {
                name: format!("WS-{}", suffix),
                slug: None,
                description: None,
                max_documents: None,
                llm_model: None,
                llm_provider: None,
                embedding_model: None,
                embedding_provider: None,
                embedding_dimension: None,
                vision_llm_model: None,
                pdf_parser_backend: None,
                entity_types: None,
                vision_llm_provider: None,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    (ws.workspace_id, tenant_id)
}

#[tokio::test]
async fn spec021_live_endpoint_returns_ok() {
    let state = AppState::test_state();
    let app = Server::new(test_config(), state).build_router();

    let resp = app
        .oneshot(Request::builder().uri("/live").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 16).await.unwrap();
    assert_eq!(body.as_ref(), b"OK");
}

#[tokio::test]
async fn spec021_queued_pdf_shell_is_idempotent() {
    let state = AppState::test_state();
    let (workspace_id, tenant_id) = setup_workspace(&state, "shell").await;
    let pdf_id = Uuid::new_v4();
    let document_id = Uuid::new_v4().to_string();

    let shell = QueuedPdfDocumentShell {
        pdf_id,
        filename: "concurrent-a.pdf".to_string(),
        tenant_id,
        workspace_id,
        track_id: "track-shell-1".to_string(),
        file_size_bytes: 1024,
        sha256_checksum: "abc".to_string(),
        page_count: Some(1),
    };

    provision_queued_pdf_document_shell(&state.storage.kv_storage, &document_id, &shell)
        .await
        .expect("first shell");
    provision_queued_pdf_document_shell(&state.storage.kv_storage, &document_id, &shell)
        .await
        .expect("second shell idempotent");

    let key = edgequake_storage::kv_keys::doc_metadata(&document_id);
    let meta = state
        .storage
        .kv_storage
        .get_by_id(&key)
        .await
        .unwrap()
        .expect("metadata exists");
    assert_eq!(meta.get("status").and_then(|v| v.as_str()), Some("queued"));
}

#[tokio::test]
async fn spec021_single_flight_reuses_active_pdf_task() {
    let state = AppState::test_state();
    let (workspace_id, tenant_id) = setup_workspace(&state, "single-flight").await;
    let pdf_id = Uuid::new_v4();

    let mut task = edgequake_tasks::Task::new(
        tenant_id,
        workspace_id,
        TaskType::PdfProcessing,
        json!({
            "pdf_id": pdf_id,
            "tenant_id": tenant_id,
            "workspace_id": workspace_id,
            "enable_vision": false,
            "vision_provider": "mock",
        }),
    );
    task.status = TaskStatus::Pending;
    let track_id = task.track_id.clone();
    state.tasks.storage.create_task(&task).await.unwrap();

    let found = state
        .tasks
        .storage
        .find_active_pdf_processing_task(pdf_id, workspace_id)
        .await
        .unwrap()
        .expect("active task");
    assert_eq!(found.track_id, track_id);

    state
        .tasks
        .pdf_admission
        .try_register(workspace_id, pdf_id, "other-track");
    let registry_hit = state.tasks.pdf_admission.get(workspace_id, pdf_id);
    assert_eq!(registry_hit.as_deref(), Some("other-track"));
}

#[tokio::test]
async fn spec021_three_queued_shells_list_as_three_documents() {
    let state = AppState::test_state();
    let (workspace_id, tenant_id) = setup_workspace(&state, "three-pdf").await;

    for i in 0..3 {
        let pdf_id = Uuid::new_v4();
        let document_id = Uuid::new_v4().to_string();
        let shell = QueuedPdfDocumentShell {
            pdf_id,
            filename: format!("batch-{}.pdf", i),
            tenant_id,
            workspace_id,
            track_id: format!("track-{}", i),
            file_size_bytes: 2048,
            sha256_checksum: format!("hash-{}", i),
            page_count: Some(2),
        };
        provision_queued_pdf_document_shell(&state.storage.kv_storage, &document_id, &shell)
            .await
            .unwrap();
    }

    let app = Server::new(test_config(), state.clone()).build_router();
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents")
                .header("X-Workspace-ID", workspace_id.to_string())
                .header("X-Tenant-ID", tenant_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();

    let total = body["total"].as_u64().unwrap_or(0);
    assert_eq!(total, 3, "three queued shells must appear in document list");

    let pending = body["status_counts"]["pending"].as_u64().unwrap_or(0);
    assert!(
        pending >= 3,
        "queued documents count toward pending/queued status bucket"
    );
}

#[tokio::test]
async fn spec021_workspace_stats_response_includes_stale_field() {
    let state = AppState::test_state();
    let (workspace_id, tenant_id) = setup_workspace(&state, "stale-field").await;
    let app = Server::new(test_config(), state).build_router();

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{workspace_id}/stats"))
                .header("X-Tenant-ID", tenant_id.to_string())
                .header("X-Workspace-ID", workspace_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();

    assert!(body.get("document_count").is_some());
    assert_eq!(body.get("stale").and_then(|v| v.as_bool()), Some(false));
}
