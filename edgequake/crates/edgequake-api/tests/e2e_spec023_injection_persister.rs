//! SPEC-023 I1 — injection ingest uses IngestionPersister (not inline merger).

mod common;

use std::sync::Arc;
use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_llm::MockProvider;
use edgequake_storage::traits::GraphStorageReadOps;
use edgequake_storage::EntityId;
use serde_json::json;
use tower::ServiceExt;

async fn wait_for_injection_completed(app: &axum::Router, workspace_id: &str, injection_id: &str) {
    for _ in 0..40 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/v1/workspaces/{workspace_id}/injections/{injection_id}"
                    ))
                    .header("X-Workspace-ID", workspace_id)
                    .header("X-Tenant-ID", common::TEST_TENANT_ID)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        if response.status() == StatusCode::OK {
            let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
                .await
                .unwrap();
            let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            match parsed["status"].as_str() {
                Some("completed") => return,
                Some("failed") => panic!(
                    "injection failed: {}",
                    parsed["error"].as_str().unwrap_or("unknown")
                ),
                _ => {}
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("injection did not reach completed status in time");
}

#[tokio::test]
async fn spec023_injection_persists_graph_via_shared_persister() {
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

    let workspace_id = common::TEST_WORKSPACE_ID;
    let body = json!({
        "name": "spec023-injection",
        "content": "Dr. Sarah Chen leads the EdgeQuake research lab in Zurich for injection testing."
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/workspaces/{workspace_id}/injection"))
                .header("Content-Type", "application/json")
                .header("X-Workspace-ID", workspace_id)
                .header("X-Tenant-ID", common::TEST_TENANT_ID)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .expect("injection put");

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let injection_id = parsed["injection_id"]
        .as_str()
        .expect("injection_id")
        .to_string();

    wait_for_injection_completed(&app, workspace_id, &injection_id).await;

    let node_id = EntityId::new("Sarah Chen").as_graph_node_id().to_string();
    assert!(
        state
            .storage
            .graph_storage
            .get_node(&node_id)
            .await
            .expect("graph read")
            .is_some(),
        "injection must persist SARAH_CHEN via IngestionPersister + merger"
    );
}

#[test]
fn spec023_injection_has_no_inline_merger() {
    let src = include_str!("../src/handlers/injection.rs");
    assert!(
        !src.contains("KnowledgeGraphMerger::new"),
        "injection must delegate persistence to IngestionPersister"
    );
    assert!(
        src.contains("persist_ingestion_result"),
        "injection must call shared persist service"
    );
    assert!(
        src.contains("tag_injection_sources"),
        "injection must tag sources via shared helper"
    );
}
