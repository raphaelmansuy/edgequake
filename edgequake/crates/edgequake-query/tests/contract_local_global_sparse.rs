//! SPEC-024 2.3 — BM25/FTS fusion on local + global KG-linked chunk stages.

use std::sync::Arc;

use edgequake_llm::reranker::BM25Reranker;
use edgequake_query::sparse_retrieval;
use edgequake_query::QueryEngineConfig;
use edgequake_storage::traits::VectorSearchResult;
use edgequake_storage::traits::VectorStorage;
use edgequake_storage::MemoryVectorStorage;

/// KG-linked chunk candidates (local/global path) must use the same sparse fusion as naive.
#[tokio::test]
async fn contract_kg_linked_chunks_bm25_reorders_vector_leaders() {
    // Simulates query_filtered results after source_chunk_id collection (local/global step 7).
    let kg_linked_results = vec![
        VectorSearchResult {
            id: "chunk-cosine-leader".to_string(),
            score: 0.98,
            metadata: serde_json::json!({
                "type": "chunk",
                "content": "General partnership overview between two organizations"
            }),
        },
        VectorSearchResult {
            id: "chunk-sparse-match".to_string(),
            score: 0.52,
            metadata: serde_json::json!({
                "type": "chunk",
                "content": "Invoice reference INV-2027-XY999 due upon delivery"
            }),
        },
    ];

    let reranker = BM25Reranker::new_enhanced();
    let config = QueryEngineConfig::default();
    let storage: Arc<dyn VectorStorage> = Arc::new(MemoryVectorStorage::new("kg-sparse", 4));

    let chunks = sparse_retrieval::fuse_vector_and_bm25_chunks(
        "INV-2027-XY999",
        &kg_linked_results,
        &storage,
        None,
        Some(&reranker),
        None,
        &config,
    )
    .await;

    assert_eq!(
        chunks.first().map(|c| c.id.as_str()),
        Some("chunk-sparse-match"),
        "local/global sparse fusion must promote lexical match over cosine leader"
    );
}

#[test]
fn contract_local_global_wire_sparse_fusion_in_modes() {
    let chunk_retrieval = include_str!("../src/engine_impl/modes/chunk_retrieval.rs");
    assert!(
        chunk_retrieval.contains("fuse_vector_and_bm25_chunks"),
        "shared local/global chunk path must fuse KG-linked chunks with BM25/FTS"
    );

    let local = include_str!("../src/engine_impl/modes/local.rs");
    let global = include_str!("../src/engine_impl/modes/global.rs");
    assert!(
        local.contains("append_score_ranked_chunks"),
        "local mode must use shared chunk retrieval helper"
    );
    assert!(
        global.contains("append_score_ranked_chunks"),
        "global mode must use shared chunk retrieval helper"
    );
}
