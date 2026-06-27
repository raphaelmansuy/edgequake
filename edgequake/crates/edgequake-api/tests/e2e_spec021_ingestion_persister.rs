//! SPEC-021 P-G2c E2E — worker upload → chunks + graph persisted (production path).

use std::time::Duration;

mod common;

#[tokio::test]
async fn spec021_worker_upload_produces_chunks_and_graph_on_success() {
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let (doc_id, _track_id, final_status) = common::upload_and_wait(
        app,
        "SPEC-021 P-G2c",
        "Dr. Sarah Chen leads the EdgeQuake research lab in Zurich.",
        Duration::from_secs(60),
    )
    .await;

    assert!(!doc_id.is_empty());

    let (_status, detail) = common::get_endpoint(app, &format!("/api/v1/documents/{doc_id}")).await;
    let chunk_count = detail["chunk_count"].as_u64().unwrap_or(0);
    assert!(
        chunk_count >= 1,
        "worker upload must produce chunks (status={final_status})"
    );

    assert_eq!(
        final_status, "completed",
        "seeded mock extraction must yield completed status, not partial_failure"
    );

    use edgequake_storage::EntityId;
    let node_id = EntityId::new("Sarah Chen").as_graph_node_id().to_string();
    assert!(
        workers
            .graph_storage
            .get_node(&node_id)
            .await
            .expect("graph read")
            .is_some(),
        "completed worker upload must persist SARAH_CHEN via P-G2 batch merge"
    );
}
