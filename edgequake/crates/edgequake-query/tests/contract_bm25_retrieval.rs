//! SPEC-023 I10 — BM25 sparse retrieval: Postgres FTS primary, in-memory fallback.

use std::sync::Arc;

use edgequake_llm::reranker::BM25Reranker;
use edgequake_query::fusion::MixFusionMode;
use edgequake_query::sparse_retrieval;
use edgequake_query::QueryEngineConfig;
use edgequake_storage::traits::VectorSearchResult;
use edgequake_storage::traits::VectorStorage;
use edgequake_storage::MemoryVectorStorage;

#[tokio::test]
async fn contract_in_memory_bm25_fallback_promotes_exact_token_match() {
    let vector_results = vec![
        VectorSearchResult {
            id: "chunk_vector_top".to_string(),
            score: 0.99,
            metadata: serde_json::json!({
                "type": "chunk",
                "content": "Generic overview of inventory management systems"
            }),
        },
        VectorSearchResult {
            id: "chunk_bm25_top".to_string(),
            score: 0.55,
            metadata: serde_json::json!({
                "type": "chunk",
                "content": "Warranty registration for product SKU-XY999 expires in 2027"
            }),
        },
    ];

    let reranker = BM25Reranker::new_enhanced();
    let config = QueryEngineConfig::default();
    let storage: Arc<dyn VectorStorage> = Arc::new(MemoryVectorStorage::new("bm25-fallback", 4));

    assert!(
        !storage.supports_native_text_search(),
        "memory adapter must use in-memory BM25 fallback"
    );

    let chunks = sparse_retrieval::fuse_vector_and_bm25_chunks(
        "SKU-XY999",
        &vector_results,
        &storage,
        None,
        Some(&reranker),
        None,
        &config,
    )
    .await;

    assert!(
        !chunks.is_empty(),
        "BM25 retrieval must return at least one chunk"
    );
    assert_eq!(
        chunks[0].id, "chunk_bm25_top",
        "in-memory BM25 fusion must rank exact token match above pure vector leader"
    );
}

#[test]
fn contract_sparse_fusion_env_modes() {
    std::env::remove_var("EDGEQUAKE_SPARSE_FUSION");
    assert_eq!(
        sparse_retrieval::sparse_fusion_mode_from_env(),
        MixFusionMode::Weighted
    );

    std::env::set_var("EDGEQUAKE_SPARSE_FUSION", "rrf");
    assert_eq!(
        sparse_retrieval::sparse_fusion_mode_from_env(),
        MixFusionMode::Rrf
    );
    std::env::remove_var("EDGEQUAKE_SPARSE_FUSION");
}

#[test]
fn contract_bm25_retrieval_default_and_env_opt_out() {
    std::env::remove_var("EDGEQUAKE_BM25_RETRIEVAL");
    let config = QueryEngineConfig::default();
    assert!(
        sparse_retrieval::bm25_retrieval_enabled(&config),
        "BM25 retrieval must be on by default"
    );

    std::env::set_var("EDGEQUAKE_BM25_RETRIEVAL", "false");
    assert!(
        !sparse_retrieval::bm25_retrieval_enabled(&config),
        "EDGEQUAKE_BM25_RETRIEVAL=false must disable sparse retrieval"
    );
    std::env::remove_var("EDGEQUAKE_BM25_RETRIEVAL");
}
