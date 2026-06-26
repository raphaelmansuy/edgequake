//! SPEC-017 query pipeline contract — single `run_query_pipeline` path (P0/P1).
//!
//! Proves: all QueryMode variants execute; default vs workspace vector storage share
//! retrieval semantics after QUERY-DRY-004 delegation.

use std::sync::Arc;

use edgequake_llm::MockProvider;
use edgequake_query::{
    modes::QueryMode, QueryContext, QueryEngine, QueryEngineConfig, QueryRequest,
};
use edgequake_storage::traits::VectorStorage;
use edgequake_storage::{MemoryGraphStorage, MemoryVectorStorage};

async fn setup_engine_with_chunk() -> QueryEngine {
    let vector_storage = Arc::new(MemoryVectorStorage::new("spec017", 1536));
    vector_storage.initialize().await.expect("init vs");
    // MockProvider::embed_one returns default [0.1; 1536] on first pop — seed chunk to match.
    let chunk_vec = vec![0.1f32; 1536];
    vector_storage
        .upsert(&[(
            "chunk_spec017".to_string(),
            chunk_vec,
            serde_json::json!({"type": "chunk", "content": "SPEC-017 contract chunk"}),
        )])
        .await
        .expect("seed chunk");

    let graph_storage = Arc::new(MemoryGraphStorage::new("spec017"));
    let provider = Arc::new(MockProvider::new());

    QueryEngine::with_mock_keywords(
        QueryEngineConfig {
            use_keyword_extraction: false,
            default_mode: QueryMode::Naive,
            ..Default::default()
        },
        vector_storage,
        graph_storage,
        provider.clone(),
        provider,
    )
}

#[tokio::test]
async fn spec017_pipeline_naive_mode_returns_chunks() {
    let engine = setup_engine_with_chunk().await;
    let request = QueryRequest::new("What is SPEC-017?").with_mode(QueryMode::Naive);
    let response = engine.query(request).await.expect("pipeline query");
    assert!(
        !response.context.chunks.is_empty(),
        "naive mode must return chunks via unified pipeline"
    );
}

#[tokio::test]
async fn spec017_pipeline_bypass_skips_retrieval() {
    let engine = setup_engine_with_chunk().await;
    let request = QueryRequest::new("direct").with_mode(QueryMode::Bypass);
    let response = engine.query(request).await.expect("bypass query");
    assert_eq!(response.mode, QueryMode::Bypass);
    assert!(response.context.chunks.is_empty());
}

#[tokio::test]
async fn spec017_pipeline_context_only_no_answer() {
    let engine = setup_engine_with_chunk().await;
    let request = QueryRequest::new("context only?")
        .with_mode(QueryMode::Naive)
        .context_only();
    let response = engine.query(request).await.expect("context_only");
    assert!(response.answer.is_empty());
    assert!(!response.context.chunks.is_empty());
}

#[tokio::test]
async fn spec017_workspace_vector_storage_matches_default() {
    let engine = setup_engine_with_chunk().await;
    let vs = engine.default_vector_storage();

    let request = QueryRequest::new("workspace parity")
        .with_mode(QueryMode::Naive)
        .context_only();

    let default_ctx = engine.get_context(&request).await.unwrap().0;
    let workspace_ctx = engine
        .query_with_vector_storage(request, vs)
        .await
        .unwrap()
        .context;

    assert_eq!(
        default_ctx.chunks.len(),
        workspace_ctx.chunks.len(),
        "default and workspace paths must share vector_queries implementation"
    );
}

#[tokio::test]
async fn spec017_pipeline_all_modes_execute() {
    let engine = setup_engine_with_chunk().await;
    let modes = [
        QueryMode::Naive,
        QueryMode::Local,
        QueryMode::Global,
        QueryMode::Hybrid,
        QueryMode::Mix,
    ];

    for mode in modes {
        let request = QueryRequest::new("mode sweep")
            .with_mode(mode)
            .context_only();
        let response = engine
            .query(request)
            .await
            .unwrap_or_else(|e| panic!("mode {:?} failed: {e}", mode));
        assert_eq!(response.mode, mode);
        let _ctx: QueryContext = response.context;
    }
}
