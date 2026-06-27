//! SPEC-024 2.4 — cross-encoder reranker env resolution via edgequake-llm factory.

#[tokio::test]
async fn contract_cross_encoder_uses_bi_encoder_with_mock_embedding() {
    use std::sync::Arc;

    use edgequake_llm::traits::EmbeddingProvider;
    use edgequake_llm::MockProvider;
    use edgequake_query::create_production_reranker_with_embedding;

    std::env::remove_var("JINA_API_KEY");
    std::env::remove_var("COHERE_API_KEY");
    std::env::remove_var("DASHSCOPE_API_KEY");
    std::env::remove_var("ALIYUN_API_KEY");
    std::env::remove_var("EDGEQUAKE_RERANKER_PROVIDER");
    std::env::set_var("EDGEQUAKE_RERANKER", "cross_encoder");

    let mock = Arc::new(MockProvider::new());
    let reranker =
        create_production_reranker_with_embedding(Some(mock as Arc<dyn EmbeddingProvider>));
    assert_eq!(reranker.name(), "bi-encoder");

    let docs = vec![
        "rust async programming".to_string(),
        "python data science".to_string(),
    ];
    let results = reranker.rerank("rust async", &docs, Some(2)).await.unwrap();
    assert_eq!(results.len(), 2);

    std::env::remove_var("EDGEQUAKE_RERANKER");
}

#[test]
fn contract_cross_encoder_without_embedding_falls_back_to_bm25() {
    let saved = (
        std::env::var("JINA_API_KEY").ok(),
        std::env::var("COHERE_API_KEY").ok(),
        std::env::var("DASHSCOPE_API_KEY").ok(),
        std::env::var("ALIYUN_API_KEY").ok(),
    );
    std::env::remove_var("JINA_API_KEY");
    std::env::remove_var("COHERE_API_KEY");
    std::env::remove_var("DASHSCOPE_API_KEY");
    std::env::remove_var("ALIYUN_API_KEY");
    std::env::remove_var("EDGEQUAKE_RERANKER_PROVIDER");
    std::env::set_var("EDGEQUAKE_RERANKER", "cross_encoder");

    let reranker = edgequake_query::create_production_reranker();
    assert_eq!(reranker.name(), "bm25");

    std::env::remove_var("EDGEQUAKE_RERANKER");
    if let Some(v) = saved.0 {
        std::env::set_var("JINA_API_KEY", v);
    }
    if let Some(v) = saved.1 {
        std::env::set_var("COHERE_API_KEY", v);
    }
    if let Some(v) = saved.2 {
        std::env::set_var("DASHSCOPE_API_KEY", v);
    }
    if let Some(v) = saved.3 {
        std::env::set_var("ALIYUN_API_KEY", v);
    }
}
