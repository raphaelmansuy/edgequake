//! SPEC-017 query production path contract — API routes through SOTA only (P0).
//!
//! Code is law: `query_execution` and Ollama handlers must not call legacy `query_engine`.

use std::path::PathBuf;

fn read_crate_src(rel: &str) -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(manifest.join(rel)).unwrap_or_else(|e| {
        panic!("read {}: {e}", rel);
    })
}

#[test]
fn spec017_query_execution_service_uses_sota_engine_only() {
    let src = read_crate_src("src/services/query_execution.rs");
    assert!(
        src.contains("sota_engine"),
        "execute_sota_query must use sota_engine"
    );
    assert!(
        !src.contains("query_engine"),
        "query_execution must not reference legacy query_engine"
    );
}

#[test]
fn spec017_ollama_handlers_use_sota_engine_only() {
    for rel in [
        "src/handlers/ollama/chat.rs",
        "src/handlers/ollama/generate.rs",
    ] {
        let src = read_crate_src(rel);
        assert!(src.contains("sota_engine"), "{rel} must use sota_engine");
        assert!(
            !src.contains("query_engine"),
            "{rel} must not reference legacy query_engine"
        );
    }
}
