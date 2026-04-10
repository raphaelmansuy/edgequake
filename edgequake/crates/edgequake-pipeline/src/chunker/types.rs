//! Core types and traits for text chunking.
//!
//! Defines the data structures and strategy trait used by the chunker module.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Result of a custom chunking operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkResult {
    /// The chunk text content.
    pub content: String,
    /// Approximate token count.
    pub tokens: usize,
    /// Zero-based index indicating the chunk's order in the document.
    pub chunk_order_index: usize,
}

/// Trait for custom chunking strategies.
///
/// Implement this trait to provide your own chunking logic for document processing.
/// This allows for flexible chunking strategies such as:
/// - Semantic chunking (based on meaning/topics)
/// - Fixed-size chunking with custom separators
/// - Language-specific chunking (code, markdown, etc.)
#[async_trait]
pub trait ChunkingStrategy: Send + Sync {
    /// Chunk the given text content into smaller pieces.
    ///
    /// # Arguments
    /// * `content` - The full text content to chunk
    /// * `config` - The chunking configuration
    ///
    /// # Returns
    /// A vector of chunk results with content, token count, and order index
    async fn chunk(&self, content: &str, config: &ChunkerConfig) -> Result<Vec<ChunkResult>>;

    /// Get the name of this chunking strategy.
    fn name(&self) -> &str;
}

/// Configuration for the chunker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkerConfig {
    /// Target chunk size in tokens.
    pub chunk_size: usize,

    /// Overlap between chunks in tokens.
    pub chunk_overlap: usize,

    /// Minimum chunk size (won't create chunks smaller than this).
    pub min_chunk_size: usize,

    /// Separator characters for splitting.
    pub separators: Vec<String>,

    /// Whether to preserve sentence boundaries.
    pub preserve_sentences: bool,

    /// Optional character to split on first (e.g., "\n" for line-by-line).
    pub split_by_character: Option<String>,

    /// If true, split only on the specified character, don't apply token limits.
    pub split_by_character_only: bool,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1200,
            chunk_overlap: 100,
            min_chunk_size: 100,
            separators: vec![
                "\n\n".to_string(),
                "\n".to_string(),
                ". ".to_string(),
                "! ".to_string(),
                "? ".to_string(),
                "; ".to_string(),
                ", ".to_string(),
                " ".to_string(),
            ],
            preserve_sentences: true,
            split_by_character: None,
            split_by_character_only: false,
        }
    }
}

/// A chunk of text with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChunk {
    /// Unique identifier for the chunk.
    pub id: String,

    /// The chunk text content.
    pub content: String,

    /// Index of this chunk in the document.
    pub index: usize,

    /// Character offset from the start of the document.
    pub start_offset: usize,

    /// Character offset to the end of the chunk.
    pub end_offset: usize,

    /// Starting line number (1-based) in the original document.
    pub start_line: usize,

    /// Ending line number (1-based, inclusive) in the original document.
    pub end_line: usize,

    /// Approximate token count.
    pub token_count: usize,

    /// Chunk embedding.
    pub embedding: Option<Vec<f32>>,
}

impl TextChunk {
    /// Create a new text chunk.
    pub fn new(
        id: impl Into<String>,
        content: impl Into<String>,
        index: usize,
        start_offset: usize,
        end_offset: usize,
    ) -> Self {
        let content = content.into();
        let token_count = super::text_utils::estimate_tokens(&content);
        Self {
            id: id.into(),
            content,
            index,
            start_offset,
            end_offset,
            start_line: 1,
            end_line: 1,
            token_count,
            embedding: None,
        }
    }

    /// Create a new text chunk with line numbers.
    pub fn with_line_numbers(
        id: impl Into<String>,
        content: impl Into<String>,
        index: usize,
        start_offset: usize,
        end_offset: usize,
        start_line: usize,
        end_line: usize,
    ) -> Self {
        let content = content.into();
        let token_count = super::text_utils::estimate_tokens(&content);
        Self {
            id: id.into(),
            content,
            index,
            start_offset,
            end_offset,
            start_line,
            end_line,
            token_count,
            embedding: None,
        }
    }

    /// Set line numbers after creation.
    pub fn set_line_numbers(&mut self, start_line: usize, end_line: usize) {
        self.start_line = start_line;
        self.end_line = end_line;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunker_config_defaults() {
        let cfg = ChunkerConfig::default();
        assert_eq!(cfg.chunk_size, 1200);
        assert_eq!(cfg.chunk_overlap, 100);
        assert_eq!(cfg.min_chunk_size, 100);
        assert!(cfg.preserve_sentences);
        assert!(cfg.split_by_character.is_none());
        assert!(!cfg.split_by_character_only);
        // First separator is paragraph break
        assert_eq!(cfg.separators[0], "\n\n");
        assert!(cfg.separators.len() >= 7);
    }

    #[test]
    fn test_text_chunk_new_token_count() {
        let chunk = TextChunk::new("id1", "hello world!!", 0, 0, 13);
        // 13 chars / 4 = 3.25 → ceil = 4
        assert_eq!(chunk.token_count, 4);
        assert_eq!(chunk.id, "id1");
        assert_eq!(chunk.index, 0);
        assert!(chunk.embedding.is_none());
        // Default line numbers
        assert_eq!(chunk.start_line, 1);
        assert_eq!(chunk.end_line, 1);
    }

    #[test]
    fn test_text_chunk_with_line_numbers() {
        let chunk = TextChunk::with_line_numbers("id2", "text", 1, 10, 14, 3, 5);
        assert_eq!(chunk.start_line, 3);
        assert_eq!(chunk.end_line, 5);
        assert_eq!(chunk.start_offset, 10);
        assert_eq!(chunk.end_offset, 14);
    }

    #[test]
    fn test_set_line_numbers() {
        let mut chunk = TextChunk::new("id3", "abc", 0, 0, 3);
        assert_eq!(chunk.start_line, 1);
        chunk.set_line_numbers(10, 20);
        assert_eq!(chunk.start_line, 10);
        assert_eq!(chunk.end_line, 20);
    }
}
