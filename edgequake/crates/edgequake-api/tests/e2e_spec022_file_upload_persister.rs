//! SPEC-022 P-H1 — sync file upload uses IngestionPersister (not inline graph loops).

mod common;

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_llm::MockProvider;
use edgequake_storage::traits::GraphStorageReadOps;
use edgequake_storage::EntityId;
use tower::ServiceExt;

#[tokio::test]
async fn spec022_sync_file_upload_persists_graph_via_merger() {
    std::env::set_var("EDGEQUAKE_ALLOW_TEST_PROVIDER_OVERRIDE", "1");
    let mock = Arc::new(MockProvider::new());
    for _ in 0..16 {
        mock.add_response(common::SPEC021_WORKER_EXTRACTION_JSON)
            .await;
    }

    let mut state = edgequake_api::AppState::build_test_state(mock.clone());
    edgequake_api::safety_limits::set_test_provider_override(
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
    );
    state.workspace_service.seed_default_workspace().await;

    let config = edgequake_api::ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    };
    let app = edgequake_api::Server::new(config, state.clone()).build_router();

    let boundary = "----Spec022Boundary";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"spec022.txt\"\r\n\
         Content-Type: text/plain\r\n\r\n\
         Dr. Sarah Chen leads the EdgeQuake research lab in Zurich.\r\n\
         --{boundary}--\r\n"
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/upload")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header("X-Workspace-ID", common::TEST_WORKSPACE_ID)
                .header("X-Tenant-ID", common::TEST_TENANT_ID)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("upload oneshot");

    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "sync upload must succeed when persist path is healthy"
    );

    let node_id = EntityId::new("Sarah Chen").as_graph_node_id().to_string();
    assert!(
        state
            .storage
            .graph_storage
            .get_node(&node_id)
            .await
            .expect("graph read")
            .is_some(),
        "sync file upload must persist SARAH_CHEN via IngestionPersister + merger"
    );
}

#[test]
fn spec022_file_upload_has_no_inline_upsert_node() {
    let src = include_str!("../src/handlers/documents/upload/file_upload.rs");
    assert!(
        !src.contains("upsert_node"),
        "file_upload must delegate graph writes to IngestionPersister"
    );
    assert!(
        !src.contains("upsert_edge"),
        "file_upload must not inline relationship graph writes"
    );
    assert!(
        src.contains("persist_ingestion_result"),
        "file_upload must call shared persist service"
    );
}

#[test]
fn spec022_batch_upload_uses_persister() {
    let src = include_str!("../src/handlers/documents/upload/batch_upload.rs");
    assert!(
        src.contains("persist_ingestion_result"),
        "batch_upload must use shared IngestionPersister path"
    );
    assert!(
        !src.contains("vector_storage\n                .upsert"),
        "batch_upload must not loop chunk upserts on global vector storage"
    );
}
