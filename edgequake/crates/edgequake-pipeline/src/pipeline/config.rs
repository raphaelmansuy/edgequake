//! Pipeline configuration and environment-variable defaults (SPEC-017 SRP).

use serde::{Deserialize, Serialize};

use crate::chunker::{ChunkStrategy, ChunkerConfig};

/// Pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Chunking configuration.
    pub chunker: ChunkerConfig,

    /// Chunk strategy selector (SPEC-026 Phase 2).
    #[serde(default)]
    pub chunk_strategy: ChunkStrategy,

    /// Batch size for LLM extraction.
    pub extraction_batch_size: usize,

    /// Batch size for embedding generation.
    pub embedding_batch_size: usize,

    /// Whether to enable entity extraction.
    pub enable_entity_extraction: bool,

    /// Whether to enable relationship extraction.
    pub enable_relationship_extraction: bool,

    /// Whether to generate chunk embeddings.
    pub enable_chunk_embeddings: bool,

    /// Whether to generate entity embeddings.
    pub enable_entity_embeddings: bool,

    /// Whether to generate relationship embeddings.
    pub enable_relationship_embeddings: bool,

    /// Maximum concurrent extraction tasks.
    pub max_concurrent_extractions: usize,

    /// Whether to track document lineage.
    pub enable_lineage_tracking: bool,

    /// Timeout per chunk extraction in seconds.
    #[serde(default = "default_chunk_timeout")]
    pub chunk_extraction_timeout_secs: u64,

    /// Maximum retry attempts per chunk.
    #[serde(default = "default_max_retries")]
    pub chunk_max_retries: u32,

    /// Initial retry delay in milliseconds (for exponential backoff).
    #[serde(default = "default_initial_retry_delay")]
    pub initial_retry_delay_ms: u64,
}

/// Default per-chunk entity-extraction timeout (seconds).
pub const DEFAULT_CHUNK_TIMEOUT_SECS: u64 = 180;

/// Minimum acceptable per-chunk timeout (seconds).
pub const MIN_CHUNK_TIMEOUT_SECS: u64 = 10;

/// Default maximum retry attempts per chunk.
pub const DEFAULT_CHUNK_MAX_RETRIES: u32 = 3;

/// Maximum allowed retry count (safety cap).
pub const MAX_CHUNK_MAX_RETRIES: u32 = 20;

/// Default initial exponential-backoff delay (milliseconds).
pub const DEFAULT_INITIAL_RETRY_DELAY_MS: u64 = 1_000;

/// Default maximum concurrent LLM extraction tasks.
pub const DEFAULT_MAX_CONCURRENT_EXTRACTIONS: usize = 16;

fn default_chunk_timeout() -> u64 {
    DEFAULT_CHUNK_TIMEOUT_SECS
}

fn default_max_retries() -> u32 {
    DEFAULT_CHUNK_MAX_RETRIES
}

fn default_initial_retry_delay() -> u64 {
    DEFAULT_INITIAL_RETRY_DELAY_MS
}

fn read_env_u64(name: &str, default: u64, min_val: u64, max_val: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(default)
        .clamp(min_val, max_val)
}

fn read_env_u32(name: &str, default: u32, min_val: u32, max_val: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(default)
        .clamp(min_val, max_val)
}

fn read_env_usize(name: &str, default: usize, min_val: usize, max_val: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(default)
        .clamp(min_val, max_val)
}

impl PipelineConfig {
    /// Create a `PipelineConfig` from environment variables, falling back to defaults.
    pub fn from_env() -> Self {
        let chunk_timeout = read_env_u64(
            "EDGEQUAKE_CHUNK_TIMEOUT_SECS",
            DEFAULT_CHUNK_TIMEOUT_SECS,
            MIN_CHUNK_TIMEOUT_SECS,
            u64::MAX,
        );
        let max_retries = read_env_u32(
            "EDGEQUAKE_CHUNK_MAX_RETRIES",
            DEFAULT_CHUNK_MAX_RETRIES,
            0,
            MAX_CHUNK_MAX_RETRIES,
        );
        let retry_delay = read_env_u64(
            "EDGEQUAKE_CHUNK_RETRY_DELAY_MS",
            DEFAULT_INITIAL_RETRY_DELAY_MS,
            0,
            60_000,
        );
        let max_concurrent = read_env_usize(
            "EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS",
            DEFAULT_MAX_CONCURRENT_EXTRACTIONS,
            1,
            256,
        );

        Self {
            chunk_extraction_timeout_secs: chunk_timeout,
            chunk_max_retries: max_retries,
            initial_retry_delay_ms: retry_delay,
            max_concurrent_extractions: max_concurrent,
            ..Self::default()
        }
    }
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            chunker: ChunkerConfig::default(),
            chunk_strategy: ChunkStrategy::default(),
            extraction_batch_size: 10,
            embedding_batch_size: 100,
            enable_entity_extraction: true,
            enable_relationship_extraction: true,
            enable_chunk_embeddings: true,
            enable_entity_embeddings: true,
            enable_relationship_embeddings: true,
            max_concurrent_extractions: DEFAULT_MAX_CONCURRENT_EXTRACTIONS,
            enable_lineage_tracking: true,
            chunk_extraction_timeout_secs: DEFAULT_CHUNK_TIMEOUT_SECS,
            chunk_max_retries: DEFAULT_CHUNK_MAX_RETRIES,
            initial_retry_delay_ms: DEFAULT_INITIAL_RETRY_DELAY_MS,
        }
    }
}
