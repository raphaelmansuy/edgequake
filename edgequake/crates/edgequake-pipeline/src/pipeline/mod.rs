//! Document processing pipeline.
//!
//! ## Implements
//!
//! - **FEAT0001**: Document Ingestion Pipeline orchestration
//! - **FEAT0017**: Pipeline configuration management
//! - **FEAT0018**: Batch processing with concurrency control
//! - **FEAT0019**: Chunk-level progress tracking with callbacks
//!
//! ## Use Cases
//!
//! - **UC2301**: System processes document through all pipeline stages
//! - **UC2302**: System batches extraction for LLM rate limiting
//! - **UC2303**: System generates embeddings for chunks and entities
//! - **UC2304**: System reports per-chunk progress during extraction
//!
//! ## Enforces
//!
//! - **BR0017**: Maximum concurrent extractions enforced
//! - **BR0018**: Pipeline stages can be independently enabled/disabled
//!
//! ## Architecture
//!
//! The pipeline is split into focused sub-modules:
//! - `extraction`: Parallel and resilient chunk extraction
//! - `helpers`: Shared helpers for embedding generation, stats, lineage
//! - `processing`: Document processing entry points

mod extraction;
mod helpers;
mod processing;

use std::sync::Arc;

use edgequake_llm::traits::EmbeddingProvider;
use serde::{Deserialize, Serialize};

use crate::chunker::{Chunker, ChunkerConfig, TextChunk};
use crate::extractor::{EntityExtractor, ExtractionResult};

// ─────────────────────────────────────────────────────────────────────────────
//                              CONFIGURATION
// ─────────────────────────────────────────────────────────────────────────────

/// Pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Chunking configuration.
    pub chunker: ChunkerConfig,

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
    ///
    /// @implements SPEC-001/Issue-8: Timeout handling for extraction
    ///
    /// WHY: LLM calls can hang indefinitely due to network issues, provider
    /// outages, or very long responses. A timeout ensures the pipeline
    /// doesn't block forever on a single chunk.
    ///
    /// Default: 60 seconds (enough for most extractions, fast enough to detect hangs)
    #[serde(default = "default_chunk_timeout")]
    pub chunk_extraction_timeout_secs: u64,

    /// Maximum retry attempts per chunk.
    ///
    /// @implements SPEC-001/Issue-8: Retry limit for extraction
    ///
    /// WHY: Transient failures (rate limits, network blips) can be recovered
    /// with retries, but permanent failures should fail fast. 3 retries balances
    /// recovery with fail-fast behavior.
    #[serde(default = "default_max_retries")]
    pub chunk_max_retries: u32,

    /// Initial retry delay in milliseconds (for exponential backoff).
    ///
    /// @implements SPEC-001/Issue-8: Exponential backoff for retries
    ///
    /// WHY: Exponential backoff prevents hammering a failing service.
    /// Starting at 1000ms (1s), delays become: 1s, 2s, 4s, etc.
    #[serde(default = "default_initial_retry_delay")]
    pub initial_retry_delay_ms: u64,
}

fn default_chunk_timeout() -> u64 {
    // WHY 180: Ollama and other local LLMs need more time for entity extraction
    // prompts. Testing showed gemma3 can take 90-120s per chunk. 180s gives margin.
    180 // 180 seconds default timeout (increased from 60s for local LLM support)
}

fn default_max_retries() -> u32 {
    3 // 3 retry attempts by default
}

fn default_initial_retry_delay() -> u64 {
    1000 // 1 second initial delay
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            chunker: ChunkerConfig::default(),
            extraction_batch_size: 10,
            embedding_batch_size: 100,
            enable_entity_extraction: true,
            enable_relationship_extraction: true,
            enable_chunk_embeddings: true,
            enable_entity_embeddings: true,
            enable_relationship_embeddings: true,
            max_concurrent_extractions: 16,
            // OODA-06: Enable lineage tracking by default
            // WHY: Lineage data is critical for provenance queries. Without it,
            // no chunk↔entity↔document traceability is possible. The overhead is
            // minimal (one in-memory tree per document processing run).
            enable_lineage_tracking: true,
            chunk_extraction_timeout_secs: default_chunk_timeout(),
            chunk_max_retries: default_max_retries(),
            initial_retry_delay_ms: default_initial_retry_delay(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//                              RESULT TYPES
// ─────────────────────────────────────────────────────────────────────────────

/// Result of processing a document through the pipeline.
#[derive(Debug, Clone)]
pub struct ProcessingResult {
    /// Document ID.
    pub document_id: String,

    /// Generated chunks.
    pub chunks: Vec<TextChunk>,

    /// Extraction results per chunk.
    pub extractions: Vec<ExtractionResult>,

    /// Processing statistics.
    pub stats: ProcessingStats,

    /// Document lineage tracking (optional).
    pub lineage: Option<crate::lineage::DocumentLineage>,
}

/// Statistics from pipeline processing.
///
/// ┌─────────────────────────────────────────────────────────────────────────────┐
/// │                    CHUNK-LEVEL RESILIENCE STATS                             │
/// └─────────────────────────────────────────────────────────────────────────────┘
///
/// WHY TRACK FAILED CHUNKS?
/// ────────────────────────
/// 1. TRANSPARENCY: Users need to know if their document was partially processed
/// 2. RETRY CAPABILITY: Failed chunk IDs can be used for targeted retry
/// 3. MONITORING: Track failure patterns over time for system health
/// 4. DEBUGGING: Chunk errors help diagnose LLM/network issues
///
/// ```text
///   ProcessingStats
///       │
///       ├── chunk_count: 10              (total chunks attempted)
///       ├── successful_chunks: 8         (chunks that succeeded)
///       ├── failed_chunks: 2             (chunks that failed)
///       ├── chunk_errors: [...]          (error details per failed chunk)
///       │
///       └── success_rate = 8/10 = 80%
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessingStats {
    /// Number of chunks created.
    pub chunk_count: usize,

    /// Number of chunks successfully extracted.
    /// WHY: Allows calculating success rate = successful_chunks / chunk_count
    #[serde(default)]
    pub successful_chunks: usize,

    /// Number of chunks that failed extraction after all retries.
    /// WHY: Non-zero value triggers partial success handling in UI
    #[serde(default)]
    pub failed_chunks: usize,

    /// Error messages for each failed chunk (chunk_id -> error).
    /// WHY: Enables targeted retry and detailed error reporting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_errors: Option<Vec<ChunkErrorInfo>>,

    /// Number of entities extracted.
    pub entity_count: usize,

    /// Number of relationships extracted.
    pub relationship_count: usize,

    /// Processing time in milliseconds.
    pub processing_time_ms: u64,

    /// Number of LLM calls made.
    pub llm_calls: usize,

    /// Total tokens used.
    pub total_tokens: usize,

    /// LLM model used for entity extraction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,

    /// SPEC-032/OODA-198: LLM provider used for entity extraction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_provider: Option<String>,

    /// Embedding model used for vector embeddings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,

    /// SPEC-032/OODA-198: Embedding provider used for vector embeddings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_provider: Option<String>,

    /// Embedding dimensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_dimensions: Option<usize>,

    /// Entity types extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_types: Option<Vec<String>>,

    /// Relationship types extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship_types: Option<Vec<String>>,

    /// Keywords extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    /// Chunking strategy used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_strategy: Option<String>,

    /// Average chunk size in characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_chunk_size: Option<usize>,

    /// Input tokens used (for LLM calls).
    #[serde(default)]
    pub input_tokens: usize,

    /// Output tokens used (for LLM calls).
    #[serde(default)]
    pub output_tokens: usize,

    /// Total cost in USD (calculated from token usage).
    #[serde(default)]
    pub cost_usd: f64,

    /// Cost breakdown by operation (extraction, embedding, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_breakdown: Option<CostBreakdownStats>,

    /// Storage-level error details (graph/vector DB failures).
    /// WHY: Captures errors from upsert_nodes_batch, upsert_edges_batch,
    /// and entity embedding storage that previously were warn-and-continue.
    /// Populated only when storage_errors occur during indexing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_details: Option<String>,
}

/// Information about a failed chunk for error reporting.
///
/// WHY SEPARATE FROM ChunkFailure?
/// ────────────────────────────────
/// ChunkFailure is internal (full details for retry logic).
/// ChunkErrorInfo is external (serializable summary for API/UI).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkErrorInfo {
    /// Chunk ID (for correlation with source document).
    pub chunk_id: String,
    /// Chunk index (0-based position in document).
    pub chunk_index: usize,
    /// Error message (user-friendly).
    pub error_message: String,
    /// Whether this was a timeout vs other error.
    pub was_timeout: bool,
    /// Number of retry attempts made.
    pub retry_attempts: u32,
}

/// Cost breakdown by operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostBreakdownStats {
    /// Cost for entity extraction.
    #[serde(default)]
    pub extraction_cost_usd: f64,

    /// Cost for embedding generation.
    #[serde(default)]
    pub embedding_cost_usd: f64,

    /// Cost for summarization.
    #[serde(default)]
    pub summarization_cost_usd: f64,

    /// Extraction input tokens.
    #[serde(default)]
    pub extraction_input_tokens: usize,

    /// Extraction output tokens.
    #[serde(default)]
    pub extraction_output_tokens: usize,

    /// Embedding tokens.
    #[serde(default)]
    pub embedding_tokens: usize,
}

/// Progress update for a single chunk during extraction.
///
/// ## Implements
/// - **FEAT0019**: Chunk-level progress tracking
/// - **UC2304**: System reports per-chunk progress during extraction
#[derive(Debug, Clone)]
pub struct ChunkProgressUpdate {
    /// Index of the chunk being processed (0-based).
    pub chunk_index: usize,
    /// Total number of chunks in the document.
    pub total_chunks: usize,
    /// Preview of the chunk content (first 100 chars).
    pub chunk_preview: String,
    /// Time taken to process this chunk in milliseconds.
    pub processing_time_ms: u64,
    /// Input tokens consumed for this chunk.
    pub input_tokens: usize,
    /// Output tokens generated for this chunk.
    pub output_tokens: usize,
    /// Cost in USD for this chunk's LLM call.
    pub chunk_cost_usd: f64,
    /// Cumulative input tokens across all processed chunks.
    pub cumulative_input_tokens: u64,
    /// Cumulative output tokens across all processed chunks.
    pub cumulative_output_tokens: u64,
    /// Cumulative cost in USD.
    pub cumulative_cost_usd: f64,
    /// Average time per chunk in milliseconds (for ETA calculation).
    pub avg_time_per_chunk_ms: f64,
    /// Estimated remaining time in seconds.
    pub eta_seconds: u64,
}

/// Callback function type for chunk progress updates.
///
/// Called after each chunk is processed during extraction.
/// The callback receives a `ChunkProgressUpdate` with details about the completed chunk.
pub type ChunkProgressCallback = Arc<dyn Fn(ChunkProgressUpdate) + Send + Sync>;

// ─────────────────────────────────────────────────────────────────────────────
//                              PIPELINE STRUCT
// ─────────────────────────────────────────────────────────────────────────────

/// Document processing pipeline.
pub struct Pipeline {
    pub(super) config: PipelineConfig,
    pub(super) chunker: Chunker,
    pub(super) extractor: Option<Arc<dyn EntityExtractor>>,
    pub(super) embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
}

impl Pipeline {
    /// Create a new pipeline with the given configuration.
    pub fn new(config: PipelineConfig) -> Self {
        let chunker = Chunker::new(config.chunker.clone());

        Self {
            config,
            chunker,
            extractor: None,
            embedding_provider: None,
        }
    }

    /// Create a pipeline with default configuration.
    pub fn default_pipeline() -> Self {
        Self::new(PipelineConfig::default())
    }

    /// Set the entity extractor.
    pub fn with_extractor(mut self, extractor: Arc<dyn EntityExtractor>) -> Self {
        self.extractor = Some(extractor);
        self
    }

    /// Set the embedding provider.
    pub fn with_embedding_provider(mut self, provider: Arc<dyn EmbeddingProvider>) -> Self {
        self.embedding_provider = Some(provider);
        self
    }

    /// Get the pipeline configuration.
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    /// Get the chunker.
    pub fn chunker(&self) -> &Chunker {
        &self.chunker
    }

    /// Get the extractor.
    pub fn extractor(&self) -> Option<Arc<dyn EntityExtractor>> {
        self.extractor.clone()
    }

    /// Get the embedding provider.
    pub fn embedding_provider(&self) -> Option<Arc<dyn EmbeddingProvider>> {
        self.embedding_provider.clone()
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::default_pipeline()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractor::SimpleExtractor;

    #[tokio::test]
    async fn test_pipeline_basic_processing() {
        let pipeline = Pipeline::default_pipeline();

        let result = pipeline
            .process("doc-1", "This is a test document with some content.")
            .await
            .unwrap();

        assert_eq!(result.document_id, "doc-1");
        assert!(!result.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_pipeline_with_extractor() {
        let extractor = Arc::new(SimpleExtractor::default());
        let pipeline = Pipeline::default_pipeline().with_extractor(extractor);

        let result = pipeline
            .process("doc-1", "John Doe works at Acme Corp in New York.")
            .await
            .unwrap();

        // Should have extraction results
        assert!(result.stats.llm_calls > 0);
    }

    #[tokio::test]
    async fn test_pipeline_batch_processing() {
        let pipeline = Pipeline::default_pipeline();

        let documents = vec![
            ("doc-1".to_string(), "First document content.".to_string()),
            ("doc-2".to_string(), "Second document content.".to_string()),
        ];

        let results = pipeline.process_batch(&documents).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].document_id, "doc-1");
        assert_eq!(results[1].document_id, "doc-2");
    }

    #[test]
    fn test_pipeline_config_defaults() {
        let config = PipelineConfig::default();

        assert_eq!(config.extraction_batch_size, 10);
        assert!(config.enable_entity_extraction);
        assert!(config.enable_chunk_embeddings);
        // OODA-06: Lineage tracking now enabled by default for provenance queries
        assert!(config.enable_lineage_tracking);
    }

    #[tokio::test]
    async fn test_pipeline_with_lineage_tracking() {
        let extractor = Arc::new(SimpleExtractor::default());
        // OODA-06: Lineage tracking is now enabled by default, no need to set it
        let config = PipelineConfig::default();
        assert!(config.enable_lineage_tracking);

        let pipeline = Pipeline::new(config).with_extractor(extractor);

        let result = pipeline
            .process("doc-1", "John Doe works at Acme Corp in New York.")
            .await
            .unwrap();

        // Should have lineage
        assert!(result.lineage.is_some());

        let lineage = result.lineage.unwrap();
        assert_eq!(lineage.document_id, "doc-1");
        assert!(!lineage.chunks.is_empty());
        assert_eq!(lineage.total_chunks, result.chunks.len());
    }

    #[tokio::test]
    async fn test_pipeline_without_lineage_tracking() {
        // OODA-06: Explicitly disable lineage to test the opt-out path
        let mut config = PipelineConfig::default();
        config.enable_lineage_tracking = false;
        let pipeline = Pipeline::new(config);

        let result = pipeline
            .process("doc-1", "Simple document content.")
            .await
            .unwrap();

        // Should not have lineage (explicitly disabled)
        assert!(result.lineage.is_none());
    }
}
