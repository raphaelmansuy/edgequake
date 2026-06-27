//! Comprehensive E2E tests for edgequake-query.
//!
//! Tests cover:
//! - Query configuration
//! - Query modes
//! - Query requests
//! - Tokenization
//! - Truncation
//! - Error handling
//! - Concurrency (keyword extraction, tokenization)

use std::sync::Arc;

use edgequake_query::{
    Keywords, MockKeywordExtractor, MockTokenizer, QueryContext, QueryEngineConfig, QueryError,
    QueryMode, QueryRequest, RetrievedContext, SimpleTokenizer, TruncationConfig,
};

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
        assert!(matches!(config.default_mode, QueryMode::Mix));
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
            max_relationships: 40,
            max_context_tokens: 8000,
            graph_depth: 3,
            min_score: 0.2,
            use_keyword_extraction: true,
            use_adaptive_mode: false,
            truncation: TruncationConfig::default(),
            keyword_cache_ttl_secs: 3600,
            enable_rerank: false,
            min_rerank_score: 0.1,
            rerank_top_k: 10,
            ..Default::default()
        };

        assert!(matches!(config.default_mode, QueryMode::Local));
        assert_eq!(config.max_chunks, 20);
        assert_eq!(config.max_entities, 50);
        assert!(!config.enable_rerank);
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
        assert!(!keywords.high_level.is_empty() || !keywords.low_level.is_empty());
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
