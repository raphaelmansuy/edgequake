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

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use edgequake_llm::traits::EmbeddingProvider;
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};

use crate::chunker::{Chunker, ChunkerConfig, TextChunk};
use crate::error::Result;
use crate::extractor::{EntityExtractor, ExtractionResult};
use crate::lineage::{DocumentLineage, ExtractionMetadata, LineageBuilder, SourceSpan};

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
            enable_lineage_tracking: false,
            chunk_extraction_timeout_secs: default_chunk_timeout(),
            chunk_max_retries: default_max_retries(),
            initial_retry_delay_ms: default_initial_retry_delay(),
        }
    }
}

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
    pub lineage: Option<DocumentLineage>,
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

/// Document processing pipeline.
pub struct Pipeline {
    config: PipelineConfig,
    chunker: Chunker,
    extractor: Option<Arc<dyn EntityExtractor>>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
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

    /// Extract entities from chunks in parallel using a semaphore.
    async fn extract_parallel(
        &self,
        chunks: &[TextChunk],
        extractor: &Arc<dyn EntityExtractor>,
    ) -> Result<Vec<ExtractionResult>> {
        // Delegate to extract_parallel_with_progress with no callback
        self.extract_parallel_with_progress(chunks, extractor, None)
            .await
    }

    /// Extract entities from chunks in parallel with optional progress callback.
    ///
    /// ## Implements
    /// - **FEAT0019**: Chunk-level progress tracking
    /// - **UC2304**: System reports per-chunk progress during extraction
    async fn extract_parallel_with_progress(
        &self,
        chunks: &[TextChunk],
        extractor: &Arc<dyn EntityExtractor>,
        progress_callback: Option<ChunkProgressCallback>,
    ) -> Result<Vec<ExtractionResult>> {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(
            self.config.max_concurrent_extractions,
        ));

        let total_chunks = chunks.len();

        // Atomic counters for cumulative tracking across concurrent extractions
        let cumulative_time_ms = Arc::new(AtomicU64::new(0));
        let cumulative_input_tokens = Arc::new(AtomicU64::new(0));
        let cumulative_output_tokens = Arc::new(AtomicU64::new(0));
        let completed_chunks = Arc::new(AtomicU32::new(0));

        // Get model pricing for cost calculation
        let pricing = crate::progress::default_model_pricing();
        let model_name = extractor.model_name();
        let model_pricing = pricing
            .get(model_name)
            .cloned()
            .unwrap_or_else(|| crate::progress::ModelPricing::new("gpt-4.1-nano", 0.00015, 0.0006));
        let model_pricing = Arc::new(model_pricing);

        // Create futures for all chunks with progress tracking
        let futures: Vec<_> = chunks
            .iter()
            .enumerate()
            .map(|(chunk_index, chunk)| {
                let semaphore = semaphore.clone();
                let extractor = extractor.clone();
                let chunk = chunk.clone();
                let progress_callback = progress_callback.clone();
                let cumulative_time_ms = cumulative_time_ms.clone();
                let cumulative_input_tokens = cumulative_input_tokens.clone();
                let cumulative_output_tokens = cumulative_output_tokens.clone();
                let completed_chunks = completed_chunks.clone();
                let model_pricing = model_pricing.clone();

                async move {
                    // Acquire permit (released on drop)
                    let _permit = semaphore
                        .acquire()
                        .await
                        .map_err(|e| crate::error::PipelineError::ExtractionError(e.to_string()))?;

                    // Extract entities from this chunk
                    let result = extractor.extract(&chunk).await?;

                    // Update cumulative counters
                    let time_ms = result.extraction_time_ms;
                    let in_tokens = result.input_tokens;
                    let out_tokens = result.output_tokens;

                    cumulative_time_ms.fetch_add(time_ms, Ordering::Relaxed);
                    cumulative_input_tokens.fetch_add(in_tokens as u64, Ordering::Relaxed);
                    cumulative_output_tokens.fetch_add(out_tokens as u64, Ordering::Relaxed);
                    let completed = completed_chunks.fetch_add(1, Ordering::Relaxed) + 1;

                    // Calculate cost for this chunk
                    let chunk_cost = model_pricing.calculate_cost(in_tokens, out_tokens);

                    // Emit progress update if callback is provided
                    if let Some(ref callback) = progress_callback {
                        let total_time = cumulative_time_ms.load(Ordering::Relaxed);
                        let total_in = cumulative_input_tokens.load(Ordering::Relaxed);
                        let total_out = cumulative_output_tokens.load(Ordering::Relaxed);

                        // Calculate average time per chunk and ETA
                        let avg_time_ms = if completed > 0 {
                            total_time as f64 / completed as f64
                        } else {
                            0.0
                        };
                        let remaining = total_chunks.saturating_sub(completed as usize);
                        let eta_seconds = ((avg_time_ms * remaining as f64) / 1000.0) as u64;

                        // Calculate cumulative cost
                        let cumulative_cost =
                            model_pricing.calculate_cost(total_in as usize, total_out as usize);

                        // Truncate chunk preview to 100 chars (OODA-02: Fixed UTF-8 char boundary panic)
                        let chunk_preview = if chunk.content.len() > 100 {
                            // Use char_indices() to ensure we don't split multi-byte UTF-8 characters
                            let truncate_at = chunk
                                .content
                                .char_indices()
                                .nth(97)
                                .map(|(idx, _)| idx)
                                .unwrap_or(chunk.content.len());
                            format!("{}...", &chunk.content[..truncate_at])
                        } else {
                            chunk.content.clone()
                        };

                        let update = ChunkProgressUpdate {
                            chunk_index,
                            total_chunks,
                            chunk_preview,
                            processing_time_ms: time_ms,
                            input_tokens: in_tokens,
                            output_tokens: out_tokens,
                            chunk_cost_usd: chunk_cost,
                            cumulative_input_tokens: total_in,
                            cumulative_output_tokens: total_out,
                            cumulative_cost_usd: cumulative_cost,
                            avg_time_per_chunk_ms: avg_time_ms,
                            eta_seconds,
                        };

                        callback(update);
                    }

                    Ok(result)
                }
            })
            .collect();

        // Execute concurrently with buffer to respect semaphore
        let results: Vec<Result<ExtractionResult>> = stream::iter(futures)
            .buffer_unordered(self.config.max_concurrent_extractions)
            .collect()
            .await;

        // Collect results, propagating first error
        results.into_iter().collect()
    }

    // ═══════════════════════════════════════════════════════════════════════════
    //                    RESILIENT PARALLEL EXTRACTION (MAP-REDUCE)
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // WHY RESILIENT EXTRACTION?
    // ────────────────────────────
    // The original extract_parallel_with_progress fails fast on the first error.
    // This is problematic for large documents where:
    // - A single chunk timeout shouldn't discard 99 successful extractions
    // - Users expect partial results with clear reporting of failures
    // - Retry logic should be at chunk level, not document level
    //
    // ARCHITECTURE (MAP-REDUCE PATTERN):
    // ────────────────────────────────────────────────────────────────────────────
    //
    //   ┌─────────────────────────────────────────────────────────────────────────┐
    //   │                           MAP PHASE                                     │
    //   │  (Parallel chunk processing with per-chunk retry and timeout)          │
    //   └─────────────────────────────────────────────────────────────────────────┘
    //
    //   Document (N chunks)
    //        │
    //        ▼
    //   ┌────┬────┬────┬────┬────┐
    //   │ C1 │ C2 │ C3 │ C4 │ CN │   (chunks distributed to workers)
    //   └─┬──┴─┬──┴─┬──┴─┬──┴─┬──┘
    //     │    │    │    │    │
    //     ▼    ▼    ▼    ▼    ▼      (parallel LLM calls with semaphore)
    //   ┌───┐┌───┐┌───┐┌───┐┌───┐
    //   │ E ││ E ││ E ││ E ││ E │    (each E = extract_with_retry)
    //   │ x ││ x ││ x ││ x ││ x │
    //   │ t ││ t ││ t ││ t ││ t │
    //   │ r ││ r ││ r ││ r ││ r │
    //   │ a ││ a ││ a ││ a ││ a │
    //   │ c ││ c ││ c ││ c ││ c │
    //   │ t ││ t ││ t ││ t ││ t │
    //   └─┬─┘└─┬─┘└─┬─┘└─┬─┘└─┬─┘
    //     │    │    │    │    │
    //     ▼    ▼    ▼    ▼    ▼      (each returns ChunkExtractionOutcome)
    //   ┌───┐┌───┐┌───┐┌───┐┌───┐
    //   │ ✓ ││ ✗ ││ ✓ ││ ✓ ││ ✓ │    (✓ = Success, ✗ = Failed)
    //   └───┘└───┘└───┘└───┘└───┘
    //
    //   ┌─────────────────────────────────────────────────────────────────────────┐
    //   │                          REDUCE PHASE                                   │
    //   │  (Aggregate successes and failures into ResilientExtractionResult)     │
    //   └─────────────────────────────────────────────────────────────────────────┘
    //
    //   All outcomes collected
    //        │
    //        ▼
    //   ┌────────────────────────────────────────────────────────────────────────┐
    //   │  Partition: successes = [C1, C3, C4, CN], failures = [C2]             │
    //   │  Sort by chunk_index (maintain document order)                        │
    //   │  Calculate stats: 4/5 = 80% success rate                              │
    //   └────────────────────────────────────────────────────────────────────────┘
    //        │
    //        ▼
    //   ResilientExtractionResult {
    //       successful_extractions: [4 results],
    //       failed_chunks: [1 failure with details],
    //       total_chunks: 5,
    //       success_rate: 0.80
    //   }
    //
    // RETRY STRATEGY (PER CHUNK):
    // ────────────────────────────────────────────────────────────────────────────
    //
    //   ┌─────────────────────────────────────────────────────────────────────────┐
    //   │  Attempt 1: base_timeout = 60s (or config.chunk_extraction_timeout)    │
    //   │       │                                                                │
    //   │       ├─> Success ✓ → Return ChunkExtractionOutcome::Success           │
    //   │       │                                                                │
    //   │       └─> Failure → Wait (exponential backoff: 1s, 2s, 4s)             │
    //   │                                                                        │
    //   │  Attempt 2: retry with 2x delay                                        │
    //   │       │                                                                │
    //   │       ├─> Success ✓ → Return ChunkExtractionOutcome::Success           │
    //   │       │                                                                │
    //   │       └─> Failure → Wait (double delay)                                │
    //   │                                                                        │
    //   │  Attempt 3: final attempt                                              │
    //   │       │                                                                │
    //   │       ├─> Success ✓ → Return ChunkExtractionOutcome::Success           │
    //   │       │                                                                │
    //   │       └─> Failure → Return ChunkExtractionOutcome::Failed              │
    //   │                     (with ChunkFailure containing all details)         │
    //   └─────────────────────────────────────────────────────────────────────────┘
    //

    /// Extract entities from chunks with resilient error handling.
    ///
    /// Unlike `extract_parallel`, this method does NOT fail fast on errors.
    /// Instead, it processes all chunks and returns both successes and failures,
    /// allowing partial results to be used.
    ///
    /// ## Implements
    /// - **FEAT0020**: Chunk-level resilience and error isolation
    /// - **UC2305**: System continues processing when individual chunks fail
    ///
    /// ## Returns
    /// `ResilientExtractionResult` containing:
    /// - All successful extractions (sorted by chunk order)
    /// - All failures with detailed error information
    /// - Statistics for monitoring and alerting
    pub async fn resilient_extract_parallel(
        &self,
        chunks: &[TextChunk],
        extractor: &Arc<dyn EntityExtractor>,
        progress_callback: Option<ChunkProgressCallback>,
    ) -> crate::error::ResilientExtractionResult {
        use crate::error::{ChunkExtractionOutcome, ChunkFailure, ResilientExtractionResult};

        let semaphore = Arc::new(tokio::sync::Semaphore::new(
            self.config.max_concurrent_extractions,
        ));

        let total_chunks = chunks.len();
        let timeout_secs = self.config.chunk_extraction_timeout_secs;
        let max_retries = self.config.chunk_max_retries;
        let initial_delay_ms = self.config.initial_retry_delay_ms;

        // Atomic counters for cumulative tracking
        let cumulative_time_ms = Arc::new(AtomicU64::new(0));
        let cumulative_input_tokens = Arc::new(AtomicU64::new(0));
        let cumulative_output_tokens = Arc::new(AtomicU64::new(0));
        let completed_chunks = Arc::new(AtomicU32::new(0));

        // Get model pricing for cost calculation
        let pricing = crate::progress::default_model_pricing();
        let model_name = extractor.model_name();
        let model_pricing = pricing
            .get(model_name)
            .cloned()
            .unwrap_or_else(|| crate::progress::ModelPricing::new("gpt-4.1-nano", 0.00015, 0.0006));
        let model_pricing = Arc::new(model_pricing);

        // ═══════════════════════════════════════════════════════════════════════
        //                           MAP PHASE
        // ═══════════════════════════════════════════════════════════════════════
        // Create futures for all chunks. Each future handles its own retry logic
        // and returns ChunkExtractionOutcome (never propagates error upward).

        let futures: Vec<_> = chunks
            .iter()
            .enumerate()
            .map(|(chunk_index, chunk)| {
                let semaphore = semaphore.clone();
                let extractor = extractor.clone();
                let chunk = chunk.clone();
                let progress_callback = progress_callback.clone();
                let cumulative_time_ms = cumulative_time_ms.clone();
                let cumulative_input_tokens = cumulative_input_tokens.clone();
                let cumulative_output_tokens = cumulative_output_tokens.clone();
                let completed_chunks = completed_chunks.clone();
                let model_pricing = model_pricing.clone();

                async move {
                    let chunk_start = std::time::Instant::now();

                    // Acquire permit (released on drop)
                    let _permit = match semaphore.acquire().await {
                        Ok(p) => p,
                        Err(e) => {
                            return ChunkExtractionOutcome::Failed(ChunkFailure {
                                chunk_index,
                                chunk_id: chunk.id.clone(),
                                error: format!("Semaphore acquisition failed: {}", e),
                                retry_attempts: 0,
                                was_timeout: false,
                                processing_time_ms: chunk_start.elapsed().as_millis() as u64,
                            });
                        }
                    };

                    // ─────────────────────────────────────────────────────────────
                    // PER-CHUNK RETRY LOOP
                    // ─────────────────────────────────────────────────────────────
                    // WHY RETRY AT CHUNK LEVEL?
                    // - Transient errors (rate limits, network blips) are common
                    // - Retrying specific chunks is more efficient than whole doc
                    // - Exponential backoff prevents overwhelming the LLM provider

                    let mut last_error = String::new();
                    let mut was_timeout = false;

                    for attempt in 1..=max_retries {
                        // Apply timeout to the extraction call
                        let extraction_future = extractor.extract(&chunk);
                        let timeout_duration = tokio::time::Duration::from_secs(timeout_secs);

                        match tokio::time::timeout(timeout_duration, extraction_future).await {
                            Ok(Ok(result)) => {
                                // ═══════════════════════════════════════════════════
                                // SUCCESS PATH
                                // ═══════════════════════════════════════════════════
                                let time_ms = result.extraction_time_ms;
                                let in_tokens = result.input_tokens;
                                let out_tokens = result.output_tokens;

                                cumulative_time_ms.fetch_add(time_ms, Ordering::Relaxed);
                                cumulative_input_tokens
                                    .fetch_add(in_tokens as u64, Ordering::Relaxed);
                                cumulative_output_tokens
                                    .fetch_add(out_tokens as u64, Ordering::Relaxed);
                                let completed =
                                    completed_chunks.fetch_add(1, Ordering::Relaxed) + 1;

                                // Emit progress update if callback is provided
                                if let Some(ref callback) = progress_callback {
                                    let total_time = cumulative_time_ms.load(Ordering::Relaxed);
                                    let total_in = cumulative_input_tokens.load(Ordering::Relaxed);
                                    let total_out =
                                        cumulative_output_tokens.load(Ordering::Relaxed);

                                    let avg_time_ms = if completed > 0 {
                                        total_time as f64 / completed as f64
                                    } else {
                                        0.0
                                    };
                                    let remaining = total_chunks.saturating_sub(completed as usize);
                                    let eta_seconds =
                                        ((avg_time_ms * remaining as f64) / 1000.0) as u64;

                                    let cumulative_cost = model_pricing
                                        .calculate_cost(total_in as usize, total_out as usize);

                                    let chunk_preview = if chunk.content.len() > 100 {
                                        let truncate_at = chunk
                                            .content
                                            .char_indices()
                                            .nth(97)
                                            .map(|(idx, _)| idx)
                                            .unwrap_or(chunk.content.len());
                                        format!("{}...", &chunk.content[..truncate_at])
                                    } else {
                                        chunk.content.clone()
                                    };

                                    let chunk_cost =
                                        model_pricing.calculate_cost(in_tokens, out_tokens);

                                    callback(ChunkProgressUpdate {
                                        chunk_index,
                                        total_chunks,
                                        chunk_preview,
                                        processing_time_ms: time_ms,
                                        input_tokens: in_tokens,
                                        output_tokens: out_tokens,
                                        chunk_cost_usd: chunk_cost,
                                        cumulative_input_tokens: total_in,
                                        cumulative_output_tokens: total_out,
                                        cumulative_cost_usd: cumulative_cost,
                                        avg_time_per_chunk_ms: avg_time_ms,
                                        eta_seconds,
                                    });
                                }

                                return ChunkExtractionOutcome::Success {
                                    chunk_index,
                                    result,
                                };
                            }
                            Ok(Err(e)) => {
                                // Extraction error (not timeout)
                                last_error = format!("{}", e);
                                was_timeout = false;
                                tracing::warn!(
                                    chunk_index = chunk_index,
                                    chunk_id = %chunk.id,
                                    attempt = attempt,
                                    max_retries = max_retries,
                                    error = %e,
                                    "Chunk extraction failed, will retry"
                                );
                            }
                            Err(_) => {
                                // Timeout
                                last_error = format!(
                                    "Timeout after {}s (attempt {}/{})",
                                    timeout_secs, attempt, max_retries
                                );
                                was_timeout = true;
                                tracing::warn!(
                                    chunk_index = chunk_index,
                                    chunk_id = %chunk.id,
                                    attempt = attempt,
                                    max_retries = max_retries,
                                    timeout_secs = timeout_secs,
                                    "Chunk extraction timed out, will retry"
                                );
                            }
                        }

                        // Exponential backoff before retry
                        if attempt < max_retries {
                            let delay_ms = initial_delay_ms * 2_u64.pow(attempt - 1);
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        }
                    }

                    // ═══════════════════════════════════════════════════════════════
                    // FAILURE PATH (all retries exhausted)
                    // ═══════════════════════════════════════════════════════════════
                    // WHY RETURN FAILURE INSTEAD OF ERROR?
                    // - Allows other chunks to continue processing
                    // - Caller can decide what to do with partial results
                    // - Failure details are preserved for retry/debugging

                    // Still update completed count for accurate progress
                    completed_chunks.fetch_add(1, Ordering::Relaxed);

                    ChunkExtractionOutcome::Failed(ChunkFailure {
                        chunk_index,
                        chunk_id: chunk.id.clone(),
                        error: last_error,
                        retry_attempts: max_retries,
                        was_timeout,
                        processing_time_ms: chunk_start.elapsed().as_millis() as u64,
                    })
                }
            })
            .collect();

        // ═══════════════════════════════════════════════════════════════════════
        //                          REDUCE PHASE
        // ═══════════════════════════════════════════════════════════════════════
        // Execute all futures concurrently, then aggregate results.
        // Note: buffer_unordered allows completing in any order while
        // respecting the semaphore limit.

        let outcomes: Vec<ChunkExtractionOutcome> = stream::iter(futures)
            .buffer_unordered(self.config.max_concurrent_extractions)
            .collect()
            .await;

        // Aggregate into final result
        ResilientExtractionResult::from_outcomes(outcomes)
    }

    /// Process a document through the pipeline.
    pub async fn process(&self, document_id: &str, content: &str) -> Result<ProcessingResult> {
        let start = std::time::Instant::now();
        let mut stats = ProcessingStats::default();

        // Step 1: Chunk the document
        let mut chunks = self.chunker.chunk(content, document_id)?;
        stats.chunk_count = chunks.len();

        // Track chunking strategy and average chunk size
        stats.chunking_strategy =
            Some(format!("sliding_window_{}", self.config.chunker.chunk_size));
        if !chunks.is_empty() {
            let total_chars: usize = chunks.iter().map(|c| c.content.len()).sum();
            stats.avg_chunk_size = Some(total_chars / chunks.len());
        }

        // Step 2: Extract entities and relationships
        let mut extractions = Vec::new();
        let mut entity_types_set = std::collections::HashSet::new();
        let mut relationship_types_set = std::collections::HashSet::new();
        let mut keywords_set = std::collections::HashSet::new();
        let mut total_input_tokens = 0usize;
        let mut total_output_tokens = 0usize;

        if self.config.enable_entity_extraction || self.config.enable_relationship_extraction {
            if let Some(extractor) = &self.extractor {
                // Capture LLM model and provider names
                // @implements SPEC-032/OODA-226: Provider tracking in ProcessingStats
                stats.llm_model = Some(extractor.model_name().to_string());
                stats.llm_provider = Some(extractor.provider_name().to_string());

                // Use parallel extraction for better performance
                extractions = self.extract_parallel(&chunks, extractor).await?;

                // CRITICAL FIX: Link entities and relationships to their source chunks
                // Without this, Local/Global modes cannot find related chunks during query
                for extraction in &mut extractions {
                    let chunk_id = extraction.source_chunk_id.clone();
                    tracing::info!(
                        "Linking {} entities and {} relationships to chunk {}",
                        extraction.entities.len(),
                        extraction.relationships.len(),
                        chunk_id
                    );
                    for entity in &mut extraction.entities {
                        entity.add_source_chunk_id(&chunk_id);
                    }
                    for rel in &mut extraction.relationships {
                        if rel.source_chunk_id.is_none() {
                            rel.source_chunk_id = Some(chunk_id.clone());
                        }
                    }
                }

                // Aggregate statistics from all extractions
                for extraction in &extractions {
                    stats.entity_count += extraction.entities.len();
                    stats.relationship_count += extraction.relationships.len();
                    stats.llm_calls += 1;
                    total_input_tokens += extraction.input_tokens;
                    total_output_tokens += extraction.output_tokens;

                    // Collect unique entity types
                    for entity in &extraction.entities {
                        entity_types_set.insert(entity.entity_type.clone());
                    }

                    // Collect unique relationship types and keywords
                    for rel in &extraction.relationships {
                        relationship_types_set.insert(rel.relation_type.clone());
                        for keyword in &rel.keywords {
                            keywords_set.insert(keyword.clone());
                        }
                    }
                }

                stats.total_tokens = total_input_tokens + total_output_tokens;
                stats.input_tokens = total_input_tokens;
                stats.output_tokens = total_output_tokens;

                // Calculate extraction cost using model pricing
                let model_name = extractor.model_name();
                let pricing = crate::progress::default_model_pricing();
                let model_pricing = pricing.get(model_name).cloned().unwrap_or_else(|| {
                    crate::progress::ModelPricing::new("gpt-4.1-nano", 0.00015, 0.0006)
                });

                let extraction_cost =
                    model_pricing.calculate_cost(total_input_tokens, total_output_tokens);
                stats.cost_usd += extraction_cost;

                // Initialize cost breakdown
                let cost_breakdown = CostBreakdownStats {
                    extraction_cost_usd: extraction_cost,
                    extraction_input_tokens: total_input_tokens,
                    extraction_output_tokens: total_output_tokens,
                    ..CostBreakdownStats::default()
                };
                stats.cost_breakdown = Some(cost_breakdown);
            }
        }

        // Store collected types and keywords
        if !entity_types_set.is_empty() {
            stats.entity_types = Some(entity_types_set.into_iter().collect());
        }
        if !relationship_types_set.is_empty() {
            stats.relationship_types = Some(relationship_types_set.into_iter().collect());
        }
        if !keywords_set.is_empty() {
            let mut keywords: Vec<String> = keywords_set.into_iter().collect();
            keywords.sort();
            // Limit to top 50 keywords
            keywords.truncate(50);
            stats.keywords = Some(keywords);
        }

        // Step 3: Generate embeddings
        if let Some(provider) = &self.embedding_provider {
            // Capture embedding model and provider info
            // @implements SPEC-032/OODA-226: Provider tracking in ProcessingStats
            stats.embedding_model = Some(provider.model().to_string());
            stats.embedding_provider = Some(provider.name().to_string());
            stats.embedding_dimensions = Some(provider.dimension());

            // Chunk embeddings
            if self.config.enable_chunk_embeddings {
                let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
                if !texts.is_empty() {
                    let embeddings = provider
                        .embed(&texts)
                        .await
                        .map_err(|e| crate::error::PipelineError::EmbeddingError(e.to_string()))?;

                    for (chunk, embedding) in chunks.iter_mut().zip(embeddings) {
                        chunk.embedding = Some(embedding);
                    }
                }
            }

            // Entity embeddings - OPTIMIZED: Batch all entities together
            if self.config.enable_entity_embeddings {
                // Collect all entity texts with their indices for reassignment
                let mut all_entity_texts: Vec<String> = Vec::new();
                let mut entity_indices: Vec<(usize, usize)> = Vec::new(); // (extraction_idx, entity_idx)

                for (ext_idx, extraction) in extractions.iter().enumerate() {
                    for (ent_idx, entity) in extraction.entities.iter().enumerate() {
                        all_entity_texts.push(format!("{}: {}", entity.name, entity.description));
                        entity_indices.push((ext_idx, ent_idx));
                    }
                }

                if !all_entity_texts.is_empty() {
                    // Single batch call for all entities
                    let all_embeddings = provider
                        .embed(&all_entity_texts)
                        .await
                        .map_err(|e| crate::error::PipelineError::EmbeddingError(e.to_string()))?;

                    // Reassign embeddings to their respective entities
                    for (embedding, (ext_idx, ent_idx)) in
                        all_embeddings.into_iter().zip(entity_indices)
                    {
                        extractions[ext_idx].entities[ent_idx].embedding = Some(embedding);
                    }
                }
            }

            // Relationship embeddings - OPTIMIZED: Batch all relationships together
            if self.config.enable_relationship_embeddings {
                // Collect all relationship texts with their indices for reassignment
                let mut all_relationship_texts: Vec<String> = Vec::new();
                let mut relationship_indices: Vec<(usize, usize)> = Vec::new(); // (extraction_idx, rel_idx)

                for (ext_idx, extraction) in extractions.iter().enumerate() {
                    for (rel_idx, r) in extraction.relationships.iter().enumerate() {
                        // Format: "keywords\tsource->target\ndescription"
                        // Matches LightRAG's relationship embedding format
                        all_relationship_texts.push(format!(
                            "{}\t{}->{}\n{}",
                            r.keywords.join(", "),
                            r.source,
                            r.target,
                            r.description
                        ));
                        relationship_indices.push((ext_idx, rel_idx));
                    }
                }

                if !all_relationship_texts.is_empty() {
                    // Single batch call for all relationships
                    let all_embeddings = provider
                        .embed(&all_relationship_texts)
                        .await
                        .map_err(|e| crate::error::PipelineError::EmbeddingError(e.to_string()))?;

                    // Reassign embeddings to their respective relationships
                    for (embedding, (ext_idx, rel_idx)) in
                        all_embeddings.into_iter().zip(relationship_indices)
                    {
                        extractions[ext_idx].relationships[rel_idx].embedding = Some(embedding);
                    }
                }
            }

            // Calculate embedding costs
            // Estimate token count based on text length (approx 4 chars per token)
            let mut total_embed_tokens = 0usize;

            // Chunk tokens
            if self.config.enable_chunk_embeddings {
                let chunk_text_len: usize = chunks.iter().map(|c| c.content.len()).sum();
                total_embed_tokens += chunk_text_len / 4;
            }

            // Entity tokens
            if self.config.enable_entity_embeddings {
                for extraction in &extractions {
                    for entity in &extraction.entities {
                        total_embed_tokens += (entity.name.len() + entity.description.len()) / 4;
                    }
                }
            }

            // Relationship tokens
            if self.config.enable_relationship_embeddings {
                for extraction in &extractions {
                    for rel in &extraction.relationships {
                        total_embed_tokens +=
                            (rel.source.len() + rel.target.len() + rel.description.len()) / 4;
                    }
                }
            }

            // Calculate embedding cost
            let embed_model_name = provider.model();
            let pricing = crate::progress::default_model_pricing();
            let embed_pricing = pricing.get(embed_model_name).cloned().unwrap_or_else(|| {
                crate::progress::ModelPricing::new("text-embedding-3-small", 0.00002, 0.0)
            });

            let embedding_cost = embed_pricing.calculate_cost(total_embed_tokens, 0);
            stats.cost_usd += embedding_cost;

            // Update cost breakdown
            if let Some(ref mut breakdown) = stats.cost_breakdown {
                breakdown.embedding_cost_usd = embedding_cost;
                breakdown.embedding_tokens = total_embed_tokens;
            } else {
                let breakdown = CostBreakdownStats {
                    embedding_cost_usd: embedding_cost,
                    embedding_tokens: total_embed_tokens,
                    ..CostBreakdownStats::default()
                };
                stats.cost_breakdown = Some(breakdown);
            }
        }

        stats.processing_time_ms = start.elapsed().as_millis() as u64;

        // Step 4: Build lineage if enabled
        let lineage = if self.config.enable_lineage_tracking {
            let job_id = uuid::Uuid::new_v4().to_string();
            let mut builder = LineageBuilder::new(document_id, document_id, &job_id);

            // Record chunks with their line numbers
            for chunk in &chunks {
                let metadata =
                    ExtractionMetadata::new(stats.llm_model.as_deref().unwrap_or("unknown"));
                builder.record_chunk(
                    &chunk.id,
                    chunk.index,
                    chunk.start_line,
                    chunk.end_line,
                    chunk.start_offset,
                    chunk.end_offset,
                    metadata,
                );
            }

            // Record entities and relationships from extractions
            for extraction in &extractions {
                for entity in &extraction.entities {
                    let entity_id = format!("{}_{}", extraction.source_chunk_id, entity.name);
                    let span = SourceSpan::new(0, 0, 0, 0); // Detailed span would require chunk info
                    builder.record_entity(
                        &entity_id,
                        &entity.name,
                        &extraction.source_chunk_id,
                        span,
                        &entity.description,
                    );
                }

                for rel in &extraction.relationships {
                    let rel_id = format!(
                        "{}_{}_{}",
                        extraction.source_chunk_id, rel.source, rel.target
                    );
                    let span = SourceSpan::new(0, 0, 0, 0);
                    builder.record_relationship(
                        &rel_id,
                        &rel.source,
                        &rel.target,
                        &rel.relation_type,
                        &extraction.source_chunk_id,
                        span,
                        &rel.description,
                    );
                }
            }

            Some(builder.build())
        } else {
            None
        };

        Ok(ProcessingResult {
            document_id: document_id.to_string(),
            chunks,
            extractions,
            stats,
            lineage,
        })
    }

    /// Process a document through the pipeline with chunk-level progress callbacks.
    ///
    /// This method is identical to `process` but invokes the provided callback
    /// after each chunk is processed during entity extraction, enabling real-time
    /// progress tracking.
    ///
    /// ## Implements
    /// - **FEAT0019**: Chunk-level progress tracking
    /// - **UC2304**: System reports per-chunk progress during extraction
    ///
    /// ## Example
    /// ```ignore
    /// let callback = Arc::new(|update: ChunkProgressUpdate| {
    ///     println!("Chunk {}/{}: {:.1}% complete, ETA: {}s",
    ///         update.chunk_index + 1,
    ///         update.total_chunks,
    ///         (update.chunk_index + 1) as f64 / update.total_chunks as f64 * 100.0,
    ///         update.eta_seconds
    ///     );
    /// });
    /// let result = pipeline.process_with_progress("doc1", content, Some(callback)).await?;
    /// ```
    pub async fn process_with_progress(
        &self,
        document_id: &str,
        content: &str,
        progress_callback: Option<ChunkProgressCallback>,
    ) -> Result<ProcessingResult> {
        let start = std::time::Instant::now();
        let mut stats = ProcessingStats::default();

        // Step 1: Chunk the document
        let mut chunks = self.chunker.chunk(content, document_id)?;
        stats.chunk_count = chunks.len();

        // Track chunking strategy and average chunk size
        stats.chunking_strategy =
            Some(format!("sliding_window_{}", self.config.chunker.chunk_size));
        if !chunks.is_empty() {
            let total_chars: usize = chunks.iter().map(|c| c.content.len()).sum();
            stats.avg_chunk_size = Some(total_chars / chunks.len());
        }

        // Step 2: Extract entities and relationships WITH PROGRESS CALLBACK
        let mut extractions = Vec::new();
        let mut entity_types_set = std::collections::HashSet::new();
        let mut relationship_types_set = std::collections::HashSet::new();
        let mut keywords_set = std::collections::HashSet::new();
        let mut total_input_tokens = 0usize;
        let mut total_output_tokens = 0usize;

        if self.config.enable_entity_extraction || self.config.enable_relationship_extraction {
            if let Some(extractor) = &self.extractor {
                // Capture LLM model and provider names
                stats.llm_model = Some(extractor.model_name().to_string());
                stats.llm_provider = Some(extractor.provider_name().to_string());

                // Use parallel extraction WITH PROGRESS CALLBACK
                extractions = self
                    .extract_parallel_with_progress(&chunks, extractor, progress_callback)
                    .await?;

                // Link entities and relationships to their source chunks
                for extraction in &mut extractions {
                    let chunk_id = extraction.source_chunk_id.clone();
                    tracing::debug!(
                        "Linking {} entities and {} relationships to chunk {}",
                        extraction.entities.len(),
                        extraction.relationships.len(),
                        chunk_id
                    );
                    for entity in &mut extraction.entities {
                        entity.add_source_chunk_id(&chunk_id);
                    }
                    for rel in &mut extraction.relationships {
                        if rel.source_chunk_id.is_none() {
                            rel.source_chunk_id = Some(chunk_id.clone());
                        }
                    }
                }

                // Aggregate statistics from all extractions
                for extraction in &extractions {
                    stats.entity_count += extraction.entities.len();
                    stats.relationship_count += extraction.relationships.len();
                    stats.llm_calls += 1;
                    total_input_tokens += extraction.input_tokens;
                    total_output_tokens += extraction.output_tokens;

                    for entity in &extraction.entities {
                        entity_types_set.insert(entity.entity_type.clone());
                    }
                    for rel in &extraction.relationships {
                        relationship_types_set.insert(rel.relation_type.clone());
                        for keyword in &rel.keywords {
                            keywords_set.insert(keyword.clone());
                        }
                    }
                }

                stats.total_tokens = total_input_tokens + total_output_tokens;
                stats.input_tokens = total_input_tokens;
                stats.output_tokens = total_output_tokens;

                // Calculate extraction cost
                let model_name = extractor.model_name();
                let pricing = crate::progress::default_model_pricing();
                let model_pricing = pricing.get(model_name).cloned().unwrap_or_else(|| {
                    crate::progress::ModelPricing::new("gpt-4.1-nano", 0.00015, 0.0006)
                });

                let extraction_cost =
                    model_pricing.calculate_cost(total_input_tokens, total_output_tokens);
                stats.cost_usd += extraction_cost;

                let cost_breakdown = CostBreakdownStats {
                    extraction_cost_usd: extraction_cost,
                    extraction_input_tokens: total_input_tokens,
                    extraction_output_tokens: total_output_tokens,
                    ..CostBreakdownStats::default()
                };
                stats.cost_breakdown = Some(cost_breakdown);
            }
        }

        // Store collected types and keywords
        if !entity_types_set.is_empty() {
            stats.entity_types = Some(entity_types_set.into_iter().collect());
        }
        if !relationship_types_set.is_empty() {
            stats.relationship_types = Some(relationship_types_set.into_iter().collect());
        }
        if !keywords_set.is_empty() {
            let mut keywords: Vec<String> = keywords_set.into_iter().collect();
            keywords.sort();
            keywords.truncate(50);
            stats.keywords = Some(keywords);
        }

        // Step 3: Generate embeddings (same as process() - no progress callback needed)
        if let Some(provider) = &self.embedding_provider {
            stats.embedding_model = Some(provider.model().to_string());
            stats.embedding_provider = Some(provider.name().to_string());
            stats.embedding_dimensions = Some(provider.dimension());

            // Chunk embeddings
            if self.config.enable_chunk_embeddings {
                let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
                if !texts.is_empty() {
                    let embeddings = provider
                        .embed(&texts)
                        .await
                        .map_err(|e| crate::error::PipelineError::EmbeddingError(e.to_string()))?;
                    for (chunk, embedding) in chunks.iter_mut().zip(embeddings) {
                        chunk.embedding = Some(embedding);
                    }
                }
            }

            // Entity embeddings
            if self.config.enable_entity_embeddings {
                let mut all_entity_texts: Vec<String> = Vec::new();
                let mut entity_indices: Vec<(usize, usize)> = Vec::new();

                for (ext_idx, extraction) in extractions.iter().enumerate() {
                    for (ent_idx, entity) in extraction.entities.iter().enumerate() {
                        all_entity_texts.push(format!("{}: {}", entity.name, entity.description));
                        entity_indices.push((ext_idx, ent_idx));
                    }
                }

                if !all_entity_texts.is_empty() {
                    let all_embeddings = provider
                        .embed(&all_entity_texts)
                        .await
                        .map_err(|e| crate::error::PipelineError::EmbeddingError(e.to_string()))?;
                    for (embedding, (ext_idx, ent_idx)) in
                        all_embeddings.into_iter().zip(entity_indices)
                    {
                        extractions[ext_idx].entities[ent_idx].embedding = Some(embedding);
                    }
                }
            }

            // Relationship embeddings
            if self.config.enable_relationship_embeddings {
                let mut all_relationship_texts: Vec<String> = Vec::new();
                let mut relationship_indices: Vec<(usize, usize)> = Vec::new();

                for (ext_idx, extraction) in extractions.iter().enumerate() {
                    for (rel_idx, r) in extraction.relationships.iter().enumerate() {
                        all_relationship_texts.push(format!(
                            "{}\t{}->{}\n{}",
                            r.keywords.join(", "),
                            r.source,
                            r.target,
                            r.description
                        ));
                        relationship_indices.push((ext_idx, rel_idx));
                    }
                }

                if !all_relationship_texts.is_empty() {
                    let all_embeddings = provider
                        .embed(&all_relationship_texts)
                        .await
                        .map_err(|e| crate::error::PipelineError::EmbeddingError(e.to_string()))?;
                    for (embedding, (ext_idx, rel_idx)) in
                        all_embeddings.into_iter().zip(relationship_indices)
                    {
                        extractions[ext_idx].relationships[rel_idx].embedding = Some(embedding);
                    }
                }
            }

            // Calculate embedding costs
            let mut total_embed_tokens = 0usize;
            if self.config.enable_chunk_embeddings {
                let chunk_text_len: usize = chunks.iter().map(|c| c.content.len()).sum();
                total_embed_tokens += chunk_text_len / 4;
            }
            if self.config.enable_entity_embeddings {
                for extraction in &extractions {
                    for entity in &extraction.entities {
                        total_embed_tokens += (entity.name.len() + entity.description.len()) / 4;
                    }
                }
            }
            if self.config.enable_relationship_embeddings {
                for extraction in &extractions {
                    for rel in &extraction.relationships {
                        total_embed_tokens +=
                            (rel.source.len() + rel.target.len() + rel.description.len()) / 4;
                    }
                }
            }

            let embed_model_name = provider.model();
            let pricing = crate::progress::default_model_pricing();
            let embed_pricing = pricing.get(embed_model_name).cloned().unwrap_or_else(|| {
                crate::progress::ModelPricing::new("text-embedding-3-small", 0.00002, 0.0)
            });

            let embedding_cost = embed_pricing.calculate_cost(total_embed_tokens, 0);
            stats.cost_usd += embedding_cost;

            if let Some(ref mut breakdown) = stats.cost_breakdown {
                breakdown.embedding_cost_usd = embedding_cost;
                breakdown.embedding_tokens = total_embed_tokens;
            } else {
                let breakdown = CostBreakdownStats {
                    embedding_cost_usd: embedding_cost,
                    embedding_tokens: total_embed_tokens,
                    ..CostBreakdownStats::default()
                };
                stats.cost_breakdown = Some(breakdown);
            }
        }

        stats.processing_time_ms = start.elapsed().as_millis() as u64;

        // Step 4: Build lineage if enabled
        let lineage = if self.config.enable_lineage_tracking {
            let job_id = uuid::Uuid::new_v4().to_string();
            let mut builder = LineageBuilder::new(document_id, document_id, &job_id);

            for chunk in &chunks {
                let metadata =
                    ExtractionMetadata::new(stats.llm_model.as_deref().unwrap_or("unknown"));
                builder.record_chunk(
                    &chunk.id,
                    chunk.index,
                    chunk.start_line,
                    chunk.end_line,
                    chunk.start_offset,
                    chunk.end_offset,
                    metadata,
                );
            }

            for extraction in &extractions {
                for entity in &extraction.entities {
                    let entity_id = format!("{}_{}", extraction.source_chunk_id, entity.name);
                    let span = SourceSpan::new(0, 0, 0, 0);
                    builder.record_entity(
                        &entity_id,
                        &entity.name,
                        &extraction.source_chunk_id,
                        span,
                        &entity.description,
                    );
                }
                for rel in &extraction.relationships {
                    let rel_id = format!(
                        "{}_{}_{}",
                        extraction.source_chunk_id, rel.source, rel.target
                    );
                    let span = SourceSpan::new(0, 0, 0, 0);
                    builder.record_relationship(
                        &rel_id,
                        &rel.source,
                        &rel.target,
                        &rel.relation_type,
                        &extraction.source_chunk_id,
                        span,
                        &rel.description,
                    );
                }
            }

            Some(builder.build())
        } else {
            None
        };

        Ok(ProcessingResult {
            document_id: document_id.to_string(),
            chunks,
            extractions,
            stats,
            lineage,
        })
    }

    // ═══════════════════════════════════════════════════════════════════════════
    //                    RESILIENT DOCUMENT PROCESSING
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // WHY PROCESS_WITH_RESILIENCE?
    // ────────────────────────────────
    // This method uses the resilient extraction strategy to ensure:
    // - Partial failures don't discard successful extractions
    // - Failed chunks are tracked for reporting and potential retry
    // - Users can see exactly which parts of their document were processed
    //
    // DECISION TREE FOR FAILURE HANDLING:
    // ────────────────────────────────────────────────────────────────────────────
    //
    //   After extraction completes:
    //        │
    //        ▼
    //   ┌─────────────────────────────────────────────────────────────────────────┐
    //   │  Is success_rate == 1.0? (100% success)                                │
    //   └───────────────────────────────┬─────────────────────────────────────────┘
    //                │                  │
    //       YES      │                  │ NO
    //                ▼                  ▼
    //   ┌─────────────────┐   ┌─────────────────────────────────────────────────┐
    //   │ Return normal   │   │  Is success_rate > 0.0? (at least 1 success)   │
    //   │ ProcessingResult│   └───────────────────────┬─────────────────────────┘
    //   └─────────────────┘             │             │
    //                          YES      │             │ NO (all failed)
    //                                   ▼             ▼
    //                   ┌───────────────────┐   ┌─────────────────────────────┐
    //                   │ Return partial    │   │ Return error with all       │
    //                   │ result with       │   │ failure details             │
    //                   │ stats.failed_     │   │                             │
    //                   │ chunks populated  │   │ PipelineError::Extraction   │
    //                   └───────────────────┘   │ Error("All N chunks failed")|
    //                                           └─────────────────────────────┘
    //

    /// Process a document with resilient chunk-level error handling.
    ///
    /// Unlike `process` and `process_with_progress`, this method does NOT fail
    /// the entire document if individual chunks fail. Instead, it:
    /// - Extracts as many chunks as possible
    /// - Reports failures in `stats.chunk_errors`
    /// - Allows partial results to be used
    ///
    /// ## Implements
    /// - **FEAT0020**: Chunk-level resilience and error isolation
    /// - **UC2305**: System continues processing when individual chunks fail
    ///
    /// ## Failure Behavior
    /// - If ALL chunks fail, returns `Err(PipelineError::ExtractionError)`
    /// - If SOME chunks fail, returns `Ok(ProcessingResult)` with:
    ///   - `stats.failed_chunks > 0`
    ///   - `stats.chunk_errors` populated with failure details
    ///   - Only successful extractions in `extractions`
    ///
    /// ## Example
    /// ```ignore
    /// let result = pipeline.process_with_resilience("doc1", content, Some(callback)).await?;
    /// if result.stats.failed_chunks > 0 {
    ///     println!("{} chunks failed, processing partial result", result.stats.failed_chunks);
    /// }
    /// ```
    pub async fn process_with_resilience(
        &self,
        document_id: &str,
        content: &str,
        progress_callback: Option<ChunkProgressCallback>,
    ) -> Result<ProcessingResult> {
        let start = std::time::Instant::now();
        let mut stats = ProcessingStats::default();

        // Step 1: Chunk the document
        let mut chunks = self.chunker.chunk(content, document_id)?;
        stats.chunk_count = chunks.len();

        // Track chunking strategy and average chunk size
        stats.chunking_strategy =
            Some(format!("sliding_window_{}", self.config.chunker.chunk_size));
        if !chunks.is_empty() {
            let total_chars: usize = chunks.iter().map(|c| c.content.len()).sum();
            stats.avg_chunk_size = Some(total_chars / chunks.len());
        }

        // Step 2: Extract entities and relationships WITH RESILIENCE
        let mut extractions = Vec::new();
        let mut entity_types_set = std::collections::HashSet::new();
        let mut relationship_types_set = std::collections::HashSet::new();
        let mut keywords_set = std::collections::HashSet::new();
        let mut total_input_tokens = 0usize;
        let mut total_output_tokens = 0usize;

        if self.config.enable_entity_extraction || self.config.enable_relationship_extraction {
            if let Some(extractor) = &self.extractor {
                // Capture LLM model and provider names
                stats.llm_model = Some(extractor.model_name().to_string());
                stats.llm_provider = Some(extractor.provider_name().to_string());

                // ═══════════════════════════════════════════════════════════════
                // USE RESILIENT EXTRACTION (map-reduce pattern)
                // ═══════════════════════════════════════════════════════════════
                let resilient_result = self
                    .resilient_extract_parallel(&chunks, extractor, progress_callback)
                    .await;

                // Log the result summary
                tracing::info!(
                    document_id = %document_id,
                    total_chunks = resilient_result.total_chunks,
                    successful = resilient_result.successful_extractions.len(),
                    failed = resilient_result.failed_chunks.len(),
                    success_rate = %format!("{:.1}%", resilient_result.success_rate() * 100.0),
                    "Resilient extraction completed"
                );

                // ═══════════════════════════════════════════════════════════════
                // HANDLE COMPLETE FAILURE CASE
                // ═══════════════════════════════════════════════════════════════
                // WHY: If ALL chunks failed, there's no useful result to return.
                // Better to return an error with all failure details.
                if resilient_result.is_complete_failure() {
                    let failure_summary: Vec<String> = resilient_result
                        .failed_chunks
                        .iter()
                        .map(|f| format!("Chunk {}: {}", f.chunk_index, f.error))
                        .collect();

                    return Err(crate::error::PipelineError::ExtractionError(format!(
                        "All {} chunks failed extraction. Failures: {}",
                        resilient_result.total_chunks,
                        failure_summary.join("; ")
                    )));
                }

                // ═══════════════════════════════════════════════════════════════
                // POPULATE STATS WITH FAILURE INFO
                // ═══════════════════════════════════════════════════════════════
                stats.successful_chunks = resilient_result.successful_extractions.len();
                stats.failed_chunks = resilient_result.failed_chunks.len();

                if !resilient_result.failed_chunks.is_empty() {
                    stats.chunk_errors = Some(
                        resilient_result
                            .failed_chunks
                            .iter()
                            .map(|f| ChunkErrorInfo {
                                chunk_id: f.chunk_id.clone(),
                                chunk_index: f.chunk_index,
                                error_message: f.error.clone(),
                                was_timeout: f.was_timeout,
                                retry_attempts: f.retry_attempts,
                            })
                            .collect(),
                    );

                    tracing::warn!(
                        document_id = %document_id,
                        failed_count = resilient_result.failed_chunks.len(),
                        "Some chunks failed extraction, continuing with partial results"
                    );
                }

                // Take successful extractions
                extractions = resilient_result.successful_extractions;

                // Link entities and relationships to their source chunks
                for extraction in &mut extractions {
                    let chunk_id = extraction.source_chunk_id.clone();
                    tracing::debug!(
                        "Linking {} entities and {} relationships to chunk {}",
                        extraction.entities.len(),
                        extraction.relationships.len(),
                        chunk_id
                    );
                    for entity in &mut extraction.entities {
                        entity.add_source_chunk_id(&chunk_id);
                    }
                    for rel in &mut extraction.relationships {
                        if rel.source_chunk_id.is_none() {
                            rel.source_chunk_id = Some(chunk_id.clone());
                        }
                    }
                }

                // Aggregate statistics from all successful extractions
                for extraction in &extractions {
                    stats.entity_count += extraction.entities.len();
                    stats.relationship_count += extraction.relationships.len();
                    stats.llm_calls += 1;
                    total_input_tokens += extraction.input_tokens;
                    total_output_tokens += extraction.output_tokens;

                    for entity in &extraction.entities {
                        entity_types_set.insert(entity.entity_type.clone());
                    }
                    for rel in &extraction.relationships {
                        relationship_types_set.insert(rel.relation_type.clone());
                        for keyword in &rel.keywords {
                            keywords_set.insert(keyword.clone());
                        }
                    }
                }

                stats.total_tokens = total_input_tokens + total_output_tokens;
                stats.input_tokens = total_input_tokens;
                stats.output_tokens = total_output_tokens;

                // Calculate extraction cost
                let model_name = extractor.model_name();
                let pricing = crate::progress::default_model_pricing();
                let model_pricing = pricing.get(model_name).cloned().unwrap_or_else(|| {
                    crate::progress::ModelPricing::new("gpt-4.1-nano", 0.00015, 0.0006)
                });

                let extraction_cost =
                    model_pricing.calculate_cost(total_input_tokens, total_output_tokens);
                stats.cost_usd += extraction_cost;

                let cost_breakdown = CostBreakdownStats {
                    extraction_cost_usd: extraction_cost,
                    extraction_input_tokens: total_input_tokens,
                    extraction_output_tokens: total_output_tokens,
                    ..CostBreakdownStats::default()
                };
                stats.cost_breakdown = Some(cost_breakdown);
            }
        }

        // Store collected types and keywords
        if !entity_types_set.is_empty() {
            stats.entity_types = Some(entity_types_set.into_iter().collect());
        }
        if !relationship_types_set.is_empty() {
            stats.relationship_types = Some(relationship_types_set.into_iter().collect());
        }
        if !keywords_set.is_empty() {
            let mut keywords: Vec<String> = keywords_set.into_iter().collect();
            keywords.sort();
            keywords.truncate(50);
            stats.keywords = Some(keywords);
        }

        // Step 3: Generate embeddings (same logic as process_with_progress)
        if let Some(provider) = &self.embedding_provider {
            stats.embedding_model = Some(provider.model().to_string());
            stats.embedding_provider = Some(provider.name().to_string());
            stats.embedding_dimensions = Some(provider.dimension());

            // Chunk embeddings
            if self.config.enable_chunk_embeddings {
                let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
                if !texts.is_empty() {
                    let embeddings = provider
                        .embed(&texts)
                        .await
                        .map_err(|e| crate::error::PipelineError::EmbeddingError(e.to_string()))?;
                    for (chunk, embedding) in chunks.iter_mut().zip(embeddings) {
                        chunk.embedding = Some(embedding);
                    }
                }
            }

            // Entity embeddings
            if self.config.enable_entity_embeddings {
                let mut all_entity_texts: Vec<String> = Vec::new();
                let mut entity_indices: Vec<(usize, usize)> = Vec::new();

                for (ext_idx, extraction) in extractions.iter().enumerate() {
                    for (ent_idx, entity) in extraction.entities.iter().enumerate() {
                        all_entity_texts.push(format!("{}: {}", entity.name, entity.description));
                        entity_indices.push((ext_idx, ent_idx));
                    }
                }

                if !all_entity_texts.is_empty() {
                    let all_embeddings = provider
                        .embed(&all_entity_texts)
                        .await
                        .map_err(|e| crate::error::PipelineError::EmbeddingError(e.to_string()))?;
                    for (embedding, (ext_idx, ent_idx)) in
                        all_embeddings.into_iter().zip(entity_indices)
                    {
                        extractions[ext_idx].entities[ent_idx].embedding = Some(embedding);
                    }
                }
            }

            // Relationship embeddings
            if self.config.enable_relationship_embeddings {
                let mut all_relationship_texts: Vec<String> = Vec::new();
                let mut relationship_indices: Vec<(usize, usize)> = Vec::new();

                for (ext_idx, extraction) in extractions.iter().enumerate() {
                    for (rel_idx, r) in extraction.relationships.iter().enumerate() {
                        all_relationship_texts.push(format!(
                            "{}\t{}->{}\n{}",
                            r.keywords.join(", "),
                            r.source,
                            r.target,
                            r.description
                        ));
                        relationship_indices.push((ext_idx, rel_idx));
                    }
                }

                if !all_relationship_texts.is_empty() {
                    let all_embeddings = provider
                        .embed(&all_relationship_texts)
                        .await
                        .map_err(|e| crate::error::PipelineError::EmbeddingError(e.to_string()))?;
                    for (embedding, (ext_idx, rel_idx)) in
                        all_embeddings.into_iter().zip(relationship_indices)
                    {
                        extractions[ext_idx].relationships[rel_idx].embedding = Some(embedding);
                    }
                }
            }

            // Calculate embedding costs
            let mut total_embed_tokens = 0usize;
            if self.config.enable_chunk_embeddings {
                let chunk_text_len: usize = chunks.iter().map(|c| c.content.len()).sum();
                total_embed_tokens += chunk_text_len / 4;
            }
            if self.config.enable_entity_embeddings {
                for extraction in &extractions {
                    for entity in &extraction.entities {
                        total_embed_tokens += (entity.name.len() + entity.description.len()) / 4;
                    }
                }
            }
            if self.config.enable_relationship_embeddings {
                for extraction in &extractions {
                    for rel in &extraction.relationships {
                        total_embed_tokens +=
                            (rel.source.len() + rel.target.len() + rel.description.len()) / 4;
                    }
                }
            }

            let embed_model_name = provider.model();
            let pricing = crate::progress::default_model_pricing();
            let embed_pricing = pricing.get(embed_model_name).cloned().unwrap_or_else(|| {
                crate::progress::ModelPricing::new("text-embedding-3-small", 0.00002, 0.0)
            });

            let embedding_cost = embed_pricing.calculate_cost(total_embed_tokens, 0);
            stats.cost_usd += embedding_cost;

            if let Some(ref mut breakdown) = stats.cost_breakdown {
                breakdown.embedding_cost_usd = embedding_cost;
                breakdown.embedding_tokens = total_embed_tokens;
            } else {
                let breakdown = CostBreakdownStats {
                    embedding_cost_usd: embedding_cost,
                    embedding_tokens: total_embed_tokens,
                    ..CostBreakdownStats::default()
                };
                stats.cost_breakdown = Some(breakdown);
            }
        }

        stats.processing_time_ms = start.elapsed().as_millis() as u64;

        // Step 4: Build lineage if enabled
        let lineage = if self.config.enable_lineage_tracking {
            let job_id = uuid::Uuid::new_v4().to_string();
            let mut builder = LineageBuilder::new(document_id, document_id, &job_id);

            for chunk in &chunks {
                let metadata =
                    ExtractionMetadata::new(stats.llm_model.as_deref().unwrap_or("unknown"));
                builder.record_chunk(
                    &chunk.id,
                    chunk.index,
                    chunk.start_line,
                    chunk.end_line,
                    chunk.start_offset,
                    chunk.end_offset,
                    metadata,
                );
            }

            for extraction in &extractions {
                for entity in &extraction.entities {
                    let entity_id = format!("{}_{}", extraction.source_chunk_id, entity.name);
                    let span = SourceSpan::new(0, 0, 0, 0);
                    builder.record_entity(
                        &entity_id,
                        &entity.name,
                        &extraction.source_chunk_id,
                        span,
                        &entity.description,
                    );
                }
                for rel in &extraction.relationships {
                    let rel_id = format!(
                        "{}_{}_{}",
                        extraction.source_chunk_id, rel.source, rel.target
                    );
                    let span = SourceSpan::new(0, 0, 0, 0);
                    builder.record_relationship(
                        &rel_id,
                        &rel.source,
                        &rel.target,
                        &rel.relation_type,
                        &extraction.source_chunk_id,
                        span,
                        &rel.description,
                    );
                }
            }

            Some(builder.build())
        } else {
            None
        };

        // FIX-2: Validate processing results before returning Ok
        // WHY: Prevent silent failures where Ok() returned but no entities extracted
        // CRITICAL: This ensures pipeline failures are visible to caller (processor.rs)
        // SCENARIO: Chunks created successfully but LLM extraction returns 0 entities
        if stats.chunk_count == 0 {
            return Err(crate::error::PipelineError::ChunkingError(
                "Document chunking produced 0 chunks - content may be empty or malformed"
                    .to_string(),
            ));
        }

        // Note: complete chunk failure already caught at line 1650
        // This check handles: "some chunks succeeded but extracted 0 entities"
        if stats.entity_count == 0 && stats.chunk_count > 0 {
            tracing::warn!(
                document_id = document_id,
                chunk_count = stats.chunk_count,
                successful_chunks = stats.successful_chunks,
                failed_chunks = stats.failed_chunks,
                "Pipeline processed {} chunks but extracted 0 entities - possible LLM failure or content without extractable entities",
                stats.chunk_count
            );

            // Return error to trigger "failed" status instead of "completed"
            return Err(crate::error::PipelineError::ExtractionError(
                format!(
                    "Extracted 0 entities from {} chunks ({} succeeded, {} failed) - document cannot be indexed",
                    stats.chunk_count, stats.successful_chunks, stats.failed_chunks
                )
            ));
        }

        Ok(ProcessingResult {
            document_id: document_id.to_string(),
            chunks,
            extractions,
            stats,
            lineage,
        })
    }

    /// Process multiple documents in parallel.
    ///
    /// Uses concurrent processing with a configurable limit based on
    /// `max_concurrent_extractions` to process multiple documents simultaneously.
    pub async fn process_batch(
        &self,
        documents: &[(String, String)],
    ) -> Result<Vec<ProcessingResult>> {
        // Use the same concurrency limit as extraction for document processing
        let max_concurrent_docs = self.config.max_concurrent_extractions.max(4);

        // Create futures for all documents
        let futures: Vec<_> = documents
            .iter()
            .map(|(doc_id, content)| self.process(doc_id, content))
            .collect();

        // Execute concurrently with buffer to limit parallelism
        let results: Vec<Result<ProcessingResult>> = stream::iter(futures)
            .buffer_unordered(max_concurrent_docs)
            .collect()
            .await;

        // Collect results, propagating first error
        results.into_iter().collect()
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
        assert!(!config.enable_lineage_tracking);
    }

    #[tokio::test]
    async fn test_pipeline_with_lineage_tracking() {
        let extractor = Arc::new(SimpleExtractor::default());
        let mut config = PipelineConfig::default();
        config.enable_lineage_tracking = true;

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
        let pipeline = Pipeline::default_pipeline();

        let result = pipeline
            .process("doc-1", "Simple document content.")
            .await
            .unwrap();

        // Should not have lineage (disabled by default)
        assert!(result.lineage.is_none());
    }
}
