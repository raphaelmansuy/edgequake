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
//! - **BR0702**: PDFs cannot exceed 50 MiB (SPEC-083 D-44 / MAX_UPLOAD_BYTES)
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

/// Lightweight PDF header: everything needed to authorise and size a download
/// without loading the (possibly large) blob.
#[derive(Debug, Clone)]
pub struct PdfBlobInfo {
    /// Workspace the PDF belongs to (isolation check).
    pub workspace_id: Uuid,
    /// Original filename.
    pub filename: String,
    /// Exact stored byte length.
    pub total_bytes: u64,
}

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
    /// Cancelled by user/system (SPEC-057 P0 — not Failed).
    Cancelled,
}

impl PdfProcessingStatus {
    /// Convert to string for database storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Terminal statuses that should stamp `processed_at`.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
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
            "cancelled" => Ok(Self::Cancelled),
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
    /// EdgeParse CPU-only extraction.
    EdgeParse,
}

impl ExtractionMethod {
    /// Convert to string for database storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Vision => "vision",
            Self::Hybrid => "hybrid",
            Self::EdgeParse => "edgeparse",
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
            "edgeparse" => Ok(Self::EdgeParse),
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
    /// Pagination: page number (1-indexed). Ignored for the row window when
    /// both keyset fields are set.
    pub page: Option<usize>,
    /// Pagination: items per page.
    pub page_size: Option<usize>,
    /// Keyset bound. Both this and [`Self::after_pdf_id`] must be set.
    /// Rows are those that sort after `(created_at DESC, pdf_id DESC)`.
    pub after_created_at: Option<DateTime<Utc>>,
    /// Tie-break for [`Self::after_created_at`].
    pub after_pdf_id: Option<Uuid>,
}

impl ListPdfFilter {
    /// Cursor the caller passed. One field alone stays an offset page.
    pub fn keyset_cursor(&self) -> Option<(DateTime<Utc>, Uuid)> {
        match (self.after_created_at, self.after_pdf_id) {
            (Some(at), Some(id)) => Some((at, id)),
            _ => None,
        }
    }
}

/// Denormalized per-document stats written to the relational `documents` table
/// after every ingestion (SPEC-021 P-A1).
///
/// WHY: the `documents.entity_count` / `chunk_count` columns existed but were
/// never refreshed by any production writer, so the relational read-model
/// fallback (P5-01) served stale `0` values — the "Completed / 0 entities"
/// screenshot. This struct is the single payload passed to
/// [`PdfDocumentStorage::update_document_stats`], keeping the writer contract
/// DRY across the postgres and memory adapters.
///
/// All counts are clamped to `>= 0` by the implementation before being written.
/// `cost_usd` / token fields are `Option` because mock-provider ingestions and
/// legacy rows legitimately carry no cost.
#[derive(Debug, Clone, Default)]
pub struct DocumentStatsUpdate<'a> {
    /// Document id whose row should be refreshed.
    pub document_id: Uuid,
    /// Final ingestion status string (`"completed"`, `"partial_failure"`, ...).
    pub status: &'a str,
    /// Number of chunks produced by the pipeline.
    pub chunk_count: i32,
    /// Number of entities extracted (matches KV `metadata.entity_count`).
    pub entity_count: i32,
    /// Number of relationships extracted.
    pub relationship_count: i32,
    /// LLM cost in USD, if tracked.
    pub cost_usd: Option<f64>,
    /// LLM input tokens, if tracked.
    pub input_tokens: Option<i64>,
    /// LLM output tokens, if tracked.
    pub output_tokens: Option<i64>,
    /// LLM total tokens, if tracked.
    pub total_tokens: Option<i64>,
    /// Terminal/partial-failure diagnostic text, if any.
    pub error_message: Option<&'a str>,
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
    /// Another row exists after `items`.
    pub has_more: bool,
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

    /// Get PDF authorisation/size info **without** loading the blob.
    ///
    /// Backends that keep bytes in a separate row should override this; the
    /// default loads the whole PDF (correct, just not cheap).
    async fn get_pdf_blob_info(&self, pdf_id: &Uuid) -> Result<Option<PdfBlobInfo>> {
        Ok(self.get_pdf(pdf_id).await?.map(|pdf| PdfBlobInfo {
            workspace_id: pdf.workspace_id,
            filename: pdf.filename,
            total_bytes: pdf.pdf_data.len() as u64,
        }))
    }

    /// Read the inclusive byte window `start..=end` of a stored PDF.
    ///
    /// Powers HTTP `Range` downloads so the viewer can load pages on demand.
    /// Callers must pass a window already validated against
    /// [`PdfBlobInfo::total_bytes`]. The default slices a full read.
    async fn get_pdf_bytes_range(
        &self,
        pdf_id: &Uuid,
        start: u64,
        end: u64,
    ) -> Result<Option<Vec<u8>>> {
        Ok(self.get_pdf(pdf_id).await?.and_then(|pdf| {
            let (s, e) = (start as usize, end as usize);
            (e < pdf.pdf_data.len() && s <= e).then(|| pdf.pdf_data[s..=e].to_vec())
        }))
    }

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

    /// Heal / persist accurate `page_count` discovered after upload.
    ///
    /// Used when the create-time heuristic missed object-stream PDFs and
    /// pdfium later counted pages accurately (vision timeout budgeting).
    async fn update_pdf_page_count(&self, pdf_id: &Uuid, page_count: i32) -> Result<()>;

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

    /// Clear the cached markdown conversion for a PDF.
    ///
    /// WHY: `update_pdf_processing` uses `COALESCE($markdown, markdown_content)`,
    /// so passing `None` leaves the old markdown untouched. To force a true
    /// PDF -> markdown re-conversion (Replace / "Re-convert from PDF"), the
    /// cached markdown must be explicitly NULLed out so the resume shortcut in
    /// `pdf_processing` cannot reuse it.
    ///
    /// Also clears `extraction_method` and `extraction_errors` so stale
    /// metadata from a previous conversion does not leak into the new run.
    ///
    /// # Arguments
    ///
    /// * `pdf_id` - PDF identifier
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Markdown cleared (or PDF row not found, treated as success)
    /// * `Err(StorageError)` - If the update fails
    async fn clear_markdown(&self, pdf_id: &Uuid) -> Result<()>;

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
    /// **Content invariant:** on conflict, `content` must not shrink — a shorter
    /// payload must leave the longer stored body intact (guards against summary
    /// truncation bugs writing into the detail SSOT column).
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

    /// Refresh the denormalized stat/cost columns of a `documents` row after
    /// ingestion (SPEC-021 P-A1/P-A2).
    ///
    /// WHY: `ensure_document_record` only inserts/updates `title/content/status`.
    /// The `chunk_count`/`entity_count`/`relationship_count`/cost columns were
    /// never written, so the relational read-model fallback (P5-01) returned
    /// stale `0` values. This method closes the write-path gap.
    ///
    /// The implementation MUST be idempotent and race-safe (upsert semantics):
    /// if the `documents` row does not yet exist (race with
    /// `ensure_document_record`), the call is a no-op that logs a warning
    /// rather than an error — the next `ensure_document_record` + a follow-up
    /// stats refresh will converge. Implementations clamp counts to `>= 0`.
    ///
    /// Best-effort by design: callers MUST treat an error here as non-fatal
    /// (the KV metadata write remains the authoritative stats carrier).
    async fn update_document_stats(&self, stats: &DocumentStatsUpdate<'_>) -> Result<()>;

    /// Best-effort status-only refresh of the relational `documents` row
    /// (SPEC-047 P1 — early sync during soft-resume / indexing).
    ///
    /// Unlike [`Self::update_document_stats`], this does **not** overwrite
    /// chunk/entity counts with zeros. Call on every non-terminal stage so the
    /// Documents list does not stay stuck on a prior `failed` / stale status
    /// while KV already shows `indexing`.
    async fn touch_document_status(&self, document_id: &Uuid, status: &str) -> Result<()>;

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
/// - Size is within limits (0 < size <= 50 MiB; SPEC-083 D-44)
pub fn validate_pdf_data(data: &[u8]) -> Result<()> {
    // Check size
    if data.is_empty() {
        return Err(StorageError::InvalidData("PDF data is empty".to_string()));
    }

    // SPEC-083 D-44: must match edgequake_core::MAX_UPLOAD_BYTES (50 MiB).
    const MAX_PDF_BYTES: usize = 50 * 1024 * 1024;
    if data.len() > MAX_PDF_BYTES {
        return Err(StorageError::InvalidData(format!(
            "PDF size {} exceeds 50 MiB limit",
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
        assert_eq!(PdfProcessingStatus::Cancelled.as_str(), "cancelled");
        assert!(PdfProcessingStatus::Cancelled.is_terminal());
        assert!(!PdfProcessingStatus::Processing.is_terminal());

        assert_eq!(
            PdfProcessingStatus::from_str("pending").unwrap(),
            PdfProcessingStatus::Pending
        );
        assert_eq!(
            PdfProcessingStatus::from_str("cancelled").unwrap(),
            PdfProcessingStatus::Cancelled
        );
        assert!(PdfProcessingStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_extraction_method_conversions() {
        assert_eq!(ExtractionMethod::Text.as_str(), "text");
        assert_eq!(ExtractionMethod::Vision.as_str(), "vision");
        assert_eq!(ExtractionMethod::Hybrid.as_str(), "hybrid");
        assert_eq!(ExtractionMethod::EdgeParse.as_str(), "edgeparse");

        assert_eq!(
            ExtractionMethod::from_str("vision").unwrap(),
            ExtractionMethod::Vision
        );
        assert_eq!(
            ExtractionMethod::from_str("edgeparse").unwrap(),
            ExtractionMethod::EdgeParse
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
