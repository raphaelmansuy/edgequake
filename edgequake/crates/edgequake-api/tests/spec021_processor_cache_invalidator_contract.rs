//! SPEC-021 P-G9 — processor depends on cache invalidator port (DIP), not concrete engine.

#[test]
fn spec021_processor_wires_query_cache_invalidator_trait() {
    let mod_src = include_str!("../src/processor/mod.rs");
    let persist_src = include_str!("../src/processor/text_insert/persist.rs");

    assert!(
        mod_src.contains("QueryResultCacheInvalidator"),
        "processor must depend on QueryResultCacheInvalidator port"
    );
    assert!(
        mod_src.contains("with_query_cache_invalidator"),
        "processor must expose explicit invalidator wiring"
    );
    assert!(
        persist_src.contains("invalidate_query_result_cache"),
        "worker persist must invalidate via trait method"
    );
    assert!(
        !mod_src.contains("query_engine: Option"),
        "processor must not hold concrete QueryEngine for cache invalidation"
    );
}
