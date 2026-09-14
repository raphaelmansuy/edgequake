//! Task failure information types.
//!
//! Structured error information for failed tasks, including
//! factory methods for common failure categories and circuit
//! breaker timeout detection.

use serde::{Deserialize, Serialize};

/// Detailed error information for failed tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFailureInfo {
    /// Human-readable error message.
    pub message: String,
    /// Processing step where the error occurred.
    pub step: String,
    /// Technical reason for the error.
    pub reason: String,
    /// Suggested action to resolve the error.
    pub suggestion: String,
    /// Whether this error is retryable.
    pub retryable: bool,
    /// Whether real progress was made before this timeout (vision stall watchdog).
    ///
    /// When `true`, the circuit breaker must NOT advance toward permanent failure —
    /// the attempt progressed and checkpoints can resume. Only no-progress hangs trip
    /// the breaker. Defaults to `false` for backward-compatible deserialization.
    #[serde(default)]
    pub made_progress: bool,
}

impl TaskFailureInfo {
    /// Create a new task error.
    pub fn new(
        message: impl Into<String>,
        step: impl Into<String>,
        reason: impl Into<String>,
        suggestion: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            message: message.into(),
            step: step.into(),
            reason: reason.into(),
            suggestion: suggestion.into(),
            retryable,
            made_progress: false,
        }
    }

    /// Attach progress-aware flag (vision stall / checkpoint resume).
    pub fn with_made_progress(mut self, made_progress: bool) -> Self {
        self.made_progress = made_progress;
        self
    }

    /// Create a chunking error.
    pub fn chunking(reason: impl Into<String>) -> Self {
        Self::new(
            "Document chunking failed",
            "chunking",
            reason,
            "Check document format and encoding",
            true,
        )
    }

    /// Create a timeout error (LLM or embedding).
    ///
    /// @implements CIRCUIT_BREAKER: Timeout classification
    ///
    /// WHY: Timeouts need special handling via circuit breaker pattern.
    /// Consecutive **no-progress** timeouts trip the breaker (which sets
    /// `retryable = false`). Until then, timeouts remain retryable so Vision
    /// stall watchdog can resume from durable checkpoints (`max_retries`).
    pub fn timeout(step: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::new(
            "Operation timed out",
            step,
            reason,
            "Document may be too large. Try: 1) Use smaller chunk size, 2) Split document, 3) Use provider with longer timeout",
            true, // Retryable until circuit breaker trips
        )
    }

    /// Check if this error represents a timeout.
    ///
    /// X-30: typed detection only — the [`Self::timeout`] factory sets a fixed
    /// message. Business text containing the word "timeout" must not trip the breaker.
    pub fn is_timeout(&self) -> bool {
        self.message == "Operation timed out"
    }

    /// Create an embedding error.
    pub fn embedding(reason: impl Into<String>) -> Self {
        Self::new(
            "Embedding generation failed",
            "embedding",
            reason,
            "Check LLM provider connectivity and API limits",
            true,
        )
    }

    /// Create an extraction error.
    pub fn extraction(reason: impl Into<String>) -> Self {
        Self::new(
            "Entity extraction failed",
            "extraction",
            reason,
            "Check LLM provider connectivity and API limits",
            true,
        )
    }

    /// Create an indexing error.
    pub fn indexing(reason: impl Into<String>) -> Self {
        Self::new(
            "Graph indexing failed",
            "indexing",
            reason,
            "Check storage backend connectivity",
            true,
        )
    }

    /// Create a rate limit error.
    pub fn rate_limit(step: impl Into<String>) -> Self {
        Self::new(
            "Rate limit exceeded",
            step,
            "API rate limit exceeded",
            "Wait 30 seconds and retry, or reduce batch size",
            true,
        )
    }

    /// Build structured failure from a raw processing error (SPEC-045 SSOT).
    pub fn from_processing_error(message: impl Into<String>) -> Self {
        use crate::ingestion_reliability::{
            classify_ingestion_failure, failure_step, is_permanent_ingestion_failure,
            IngestionFailureClass,
        };

        let message = message.into();
        let class = classify_ingestion_failure(&message);
        let step = failure_step(class);
        let retryable = !is_permanent_ingestion_failure(&message);
        let suggestion = class.recommended_action();
        // Progress-aware circuit breaker: vision stall watchdog embeds this marker.
        let made_progress = message.contains("[vision_progress=1]");

        // X-30: TimeoutPhase* must use the timeout() factory so `is_timeout()`
        // is true (fixed message) while `reason` preserves the raw payload.
        if matches!(
            class,
            IngestionFailureClass::TimeoutPhaseConvert | IngestionFailureClass::TimeoutPhaseExtract
        ) {
            return Self::timeout(step, message).with_made_progress(made_progress);
        }

        Self::new(message.clone(), step, message, suggestion, retryable)
            .with_made_progress(made_progress)
    }
}
