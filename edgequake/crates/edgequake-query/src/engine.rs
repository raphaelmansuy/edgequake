//! Query engine implementation.

use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use serde::{Deserialize, Serialize};

use crate::context::{QueryContext, RetrievedChunk, RetrievedEntity, RetrievedRelationship};
use crate::error::{QueryError, Result};
use crate::keywords::KeywordExtractor;
use crate::modes::QueryMode;
use crate::tokenizer::{SimpleTokenizer, Tokenizer};
use crate::truncation::{balance_context, TruncationConfig};

use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_storage::traits::{GraphStorage, VectorStorage};

/// Configuration for the query engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryEngineConfig {
    /// Default query mode.
    pub default_mode: QueryMode,

    /// Maximum number of chunks to retrieve.
    pub max_chunks: usize,

    /// Maximum number of entities to retrieve.
    pub max_entities: usize,

    /// Maximum context tokens.
    pub max_context_tokens: usize,

    /// Graph traversal depth.
    pub graph_depth: usize,

    /// Minimum similarity score threshold.
    pub min_score: f32,

    /// Whether to include sources in the response.
    pub include_sources: bool,

    /// Whether to use keyword extraction.
    pub use_keyword_extraction: bool,

    /// Token-based truncation configuration.
    pub truncation: TruncationConfig,
}

impl Default for QueryEngineConfig {
    fn default() -> Self {
        Self {
            default_mode: QueryMode::Hybrid,
            max_chunks: 10,
            max_entities: 20,
            max_context_tokens: 4000,
            graph_depth: 2,
            min_score: 0.1,
            include_sources: true,
            use_keyword_extraction: false,
            truncation: TruncationConfig::default(),
        }
    }
}

/// A query request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    /// The query text.
    pub query: String,

    /// Query mode override.
    pub mode: Option<QueryMode>,

    /// Maximum results.
    pub max_results: Option<usize>,

    /// Whether to only retrieve context (no LLM generation).
    pub context_only: bool,

    /// Additional parameters.
    pub params: HashMap<String, serde_json::Value>,
}

impl QueryRequest {
    /// Create a new query request.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            mode: None,
            max_results: None,
            context_only: false,
            params: HashMap::new(),
        }
    }

    /// Set the query mode.
    pub fn with_mode(mut self, mode: QueryMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Set context-only mode.
    pub fn context_only(mut self) -> Self {
        self.context_only = true;
        self
    }
}

/// A query response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    /// The generated answer.
    pub answer: String,

    /// Query context used for the answer.
    pub context: QueryContext,

    /// Query mode used.
    pub mode: QueryMode,

    /// Processing statistics.
    pub stats: QueryStats,
}

/// Query processing statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryStats {
    /// Time for embedding generation (ms).
    pub embedding_time_ms: u64,

    /// Time for retrieval (ms).
    pub retrieval_time_ms: u64,

    /// Time for LLM generation (ms).
    pub generation_time_ms: u64,

    /// Total time (ms).
    pub total_time_ms: u64,

    /// Number of tokens in the context.
    pub context_tokens: usize,

    /// Number of tokens generated.
    pub generated_tokens: usize,
}

/// The query engine for RAG.
pub struct QueryEngine {
    config: QueryEngineConfig,
    vector_storage: Arc<dyn VectorStorage>,
    graph_storage: Arc<dyn GraphStorage>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    llm_provider: Arc<dyn LLMProvider>,
    keyword_extractor: Option<Arc<dyn KeywordExtractor>>,
    tokenizer: Arc<dyn Tokenizer>,
}

impl QueryEngine {
    /// Create a new query engine.
    pub fn new(
        config: QueryEngineConfig,
        vector_storage: Arc<dyn VectorStorage>,
        graph_storage: Arc<dyn GraphStorage>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        llm_provider: Arc<dyn LLMProvider>,
    ) -> Self {
        Self {
            config,
            vector_storage,
            graph_storage,
            embedding_provider,
            llm_provider,
            keyword_extractor: None,
            tokenizer: Arc::new(SimpleTokenizer),
        }
    }

    /// Set a custom keyword extractor.
    pub fn with_keyword_extractor(mut self, extractor: Arc<dyn KeywordExtractor>) -> Self {
        self.keyword_extractor = Some(extractor);
        self
    }

    /// Set a custom tokenizer.
    pub fn with_tokenizer(mut self, tokenizer: Arc<dyn Tokenizer>) -> Self {
        self.tokenizer = tokenizer;
        self
    }

    /// Execute a query.
    pub async fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
        let start = std::time::Instant::now();
        let mut stats = QueryStats::default();

        let mode = request.mode.unwrap_or(self.config.default_mode);

        // Step 1: Generate query embedding
        let embed_start = std::time::Instant::now();
        let query_embedding = self
            .embedding_provider
            .embed_one(&request.query)
            .await?;
        stats.embedding_time_ms = embed_start.elapsed().as_millis() as u64;

        // Step 2: Retrieve context based on mode
        let retrieval_start = std::time::Instant::now();
        let context = self.retrieve_context(&request.query, &query_embedding, mode).await?;
        stats.retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;
        stats.context_tokens = context.token_count;

        // Step 3: Generate answer (if not context-only)
        let answer = if request.context_only {
            String::new()
        } else {
            let gen_start = std::time::Instant::now();
            let answer = self.generate_answer(&request.query, &context).await?;
            stats.generation_time_ms = gen_start.elapsed().as_millis() as u64;
            answer
        };

        stats.total_time_ms = start.elapsed().as_millis() as u64;

        Ok(QueryResponse {
            answer,
            context,
            mode,
            stats,
        })
    }

    /// Execute a streaming query.
    pub async fn query_stream(&self, request: QueryRequest) -> Result<futures::stream::BoxStream<'static, Result<String>>> {
        let mode = request.mode.unwrap_or(self.config.default_mode);

        // Step 1: Generate query embedding
        let query_embedding = self
            .embedding_provider
            .embed_one(&request.query)
            .await?;

        // Step 2: Retrieve context based on mode
        let context = self.retrieve_context(&request.query, &query_embedding, mode).await?;

        if context.is_empty() {
            use futures::StreamExt;
            return Ok(futures::stream::once(async { 
                Ok("I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string()) 
            }).boxed());
        }

        // Step 3: Generate streaming answer
        let context_text = context.to_context_string();

        let prompt = format!(
            r#"You are a helpful assistant. Answer the user's question based on the following context.

## Context
{context_text}

## Question
{query}

## Answer
Provide a clear, accurate answer based on the context above. If the context doesn't contain enough information to answer the question, say so."#,
            context_text = context_text,
            query = request.query
        );

        self.llm_provider
            .stream(&prompt)
            .await
            .map(|stream| stream.map(|res| res.map_err(QueryError::from)).boxed())
            .map_err(QueryError::from)
    }

    /// Retrieve context for a query.
    async fn retrieve_context(
        &self,
        _query: &str,
        query_embedding: &[f32],
        mode: QueryMode,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();

        // Vector search for chunks
        if mode.uses_vector_search() {
            let results = self
                .vector_storage
                .query(query_embedding, self.config.max_chunks, None)
                .await?;

            for result in results {
                if result.score >= self.config.min_score {
                    let content = result
                        .metadata
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    context.add_chunk(RetrievedChunk::new(&result.id, content, result.score));
                }
            }
        }

        // Graph search for entities and relationships
        if mode.uses_graph() {
            // Get top entities by popularity
            let popular = self
                .graph_storage
                .get_popular_labels(self.config.max_entities)
                .await?;

            for entity_id in popular.iter().take(self.config.max_entities) {
                if let Some(node) = self.graph_storage.get_node(entity_id).await? {
                    let entity_type = node
                        .properties
                        .get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("UNKNOWN")
                        .to_string();

                    let description = node
                        .properties
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let degree = self.graph_storage.node_degree(entity_id).await?;

                    context.add_entity(
                        RetrievedEntity::new(&node.id, entity_type, description)
                            .with_degree(degree),
                    );

                    // Get relationships
                    let edges = self.graph_storage.get_node_edges(entity_id).await?;
                    for edge in edges.iter().take(5) {
                        let rel_type = edge
                            .properties
                            .get("relation_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("RELATED_TO")
                            .to_string();

                        context.add_relationship(RetrievedRelationship::new(
                            &edge.source,
                            &edge.target,
                            rel_type,
                        ));
                    }
                }
            }
        }

        // Apply truncation to ensure we don't exceed token limits
        let (truncated_entities, truncated_relationships, truncated_chunks) = balance_context(
            context.entities.clone(),
            context.relationships.clone(),
            context.chunks.clone(),
            &self.config.truncation,
            self.tokenizer.as_ref(),
        );

        context.entities = truncated_entities;
        context.relationships = truncated_relationships;
        context.chunks = truncated_chunks;

        Ok(context)
    }

    /// Generate an answer using the LLM.
    async fn generate_answer(&self, query: &str, context: &QueryContext) -> Result<String> {
        if context.is_empty() {
            return Ok("I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string());
        }

        let context_text = context.to_context_string();

        let prompt = format!(
            r#"You are a helpful assistant. Answer the user's question based on the following context.

## Context
{context_text}

## Question
{query}

## Answer
Provide a clear, accurate answer based on the context above. If the context doesn't contain enough information to answer the question, say so."#
        );

        let response = self.llm_provider.complete(&prompt).await?;

        Ok(response.content)
    }

    /// Get the engine configuration.
    pub fn config(&self) -> &QueryEngineConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_request_builder() {
        let request = QueryRequest::new("What is Rust?")
            .with_mode(QueryMode::Local)
            .context_only();

        assert_eq!(request.query, "What is Rust?");
        assert_eq!(request.mode, Some(QueryMode::Local));
        assert!(request.context_only);
    }

    #[test]
    fn test_query_engine_config_default() {
        let config = QueryEngineConfig::default();

        assert_eq!(config.default_mode, QueryMode::Hybrid);
        assert_eq!(config.max_chunks, 10);
        assert!(config.include_sources);
    }
}
