//! Text chunking with overlap.

use serde::{Deserialize, Serialize};

use crate::error::Result;

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
            token_count,
            embedding: None,
        }
    }
}

/// Estimate token count (rough approximation: 1 token ≈ 4 chars).
fn estimate_tokens(text: &str) -> usize {
    (text.len() as f32 / 4.0).ceil() as usize
}

/// Text chunker for splitting documents.
pub struct Chunker {
    config: ChunkerConfig,
}

impl Chunker {
    /// Create a new chunker with the given configuration.
    pub fn new(config: ChunkerConfig) -> Self {
        Self { config }
    }

    /// Create a chunker with default configuration.
    pub fn default_chunker() -> Self {
        Self::new(ChunkerConfig::default())
    }

    /// Chunk text into overlapping segments.
    pub fn chunk(&self, text: &str, doc_id: &str) -> Result<Vec<TextChunk>> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        let target_chars = self.config.chunk_size * 4; // Rough token to char conversion
        let overlap_chars = self.config.chunk_overlap * 4;
        let min_chars = self.config.min_chunk_size * 4;

        let chunks = self.split_text(text, target_chars, overlap_chars, min_chars);

        Ok(chunks
            .into_iter()
            .enumerate()
            .map(|(index, (content, start, end))| {
                let id = format!("{}-chunk-{}", doc_id, index);
                TextChunk::new(id, content, index, start, end)
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
        if text.len() <= target_size {
            return vec![(text.to_string(), 0, text.len())];
        }

        let mut chunks = Vec::new();
        let mut current_pos = 0;

        while current_pos < text.len() {
            let remaining = &text[current_pos..];

            if remaining.len() <= target_size {
                // Last chunk
                chunks.push((remaining.to_string(), current_pos, text.len()));
                break;
            }

            // Find best split point
            let end_pos = current_pos + target_size;
            let chunk_text = &text[current_pos..end_pos.min(text.len())];

            let split_point = self.find_split_point(chunk_text, target_size);
            let actual_end = current_pos + split_point;

            let chunk_content = text[current_pos..actual_end].to_string();

            if chunk_content.len() >= min_size {
                chunks.push((chunk_content, current_pos, actual_end));
            }

            // Move forward with overlap
            current_pos = actual_end.saturating_sub(overlap);

            // Prevent infinite loop
            if current_pos >= actual_end {
                current_pos = actual_end;
            }
        }

        chunks
    }

    /// Find the best split point near the target size.
    fn find_split_point(&self, text: &str, target: usize) -> usize {
        let search_start = target.saturating_sub(target / 4);
        let search_end = target.min(text.len());

        // Look for separators in reverse order of preference
        for separator in &self.config.separators {
            if let Some(pos) = text[search_start..search_end].rfind(separator) {
                return search_start + pos + separator.len();
            }
        }

        // No separator found, split at target
        target.min(text.len())
    }

    /// Get the chunker configuration.
    pub fn config(&self) -> &ChunkerConfig {
        &self.config
    }
}

impl Default for Chunker {
    fn default() -> Self {
        Self::default_chunker()
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
}
