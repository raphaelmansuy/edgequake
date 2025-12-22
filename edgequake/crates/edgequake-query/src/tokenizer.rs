//! Tokenization for context management.
//!
//! This module provides tokenization functionality to manage LLM context windows
//! and ensure we don't exceed token limits.

/// Trait for tokenization.
pub trait Tokenizer: Send + Sync {
    /// Encode text into tokens.
    fn encode(&self, text: &str) -> Vec<u32>;

    /// Decode tokens back to text.
    fn decode(&self, tokens: &[u32]) -> String;

    /// Count tokens in text (convenience method).
    fn count_tokens(&self, text: &str) -> usize {
        self.encode(text).len()
    }
}

/// Simple tokenizer that estimates tokens (for testing and fallback).
/// Uses a simple heuristic: ~4 characters per token.
pub struct SimpleTokenizer;

impl SimpleTokenizer {
    /// Create a new simple tokenizer.
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tokenizer for SimpleTokenizer {
    fn encode(&self, text: &str) -> Vec<u32> {
        // Simple estimation: split by whitespace and punctuation
        let estimated_tokens = (text.len() as f32 / 4.0).ceil() as usize;
        (0..estimated_tokens).map(|i| i as u32).collect()
    }

    fn decode(&self, _tokens: &[u32]) -> String {
        // Simple tokenizer doesn't support actual decoding
        String::from("[decoded text]")
    }

    fn count_tokens(&self, text: &str) -> usize {
        // Heuristic: ~4 characters per token (GPT average)
        // Also count words as minimum
        let char_estimate = (text.len() as f32 / 4.0).ceil() as usize;
        let word_count = text.split_whitespace().count();
        char_estimate.max(word_count)
    }
}

/// Mock tokenizer for testing with configurable token counts.
pub struct MockTokenizer {
    tokens_per_char: f32,
}

impl MockTokenizer {
    /// Create a new mock tokenizer.
    pub fn new() -> Self {
        Self {
            tokens_per_char: 0.25, // Default: 4 chars per token
        }
    }

    /// Create with custom token rate.
    pub fn with_rate(tokens_per_char: f32) -> Self {
        Self { tokens_per_char }
    }
}

impl Default for MockTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tokenizer for MockTokenizer {
    fn encode(&self, text: &str) -> Vec<u32> {
        let token_count = (text.len() as f32 * self.tokens_per_char).ceil() as usize;
        (0..token_count).map(|i| i as u32).collect()
    }

    fn decode(&self, _tokens: &[u32]) -> String {
        String::from("[mock decoded]")
    }

    fn count_tokens(&self, text: &str) -> usize {
        (text.len() as f32 * self.tokens_per_char).ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokenizer_count() {
        let tokenizer = SimpleTokenizer::new();

        // Short text
        let count = tokenizer.count_tokens("Hello world");
        assert!(count > 0);
        assert!(count < 10);

        // Longer text
        let long_text = "This is a much longer piece of text that should have more tokens";
        let long_count = tokenizer.count_tokens(long_text);
        assert!(long_count > count);
    }

    #[test]
    fn test_simple_tokenizer_encode_decode() {
        let tokenizer = SimpleTokenizer::new();

        let tokens = tokenizer.encode("test");
        assert!(tokens.len() > 0);

        let decoded = tokenizer.decode(&tokens);
        assert!(!decoded.is_empty());
    }

    #[test]
    fn test_mock_tokenizer_custom_rate() {
        let tokenizer = MockTokenizer::with_rate(0.5); // 2 chars per token

        let count = tokenizer.count_tokens("test"); // 4 chars = 2 tokens
        assert_eq!(count, 2);
    }

    #[test]
    fn test_mock_tokenizer_default() {
        let tokenizer = MockTokenizer::default();

        // 4 chars per token by default
        let count = tokenizer.count_tokens("test"); // 4 chars = 1 token
        assert_eq!(count, 1);
    }

    #[test]
    fn test_tokenizer_trait() {
        fn test_tokenizer<T: Tokenizer>(tokenizer: &T) {
            let count = tokenizer.count_tokens("hello");
            assert!(count > 0);
        }

        test_tokenizer(&SimpleTokenizer::new());
        test_tokenizer(&MockTokenizer::new());
    }
}
