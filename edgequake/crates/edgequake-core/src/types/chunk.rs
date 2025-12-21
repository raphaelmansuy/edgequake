//! Chunk type definition.
//!
//! A Chunk represents a segment of a document, sized appropriately for
//! LLM context windows.

use serde::{Deserialize, Serialize};

/// A segment of a document.
///
/// Documents are split into chunks to fit within LLM context windows.
/// Each chunk maintains a reference back to its parent document and
/// its position within the document.
///
/// # Example
///
/// ```rust
/// use edgequake_core::types::Chunk;
///
/// let chunk = Chunk::new(
///     "This is chunk content".to_string(),
///     150,
///     0,
///     "doc-abc123".to_string(),
///     None,
/// );
/// assert!(chunk.id.starts_with("chunk-"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// MD5 hash of content - primary key
    pub id: String,
    /// Chunk text content
    pub content: String,
    /// Token count
    pub tokens: u32,
    /// Position in document (0-indexed)
    pub chunk_order_index: u32,
    /// Parent document ID
    pub full_doc_id: String,
    /// Source file path (inherited from document)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

impl Chunk {
    /// Generate chunk ID from content (MD5 hash).
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgequake_core::types::Chunk;
    ///
    /// let id = Chunk::generate_id("chunk content");
    /// assert!(id.starts_with("chunk-"));
    /// ```
    pub fn generate_id(content: &str) -> String {
        format!("chunk-{:x}", md5::compute(content.as_bytes()))
    }

    /// Create a new chunk.
    ///
    /// # Arguments
    ///
    /// * `content` - The text content of the chunk
    /// * `tokens` - Number of tokens in the chunk
    /// * `chunk_order_index` - Position in the parent document (0-indexed)
    /// * `full_doc_id` - ID of the parent document
    /// * `file_path` - Optional source file path
    pub fn new(
        content: String,
        tokens: u32,
        chunk_order_index: u32,
        full_doc_id: String,
        file_path: Option<String>,
    ) -> Self {
        Self {
            id: Self::generate_id(&content),
            content,
            tokens,
            chunk_order_index,
            full_doc_id,
            file_path,
        }
    }

    /// Check if the chunk is empty.
    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }

    /// Get the content length in bytes.
    pub fn content_len(&self) -> usize {
        self.content.len()
    }

    /// Get the content length in characters.
    pub fn char_count(&self) -> usize {
        self.content.chars().count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_id_generation() {
        let id1 = Chunk::generate_id("Hello chunk");
        let id2 = Chunk::generate_id("Hello chunk");
        let id3 = Chunk::generate_id("Different chunk");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert!(id1.starts_with("chunk-"));
    }

    #[test]
    fn test_chunk_creation() {
        let chunk = Chunk::new(
            "Test chunk content".to_string(),
            100,
            0,
            "doc-123".to_string(),
            Some("/test.txt".to_string()),
        );

        assert_eq!(chunk.tokens, 100);
        assert_eq!(chunk.chunk_order_index, 0);
        assert_eq!(chunk.full_doc_id, "doc-123");
        assert_eq!(chunk.file_path, Some("/test.txt".to_string()));
    }

    #[test]
    fn test_chunk_empty_check() {
        let chunk1 = Chunk::new("".to_string(), 0, 0, "doc-1".to_string(), None);
        let chunk2 = Chunk::new("   ".to_string(), 0, 0, "doc-1".to_string(), None);
        let chunk3 = Chunk::new("Content".to_string(), 10, 0, "doc-1".to_string(), None);

        assert!(chunk1.is_empty());
        assert!(chunk2.is_empty());
        assert!(!chunk3.is_empty());
    }
}
