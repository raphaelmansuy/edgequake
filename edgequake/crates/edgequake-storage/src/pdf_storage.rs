//! PDF document storage operations.
//!
//! @implements SPEC-007: PDF Upload Support with Vision LLM
//! @implements BR0001: Document deduplication via SHA-256
//! @implements BR0201: Workspace isolation
//!
//! # Implements
//!
//! - **FEAT0701**: Store raw PDF files with metadata
//! - **FEAT0702**: PDF deduplication by checksum
//! - **FEAT0703**: Track PDF processing status
//! - **FEAT0704**: Link PDFs to processed documents
//!
//! # Enforces
//!
//! - **BR0701**: PDFs must be scoped to workspace
//! - **BR0702**: PDFs cannot exceed 100MB
//! - **BR0703**: Processing status must be valid enum
//!
//! # Use Cases
//!
//! - **UC0701**: Store uploaded PDF for processing
//! - **UC0702**: Check for duplicate PDF uploads
//! - **UC0703**: Update PDF processing status
//! - **UC0704**: Retrieve PDF for reprocessing

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Result, StorageError};

// ============================================================================
// Data Structures
// ============================================================================

/// PDF document stored in database.
///
/// This structure represents a PDF file in the `pdf_documents` table with
/// all its metadata, processing state, and extracted content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfDocument {
    /// Unique PDF identifier.
    pub pdf_id: Uuid,

    /// Workspace this PDF belongs to (isolation).
    pub workspace_id: Uuid,

    /// Link to processed document (NULL during processing).
    pub document_id: Option<Uuid>,

    /// Original filename from upload.
    pub filename: String,

    /// MIME type (typically "application/pdf").
    pub content_type: String,

    /// File size in bytes.
    pub file_size_bytes: i64,

    /// SHA-256 checksum for deduplication.
    pub sha256_checksum: String,

    /// Number of pages in PDF.
    pub page_count: Option<i32>,

    /// Raw PDF bytes.
    pub pdf_data: Vec<u8>,

    /// Current processing status.
    pub processing_status: PdfProcessingStatus,

    /// Extraction method used (if processed).
    pub extraction_method: Option<ExtractionMethod>,

    /// Vision model used (if applicable).
    pub vision_model: Option<String>,

    /// Extracted markdown content.
    pub markdown_content: Option<String>,

    /// Extraction errors/warnings (JSON).
    pub extraction_errors: Option<serde_json::Value>,

    /// When PDF was uploaded.
    pub created_at: DateTime<Utc>,

    /// When processing completed.
    pub processed_at: Option<DateTime<Utc>>,

    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// PDF processing status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PdfProcessingStatus {
    /// Waiting for processing.
    Pending,
    /// Currently being processed.
    Processing,
    /// Successfully processed.
    Completed,
    /// Processing failed.
    Failed,
}

impl PdfProcessingStatus {
    /// Convert to string for database storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

impl std::str::FromStr for PdfProcessingStatus {
    type Err = StorageError;

    /// Parse from database string.
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(Self::Pending),
            "processing" => Ok(Self::Processing),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(StorageError::InvalidData(format!(
                "Invalid processing status: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for PdfProcessingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// PDF extraction method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtractionMethod {
    /// Text-based extraction.
    Text,
    /// Vision LLM extraction.
    Vision,
    /// Hybrid (text + vision).
    Hybrid,
}

impl ExtractionMethod {
    /// Convert to string for database storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Vision => "vision",
            Self::Hybrid => "hybrid",
        }
    }
}

impl std::str::FromStr for ExtractionMethod {
    type Err = StorageError;

    /// Parse from database string.
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "text" => Ok(Self::Text),
            "vision" => Ok(Self::Vision),
            "hybrid" => Ok(Self::Hybrid),
            _ => Err(StorageError::InvalidData(format!(
                "Invalid extraction method: {}",
                s
            ))),
        }
    }
}

/// Request to create a new PDF document.
#[derive(Debug, Clone)]
pub struct CreatePdfRequest {
    /// Workspace this PDF belongs to.
    pub workspace_id: Uuid,
    /// Original filename.
    pub filename: String,
    /// MIME type.
    pub content_type: String,
    /// File size in bytes.
    pub file_size_bytes: i64,
    /// SHA-256 checksum.
    pub sha256_checksum: String,
    /// Number of pages.
    pub page_count: Option<i32>,
    /// Raw PDF data.
    pub pdf_data: Vec<u8>,
    /// Vision model to use (optional).
    pub vision_model: Option<String>,
}

/// Request to update PDF processing results.
#[derive(Debug, Clone)]
pub struct UpdatePdfProcessingRequest {
    /// PDF identifier.
    pub pdf_id: Uuid,
    /// New status.
    pub processing_status: PdfProcessingStatus,
    /// Extraction method used.
    pub extraction_method: Option<ExtractionMethod>,
    /// Extracted markdown.
    pub markdown_content: Option<String>,
    /// Processing errors.
    pub extraction_errors: Option<serde_json::Value>,
    /// Link to document (if indexed).
    pub document_id: Option<Uuid>,
    /// Vision model used (when extraction_method is Vision).
    /// SPEC-007: Tracks which vision LLM model was used for extraction.
    pub vision_model: Option<String>,
}

/// Filter for listing PDFs.
#[derive(Debug, Clone, Default)]
pub struct ListPdfFilter {
    /// Filter by workspace.
    pub workspace_id: Option<Uuid>,
    /// Filter by status.
    pub processing_status: Option<PdfProcessingStatus>,
    /// Pagination: page number (1-indexed).
    pub page: Option<usize>,
    /// Pagination: items per page.
    pub page_size: Option<usize>,
}

/// PDF list response with pagination.
#[derive(Debug, Clone, Serialize)]
pub struct PdfList {
    /// PDF documents.
    pub items: Vec<PdfDocument>,
    /// Total count (for pagination).
    pub total_count: i64,
    /// Current page (1-indexed).
    pub page: usize,
    /// Page size.
    pub page_size: usize,
}

// ============================================================================
// Storage Trait
// ============================================================================

/// Trait for PDF document storage operations.
///
/// This trait abstracts PDF storage to support multiple backends
/// (PostgreSQL, in-memory, etc.).
#[async_trait]
pub trait PdfDocumentStorage: Send + Sync {
    /// Store a new PDF document.
    ///
    /// # Arguments
    ///
    /// * `request` - PDF creation request with all metadata and data
    ///
    /// # Returns
    ///
    /// * `Ok(pdf_id)` - UUID of created PDF
    /// * `Err(StorageError)` - If storage fails
    ///
    /// # Errors
    ///
    /// - `StorageError::Conflict` - PDF with same checksum already exists
    /// - `StorageError::Database` - Database operation failed
    async fn create_pdf(&self, request: CreatePdfRequest) -> Result<Uuid>;

    /// Get PDF by ID.
    ///
    /// # Arguments
    ///
    /// * `pdf_id` - PDF identifier
    ///
    /// # Returns
    ///
    /// * `Ok(Some(pdf))` - PDF document found
    /// * `Ok(None)` - PDF not found
    /// * `Err(StorageError)` - If retrieval fails
    async fn get_pdf(&self, pdf_id: &Uuid) -> Result<Option<PdfDocument>>;

    /// Find PDF by checksum (deduplication).
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace to search in
    /// * `checksum` - SHA-256 checksum
    ///
    /// # Returns
    ///
    /// * `Ok(Some(pdf))` - Duplicate found
    /// * `Ok(None)` - No duplicate
    /// * `Err(StorageError)` - If search fails
    async fn find_pdf_by_checksum(
        &self,
        workspace_id: &Uuid,
        checksum: &str,
    ) -> Result<Option<PdfDocument>>;

    /// Update PDF processing status.
    ///
    /// # Arguments
    ///
    /// * `pdf_id` - PDF identifier
    /// * `status` - New processing status
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Status updated
    /// * `Err(StorageError)` - If update fails
    async fn update_pdf_status(&self, pdf_id: &Uuid, status: PdfProcessingStatus) -> Result<()>;

    /// Update PDF processing results (full update).
    ///
    /// # Arguments
    ///
    /// * `request` - Update request with all fields
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Update successful
    /// * `Err(StorageError)` - If update fails
    async fn update_pdf_processing(&self, request: UpdatePdfProcessingRequest) -> Result<()>;

    /// Link PDF to processed document.
    ///
    /// # Arguments
    ///
    /// * `pdf_id` - PDF identifier
    /// * `document_id` - Document identifier
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Link created
    /// * `Err(StorageError)` - If linking fails
    async fn link_pdf_to_document(&self, pdf_id: &Uuid, document_id: &Uuid) -> Result<()>;

    /// List PDFs with filtering and pagination.
    ///
    /// # Arguments
    ///
    /// * `filter` - Filter and pagination options
    ///
    /// # Returns
    ///
    /// * `Ok(list)` - PDF list with pagination info
    /// * `Err(StorageError)` - If query fails
    async fn list_pdfs(&self, filter: ListPdfFilter) -> Result<PdfList>;

    /// Delete PDF by ID.
    ///
    /// # Arguments
    ///
    /// * `pdf_id` - PDF identifier
    ///
    /// # Returns
    ///
    /// * `Ok(())` - PDF deleted
    /// * `Err(StorageError)` - If deletion fails
    async fn delete_pdf(&self, pdf_id: &Uuid) -> Result<()>;

    /// Get PDF count by workspace and status.
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace identifier
    /// * `status` - Optional status filter
    ///
    /// # Returns
    ///
    /// * `Ok(count)` - Number of PDFs matching criteria
    /// * `Err(StorageError)` - If query fails
    async fn count_pdfs(
        &self,
        workspace_id: &Uuid,
        status: Option<PdfProcessingStatus>,
    ) -> Result<i64>;

    /// Ensure a row exists in the `documents` relational table.
    ///
    /// WHY: The `pdf_documents.document_id` column has a foreign key constraint
    /// referencing `documents(id)`. Without a corresponding row in `documents`,
    /// `link_pdf_to_document` fails with a FK violation (GitHub Issue #74).
    /// This also enables cascade deletes when a document is removed (Issue #73).
    ///
    /// Uses INSERT ... ON CONFLICT DO UPDATE to be idempotent.
    ///
    /// @implements FIX-ISSUE-74: Ensure document record exists before FK link
    async fn ensure_document_record(
        &self,
        document_id: &Uuid,
        workspace_id: &Uuid,
        tenant_id: Option<&Uuid>,
        title: &str,
        content: &str,
        status: &str,
    ) -> Result<()>;

    /// Delete a document row from the `documents` relational table.
    ///
    /// WHY: Cascade-deletes related rows in `pdf_documents` and `chunks`
    /// via ON DELETE CASCADE foreign keys (GitHub Issue #73).
    ///
    /// @implements FIX-ISSUE-73: Cascade delete pdf_documents/chunks on document removal
    async fn delete_document_record(&self, document_id: &Uuid) -> Result<()>;
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate SHA-256 checksum of PDF data.
///
/// # Arguments
///
/// * `data` - PDF file bytes
///
/// # Returns
///
/// Hex-encoded SHA-256 checksum (64 chars)
pub fn calculate_pdf_checksum(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Validate PDF data format.
///
/// # Arguments
///
/// * `data` - PDF file bytes
///
/// # Returns
///
/// * `Ok(())` - Valid PDF
/// * `Err(StorageError)` - Invalid format
///
/// # Checks
///
/// - Starts with PDF magic number (%PDF-)
/// - Size is within limits (0 < size <= 100MB)
pub fn validate_pdf_data(data: &[u8]) -> Result<()> {
    // Check size
    if data.is_empty() {
        return Err(StorageError::InvalidData("PDF data is empty".to_string()));
    }

    if data.len() > 104_857_600 {
        // 100MB
        return Err(StorageError::InvalidData(format!(
            "PDF size {} exceeds 100MB limit",
            data.len()
        )));
    }

    // Check PDF signature
    if !data.starts_with(b"%PDF-") {
        return Err(StorageError::InvalidData(
            "Invalid PDF format (missing %PDF- signature)".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_pdf_processing_status_conversions() {
        assert_eq!(PdfProcessingStatus::Pending.as_str(), "pending");
        assert_eq!(PdfProcessingStatus::Processing.as_str(), "processing");
        assert_eq!(PdfProcessingStatus::Completed.as_str(), "completed");
        assert_eq!(PdfProcessingStatus::Failed.as_str(), "failed");

        assert_eq!(
            PdfProcessingStatus::from_str("pending").unwrap(),
            PdfProcessingStatus::Pending
        );
        assert!(PdfProcessingStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_extraction_method_conversions() {
        assert_eq!(ExtractionMethod::Text.as_str(), "text");
        assert_eq!(ExtractionMethod::Vision.as_str(), "vision");
        assert_eq!(ExtractionMethod::Hybrid.as_str(), "hybrid");

        assert_eq!(
            ExtractionMethod::from_str("vision").unwrap(),
            ExtractionMethod::Vision
        );
        assert!(ExtractionMethod::from_str("invalid").is_err());
    }

    #[test]
    fn test_calculate_pdf_checksum() {
        let data = b"test data";
        let checksum = calculate_pdf_checksum(data);
        assert_eq!(checksum.len(), 64); // SHA-256 is 64 hex chars
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_validate_pdf_data() {
        // Valid PDF
        let valid_pdf = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n";
        assert!(validate_pdf_data(valid_pdf).is_ok());

        // Empty data
        assert!(validate_pdf_data(&[]).is_err());

        // Invalid signature
        let invalid_pdf = b"Not a PDF";
        assert!(validate_pdf_data(invalid_pdf).is_err());

        // Too large (simulate)
        let large_data = vec![b'%'; 105_000_000];
        assert!(validate_pdf_data(&large_data).is_err());
    }
}
