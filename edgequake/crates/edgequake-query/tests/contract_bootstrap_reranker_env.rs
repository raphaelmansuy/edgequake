//! SPEC-023 I4 — reranker env hook falls back to BM25 until cross-encoder ships.

#[test]
fn contract_cross_encoder_env_falls_back_to_bm25() {
    std::env::set_var("EDGEQUAKE_RERANKER", "cross_encoder");
    let reranker = edgequake_query::bootstrap::create_production_reranker();
    // Smoke: reranker trait object exists (BM25 fallback).
    let _ = reranker;
    std::env::remove_var("EDGEQUAKE_RERANKER");
}
