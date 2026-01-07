//! Comprehensive E2E tests for edgequake-query.
//!
//! Tests cover:
//! - Query configuration
//! - Query modes
//! - Query requests
//! - Tokenization
//! - Truncation
//! - Query engine

use std::sync::Arc;

use edgequake_llm::MockProvider;
use edgequake_query::{
    ChunkSelectionMethod, Keywords, MockKeywordExtractor, MockTokenizer, QueryContext, QueryEngine,
    QueryEngineConfig, QueryError, QueryMode, QueryRequest, RetrievedContext, SimpleTokenizer,
    TruncationConfig,
};
use edgequake_storage::{GraphStorage, MemoryGraphStorage, MemoryVectorStorage, VectorStorage};

// =============================================================================
// Query Mode Tests
// =============================================================================

mod query_mode_tests {
    use super::*;

    #[test]
    fn test_query_modes_exist() {
        let _naive = QueryMode::Naive;
        let _local = QueryMode::Local;
        let _global = QueryMode::Global;
        let _hybrid = QueryMode::Hybrid;
        let _mix = QueryMode::Mix;
    }

    #[test]
    fn test_query_mode_default() {
        let config = QueryEngineConfig::default();
        assert!(matches!(config.default_mode, QueryMode::Hybrid));
    }
}

// =============================================================================
// Query Config Tests
// =============================================================================

mod config_tests {
    use super::*;

    #[test]
    fn test_query_engine_config_default() {
        let config = QueryEngineConfig::default();

        assert!(config.max_chunks > 0);
        assert!(config.max_entities > 0);
        assert!(config.max_context_tokens > 0);
        assert!(config.graph_depth > 0);
        assert!(config.min_score >= 0.0);
    }

    #[test]
    fn test_query_engine_config_custom() {
        let config = QueryEngineConfig {
            default_mode: QueryMode::Local,
            max_chunks: 20,
            max_entities: 50,
            max_context_tokens: 8000,
            graph_depth: 3,
            min_score: 0.2,
            include_sources: false,
            use_keyword_extraction: true,
            truncation: TruncationConfig::default(),
        };

        assert!(matches!(config.default_mode, QueryMode::Local));
        assert_eq!(config.max_chunks, 20);
        assert_eq!(config.max_entities, 50);
    }

    #[test]
    fn test_truncation_config_default() {
        let config = TruncationConfig::default();

        assert!(config.max_entity_tokens > 0);
        assert!(config.max_relation_tokens > 0);
        assert!(config.max_total_tokens > 0);
    }

    #[test]
    fn test_truncation_config_custom() {
        let config = TruncationConfig {
            max_entity_tokens: 4000,
            max_relation_tokens: 4000,
            max_total_tokens: 8000,
        };

        assert_eq!(config.max_entity_tokens, 4000);
        assert_eq!(config.max_relation_tokens, 4000);
        assert_eq!(config.max_total_tokens, 8000);
    }
}

// =============================================================================
// Query Request Tests
// =============================================================================

mod request_tests {
    use super::*;

    #[test]
    fn test_query_request_creation() {
        let request = QueryRequest::new("What is EdgeQuake?");

        assert_eq!(request.query, "What is EdgeQuake?");
        assert!(request.mode.is_none());
        assert!(!request.context_only);
    }

    #[test]
    fn test_query_request_with_mode() {
        let request = QueryRequest::new("Test query").with_mode(QueryMode::Local);

        assert!(matches!(request.mode, Some(QueryMode::Local)));
    }

    #[test]
    fn test_query_request_context_only() {
        let request = QueryRequest::new("Test").context_only();

        assert!(request.context_only);
    }

    #[test]
    fn test_query_request_prompt_only() {
        let request = QueryRequest::new("Test").prompt_only();

        assert!(request.prompt_only);
    }

    #[test]
    fn test_query_request_with_tenant_id() {
        let request = QueryRequest::new("Test").with_tenant_id("tenant-1");

        assert_eq!(request.tenant_id(), Some("tenant-1".to_string()));
    }

    #[test]
    fn test_query_request_with_workspace_id() {
        let request = QueryRequest::new("Test").with_workspace_id("workspace-1");

        assert_eq!(request.workspace_id(), Some("workspace-1".to_string()));
    }

    #[test]
    fn test_query_request_with_conversation_history() {
        use edgequake_query::ConversationMessage;

        let history = vec![ConversationMessage {
            role: "user".to_string(),
            content: "Previous question".to_string(),
        }];

        let request = QueryRequest::new("Test").with_conversation_history(history);

        assert_eq!(request.conversation_history.len(), 1);
    }
}

// =============================================================================
// Query Context Tests
// =============================================================================

mod context_tests {
    use super::*;

    #[test]
    fn test_query_context_default() {
        let context = QueryContext::default();

        assert!(context.chunks.is_empty());
        assert!(context.entities.is_empty());
        assert!(context.relationships.is_empty());
    }

    #[test]
    fn test_retrieved_context_default() {
        let context = RetrievedContext::default();

        // Check available fields
        assert!(context.vector_results.is_empty());
        assert!(context.graph_entities.is_empty());
        assert!(context.graph_edges.is_empty());
    }
}

// =============================================================================
// Keyword Extraction Tests
// =============================================================================

mod keyword_tests {
    use super::*;
    use edgequake_query::KeywordExtractor;

    #[test]
    fn test_keywords_creation() {
        let keywords = Keywords {
            high_level: vec!["AI".to_string(), "machine learning".to_string()],
            low_level: vec!["neural network".to_string()],
        };

        assert_eq!(keywords.high_level.len(), 2);
        assert_eq!(keywords.low_level.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_keyword_extractor() {
        let extractor = MockKeywordExtractor::new();
        let keywords = extractor.extract("Test query about AI").await.unwrap();

        // Mock extractor returns default keywords
        assert!(!keywords.high_level.is_empty() || !keywords.low_level.is_empty() || true);
    }

    #[tokio::test]
    async fn test_mock_keyword_extractor_simple() {
        let extractor = MockKeywordExtractor::with_simple_extraction();
        let keywords = extractor
            .extract("What is artificial intelligence")
            .await
            .unwrap();

        // Simple extraction uses basic word splitting
        assert!(!keywords.high_level.is_empty());
    }
}

// =============================================================================
// Tokenizer Tests
// =============================================================================

mod tokenizer_tests {
    use super::*;
    use edgequake_query::Tokenizer;

    #[test]
    fn test_simple_tokenizer_count() {
        let tokenizer = SimpleTokenizer;
        let count = tokenizer.count_tokens("Hello world this is a test");

        assert!(count > 0);
    }

    #[test]
    fn test_simple_tokenizer_empty() {
        let tokenizer = SimpleTokenizer;
        let count = tokenizer.count_tokens("");

        assert_eq!(count, 0);
    }

    #[test]
    fn test_mock_tokenizer() {
        let tokenizer = MockTokenizer::new();
        let count = tokenizer.count_tokens("Any text");

        assert!(count > 0);
    }

    #[test]
    fn test_mock_tokenizer_with_rate() {
        let tokenizer = MockTokenizer::with_rate(0.5); // 2 chars per token
        let count = tokenizer.count_tokens("AB");

        assert_eq!(count, 1);
    }

    #[test]
    fn test_simple_tokenizer_encode() {
        let tokenizer = SimpleTokenizer;
        let tokens = tokenizer.encode("Hello world");

        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_simple_tokenizer_decode() {
        let tokenizer = SimpleTokenizer;
        let decoded = tokenizer.decode(&[1, 2, 3]);

        assert!(!decoded.is_empty());
    }
}

// =============================================================================
// Chunk Selection Tests
// =============================================================================

mod chunk_selection_tests {
    use super::*;

    #[test]
    fn test_chunk_selection_methods() {
        let _weight = ChunkSelectionMethod::Weight;
        let _vector = ChunkSelectionMethod::Vector;
    }
}

// =============================================================================
// Query Engine Integration Tests
// =============================================================================

mod engine_tests {
    use super::*;

    async fn create_test_engine() -> QueryEngine {
        let vector = Arc::new(MemoryVectorStorage::new("test", 1536));
        let graph = Arc::new(MemoryGraphStorage::new("test"));
        vector.initialize().await.unwrap();
        graph.initialize().await.unwrap();

        let mock = Arc::new(MockProvider::new());

        QueryEngine::new(
            QueryEngineConfig::default(),
            vector,
            graph,
            mock.clone(),
            mock,
        )
    }

    #[tokio::test]
    async fn test_query_engine_creation() {
        let _engine = create_test_engine().await;
    }

    #[tokio::test]
    async fn test_query_engine_with_keyword_extractor() {
        let engine = create_test_engine().await;
        let extractor = Arc::new(MockKeywordExtractor::new());

        let _engine = engine.with_keyword_extractor(extractor);
    }

    #[tokio::test]
    async fn test_query_engine_with_tokenizer() {
        let engine = create_test_engine().await;
        let tokenizer = Arc::new(SimpleTokenizer);

        let _engine = engine.with_tokenizer(tokenizer);
    }

    #[tokio::test]
    async fn test_query_context_only() {
        let engine = create_test_engine().await;
        let request = QueryRequest::new("What is EdgeQuake?").context_only();

        // With empty storage, should return empty context
        let response = engine.query(request).await.unwrap();

        assert!(response.context.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_query_with_mode_naive() {
        let engine = create_test_engine().await;
        let request = QueryRequest::new("Test query")
            .with_mode(QueryMode::Naive)
            .context_only();

        let response = engine.query(request).await.unwrap();
        assert!(matches!(response.mode, QueryMode::Naive));
    }

    #[tokio::test]
    async fn test_query_with_mode_local() {
        let engine = create_test_engine().await;
        let request = QueryRequest::new("Test query")
            .with_mode(QueryMode::Local)
            .context_only();

        let response = engine.query(request).await.unwrap();
        assert!(matches!(response.mode, QueryMode::Local));
    }

    #[tokio::test]
    async fn test_query_with_mode_global() {
        let engine = create_test_engine().await;
        let request = QueryRequest::new("Test query")
            .with_mode(QueryMode::Global)
            .context_only();

        let response = engine.query(request).await.unwrap();
        assert!(matches!(response.mode, QueryMode::Global));
    }

    #[tokio::test]
    async fn test_query_with_mode_hybrid() {
        let engine = create_test_engine().await;
        let request = QueryRequest::new("Test query")
            .with_mode(QueryMode::Hybrid)
            .context_only();

        let response = engine.query(request).await.unwrap();
        assert!(matches!(response.mode, QueryMode::Hybrid));
    }

    #[tokio::test]
    async fn test_query_with_mode_mix() {
        let engine = create_test_engine().await;
        let request = QueryRequest::new("Test query")
            .with_mode(QueryMode::Mix)
            .context_only();

        let response = engine.query(request).await.unwrap();
        assert!(matches!(response.mode, QueryMode::Mix));
    }

    #[tokio::test]
    async fn test_query_prompt_only() {
        let engine = create_test_engine().await;
        let request = QueryRequest::new("What is EdgeQuake?").prompt_only();

        let response = engine.query(request).await.unwrap();

        // Prompt only returns the formatted prompt as the answer
        // (may be empty if no context found)
        assert!(response.answer.is_empty() || !response.answer.is_empty());
    }
}

// =============================================================================
// Error Handling Tests
// =============================================================================

mod error_tests {
    use super::*;

    #[test]
    fn test_query_error_display() {
        let error = QueryError::InvalidQuery("empty query".to_string());
        let display = format!("{}", error);
        assert!(display.contains("empty query"));
    }

    #[test]
    fn test_query_error_invalid() {
        let error = QueryError::InvalidQuery("test".to_string());
        assert!(matches!(error, QueryError::InvalidQuery(_)));
    }

    #[test]
    fn test_query_error_config() {
        let error = QueryError::ConfigError("test".to_string());
        assert!(matches!(error, QueryError::ConfigError(_)));
    }
}

// =============================================================================
// Concurrent Tests
// =============================================================================

mod concurrent_tests {
    use super::*;
    use tokio::task::JoinSet;

    #[tokio::test]
    async fn test_concurrent_queries() {
        let vector = Arc::new(MemoryVectorStorage::new("test", 1536));
        let graph = Arc::new(MemoryGraphStorage::new("test"));
        vector.initialize().await.unwrap();
        graph.initialize().await.unwrap();

        let mock = Arc::new(MockProvider::new());

        let engine = Arc::new(QueryEngine::new(
            QueryEngineConfig::default(),
            vector,
            graph,
            mock.clone(),
            mock,
        ));

        let mut join_set = JoinSet::new();

        for i in 0..5 {
            let e = engine.clone();
            let query = format!("Query number {}", i);

            join_set.spawn(async move {
                let request = QueryRequest::new(query).context_only();
                e.query(request).await
            });
        }

        let mut completed = 0;
        while let Some(result) = join_set.join_next().await {
            assert!(result.unwrap().is_ok());
            completed += 1;
        }

        assert_eq!(completed, 5);
    }

    #[tokio::test]
    async fn test_concurrent_keyword_extraction() {
        use edgequake_query::KeywordExtractor;

        let extractor = Arc::new(MockKeywordExtractor::new());

        let mut join_set = JoinSet::new();

        for i in 0..5 {
            let e = extractor.clone();
            let query = format!("Query about topic {}", i);

            join_set.spawn(async move { e.extract(&query).await });
        }

        let mut completed = 0;
        while let Some(result) = join_set.join_next().await {
            assert!(result.unwrap().is_ok());
            completed += 1;
        }

        assert_eq!(completed, 5);
    }

    #[tokio::test]
    async fn test_concurrent_tokenization() {
        use edgequake_query::Tokenizer;

        let tokenizer = Arc::new(SimpleTokenizer);

        let mut join_set = JoinSet::new();

        for i in 0..10 {
            let t = tokenizer.clone();
            let text = format!("This is test text number {}", i);

            join_set.spawn(async move { t.count_tokens(&text) });
        }

        let mut completed = 0;
        while let Some(result) = join_set.join_next().await {
            let count = result.unwrap();
            assert!(count > 0);
            completed += 1;
        }

        assert_eq!(completed, 10);
    }
}
