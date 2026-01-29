//! Pipeline error types.
//!
//! @implements SPEC-001/Issue-8: Comprehensive error handling for extraction
//!
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                    CHUNK-LEVEL RESILIENCE ARCHITECTURE                       │
//! └─────────────────────────────────────────────────────────────────────────────┘
//!
//! WHY CHUNK-LEVEL RESILIENCE?
//! ─────────────────────────────
//! In a document with N chunks, we want to extract as much knowledge as possible.
//! If chunk #7 fails (timeout, rate limit, malformed JSON), we should NOT lose
//! the extractions from chunks 1-6 and 8-N.
//!
//! FIRST PRINCIPLES:
//! ─────────────────
//! 1. ISOLATION: Each chunk is independent; failure in one doesn't affect others
//! 2. TRANSPARENCY: Caller must know which chunks succeeded/failed for:
//!    - Retry decisions
//!    - Partial result handling
//!    - Error reporting to users
//! 3. GRACEFUL DEGRADATION: Better to extract 90% of entities than 0%
//!
//! DATA FLOW:
//! ─────────────────────────────────────────────────────────────────────────────
//!
//!   Document
//!      │
//!      ▼
//!   ┌──────────────────────────────────────────────────────────────────────┐
//!   │                        MAP PHASE (Parallel)                          │
//!   │                                                                      │
//!   │  ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐              │
//!   │  │ Chunk 1 │   │ Chunk 2 │   │ Chunk 3 │   │ Chunk N │              │
//!   │  └────┬────┘   └────┬────┘   └────┬────┘   └────┬────┘              │
//!   │       │             │             │             │                   │
//!   │       ▼             ▼             ▼             ▼                   │
//!   │  ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐              │
//!   │  │ Extract │   │ Extract │   │ Extract │   │ Extract │              │
//!   │  │ + Retry │   │ + Retry │   │ + Retry │   │ + Retry │              │
//!   │  └────┬────┘   └────┬────┘   └────┬────┘   └────┬────┘              │
//!   │       │             │             │             │                   │
//!   │       ▼             ▼             ▼             ▼                   │
//!   │  ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐              │
//!   │  │ Success │   │ FAILED  │   │ Success │   │ Success │              │
//!   │  └─────────┘   └─────────┘   └─────────┘   └─────────┘              │
//!   └──────────────────────────────────────────────────────────────────────┘
//!      │             │             │             │
//!      ▼             ▼             ▼             ▼
//!   ┌──────────────────────────────────────────────────────────────────────┐
//!   │                      REDUCE PHASE (Aggregate)                        │
//!   │                                                                      │
//!   │  Successes:  [Chunk1, Chunk3, ChunkN] → Merged ExtractionResult      │
//!   │  Failures:   [Chunk2 → ChunkFailure]  → PartialFailureInfo           │
//!   │                                                                      │
//!   │  Stats: 3/4 chunks succeeded (75%), 1 failure logged for retry       │
//!   └──────────────────────────────────────────────────────────────────────┘
//!
//! USAGE EXAMPLE:
//! ─────────────────
//! ```ignore
//! let outcomes = pipeline.resilient_extract_parallel(&chunks, extractor).await;
//! let (successes, failures): (Vec<_>, Vec<_>) = outcomes
//!     .into_iter()
//!     .partition(|o| o.is_success());
//!
//! if !failures.is_empty() {
//!     tracing::warn!("{} chunks failed, {} succeeded", failures.len(), successes.len());
//! }
//! ```

use thiserror::Error;

use crate::extractor::ExtractionResult;

/// Result type for pipeline operations.
pub type Result<T> = std::result::Result<T, PipelineError>;

// ═══════════════════════════════════════════════════════════════════════════════
//                         CHUNK EXTRACTION OUTCOME TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// Information about a failed chunk extraction.
///
/// WHY THIS TYPE?
/// ───────────────
/// When a chunk fails, we need to capture enough context to:
/// 1. Log meaningful error messages for debugging
/// 2. Allow retry of specific failed chunks later
/// 3. Report to users which parts of their document were not processed
/// 4. Track failure patterns for circuit breaker logic
#[derive(Debug, Clone)]
pub struct ChunkFailure {
    /// Index of the chunk that failed (0-based).
    pub chunk_index: usize,
    /// ID of the chunk (for correlation with source document).
    pub chunk_id: String,
    /// The error that caused the failure.
    pub error: String,
    /// Number of retry attempts made before giving up.
    pub retry_attempts: u32,
    /// Whether the failure was due to timeout vs other errors.
    pub was_timeout: bool,
    /// Processing time in milliseconds (including retries).
    pub processing_time_ms: u64,
}

/// Outcome of attempting to extract entities from a single chunk.
///
/// WHY AN ENUM INSTEAD OF Result<ExtractionResult, ChunkFailure>?
/// ─────────────────────────────────────────────────────────────────
/// 1. SEMANTIC CLARITY: "Outcome" conveys that both paths are expected
/// 2. PATTERN MATCHING: Easier to work with in reduce phase
/// 3. EXTENSION: Can add more variants later (e.g., Skipped, Cached)
///
/// ```text
///                      ChunkExtractionOutcome
///                              │
///               ┌──────────────┼──────────────┐
///               ▼              ▼              ▼
///           Success        Failed         (Future: Cached)
///              │              │
///              ▼              ▼
///     ExtractionResult   ChunkFailure
/// ```
#[derive(Debug)]
pub enum ChunkExtractionOutcome {
    /// Extraction succeeded for this chunk.
    Success {
        /// The chunk index (0-based).
        chunk_index: usize,
        /// The extraction result containing entities and relationships.
        result: ExtractionResult,
    },
    /// Extraction failed for this chunk after all retries.
    Failed(ChunkFailure),
}

impl ChunkExtractionOutcome {
    /// Returns true if this outcome represents a successful extraction.
    #[inline]
    pub fn is_success(&self) -> bool {
        matches!(self, ChunkExtractionOutcome::Success { .. })
    }

    /// Returns true if this outcome represents a failed extraction.
    #[inline]
    pub fn is_failure(&self) -> bool {
        matches!(self, ChunkExtractionOutcome::Failed(_))
    }

    /// Get the chunk index for this outcome.
    pub fn chunk_index(&self) -> usize {
        match self {
            ChunkExtractionOutcome::Success { chunk_index, .. } => *chunk_index,
            ChunkExtractionOutcome::Failed(failure) => failure.chunk_index,
        }
    }

    /// Try to get the extraction result if successful.
    pub fn as_result(&self) -> Option<&ExtractionResult> {
        match self {
            ChunkExtractionOutcome::Success { result, .. } => Some(result),
            ChunkExtractionOutcome::Failed(_) => None,
        }
    }

    /// Consume and return the extraction result if successful.
    pub fn into_result(self) -> Option<ExtractionResult> {
        match self {
            ChunkExtractionOutcome::Success { result, .. } => Some(result),
            ChunkExtractionOutcome::Failed(_) => None,
        }
    }

    /// Try to get the failure info if failed.
    pub fn as_failure(&self) -> Option<&ChunkFailure> {
        match self {
            ChunkExtractionOutcome::Success { .. } => None,
            ChunkExtractionOutcome::Failed(failure) => Some(failure),
        }
    }
}

/// Aggregated results from resilient parallel extraction.
///
/// WHY THIS STRUCT?
/// ────────────────
/// After the reduce phase, we need a clear summary of:
/// 1. All successful extractions (for graph building)
/// 2. All failures (for error reporting and potential retry)
/// 3. Statistics (for monitoring and alerting)
///
/// ```text
///   ┌─────────────────────────────────────────────────────┐
///   │            ResilientExtractionResult                │
///   ├─────────────────────────────────────────────────────┤
///   │  successful_extractions: Vec<ExtractionResult>      │
///   │  failed_chunks: Vec<ChunkFailure>                   │
///   │  total_chunks: usize                                │
///   │  success_rate: f64 (e.g., 0.85 = 85%)               │
///   │  total_processing_time_ms: u64                      │
///   └─────────────────────────────────────────────────────┘
/// ```
#[derive(Debug)]
pub struct ResilientExtractionResult {
    /// Successfully extracted results (ordered by chunk index).
    pub successful_extractions: Vec<ExtractionResult>,
    /// Chunks that failed extraction after all retries.
    pub failed_chunks: Vec<ChunkFailure>,
    /// Total number of chunks attempted.
    pub total_chunks: usize,
    /// Total processing time including all retries (milliseconds).
    pub total_processing_time_ms: u64,
}

impl ResilientExtractionResult {
    /// Create a new resilient extraction result from outcomes.
    ///
    /// This implements the REDUCE phase of the map-reduce pattern:
    /// 1. Separate successes from failures
    /// 2. Sort by chunk index to maintain document order
    /// 3. Calculate aggregate statistics
    pub fn from_outcomes(outcomes: Vec<ChunkExtractionOutcome>) -> Self {
        let total_chunks = outcomes.len();
        let mut successful_extractions = Vec::new();
        let mut failed_chunks = Vec::new();
        let mut total_processing_time_ms = 0u64;

        for outcome in outcomes {
            match outcome {
                ChunkExtractionOutcome::Success { result, .. } => {
                    total_processing_time_ms += result.extraction_time_ms;
                    successful_extractions.push(result);
                }
                ChunkExtractionOutcome::Failed(failure) => {
                    total_processing_time_ms += failure.processing_time_ms;
                    failed_chunks.push(failure);
                }
            }
        }

        // Sort by source_chunk_id to maintain document order
        // This is important for lineage tracking
        successful_extractions.sort_by(|a, b| a.source_chunk_id.cmp(&b.source_chunk_id));
        failed_chunks.sort_by_key(|f| f.chunk_index);

        Self {
            successful_extractions,
            failed_chunks,
            total_chunks,
            total_processing_time_ms,
        }
    }

    /// Calculate the success rate as a percentage (0.0 to 1.0).
    pub fn success_rate(&self) -> f64 {
        if self.total_chunks == 0 {
            1.0 // Empty document is technically 100% success
        } else {
            self.successful_extractions.len() as f64 / self.total_chunks as f64
        }
    }

    /// Returns true if all chunks were successfully extracted.
    pub fn is_complete_success(&self) -> bool {
        self.failed_chunks.is_empty()
    }

    /// Returns true if at least one chunk was extracted successfully.
    pub fn has_any_success(&self) -> bool {
        !self.successful_extractions.is_empty()
    }

    /// Returns true if all chunks failed.
    pub fn is_complete_failure(&self) -> bool {
        self.successful_extractions.is_empty() && !self.failed_chunks.is_empty()
    }

    /// Get a summary string for logging.
    pub fn summary(&self) -> String {
        format!(
            "{}/{} chunks succeeded ({:.1}%), {} failed, {:.1}s total",
            self.successful_extractions.len(),
            self.total_chunks,
            self.success_rate() * 100.0,
            self.failed_chunks.len(),
            self.total_processing_time_ms as f64 / 1000.0
        )
    }
}

/// Errors that can occur during pipeline processing.
#[derive(Debug, Error)]
pub enum PipelineError {
    /// Error during document processing.
    #[error("Document processing error: {0}")]
    DocumentError(String),

    /// Error during chunking.
    #[error("Chunking error: {0}")]
    ChunkingError(String),

    /// Error during entity extraction.
    #[error("Entity extraction error: {0}")]
    ExtractionError(String),

    /// Error during embedding generation.
    #[error("Embedding error: {0}")]
    EmbeddingError(String),

    /// Error during graph operations.
    #[error("Graph error: {0}")]
    GraphError(String),

    /// Storage error.
    #[error("Storage error: {0}")]
    StorageError(#[from] edgequake_storage::error::StorageError),

    /// LLM error.
    #[error("LLM error: {0}")]
    LlmError(#[from] edgequake_llm::error::LlmError),

    /// Invalid configuration.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Document not found.
    #[error("Document not found: {0}")]
    NotFound(String),

    /// Invalid document format.
    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    /// Extraction timeout error.
    ///
    /// @implements SPEC-001/Issue-8: Timeout handling
    ///
    /// WHY: LLM calls can hang indefinitely. This error indicates the
    /// extraction exceeded the configured timeout and was aborted.
    #[error("Extraction timeout after {timeout_secs}s for chunk {chunk_index}: {message}")]
    ExtractionTimeout {
        /// Chunk index that timed out.
        chunk_index: usize,
        /// Configured timeout in seconds.
        timeout_secs: u64,
        /// Additional context message.
        message: String,
    },

    /// Retry limit exhausted.
    ///
    /// @implements SPEC-001/Issue-8: Retry limit handling
    ///
    /// WHY: After N retry attempts, we stop retrying to prevent infinite loops.
    /// This error provides visibility into how many retries were attempted.
    #[error("Extraction failed after {attempts} retries for chunk {chunk_index}: {message}")]
    RetryExhausted {
        /// Chunk index that failed.
        chunk_index: usize,
        /// Number of attempts made.
        attempts: u32,
        /// Last error message.
        message: String,
    },

    /// Circuit breaker open - LLM provider is failing.
    ///
    /// @implements SPEC-001/Issue-8: Circuit breaker pattern
    ///
    /// WHY: When the LLM provider is having issues (rate limits, outages),
    /// we should stop hammering it and fail fast. The circuit breaker
    /// opens after too many consecutive failures.
    #[error("Circuit breaker open: LLM provider is unavailable. {failures} consecutive failures. Retry after {retry_after_secs}s")]
    CircuitBreakerOpen {
        /// Number of consecutive failures.
        failures: u32,
        /// Seconds until next retry allowed.
        retry_after_secs: u64,
    },

    /// Document validation error.
    ///
    /// @implements SPEC-001/Issue-13: Comprehensive edge case handling
    ///
    /// WHY: Documents must be validated before processing to catch edge cases
    /// early and provide clear error messages to users.
    #[error("Validation error: {0}")]
    Validation(String),
}
