//! Query engine implementation.
//!
//! ## Implements
//!
//! - **FEAT0007**: Multi-Mode Query Execution orchestration
//! - **FEAT0111**: Query engine configuration management
//! - **FEAT0112**: Retrieval pipeline coordination
//!
//! ## Use Cases
//!
//! - **UC2210**: System executes query with configured mode
//! - **UC2211**: System combines vector and graph results
//! - **UC2212**: System generates LLM response with context
//!
//! ## Enforces
//!
//! - **BR0111**: Default mode must be Hybrid if not specified
//! - **BR0112**: Minimum similarity score threshold enforced

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
            // WHY 20/60/30000: Aligned with SOTAQueryConfig LightRAG-parity defaults.
            max_chunks: 20,
            max_entities: 60,
            max_context_tokens: 30000,
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

    /// Whether to return the formatted prompt instead of calling LLM.
    /// Useful for debugging or using your own LLM.
    pub prompt_only: bool,

    /// Additional parameters.
    pub params: HashMap<String, serde_json::Value>,

    /// Conversation history for multi-turn context.
    #[serde(default)]
    pub conversation_history: Vec<ConversationMessage>,

    /// Override: enable or disable reranking for this request.
    #[serde(default)]
    pub enable_rerank: Option<bool>,

    /// Override: rerank top K results.
    #[serde(default)]
    pub rerank_top_k: Option<usize>,

    /// Override: LLM provider to use for answer generation.
    /// Format: provider name (e.g., "ollama", "openai", "lmstudio").
    /// If not provided, uses the server default.
    /// @implements SPEC-032: Provider selection at query time
    #[serde(default)]
    pub llm_provider: Option<String>,

    /// Override: LLM model to use for answer generation.
    /// If not provided, uses the provider's default model.
    /// @implements SPEC-032: Model selection at query time
    #[serde(default)]
    pub llm_model: Option<String>,
}

/// A single message in conversation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// Role of the message sender (user, assistant, system).
    pub role: String,

    /// Content of the message.
    pub content: String,
}

impl QueryRequest {
    /// Create a new query request.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            mode: None,
            max_results: None,
            context_only: false,
            prompt_only: false,
            params: HashMap::new(),
            conversation_history: Vec::new(),
            enable_rerank: None,
            rerank_top_k: None,
            llm_provider: None,
            llm_model: None,
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

    /// Set prompt-only mode.
    pub fn prompt_only(mut self) -> Self {
        self.prompt_only = true;
        self
    }

    /// Add conversation history.
    pub fn with_conversation_history(mut self, history: Vec<ConversationMessage>) -> Self {
        self.conversation_history = history;
        self
    }

    /// Set the LLM provider override for answer generation.
    /// Format: provider name (e.g., "ollama", "openai", "lmstudio").
    /// @implements SPEC-032: Provider selection at query time
    pub fn with_llm_provider(mut self, provider: impl Into<String>) -> Self {
        self.llm_provider = Some(provider.into());
        self
    }

    /// Set the LLM model override for answer generation.
    /// @implements SPEC-032: Model selection at query time
    pub fn with_llm_model(mut self, model: impl Into<String>) -> Self {
        self.llm_model = Some(model.into());
        self
    }

    /// Set both LLM provider and model from a full model ID.
    /// Format: "provider/model" (e.g., "ollama/gemma3:12b").
    /// @implements SPEC-032: Full model ID parsing
    pub fn with_llm_full_id(mut self, full_id: impl AsRef<str>) -> Self {
        let full_id = full_id.as_ref();
        if let Some((provider, model)) = full_id.split_once('/') {
            self.llm_provider = Some(provider.to_string());
            self.llm_model = Some(model.to_string());
        } else {
            // No slash - treat as provider only
            self.llm_provider = Some(full_id.to_string());
        }
        self
    }

    /// Set tenant ID for filtering.
    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.params
            .insert("tenant_id".to_string(), serde_json::json!(tenant_id.into()));
        self
    }

    /// Set workspace ID for filtering.
    pub fn with_workspace_id(mut self, workspace_id: impl Into<String>) -> Self {
        self.params.insert(
            "workspace_id".to_string(),
            serde_json::json!(workspace_id.into()),
        );
        self
    }

    /// Get tenant ID from params.
    pub fn tenant_id(&self) -> Option<String> {
        self.params
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Get workspace ID from params.
    pub fn workspace_id(&self) -> Option<String> {
        self.params
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Override reranking for this request.
    pub fn with_rerank(mut self, enable: bool) -> Self {
        self.enable_rerank = Some(enable);
        self
    }

    /// Set the rerank top K for this request.
    pub fn with_rerank_top_k(mut self, top_k: usize) -> Self {
        self.rerank_top_k = Some(top_k);
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
        let query_embedding = self.embedding_provider.embed_one(&request.query).await?;
        stats.embedding_time_ms = embed_start.elapsed().as_millis() as u64;

        // Step 2: Retrieve context based on mode
        let retrieval_start = std::time::Instant::now();
        let context = self
            .retrieve_context(
                &request.query,
                &query_embedding,
                mode,
                request.tenant_id(),
                request.workspace_id(),
            )
            .await?;
        stats.retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;
        stats.context_tokens = context.token_count;

        // Step 3: Generate answer (if not context-only or prompt-only)
        let (answer, generated_tokens) = if request.context_only {
            (String::new(), 0)
        } else if request.prompt_only {
            // Return the formatted prompt without calling the LLM
            (self.build_prompt(&request.query, &context), 0)
        } else {
            let gen_start = std::time::Instant::now();
            let (answer, tokens) = self
                .generate_answer_with_tokens(&request.query, &context)
                .await?;
            stats.generation_time_ms = gen_start.elapsed().as_millis() as u64;
            (answer, tokens)
        };

        stats.generated_tokens = generated_tokens;

        stats.total_time_ms = start.elapsed().as_millis() as u64;

        Ok(QueryResponse {
            answer,
            context,
            mode,
            stats,
        })
    }

    /// Execute a streaming query.
    pub async fn query_stream(
        &self,
        request: QueryRequest,
    ) -> Result<futures::stream::BoxStream<'static, Result<String>>> {
        let mode = request.mode.unwrap_or(self.config.default_mode);

        // Step 1: Generate query embedding
        let query_embedding = self.embedding_provider.embed_one(&request.query).await?;

        // Step 2: Retrieve context based on mode
        let context = self
            .retrieve_context(
                &request.query,
                &query_embedding,
                mode,
                request.tenant_id(),
                request.workspace_id(),
            )
            .await?;

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
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();

        // Helper closure to check if properties match tenant context
        let matches_tenant = |properties: &std::collections::HashMap<String, serde_json::Value>| {
            // If no tenant context is set, allow all
            if tenant_id.is_none() {
                return true;
            }

            // Check if properties have matching tenant_id
            if let Some(ref ctx_tenant_id) = tenant_id {
                if let Some(prop_tenant_id) = properties.get("tenant_id").and_then(|v| v.as_str()) {
                    if prop_tenant_id != ctx_tenant_id {
                        return false;
                    }
                }
                // If no tenant_id in properties but context has one, still include for backward compatibility
            }

            // Check workspace_id if set
            if let Some(ref ctx_workspace_id) = workspace_id {
                if let Some(prop_workspace_id) =
                    properties.get("workspace_id").and_then(|v| v.as_str())
                {
                    if prop_workspace_id != ctx_workspace_id {
                        return false;
                    }
                }
            }

            true
        };

        // Vector search for chunks
        if mode.uses_vector_search() {
            let results = self
                .vector_storage
                .query(query_embedding, self.config.max_chunks, None)
                .await?;

            for result in results {
                if result.score >= self.config.min_score {
                    // Filter vector results by tenant context
                    let metadata_map: HashMap<String, serde_json::Value> = result
                        .metadata
                        .as_object()
                        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                        .unwrap_or_default();

                    if !matches_tenant(&metadata_map) {
                        continue;
                    }

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
            // Get top entities by popularity (fetch more to account for filtering)
            let popular = self
                .graph_storage
                .get_popular_labels(self.config.max_entities * 2)
                .await?;

            let mut entity_count = 0;
            for entity_id in popular.iter() {
                if entity_count >= self.config.max_entities {
                    break;
                }
                if let Some(node) = self.graph_storage.get_node(entity_id).await? {
                    // Filter by tenant context
                    if !matches_tenant(&node.properties) {
                        continue;
                    }

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
                    entity_count += 1;

                    // Get relationships (also filtered by tenant)
                    let edges = self.graph_storage.get_node_edges(entity_id).await?;
                    for edge in edges.iter().take(5) {
                        // Filter edges by tenant context
                        if !matches_tenant(&edge.properties) {
                            continue;
                        }

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

    /// Build the prompt string for a query (used by prompt_only mode).
    fn build_prompt(&self, query: &str, context: &QueryContext) -> String {
        if context.is_empty() {
            return "I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string();
        }

        let context_text = context.to_context_string();

        format!(
            r#"You are a helpful assistant. Answer the user's question based on the following context.

## Context
{context_text}

## Question
{query}

## Answer
Provide a clear, accurate answer based on the context above. If the context doesn't contain enough information to answer the question, say so."#
        )
    }

    /// Generate an answer using the LLM and return the token count.
    async fn generate_answer_with_tokens(
        &self,
        query: &str,
        context: &QueryContext,
    ) -> Result<(String, usize)> {
        if context.is_empty() {
            return Ok(("I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string(), 0));
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

        Ok((response.content, response.completion_tokens))
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
        assert!(!request.prompt_only);

        // Test prompt_only mode
        let prompt_request = QueryRequest::new("What is Python?").prompt_only();

        assert!(prompt_request.prompt_only);
        assert!(!prompt_request.context_only);
    }

    #[test]
    fn test_query_engine_config_default() {
        let config = QueryEngineConfig::default();

        assert_eq!(config.default_mode, QueryMode::Hybrid);
        assert_eq!(config.max_chunks, 20);
        assert!(config.include_sources);
    }
}
