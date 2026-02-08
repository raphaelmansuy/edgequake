//! Configuration management for EdgeQuake.
//!
//! This module provides configuration structures and loading utilities.

use serde::{Deserialize, Serialize};

/// Main configuration for EdgeQuake.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Storage configuration
    pub storage: StorageConfig,
    /// LLM configuration
    pub llm: LlmConfig,
    /// Pipeline configuration
    pub pipeline: PipelineConfig,
    /// Query configuration
    pub query: QueryConfig,
    /// API server configuration
    pub api: ApiConfig,
}

/// Storage backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Database connection URL
    pub database_url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections in the pool
    pub min_connections: u32,
    /// Connection timeout in seconds
    pub connect_timeout_secs: u64,
    /// Namespace/schema for multi-tenancy
    pub namespace: Option<String>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database_url: "postgres://localhost:5432/edgequake".to_string(),
            max_connections: 10,
            min_connections: 1,
            connect_timeout_secs: 30,
            namespace: None,
        }
    }
}

/// LLM provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// LLM provider (e.g., "openai", "anthropic", "ollama")
    pub provider: String,
    /// API key for the LLM provider
    pub api_key: Option<String>,
    /// Base URL for the API (for custom endpoints)
    pub base_url: Option<String>,
    /// Model name for completions
    pub model: String,
    /// Model name for embeddings
    pub embedding_model: String,
    /// Embedding dimension
    pub embedding_dim: usize,
    /// Maximum tokens for LLM context
    pub max_tokens: usize,
    /// Temperature for generation
    pub temperature: f32,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum retries for failed requests
    pub max_retries: u32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            api_key: None,
            base_url: None,
            model: "gpt-4.1-nano".to_string(),
            embedding_model: "text-embedding-3-small".to_string(),
            embedding_dim: 1536,
            max_tokens: 4096,
            temperature: 0.0,
            timeout_secs: 60,
            max_retries: 3,
        }
    }
}

/// Document processing pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Maximum chunk size in tokens
    pub chunk_size: usize,
    /// Overlap between chunks in tokens
    pub chunk_overlap: usize,
    /// Entity types to extract
    pub entity_types: Vec<String>,
    /// Maximum entities per chunk
    pub max_entities_per_chunk: usize,
    /// Maximum relations per chunk
    pub max_relations_per_chunk: usize,
    /// Whether to summarize long descriptions
    pub summarize_descriptions: bool,
    /// Maximum description length in tokens before summarization
    pub max_description_tokens: usize,
    /// Number of concurrent extraction tasks
    pub concurrency: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1200,
            chunk_overlap: 100,
            entity_types: vec![
                "PERSON".to_string(),
                "ORGANIZATION".to_string(),
                "LOCATION".to_string(),
                "EVENT".to_string(),
                "CONCEPT".to_string(),
                "TECHNOLOGY".to_string(),
                "PRODUCT".to_string(),
            ],
            max_entities_per_chunk: 20,
            max_relations_per_chunk: 20,
            summarize_descriptions: true,
            max_description_tokens: 1200,
            concurrency: 4,
        }
    }
}

/// Query engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryConfig {
    /// Default query mode
    pub default_mode: QueryMode,
    /// Maximum results for vector search
    pub max_vector_results: usize,
    /// Maximum graph traversal depth
    pub max_graph_depth: usize,
    /// Maximum entities in context
    pub max_context_entities: usize,
    /// Maximum relationships in context
    pub max_context_relationships: usize,
    /// Maximum chunks in context
    pub max_context_chunks: usize,
    /// Whether to stream responses
    pub stream_responses: bool,
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            default_mode: QueryMode::Hybrid,
            max_vector_results: 20,
            max_graph_depth: 3,
            max_context_entities: 30,
            max_context_relationships: 30,
            max_context_chunks: 20,
            stream_responses: true,
        }
    }
}

/// Query execution modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum QueryMode {
    /// Direct chunk retrieval only
    Naive,
    /// Entity-focused local search
    Local,
    /// High-level global search
    Global,
    /// Combined local and global
    #[default]
    Hybrid,
    /// No RAG, direct LLM query
    Bypass,
}

use std::str::FromStr;

impl FromStr for QueryMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "naive" => Ok(Self::Naive),
            "local" => Ok(Self::Local),
            "global" => Ok(Self::Global),
            "hybrid" => Ok(Self::Hybrid),
            "bypass" => Ok(Self::Bypass),
            other => Err(format!("Unknown query mode: {}", other)),
        }
    }
}

impl QueryMode {
    /// Parse query mode from string (returns Option for backward compatibility).
    pub fn parse(s: &str) -> Option<Self> {
        Self::from_str(s).ok()
    }
}

/// API server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
    /// Enable CORS
    pub cors_enabled: bool,
    /// Allowed CORS origins
    pub cors_origins: Vec<String>,
    /// Enable API key authentication
    pub auth_enabled: bool,
    /// API keys for authentication
    pub api_keys: Vec<String>,
    /// Request body size limit in bytes
    pub body_limit: usize,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            cors_enabled: true,
            cors_origins: vec!["*".to_string()],
            auth_enabled: false,
            api_keys: Vec::new(),
            // SPEC-028: 50MB body limit to support larger document uploads
            // WHY: Must match max_document_size for consistent upload handling
            body_limit: 50 * 1024 * 1024, // 50MB
            timeout_secs: 300,
        }
    }
}

impl Config {
    /// Create a new configuration with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Storage
        if let Ok(url) = std::env::var("EDGEQUAKE_DATABASE_URL") {
            config.storage.database_url = url;
        }
        if let Ok(ns) = std::env::var("EDGEQUAKE_NAMESPACE") {
            config.storage.namespace = Some(ns);
        }

        // LLM
        if let Ok(provider) = std::env::var("EDGEQUAKE_LLM_PROVIDER") {
            config.llm.provider = provider;
        }
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            config.llm.api_key = Some(key);
        }
        if let Ok(model) = std::env::var("EDGEQUAKE_LLM_MODEL") {
            config.llm.model = model;
        }
        if let Ok(emb_model) = std::env::var("EDGEQUAKE_EMBEDDING_MODEL") {
            config.llm.embedding_model = emb_model;
        }

        // API
        if let Ok(host) = std::env::var("EDGEQUAKE_HOST") {
            config.api.host = host;
        }
        if let Ok(port) = std::env::var("EDGEQUAKE_PORT") {
            if let Ok(p) = port.parse() {
                config.api.port = p;
            }
        }

        config
    }

    /// Get the socket address for the API server.
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.api.host, self.api.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.api.port, 8080);
        assert_eq!(config.llm.provider, "openai");
        assert_eq!(config.pipeline.chunk_size, 1200);
    }

    #[test]
    fn test_query_mode_parsing() {
        assert_eq!(QueryMode::parse("hybrid"), Some(QueryMode::Hybrid));
        assert_eq!(QueryMode::parse("NAIVE"), Some(QueryMode::Naive));
        assert_eq!(QueryMode::parse("invalid"), None);
    }

    #[test]
    fn test_socket_addr() {
        let config = Config::default();
        assert_eq!(config.socket_addr(), "0.0.0.0:8080");
    }
}
