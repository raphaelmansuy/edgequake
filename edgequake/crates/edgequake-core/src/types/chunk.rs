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

    // === Lineage: Model metadata ===
    // WHY: Enables per-chunk traceability of which LLM/embedding models were used.
    // Critical for reproducibility and quality auditing when models change over time.
    /// LLM model used for entity extraction from this chunk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,
    /// Embedding model used to vectorize this chunk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    /// Embedding vector dimension used for this chunk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_dimension: Option<usize>,
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
            llm_model: None,
            embedding_model: None,
            embedding_dimension: None,
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

    /// Set model metadata for lineage traceability (builder pattern).
    ///
    /// # Arguments
    ///
    /// * `llm_model` - LLM model used for entity extraction (e.g., "gpt-4.1-nano")
    /// * `embedding_model` - Embedding model used (e.g., "text-embedding-3-small")
    /// * `embedding_dimension` - Embedding vector dimension (e.g., 1536)
    pub fn with_models(
        mut self,
        llm_model: impl Into<String>,
        embedding_model: impl Into<String>,
        embedding_dimension: usize,
    ) -> Self {
        self.llm_model = Some(llm_model.into());
        self.embedding_model = Some(embedding_model.into());
        self.embedding_dimension = Some(embedding_dimension);
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
        assert!(chunk.llm_model.is_none());
        assert!(chunk.embedding_model.is_none());
    }

    #[test]
    fn test_chunk_with_models() {
        let chunk = Chunk::new("Content".to_string(), 10, 0, "doc-1".to_string(), None)
            .with_models("gpt-4.1-nano", "text-embedding-3-small", 1536);
        assert_eq!(chunk.llm_model, Some("gpt-4.1-nano".to_string()));
        assert_eq!(
            chunk.embedding_model,
            Some("text-embedding-3-small".to_string())
        );
        assert_eq!(chunk.embedding_dimension, Some(1536));
    }

    #[test]
    fn test_chunk_with_full_lineage() {
        // WHY: Test that both position and model metadata can be chained.
        let chunk = Chunk::new(
            "Full lineage chunk".to_string(),
            50,
            2,
            "doc-xyz".to_string(),
            Some("/data/file.pdf".to_string()),
        )
        .with_position(10, 20, 500, 1000)
        .with_models("gpt-4.1-nano", "text-embedding-3-small", 1536);
        assert_eq!(chunk.start_line, Some(10));
        assert_eq!(chunk.llm_model, Some("gpt-4.1-nano".to_string()));
        assert_eq!(chunk.embedding_dimension, Some(1536));
        assert_eq!(chunk.full_doc_id, "doc-xyz");
        assert_eq!(chunk.file_path, Some("/data/file.pdf".to_string()));
    }

    #[test]
    fn test_chunk_model_serialization_roundtrip() {
        let chunk = Chunk::new("Content".to_string(), 10, 0, "doc-1".to_string(), None)
            .with_models("ollama/gemma3", "nomic-embed-text", 768);
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("ollama/gemma3"));
        assert!(json.contains("nomic-embed-text"));
        let deserialized: Chunk = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.llm_model, Some("ollama/gemma3".to_string()));
        assert_eq!(deserialized.embedding_dimension, Some(768));
    }
}
