//! SPEC-021 P-G9 — query result cache invalidation wiring.

use std::sync::Arc;

use edgequake_api::AppState;
use edgequake_query::engine::QueryRequest;
use edgequake_query::QueryMode;

#[tokio::test]
async fn spec021_query_cache_invalidates_on_engine_bump() {
    let state = AppState::test_state();
    let engine = Arc::clone(&state.query.engine_impl);
    let cache = engine
        .result_cache()
        .expect("production engine has result cache");

    let mut req = QueryRequest::new("repeatable context query");
    req.context_only = true;
    req.mode = Some(QueryMode::Hybrid);

    engine.query(req.clone()).await.expect("first query");
    assert_eq!(cache.misses(), 1);

    engine.query(req.clone()).await.expect("cached query");
    assert_eq!(cache.hits(), 1);

    engine.invalidate_result_cache();

    engine.query(req).await.expect("post-invalidation query");
    assert_eq!(
        cache.misses(),
        2,
        "invalidate_result_cache must force a retrieval miss"
    );
}

#[test]
fn spec021_test_state_builds_engine_with_result_cache() {
    let state = AppState::test_state();
    assert!(
        state.query.engine_impl.result_cache().is_some(),
        "test_state must mirror production query bootstrap (embedding + result cache)"
    );
}
