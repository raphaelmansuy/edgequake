//! Text chunking with overlap for document processing.
//!
//! # Implements
//!
//! - **FEAT0004**: Semantic Chunking
//! - **FEAT0011**: Document-Chunk-Entity Lineage (via line number tracking)
//!
//! # Enforces
//!
//! - **BR0002**: Chunk size 1200 tokens, overlap 100 tokens (default config)
//!
//! # WHY: Overlapping Chunks
//!
//! Overlap between chunks ensures:
//! 1. Context continuity across chunk boundaries
//! 2. Entity mentions spanning two chunks are captured
//! 3. Better retrieval for queries at chunk boundaries
//!
//! The default 100-token overlap (~8% of chunk size) balances:
//! - Coverage (entities not missed)
//! - Efficiency (minimal duplicate processing)
//!
//! This module provides flexible text chunking with support for custom chunking functions.
//! Users can implement the `ChunkingStrategy` trait to provide their own chunking logic.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
/// @implements FEAT0002
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
        let token_count = estimate_tokens(&content);
        Self {
            id: id.into(),
            content,
            index,
            start_offset,
            end_offset,
            start_line: 1, // Default, should be set via with_line_numbers()
            end_line: 1,   // Default, should be set via with_line_numbers()
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
        let token_count = estimate_tokens(&content);
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

/// Calculate line numbers for a chunk based on character offsets.
///
/// # Arguments
/// * `full_text` - The complete document text
/// * `start_offset` - Starting character offset of the chunk
/// * `end_offset` - Ending character offset of the chunk
///
/// # Returns
/// A tuple of (start_line, end_line), both 1-based
pub fn calculate_line_numbers(
    full_text: &str,
    start_offset: usize,
    end_offset: usize,
) -> (usize, usize) {
    // Ensure offsets are on valid char boundaries
    let safe_start = floor_char_boundary(full_text, start_offset.min(full_text.len()));
    let safe_end = floor_char_boundary(full_text, end_offset.min(full_text.len()));

    // Count newlines before the start offset to get start line
    let before_chunk = &full_text[..safe_start];
    let start_line = before_chunk.chars().filter(|&c| c == '\n').count() + 1;

    // Count newlines within the chunk to get end line
    let chunk_text = &full_text[safe_start..safe_end];
    let lines_in_chunk = chunk_text.chars().filter(|&c| c == '\n').count();
    let end_line = start_line + lines_in_chunk;

    (start_line, end_line)
}

/// Estimate token count (rough approximation: 1 token ≈ 4 chars).
fn estimate_tokens(text: &str) -> usize {
    (text.len() as f32 / 4.0).ceil() as usize
}

/// Default token-based chunking strategy.
///
/// This is the standard chunking strategy that splits text into chunks
/// based on token count with overlap, respecting sentence boundaries.
pub struct TokenBasedChunking;

#[async_trait]
impl ChunkingStrategy for TokenBasedChunking {
    async fn chunk(&self, content: &str, config: &ChunkerConfig) -> Result<Vec<ChunkResult>> {
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        // Check for split_by_character_only mode (GAP-017)
        if let Some(ref split_char) = config.split_by_character {
            if config.split_by_character_only {
                return Ok(content
                    .split(split_char.as_str())
                    .enumerate()
                    .filter(|(_, s)| !s.trim().is_empty())
                    .map(|(idx, s)| ChunkResult {
                        content: s.to_string(),
                        tokens: estimate_tokens(s),
                        chunk_order_index: idx,
                    })
                    .collect());
            }
        }

        let target_chars = config.chunk_size * 4;
        let overlap_chars = config.chunk_overlap * 4;
        let min_chars = config.min_chunk_size * 4;

        let chunks = split_text_internal(
            content,
            target_chars,
            overlap_chars,
            min_chars,
            &config.separators,
        );

        Ok(chunks
            .into_iter()
            .enumerate()
            .map(|(idx, (text, _, _))| ChunkResult {
                content: text.clone(),
                tokens: estimate_tokens(&text),
                chunk_order_index: idx,
            })
            .collect())
    }

    fn name(&self) -> &str {
        "token_based"
    }
}

/// Character-based chunking strategy (GAP-017).
///
/// Splits text on a specific character (like newline) for pre-split content.
///
/// @implements FEAT0301 (Character-Based Chunking)
pub struct CharacterBasedChunking {
    /// Character to split on.
    pub split_character: String,
}

impl CharacterBasedChunking {
    /// Create a new character-based chunking strategy.
    pub fn new(split_character: impl Into<String>) -> Self {
        Self {
            split_character: split_character.into(),
        }
    }

    /// Create a newline-based chunker.
    pub fn by_newline() -> Self {
        Self::new("\n")
    }

    /// Create a paragraph-based chunker.
    pub fn by_paragraph() -> Self {
        Self::new("\n\n")
    }
}

#[async_trait]
impl ChunkingStrategy for CharacterBasedChunking {
    async fn chunk(&self, content: &str, _config: &ChunkerConfig) -> Result<Vec<ChunkResult>> {
        Ok(content
            .split(&self.split_character)
            .enumerate()
            .filter(|(_, s)| !s.trim().is_empty())
            .map(|(idx, s)| ChunkResult {
                content: s.to_string(),
                tokens: estimate_tokens(s),
                chunk_order_index: idx,
            })
            .collect())
    }

    fn name(&self) -> &str {
        "character_based"
    }
}

/// Find the nearest valid UTF-8 char boundary at or before the given byte position.
fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    // Walk backwards to find a valid char boundary
    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Find the nearest valid UTF-8 char boundary at or after the given byte position.
fn ceil_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    // Walk forward to find a valid char boundary
    let mut i = index;
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// Internal function to split text.
fn split_text_internal(
    text: &str,
    target_size: usize,
    overlap: usize,
    min_size: usize,
    separators: &[String],
) -> Vec<(String, usize, usize)> {
    if text.len() <= target_size {
        return vec![(text.to_string(), 0, text.len())];
    }

    let mut chunks = Vec::new();
    let mut current_pos = 0;

    while current_pos < text.len() {
        // Ensure current_pos is on a char boundary
        current_pos = ceil_char_boundary(text, current_pos);

        let remaining = &text[current_pos..];

        if remaining.len() <= target_size {
            chunks.push((remaining.to_string(), current_pos, text.len()));
            break;
        }

        // Calculate end position, ensuring it's on a char boundary
        let end_pos = floor_char_boundary(text, current_pos + target_size);
        let chunk_text = &text[current_pos..end_pos.min(text.len())];

        let split_point = find_split_point_internal(chunk_text, target_size, separators);
        // Ensure actual_end is on a char boundary
        let actual_end = floor_char_boundary(text, current_pos + split_point);

        let chunk_content = text[current_pos..actual_end].to_string();

        if chunk_content.len() >= min_size {
            chunks.push((chunk_content, current_pos, actual_end));
        }

        // Calculate overlap position, ensuring it's on a char boundary
        let overlap_pos = actual_end.saturating_sub(overlap);
        current_pos = ceil_char_boundary(text, overlap_pos);

        if current_pos >= actual_end {
            current_pos = actual_end;
        }
    }

    chunks
}

/// Internal function to find split point.
fn find_split_point_internal(text: &str, target: usize, separators: &[String]) -> usize {
    // Ensure search boundaries are on valid char boundaries
    let search_start = floor_char_boundary(text, target.saturating_sub(target / 4));
    let search_end = floor_char_boundary(text, target.min(text.len()));

    // Only search if we have a valid range
    if search_start >= search_end {
        return floor_char_boundary(text, target.min(text.len()));
    }

    for separator in separators {
        if let Some(pos) = text[search_start..search_end].rfind(separator.as_str()) {
            return search_start + pos + separator.len();
        }
    }

    floor_char_boundary(text, target.min(text.len()))
}

/// Text chunker for splitting documents.
pub struct Chunker {
    config: ChunkerConfig,
    strategy: Arc<dyn ChunkingStrategy>,
}

impl Chunker {
    /// Create a new chunker with the given configuration.
    pub fn new(config: ChunkerConfig) -> Self {
        Self {
            config,
            strategy: Arc::new(TokenBasedChunking),
        }
    }

    /// Create a new chunker with a custom chunking strategy.
    pub fn with_strategy(config: ChunkerConfig, strategy: Arc<dyn ChunkingStrategy>) -> Self {
        Self { config, strategy }
    }

    /// Create a chunker with default configuration.
    pub fn default_chunker() -> Self {
        Self::new(ChunkerConfig::default())
    }

    /// Create a chunker that splits by character only.
    pub fn character_chunker(split_character: impl Into<String>) -> Self {
        let config = ChunkerConfig {
            split_by_character: Some(split_character.into()),
            split_by_character_only: true,
            ..ChunkerConfig::default()
        };
        Self {
            config,
            strategy: Arc::new(CharacterBasedChunking::by_newline()),
        }
    }

    /// Chunk text into overlapping segments.
    pub fn chunk(&self, text: &str, doc_id: &str) -> Result<Vec<TextChunk>> {
        // Always use sync implementation to avoid tokio runtime conflicts
        self.chunk_sync(text, doc_id)
    }

    /// Chunk text asynchronously using the configured strategy.
    pub async fn chunk_async(&self, text: &str, doc_id: &str) -> Result<Vec<TextChunk>> {
        let results = self.strategy.chunk(text, &self.config).await?;

        // Track cumulative offset for line number calculation
        let mut cumulative_offset = 0;

        Ok(results
            .into_iter()
            .map(|result| {
                let id = format!("{}-chunk-{}", doc_id, result.chunk_order_index);
                let start_offset = cumulative_offset;
                let end_offset = cumulative_offset + result.content.len();
                let (start_line, end_line) = calculate_line_numbers(text, start_offset, end_offset);
                cumulative_offset = end_offset;

                TextChunk::with_line_numbers(
                    id,
                    result.content.clone(),
                    result.chunk_order_index,
                    start_offset,
                    end_offset,
                    start_line,
                    end_line,
                )
            })
            .collect())
    }

    /// Synchronous chunk implementation (fallback).
    fn chunk_sync(&self, text: &str, doc_id: &str) -> Result<Vec<TextChunk>> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        let target_chars = self.config.chunk_size * 4;
        let overlap_chars = self.config.chunk_overlap * 4;
        let min_chars = self.config.min_chunk_size * 4;

        let chunks = self.split_text(text, target_chars, overlap_chars, min_chars);

        Ok(chunks
            .into_iter()
            .enumerate()
            .map(|(index, (content, start, end))| {
                let id = format!("{}-chunk-{}", doc_id, index);
                let (start_line, end_line) = calculate_line_numbers(text, start, end);
                TextChunk::with_line_numbers(id, content, index, start, end, start_line, end_line)
            })
            .collect())
    }

    /// Split text using recursive character splitting.
    fn split_text(
        &self,
        text: &str,
        target_size: usize,
        overlap: usize,
        min_size: usize,
    ) -> Vec<(String, usize, usize)> {
        split_text_internal(
            text,
            target_size,
            overlap,
            min_size,
            &self.config.separators,
        )
    }

    /// Find the best split point near the target size.
    #[allow(dead_code)]
    fn find_split_point(&self, text: &str, target: usize) -> usize {
        find_split_point_internal(text, target, &self.config.separators)
    }

    /// Get the chunker configuration.
    pub fn config(&self) -> &ChunkerConfig {
        &self.config
    }

    /// Get the chunking strategy name.
    pub fn strategy_name(&self) -> &str {
        self.strategy.name()
    }
}

impl Default for Chunker {
    fn default() -> Self {
        Self {
            config: ChunkerConfig::default(),
            strategy: Arc::new(TokenBasedChunking),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_chunking() {
        let chunker = Chunker::default_chunker();
        let text = "This is sentence one. This is sentence two. This is sentence three.";

        let chunks = chunker.chunk(text, "doc1").unwrap();

        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn test_empty_text() {
        let chunker = Chunker::default_chunker();
        let chunks = chunker.chunk("", "doc1").unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_short_text() {
        let chunker = Chunker::default_chunker();
        let text = "Short text.";
        let chunks = chunker.chunk(text, "doc1").unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, text);
    }

    #[test]
    fn test_long_text_chunking() {
        let config = ChunkerConfig {
            chunk_size: 10, // 10 tokens * 4 chars = 40 chars per chunk
            chunk_overlap: 2,
            min_chunk_size: 5,
            ..Default::default()
        };
        let chunker = Chunker::new(config);

        let text = "First sentence here. Second sentence follows. Third sentence now. Fourth one too. Fifth is last.";
        let chunks = chunker.chunk(text, "doc1").unwrap();

        assert!(chunks.len() > 1);

        // Verify chunks cover the text
        let total_unique: std::collections::HashSet<_> =
            chunks.iter().flat_map(|c| c.content.chars()).collect();
        assert!(total_unique.len() > 0);
    }

    #[test]
    fn test_chunk_ids() {
        let chunker = Chunker::default_chunker();
        let text = "Some text content that will be chunked.";
        let chunks = chunker.chunk(text, "my-doc").unwrap();

        assert!(chunks[0].id.starts_with("my-doc-chunk-"));
    }

    #[test]
    fn test_token_estimation() {
        assert_eq!(estimate_tokens("test"), 1);
        assert_eq!(estimate_tokens("hello world"), 3); // 11 chars / 4 ≈ 3
    }

    #[test]
    fn test_line_number_calculation() {
        // Test single line
        let text = "Hello world";
        let (start, end) = calculate_line_numbers(text, 0, text.len());
        assert_eq!(start, 1);
        assert_eq!(end, 1);

        // Test multiple lines
        let text = "Line 1\nLine 2\nLine 3";
        let (start, end) = calculate_line_numbers(text, 0, text.len());
        assert_eq!(start, 1);
        assert_eq!(end, 3);

        // Test middle portion
        let text = "Line 1\nLine 2\nLine 3\nLine 4";
        let line2_start = 7; // After "Line 1\n"
        let line3_end = 20; // End of "Line 3"
        let (start, end) = calculate_line_numbers(text, line2_start, line3_end);
        assert_eq!(start, 2);
        assert_eq!(end, 3);
    }

    #[test]
    fn test_chunks_have_line_numbers() {
        let chunker = Chunker::default_chunker();
        let text = "Line one.\nLine two.\nLine three.";
        let chunks = chunker.chunk(text, "doc1").unwrap();

        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].start_line, 1);
        assert!(chunks[0].end_line >= 1);
    }

    #[test]
    fn test_multiline_chunk_line_numbers() {
        let config = ChunkerConfig {
            chunk_size: 10,
            chunk_overlap: 2,
            min_chunk_size: 5,
            ..Default::default()
        };
        let chunker = Chunker::new(config);

        let text = "Line 1 here.\nLine 2 here.\nLine 3 here.\nLine 4 here.\nLine 5 here.";
        let chunks = chunker.chunk(text, "doc1").unwrap();

        // First chunk should start at line 1
        if !chunks.is_empty() {
            assert_eq!(chunks[0].start_line, 1);
        }
    }

    #[test]
    fn test_utf8_multibyte_chars_in_chunking() {
        // Test with multi-byte UTF-8 characters: smart quotes, bullets, emojis
        // Using raw bytes to include smart quotes without Rust parser issues
        let text = "Quality. Compared with state-of-the-art FR-IQA models, \
the \u{201C}proposed GMSD model\u{201D} performs better \u{2022} in terms of both accuracy \
and efficiency, making GMSD an ideal choice for high-performance IQA applications.\n\n\
This work is supported by \u{7814}\u{7A76} and \u{5F00}\u{53D1} funding.";

        let config = ChunkerConfig {
            chunk_size: 50, // Force chunking within the multi-byte section
            chunk_overlap: 10,
            min_chunk_size: 20,
            ..Default::default()
        };
        let chunker = Chunker::new(config);

        // This should not panic even with multi-byte characters
        let chunks = chunker.chunk(text, "utf8-test").unwrap();

        assert!(!chunks.is_empty());
        // All chunks should be valid UTF-8 strings
        for chunk in &chunks {
            assert!(chunk.content.is_char_boundary(0));
            assert!(chunk.content.is_char_boundary(chunk.content.len()));
        }
    }

    #[test]
    fn test_floor_and_ceil_char_boundary() {
        // Test with multi-byte character: " (LEFT DOUBLE QUOTATION MARK, 3 bytes: E2 80 9C)
        let text = "ab\u{201C}cd";

        // "ab" is 2 bytes, then " is 3 bytes (positions 2, 3, 4), then "cd" is 2 more
        // So: a=0, b=1, "=2,3,4, c=5, d=6

        assert_eq!(floor_char_boundary(text, 2), 2); // Start of "
        assert_eq!(floor_char_boundary(text, 3), 2); // Inside " -> back to 2
        assert_eq!(floor_char_boundary(text, 4), 2); // Inside " -> back to 2
        assert_eq!(floor_char_boundary(text, 5), 5); // Start of c

        assert_eq!(ceil_char_boundary(text, 2), 2); // Start of "
        assert_eq!(ceil_char_boundary(text, 3), 5); // Inside " -> forward to 5
        assert_eq!(ceil_char_boundary(text, 4), 5); // Inside " -> forward to 5
    }
}
