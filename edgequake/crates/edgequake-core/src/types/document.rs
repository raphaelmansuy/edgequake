//! Document type definition.
//!
//! A Document represents a unit of text content to be processed and indexed
//! into the knowledge graph.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Document processing status.
///
/// Tracks the lifecycle state of a document through the processing pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentStatus {
    /// Document is waiting to be processed
    #[default]
    Pending,
    /// Document is currently being processed
    Processing,
    /// Document has been successfully processed
    Processed,
    /// Document processing failed
    Failed,
}

impl DocumentStatus {
    /// Returns true if the document can be processed
    pub fn can_process(&self) -> bool {
        matches!(self, Self::Pending | Self::Failed)
    }

    /// Returns true if the document has been completed (success or failure)
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Processed | Self::Failed)
    }
}

/// A document to be processed into the knowledge graph.
///
/// Documents are the primary input to the RAG pipeline. Each document
/// is split into chunks, which are then processed to extract entities
/// and relationships.
///
/// # Example
///
/// ```rust
/// use edgequake_core::types::{Document, DocumentStatus};
///
/// let doc = Document::new("Sample document content".to_string(), Some("/path/to/file.txt".to_string()));
/// assert_eq!(doc.status, DocumentStatus::Pending);
/// assert!(doc.id.starts_with("doc-"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// MD5 hash of content - primary key
    pub id: String,
    /// Raw text content
    pub content: String,
    /// Source file path (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// Processing status
    pub status: DocumentStatus,
    /// Batch tracking ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Number of chunks generated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunks_count: Option<u32>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Content summary (first 100 chars) for preview
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_summary: Option<String>,
    /// Total content length in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_length: Option<usize>,
    /// List of chunk IDs associated with this document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_ids: Option<Vec<String>>,
    /// Additional metadata
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    // === Lineage: Type-safe metadata fields ===
    // WHY: Explicit fields are better than JSON blob for type safety, query indexing,
    // and API contract clarity. These enable the complete lineage chain:
    // PDF → Document → Chunks → Entities.
    /// Document type/source format (e.g., "pdf", "markdown", "text").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_type: Option<String>,
    /// Original file size in bytes (distinct from content_length which is text length).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
    /// SHA-256 checksum of original file for deduplication and integrity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256_checksum: Option<String>,
    /// Link to originating PDF document (bidirectional with PdfDocument.document_id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdf_id: Option<String>,
    /// LLM model used for entity extraction on this document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,
    /// Embedding model used for vectorizing chunks of this document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    /// When processing completed (distinct from updated_at).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processed_at: Option<DateTime<Utc>>,
}

impl Document {
    /// Generate document ID from content (MD5 hash).
    ///
    /// The same content will always produce the same ID, enabling
    /// deduplication.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgequake_core::types::Document;
    ///
    /// let id1 = Document::generate_id("Hello");
    /// let id2 = Document::generate_id("Hello");
    /// assert_eq!(id1, id2);
    /// ```
    pub fn generate_id(content: &str) -> String {
        format!("doc-{:x}", md5::compute(content.as_bytes()))
    }

    /// Create a new document with PENDING status.
    ///
    /// # Arguments
    ///
    /// * `content` - The raw text content of the document
    /// * `file_path` - Optional source file path for traceability
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgequake_core::types::Document;
    ///
    /// let doc = Document::new("Content here".to_string(), None);
    /// ```
    pub fn new(content: String, file_path: Option<String>) -> Self {
        let now = Utc::now();
        let content_length = content.len();
        let content_summary = content.chars().take(100).collect::<String>();
        Self {
            id: Self::generate_id(&content),
            content,
            file_path,
            status: DocumentStatus::Pending,
            track_id: None,
            created_at: now,
            updated_at: now,
            chunks_count: None,
            error: None,
            content_summary: Some(content_summary),
            content_length: Some(content_length),
            chunk_ids: None,
            metadata: None,
            document_type: None,
            file_size: None,
            sha256_checksum: None,
            pdf_id: None,
            llm_model: None,
            embedding_model: None,
            processed_at: None,
        }
    }

    /// Create a new document with a specific track ID for batch processing.
    pub fn new_with_track_id(content: String, file_path: Option<String>, track_id: String) -> Self {
        let mut doc = Self::new(content, file_path);
        doc.track_id = Some(track_id);
        doc
    }

    /// Mark the document as processing.
    pub fn mark_processing(&mut self) {
        self.status = DocumentStatus::Processing;
        self.updated_at = Utc::now();
    }

    /// Mark the document as successfully processed.
    pub fn mark_processed(&mut self, chunks_count: u32) {
        self.status = DocumentStatus::Processed;
        self.chunks_count = Some(chunks_count);
        self.error = None;
        let now = Utc::now();
        self.updated_at = now;
        self.processed_at = Some(now);
    }

    /// Mark the document as successfully processed with chunk IDs.
    pub fn mark_processed_with_chunks(&mut self, chunk_ids: Vec<String>) {
        self.status = DocumentStatus::Processed;
        self.chunks_count = Some(chunk_ids.len() as u32);
        self.chunk_ids = Some(chunk_ids);
        self.error = None;
        let now = Utc::now();
        self.updated_at = now;
        self.processed_at = Some(now);
    }

    /// Mark the document as failed.
    pub fn mark_failed(&mut self, error: String) {
        self.status = DocumentStatus::Failed;
        self.error = Some(error);
        self.updated_at = Utc::now();
    }

    /// Set lineage-related metadata for full traceability.
    ///
    /// # Arguments
    ///
    /// * `document_type` - Source format ("pdf", "markdown", "text")
    /// * `file_size` - Original file size in bytes
    /// * `sha256_checksum` - SHA-256 hash of original file
    pub fn set_lineage_metadata(
        &mut self,
        document_type: impl Into<String>,
        file_size: u64,
        sha256_checksum: impl Into<String>,
    ) {
        self.document_type = Some(document_type.into());
        self.file_size = Some(file_size);
        self.sha256_checksum = Some(sha256_checksum.into());
        self.updated_at = Utc::now();
    }

    /// Link this document to its originating PDF.
    pub fn set_pdf_id(&mut self, pdf_id: impl Into<String>) {
        self.pdf_id = Some(pdf_id.into());
        self.updated_at = Utc::now();
    }

    /// Set model information used for processing this document.
    pub fn set_models(&mut self, llm_model: impl Into<String>, embedding_model: impl Into<String>) {
        self.llm_model = Some(llm_model.into());
        self.embedding_model = Some(embedding_model.into());
        self.updated_at = Utc::now();
    }

    /// Check if the document content is empty.
    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }

    /// Get the content length in bytes.
    pub fn content_len(&self) -> usize {
        self.content.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_id_generation() {
        let id1 = Document::generate_id("Hello, World!");
        let id2 = Document::generate_id("Hello, World!");
        let id3 = Document::generate_id("Different content");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert!(id1.starts_with("doc-"));
    }

    #[test]
    fn test_document_creation() {
        let doc = Document::new("Test content".to_string(), Some("/test.txt".to_string()));

        assert_eq!(doc.status, DocumentStatus::Pending);
        assert_eq!(doc.file_path, Some("/test.txt".to_string()));
        assert!(doc.track_id.is_none());
        assert!(doc.chunks_count.is_none());
        assert!(doc.error.is_none());
    }

    #[test]
    fn test_document_lifecycle() {
        let mut doc = Document::new("Test".to_string(), None);

        assert!(doc.status.can_process());
        assert!(!doc.status.is_terminal());

        doc.mark_processing();
        assert_eq!(doc.status, DocumentStatus::Processing);
        assert!(!doc.status.can_process());

        doc.mark_processed(5);
        assert_eq!(doc.status, DocumentStatus::Processed);
        assert_eq!(doc.chunks_count, Some(5));
        assert!(doc.status.is_terminal());
    }

    #[test]
    fn test_document_failure() {
        let mut doc = Document::new("Test".to_string(), None);

        doc.mark_failed("Processing error".to_string());
        assert_eq!(doc.status, DocumentStatus::Failed);
        assert_eq!(doc.error, Some("Processing error".to_string()));
        assert!(doc.status.can_process()); // Can retry
    }

    #[test]
    fn test_document_empty_check() {
        let doc1 = Document::new("".to_string(), None);
        let doc2 = Document::new("   ".to_string(), None);
        let doc3 = Document::new("Content".to_string(), None);

        assert!(doc1.is_empty());
        assert!(doc2.is_empty());
        assert!(!doc3.is_empty());
    }

    #[test]
    fn test_document_lineage_metadata_defaults() {
        let doc = Document::new("Content".to_string(), None);
        assert!(doc.document_type.is_none());
        assert!(doc.file_size.is_none());
        assert!(doc.sha256_checksum.is_none());
        assert!(doc.pdf_id.is_none());
        assert!(doc.llm_model.is_none());
        assert!(doc.embedding_model.is_none());
        assert!(doc.processed_at.is_none());
    }

    #[test]
    fn test_document_set_lineage_metadata() {
        let mut doc = Document::new("Content".to_string(), None);
        doc.set_lineage_metadata("pdf", 1024000, "abc123def456");
        assert_eq!(doc.document_type, Some("pdf".to_string()));
        assert_eq!(doc.file_size, Some(1024000));
        assert_eq!(doc.sha256_checksum, Some("abc123def456".to_string()));
    }

    #[test]
    fn test_document_set_pdf_id() {
        let mut doc = Document::new("Content".to_string(), None);
        doc.set_pdf_id("pdf-uuid-123");
        assert_eq!(doc.pdf_id, Some("pdf-uuid-123".to_string()));
    }

    #[test]
    fn test_document_set_models() {
        let mut doc = Document::new("Content".to_string(), None);
        doc.set_models("gpt-4.1-nano", "text-embedding-3-small");
        assert_eq!(doc.llm_model, Some("gpt-4.1-nano".to_string()));
        assert_eq!(
            doc.embedding_model,
            Some("text-embedding-3-small".to_string())
        );
    }

    #[test]
    fn test_document_processed_at_set_on_completion() {
        let mut doc = Document::new("Content".to_string(), None);
        assert!(doc.processed_at.is_none());
        doc.mark_processed(3);
        assert!(doc.processed_at.is_some());
    }

    #[test]
    fn test_document_backward_compat_deserialization() {
        // WHY: Old serialized documents without new lineage fields must deserialize correctly.
        let old_json = r#"{"id":"doc-abc","content":"Hello","status":"PENDING","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z"}"#;
        let doc: Document = serde_json::from_str(old_json).unwrap();
        assert_eq!(doc.content, "Hello");
        assert!(doc.document_type.is_none());
        assert!(doc.pdf_id.is_none());
        assert!(doc.llm_model.is_none());
    }

    #[test]
    fn test_document_full_lineage_serialization() {
        let mut doc = Document::new(
            "Full lineage test".to_string(),
            Some("/data/report.pdf".to_string()),
        );
        doc.set_lineage_metadata("pdf", 2048000, "sha256_hash_here");
        doc.set_pdf_id("pdf-uuid-456");
        doc.set_models("gpt-4.1-nano", "text-embedding-3-small");
        doc.mark_processed(10);

        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("\"document_type\":\"pdf\""));
        assert!(json.contains("\"file_size\":2048000"));
        assert!(json.contains("\"pdf_id\":\"pdf-uuid-456\""));
        assert!(json.contains("\"llm_model\":\"gpt-4.1-nano\""));
        assert!(json.contains("\"processed_at\""));

        let deserialized: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.document_type, Some("pdf".to_string()));
        assert_eq!(deserialized.pdf_id, Some("pdf-uuid-456".to_string()));
    }
}
