//! SPEC-023 I1 — injection ingest uses IngestionPersister via worker queue (SPEC-024 1.2).

mod common;

use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_storage::EntityId;
use serde_json::json;
use tower::ServiceExt;

async fn wait_for_injection_completed(app: &axum::Router, injection_id: &str) {
    for _ in 0..80 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/v1/workspaces/default/injections/{injection_id}"
                    ))
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
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let body = json!({
        "name": "spec023-injection",
        "content": "Dr. Sarah Chen leads the EdgeQuake research lab in Zurich for injection testing."
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/workspaces/default/injection")
                .header("Content-Type", "application/json")
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

    wait_for_injection_completed(app, &injection_id).await;

    let node_id = EntityId::new("Sarah Chen").as_graph_node_id().to_string();
    assert!(
        workers
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
    let crud_src = include_str!("../src/handlers/injection/crud.rs");
    let file_src = include_str!("../src/handlers/injection/injection_file.rs");
    let handler_src = format!("{crud_src}\n{file_src}");
    assert!(
        !handler_src.contains("KnowledgeGraphMerger::new"),
        "injection handler must not inline merger"
    );
    assert!(
        handler_src.contains("enqueue_injection_processing"),
        "injection must enqueue worker task (SPEC-024 1.2)"
    );
    assert!(
        !handler_src.contains("tokio::spawn(async move"),
        "injection must not spawn inline background task"
    );

    let worker_src = include_str!("../src/processor/injection_processing.rs");
    assert!(
        worker_src.contains("run_injection_pipeline"),
        "worker must call shared injection persist service"
    );
}
