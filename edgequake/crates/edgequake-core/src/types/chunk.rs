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

    // === Lineage: Position metadata ===
    // WHY: Enables tracing a chunk back to exact location in source document.
    // These fields are Optional to maintain backward compatibility with existing
    // serialized chunks that don't have position info.

    /// Start line number in source document (1-indexed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_line: Option<usize>,
    /// End line number in source document (1-indexed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    /// Start character offset in source document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_offset: Option<usize>,
    /// End character offset in source document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_offset: Option<usize>,
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
            start_line: None,
            end_line: None,
            start_offset: None,
            end_offset: None,
        }
    }

    /// Set position metadata for lineage traceability (builder pattern).
    ///
    /// # Arguments
    ///
    /// * `start_line` - Start line in source document (1-indexed)
    /// * `end_line` - End line in source document (1-indexed)
    /// * `start_offset` - Start character offset in source document
    /// * `end_offset` - End character offset in source document
    pub fn with_position(
        mut self,
        start_line: usize,
        end_line: usize,
        start_offset: usize,
        end_offset: usize,
    ) -> Self {
        self.start_line = Some(start_line);
        self.end_line = Some(end_line);
        self.start_offset = Some(start_offset);
        self.end_offset = Some(end_offset);
        self
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

    #[test]
    fn test_chunk_position_default_none() {
        let chunk = Chunk::new("Content".to_string(), 10, 0, "doc-1".to_string(), None);
        assert!(chunk.start_line.is_none());
        assert!(chunk.end_line.is_none());
        assert!(chunk.start_offset.is_none());
        assert!(chunk.end_offset.is_none());
    }

    #[test]
    fn test_chunk_with_position() {
        let chunk = Chunk::new("Content".to_string(), 10, 0, "doc-1".to_string(), None)
            .with_position(1, 5, 0, 200);
        assert_eq!(chunk.start_line, Some(1));
        assert_eq!(chunk.end_line, Some(5));
        assert_eq!(chunk.start_offset, Some(0));
        assert_eq!(chunk.end_offset, Some(200));
    }

    #[test]
    fn test_chunk_position_serialization_roundtrip() {
        let chunk = Chunk::new("Content".to_string(), 10, 0, "doc-1".to_string(), None)
            .with_position(10, 20, 500, 1000);
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("\"start_line\":10"));
        assert!(json.contains("\"end_line\":20"));
        let deserialized: Chunk = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.start_line, Some(10));
        assert_eq!(deserialized.end_offset, Some(1000));
    }

    #[test]
    fn test_chunk_backward_compat_deserialization() {
        // WHY: Existing serialized chunks without position fields must deserialize correctly.
        let old_json = r#"{"id":"chunk-abc","content":"Hello","tokens":5,"chunk_order_index":0,"full_doc_id":"doc-1"}"#;
        let chunk: Chunk = serde_json::from_str(old_json).unwrap();
        assert_eq!(chunk.content, "Hello");
        assert!(chunk.start_line.is_none());
        assert!(chunk.end_line.is_none());
    }
}
