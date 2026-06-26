//! SPEC-021 P-G9 — worker persist invalidates query result cache (production wiring).

use std::time::Duration;

mod common;

use edgequake_query::engine::QueryRequest;
use edgequake_query::QueryMode;

#[tokio::test]
async fn spec021_worker_persist_busts_query_result_cache() {
    let workers = common::create_test_app_with_workers().await;
    let engine = std::sync::Arc::clone(&workers.query_engine);
    let cache = engine
        .result_cache()
        .expect("worker test app must mirror production result cache");

    let mut req = QueryRequest::new("spec021 worker cache bust sarah chen edgequake");
    req.context_only = true;
    req.mode = Some(QueryMode::Hybrid);

    engine.query(req.clone()).await.expect("prime query");
    engine.query(req.clone()).await.expect("cached query");
    assert_eq!(cache.hits(), 1, "second identical query must hit cache");

    let (_doc_id, _track_id, _final_status) = common::upload_and_wait(
        workers.app(),
        "SPEC-021 P-G9 worker cache",
        "Dr. Sarah Chen leads the EdgeQuake research lab in Zurich.",
        Duration::from_secs(60),
    )
    .await;

    let (_status, detail) =
        common::get_endpoint(workers.app(), &format!("/api/v1/documents/{_doc_id}")).await;
    let chunk_count = detail["chunk_count"].as_u64().unwrap_or(0);

    engine.query(req).await.expect("post-upload query");

    if chunk_count >= 1 {
        assert_eq!(
            cache.misses(),
            2,
            "worker persist Ok must invalidate result cache (chunks={chunk_count})"
        );
    }
}
