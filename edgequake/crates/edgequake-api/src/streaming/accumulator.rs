//! Stream accumulator for collecting streaming response chunks.
//!
//! This module provides accurate tracking of streaming response content,
//! metadata, and token usage - replacing the incorrect chunk counting.
//!
//! ## Implements
//!
//! - [`FEAT0483`]: Content accumulation from chunks
//! - [`FEAT0484`]: API response metadata extraction
//! - [`FEAT0485`]: Token usage statistics
//!
//! ## Use Cases
//!
//! - [`UC2082`]: System accumulates streaming content
//! - [`UC2083`]: System extracts finish reason and model info
//!
//! ## Enforces
//!
//! - [`BR0483`]: Timing tracking from first chunk
//! - [`BR0484`]: Proper token counting from API metadata

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Metadata extracted from the final API response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiResponseMetadata {
    /// The model that generated the response
    pub model: Option<String>,

    /// API response ID (e.g., chatcmpl-xxx)
    pub response_id: Option<String>,

    /// Reason the generation stopped
    pub finish_reason: Option<String>,

    /// Token usage from the API
    pub usage: Option<TokenUsage>,
}

/// Token usage statistics from the API.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Tokens in the prompt
    pub prompt_tokens: u32,

    /// Tokens generated in completion
    pub completion_tokens: u32,

    /// Total tokens (prompt + completion)
    pub total_tokens: u32,

    /// Reasoning tokens (for o1/o3 models)
    pub reasoning_tokens: Option<u32>,
}

/// Accumulates streaming response chunks with proper tracking.
///
/// This struct properly tracks content, metadata, and timing information
/// during streaming, rather than incorrectly counting chunks as tokens.
///
/// # Example
///
/// ```rust
/// use edgequake_api::streaming::StreamAccumulator;
///
/// let mut acc = StreamAccumulator::new();
///
/// // Append chunks as they arrive
/// acc.append_content("Hello ");
/// acc.append_content("world!");
///
/// // Get estimated tokens (uses 4 chars/token heuristic)
/// let tokens = acc.estimated_tokens();
///
/// // Or use actual API-provided usage if available
/// // acc.set_usage(usage_from_api);
/// ```
#[derive(Debug)]
pub struct StreamAccumulator {
    /// Accumulated content from all chunks
    content: String,

    /// Number of chunks received
    chunk_count: u32,

    /// Accumulated character count (for progress)
    char_count: u32,

    /// Start time for duration tracking
    start_time: Instant,

    /// First chunk timestamp (for TTFT - time to first token)
    first_chunk_time: Option<Instant>,

    /// API response metadata (populated from final chunk if available)
    metadata: ApiResponseMetadata,

    /// Whether streaming has completed
    is_complete: bool,
}

impl StreamAccumulator {
    /// Create a new accumulator.
    pub fn new() -> Self {
        Self {
            content: String::with_capacity(4096), // Pre-allocate for typical response
            chunk_count: 0,
            char_count: 0,
            start_time: Instant::now(),
            first_chunk_time: None,
            metadata: ApiResponseMetadata::default(),
            is_complete: false,
        }
    }

    /// Append a content chunk.
    pub fn append_content(&mut self, chunk: &str) {
        if self.first_chunk_time.is_none() {
            self.first_chunk_time = Some(Instant::now());
        }

        self.content.push_str(chunk);
        self.chunk_count += 1;
        self.char_count += chunk.len() as u32;
    }

    /// Set metadata from the API response.
    ///
    /// This should be called when the API provides usage information,
    /// typically in the final chunk or a separate usage chunk.
    pub fn set_metadata(&mut self, metadata: ApiResponseMetadata) {
        self.metadata = metadata;
    }

    /// Update token usage from API response.
    pub fn set_usage(&mut self, usage: TokenUsage) {
        self.metadata.usage = Some(usage);
    }

    /// Mark streaming as complete.
    pub fn complete(&mut self, finish_reason: Option<String>) {
        self.is_complete = true;
        if finish_reason.is_some() {
            self.metadata.finish_reason = finish_reason;
        }
    }

    /// Get the accumulated content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the owned content (consumes accumulator).
    pub fn into_content(self) -> String {
        self.content
    }

    /// Get the duration since start.
    pub fn duration_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// Get time to first token (if received).
    pub fn ttft_ms(&self) -> Option<u64> {
        self.first_chunk_time
            .map(|t| t.duration_since(self.start_time).as_millis() as u64)
    }

    /// Get chunk count (NOT token count).
    pub fn chunk_count(&self) -> u32 {
        self.chunk_count
    }

    /// Estimate token count from content.
    ///
    /// Uses a simple heuristic: ~4 characters per token for English text.
    /// This is reasonably accurate for GPT models.
    ///
    /// For accurate token counts, use the API-provided usage when available.
    pub fn estimated_tokens(&self) -> u32 {
        // Prefer API-provided usage
        if let Some(ref usage) = self.metadata.usage {
            return usage.completion_tokens;
        }

        // Fallback: estimate ~4 chars per token (English average)
        (self.char_count / 4).max(1)
    }

    /// Get actual tokens from API usage (if available).
    pub fn actual_tokens(&self) -> Option<u32> {
        self.metadata.usage.as_ref().map(|u| u.completion_tokens)
    }

    /// Get the metadata.
    pub fn metadata(&self) -> &ApiResponseMetadata {
        &self.metadata
    }

    /// Get current content length.
    pub fn content_len(&self) -> usize {
        self.content.len()
    }

    /// Check if streaming is complete.
    pub fn is_complete(&self) -> bool {
        self.is_complete
    }
}

impl Default for StreamAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulator_basic() {
        let mut acc = StreamAccumulator::new();

        acc.append_content("Hello ");
        acc.append_content("world!");

        assert_eq!(acc.content(), "Hello world!");
        assert_eq!(acc.chunk_count(), 2);
        assert_eq!(acc.char_count, 12);
    }

    #[test]
    fn test_estimated_tokens() {
        let mut acc = StreamAccumulator::new();

        // 100 characters ≈ 25 tokens (4 chars per token)
        acc.append_content(&"a".repeat(100));

        assert_eq!(acc.estimated_tokens(), 25);
    }

    #[test]
    fn test_actual_tokens_preferred() {
        let mut acc = StreamAccumulator::new();

        acc.append_content(&"a".repeat(100)); // Would estimate 25 tokens

        acc.set_usage(TokenUsage {
            prompt_tokens: 10,
            completion_tokens: 50, // Actual count
            total_tokens: 60,
            reasoning_tokens: None,
        });

        // Should use actual count, not estimate
        assert_eq!(acc.estimated_tokens(), 50);
        assert_eq!(acc.actual_tokens(), Some(50));
    }

    #[test]
    fn test_ttft_tracking() {
        let mut acc = StreamAccumulator::new();

        assert!(acc.ttft_ms().is_none());

        std::thread::sleep(std::time::Duration::from_millis(10));
        acc.append_content("First");

        let ttft = acc.ttft_ms().unwrap();
        assert!(ttft >= 10);
    }

    #[test]
    fn test_content_length() {
        let mut acc = StreamAccumulator::new();

        acc.append_content("Hello");
        assert_eq!(acc.content_len(), 5);

        acc.append_content(" world");
        assert_eq!(acc.content_len(), 11);
    }

    #[test]
    fn test_completion_state() {
        let mut acc = StreamAccumulator::new();
        assert!(!acc.is_complete());

        acc.complete(Some("stop".to_string()));
        assert!(acc.is_complete());
        assert_eq!(acc.metadata().finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_metadata_setting() {
        let mut acc = StreamAccumulator::new();

        acc.set_metadata(ApiResponseMetadata {
            model: Some("gpt-4o-mini".to_string()),
            response_id: Some("chatcmpl-123".to_string()),
            finish_reason: None,
            usage: None,
        });

        assert_eq!(acc.metadata().model, Some("gpt-4o-mini".to_string()));
        assert_eq!(acc.metadata().response_id, Some("chatcmpl-123".to_string()));
    }

    #[test]
    fn test_duration_tracking() {
        let acc = StreamAccumulator::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(acc.duration_ms() >= 10);
    }

    #[test]
    fn test_into_content() {
        let mut acc = StreamAccumulator::new();
        acc.append_content("Hello world");

        let content = acc.into_content();
        assert_eq!(content, "Hello world");
    }

    #[test]
    fn test_empty_accumulator() {
        let acc = StreamAccumulator::new();
        assert_eq!(acc.content(), "");
        assert_eq!(acc.chunk_count(), 0);
        assert_eq!(acc.estimated_tokens(), 1); // Min 1 token
        assert_eq!(acc.content_len(), 0);
    }
}
