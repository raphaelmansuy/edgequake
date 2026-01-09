//! Token counting utilities.
//!
//! ## Implements
//!
//! - **FEAT0777**: Token counting with tiktoken
//! - **FEAT0778**: Model-specific tokenizer selection
//!
//! ## Enforces
//!
//! - **BR0302**: Token count respects context window limits
//! - **BR0777**: Fallback to cl100k_base for unknown models

use tiktoken_rs::{cl100k_base, o200k_base, CoreBPE};

/// Tokenizer for counting tokens in text.
pub struct Tokenizer {
    encoder: CoreBPE,
    model: String,
}

impl Tokenizer {
    /// Create a tokenizer for a specific model.
    ///
    /// Falls back to cl100k_base (GPT-4/3.5 tokenizer) if model is unknown.
    pub fn for_model(model: &str) -> Self {
        let encoder = match model {
            // GPT-4o and newer use o200k
            m if m.contains("gpt-4o") || m.contains("o1") || m.contains("o3") => {
                o200k_base().expect("Failed to load o200k tokenizer")
            }
            // GPT-4 and GPT-3.5 use cl100k
            m if m.contains("gpt-4") || m.contains("gpt-3.5") => {
                cl100k_base().expect("Failed to load cl100k tokenizer")
            }
            // Embedding models
            m if m.contains("text-embedding") => {
                cl100k_base().expect("Failed to load cl100k tokenizer")
            }
            // Default to cl100k (most common)
            _ => cl100k_base().expect("Failed to load cl100k tokenizer"),
        };

        Self {
            encoder,
            model: model.to_string(),
        }
    }

    /// Create a default tokenizer using cl100k_base.
    pub fn default_tokenizer() -> Self {
        Self {
            encoder: cl100k_base().expect("Failed to load cl100k tokenizer"),
            model: "default".to_string(),
        }
    }

    /// Count the number of tokens in the text.
    pub fn count_tokens(&self, text: &str) -> usize {
        self.encoder.encode_with_special_tokens(text).len()
    }

    /// Encode text to token IDs.
    pub fn encode(&self, text: &str) -> Vec<u32> {
        self.encoder.encode_with_special_tokens(text)
    }

    /// Decode token IDs back to text.
    pub fn decode(&self, tokens: &[u32]) -> String {
        self.encoder.decode(tokens.to_vec()).unwrap_or_default()
    }

    /// Truncate text to fit within a token limit.
    pub fn truncate(&self, text: &str, max_tokens: usize) -> String {
        let tokens = self.encode(text);
        if tokens.len() <= max_tokens {
            return text.to_string();
        }
        self.decode(&tokens[..max_tokens])
    }

    /// Split text into chunks that fit within a token limit.
    pub fn chunk(&self, text: &str, max_tokens: usize, overlap_tokens: usize) -> Vec<String> {
        let tokens = self.encode(text);
        let mut chunks = Vec::new();
        let mut start = 0;

        while start < tokens.len() {
            let end = (start + max_tokens).min(tokens.len());
            let chunk_tokens = &tokens[start..end];
            chunks.push(self.decode(chunk_tokens));

            if end >= tokens.len() {
                break;
            }

            start = end.saturating_sub(overlap_tokens);
        }

        chunks
    }

    /// Get the model this tokenizer is configured for.
    pub fn model(&self) -> &str {
        &self.model
    }
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::default_tokenizer()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counting() {
        let tokenizer = Tokenizer::default_tokenizer();
        let text = "Hello, world!";
        let count = tokenizer.count_tokens(text);
        assert!(count > 0);
        assert!(count < text.len()); // Tokens are typically longer than bytes
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let tokenizer = Tokenizer::default_tokenizer();
        let text = "This is a test sentence.";
        let tokens = tokenizer.encode(text);
        let decoded = tokenizer.decode(&tokens);
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_truncate() {
        let tokenizer = Tokenizer::default_tokenizer();
        let text = "This is a longer sentence that should be truncated.";
        let truncated = tokenizer.truncate(text, 5);
        let token_count = tokenizer.count_tokens(&truncated);
        assert!(token_count <= 5);
    }

    #[test]
    fn test_chunking() {
        let tokenizer = Tokenizer::default_tokenizer();
        let text = "One two three four five six seven eight nine ten.";
        let chunks = tokenizer.chunk(text, 3, 1);
        assert!(chunks.len() > 1);
    }

    #[test]
    fn test_model_specific_tokenizer() {
        let gpt4 = Tokenizer::for_model("gpt-4");
        let gpt4o = Tokenizer::for_model("gpt-4o");

        // Both should be able to tokenize
        let text = "Hello, world!";
        assert!(gpt4.count_tokens(text) > 0);
        assert!(gpt4o.count_tokens(text) > 0);
    }
}
