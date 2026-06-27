//! SPEC-022 P-H5 — Postgres worker upload → graph node (production UNNEST path).

#![cfg(feature = "postgres")]

mod common;

use std::sync::Arc;
use std::time::Duration;

use edgequake_storage::EntityId;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn spec022_postgres_worker_upload_persists_graph() {
    common::clear_provider_detection_env();
    std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
    std::env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "mock");
    std::env::set_var("EDGEQUAKE_ALLOW_TEST_PROVIDER_OVERRIDE", "1");

    let mock = Arc::new(edgequake_llm::MockProvider::new());
    for _ in 0..32 {
        mock.add_response(common::SPEC021_WORKER_EXTRACTION_JSON)
            .await;
    }
    edgequake_api::safety_limits::set_test_provider_override(
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
    );

    let Some(url) = common::spec013_postgres::try_database_url() else {
        eprintln!("SKIP spec022_postgres_worker_upload_persists_graph: DATABASE_URL not set");
        return;
    };

    if !postgres_reachable(&url).await {
        eprintln!("SKIP spec022_postgres_worker_upload_persists_graph: postgres not reachable");
        return;
    }

    let mut state = edgequake_api::AppState::new_postgres(&url, "")
        .await
        .expect("postgres app state");

    let graph_storage = Arc::clone(&state.storage.graph_storage);
    common::spec013_postgres::start_worker_pool(&mut state).await;

    let config = edgequake_api::ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    };
    let app = edgequake_api::Server::new(config, state).build_router();
    common::spec013_postgres::wait_until_app_ready(&app).await;

    let (doc_id, _track_id, final_status) = common::upload_and_wait(
        &app,
        "SPEC-022 P-H5",
        "Dr. Sarah Chen leads the EdgeQuake research lab in Zurich.",
        Duration::from_secs(120),
    )
    .await;

    assert!(!doc_id.is_empty());
    assert_eq!(
        final_status, "completed",
        "postgres worker ingest must complete (status={final_status})"
    );

    let node_id = EntityId::new("Sarah Chen").as_graph_node_id().to_string();
    assert!(
        graph_storage
            .get_node(&node_id)
            .await
            .expect("graph read")
            .is_some(),
        "postgres worker path must persist SARAH_CHEN via batched persister"
    );

    edgequake_api::safety_limits::clear_test_provider_override();
}

async fn postgres_reachable(url: &str) -> bool {
    tokio::time::timeout(Duration::from_secs(3), sqlx::PgPool::connect(url))
        .await
        .ok()
        .and_then(Result::ok)
        .is_some()
}
