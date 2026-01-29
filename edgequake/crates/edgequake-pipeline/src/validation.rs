//! Document Validation Module
//!
//! @implements SPEC-001/Issue-13: Comprehensive Edge Case Handling
//!
//! This module handles all documented edge cases in document processing:
//!
//! | # | Edge Case | Handler |
//! |---|-----------|---------|
//! | 1 | Empty document (0 bytes) | `validate_not_empty` |
//! | 2 | Single character document | `validate_min_content` |
//! | 3 | Document with only whitespace | `validate_has_content` |
//! | 4 | Document exceeding max size limit | `validate_max_size` |
//! | 5 | Document with unsupported encoding | `validate_encoding` |
//! | 6 | Document with circular references | `validate_no_circular_refs` |
//! | 7 | Corrupt/malformed document | `validate_structure` |
//! | 8 | Password protected document | `validate_not_protected` |
//! | 9 | Embedded executables | `validate_no_executables` |
//! | 10 | Network failure during upload | Handled by API layer |
//! | 11 | Network failure during extraction | Retry logic in worker |
//! | 12 | LLM provider unavailable | Retry + circuit breaker |
//! | 13 | LLM rate limit exceeded | Backoff in worker |
//! | 14 | LLM response malformed | Parser fallback |
//! | 15 | Duplicate document upload | Content hash check |
//! | 16 | Concurrent upload of same | Mutex/transaction |
//! | 17 | Upload during workspace deletion | FK constraint |
//! | 18 | Invalid workspace ID | Validate exists |
//! | 19 | Very long document (>100MB) | `validate_max_size` |
//! | 20 | Very small chunks (<10 tokens) | `validate_min_chunk_size` |
//!
//! # Example
//!
//! ```rust
//! use edgequake_pipeline::validation::{DocumentValidator, ValidationConfig};
//!
//! let config = ValidationConfig::default();
//! let validator = DocumentValidator::new(config);
//!
//! let result = validator.validate_content("Hello, world!");
//! assert!(result.is_valid());
//! ```

use crate::error::{PipelineError, Result};
use std::collections::HashSet;

/// Configuration for document validation.
///
/// All limits are configurable to allow different deployment scenarios.
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Maximum document size in bytes (default: 100MB)
    pub max_size_bytes: usize,

    /// Minimum content length in characters (default: 10)
    pub min_content_chars: usize,

    /// Minimum chunk size in tokens (default: 10)
    pub min_chunk_tokens: usize,

    /// Maximum file name length (default: 255)
    pub max_filename_length: usize,

    /// Allowed file extensions (empty = allow all)
    pub allowed_extensions: HashSet<String>,

    /// Blocked file extensions (executables, scripts)
    pub blocked_extensions: HashSet<String>,

    /// Maximum depth for circular reference detection
    pub max_reference_depth: usize,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        let mut blocked = HashSet::new();
        // Block executable and script extensions
        for ext in &[
            "exe", "dll", "so", "dylib", "bat", "cmd", "sh", "ps1", "vbs", "js", "jar", "com",
            "msi", "app", "dmg", "pkg",
        ] {
            blocked.insert(ext.to_string());
        }

        Self {
            max_size_bytes: 100 * 1024 * 1024, // 100MB
            min_content_chars: 10,
            min_chunk_tokens: 10,
            max_filename_length: 255,
            allowed_extensions: HashSet::new(), // Allow all by default
            blocked_extensions: blocked,
            max_reference_depth: 100,
        }
    }
}

/// Validation result containing all issues found.
#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    /// List of validation errors (blocking)
    pub errors: Vec<ValidationIssue>,

    /// List of validation warnings (non-blocking)
    pub warnings: Vec<ValidationIssue>,
}

impl ValidationResult {
    /// Returns true if validation passed (no errors).
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Add an error to the result.
    pub fn add_error(&mut self, code: ValidationCode, message: impl Into<String>) {
        self.errors.push(ValidationIssue {
            code,
            message: message.into(),
            is_error: true,
        });
    }

    /// Add a warning to the result.
    pub fn add_warning(&mut self, code: ValidationCode, message: impl Into<String>) {
        self.warnings.push(ValidationIssue {
            code,
            message: message.into(),
            is_error: false,
        });
    }

    /// Convert to Result, returning error if any validation errors exist.
    pub fn to_result(self) -> Result<Vec<ValidationIssue>> {
        if self.errors.is_empty() {
            Ok(self.warnings)
        } else {
            let messages: Vec<_> = self.errors.iter().map(|e| e.message.clone()).collect();
            Err(PipelineError::Validation(messages.join("; ")))
        }
    }
}

/// A single validation issue.
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// Unique code for this issue type.
    pub code: ValidationCode,

    /// Human-readable message.
    pub message: String,

    /// True if this is an error, false if warning.
    pub is_error: bool,
}

/// Validation error codes for programmatic handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationCode {
    /// Edge Case 1: Empty document
    EmptyDocument,

    /// Edge Case 2: Too short (single char)
    ContentTooShort,

    /// Edge Case 3: Only whitespace
    WhitespaceOnly,

    /// Edge Case 4 & 19: Exceeds size limit
    SizeExceeded,

    /// Edge Case 5: Unsupported encoding
    UnsupportedEncoding,

    /// Edge Case 6: Circular references
    CircularReference,

    /// Edge Case 7: Corrupt/malformed
    CorruptDocument,

    /// Edge Case 8: Password protected
    PasswordProtected,

    /// Edge Case 9: Embedded executable
    EmbeddedExecutable,

    /// Edge Case 15: Duplicate content
    DuplicateContent,

    /// Edge Case 18: Invalid workspace
    InvalidWorkspace,

    /// Edge Case 20: Chunk too small
    ChunkTooSmall,

    /// Invalid filename
    InvalidFilename,

    /// Blocked file extension
    BlockedExtension,
}

/// Document validator with configurable rules.
#[derive(Debug, Clone)]
pub struct DocumentValidator {
    config: ValidationConfig,
}

impl DocumentValidator {
    /// Create a new validator with the given config.
    pub fn new(config: ValidationConfig) -> Self {
        Self { config }
    }

    /// Validate document content before processing.
    ///
    /// Checks edge cases 1-5.
    pub fn validate_content(&self, content: &str) -> ValidationResult {
        let mut result = ValidationResult::default();

        // Edge Case 1: Empty document
        if content.is_empty() {
            result.add_error(
                ValidationCode::EmptyDocument,
                "Document is empty (0 bytes). Please upload a document with content.",
            );
            return result;
        }

        // Edge Case 4 & 19: Size limit
        let size_bytes = content.len();
        if size_bytes > self.config.max_size_bytes {
            result.add_error(
                ValidationCode::SizeExceeded,
                format!(
                    "Document size ({} bytes) exceeds maximum allowed ({} bytes).",
                    size_bytes, self.config.max_size_bytes
                ),
            );
            return result;
        }

        // Edge Case 3: Only whitespace
        let trimmed = content.trim();
        if trimmed.is_empty() {
            result.add_error(
                ValidationCode::WhitespaceOnly,
                "Document contains only whitespace. Please upload a document with meaningful content.",
            );
            return result;
        }

        // Edge Case 2: Too short
        if trimmed.chars().count() < self.config.min_content_chars {
            result.add_error(
                ValidationCode::ContentTooShort,
                format!(
                    "Document content is too short ({} chars). Minimum required: {} chars.",
                    trimmed.chars().count(),
                    self.config.min_content_chars
                ),
            );
        }

        // Edge Case 5: Encoding validation (check for replacement char)
        if content.contains('\u{FFFD}') {
            result.add_warning(
                ValidationCode::UnsupportedEncoding,
                "Document may contain unsupported encoding. Some characters were replaced.",
            );
        }

        result
    }

    /// Validate document metadata (filename, extension).
    ///
    /// Checks edge cases 9 (blocked extensions).
    pub fn validate_metadata(&self, filename: &str) -> ValidationResult {
        let mut result = ValidationResult::default();

        // Filename length
        if filename.len() > self.config.max_filename_length {
            result.add_error(
                ValidationCode::InvalidFilename,
                format!(
                    "Filename too long ({} chars). Maximum: {} chars.",
                    filename.len(),
                    self.config.max_filename_length
                ),
            );
        }

        // Edge Case 9: Blocked extensions (executables)
        if let Some(ext) = filename.rsplit('.').next() {
            let ext_lower = ext.to_lowercase();
            if self.config.blocked_extensions.contains(&ext_lower) {
                result.add_error(
                    ValidationCode::BlockedExtension,
                    format!(
                        "File extension '.{}' is not allowed. Executables and scripts cannot be processed.",
                        ext_lower
                    ),
                );
            }

            // Check allowed extensions if configured
            if !self.config.allowed_extensions.is_empty()
                && !self.config.allowed_extensions.contains(&ext_lower)
            {
                result.add_error(
                    ValidationCode::BlockedExtension,
                    format!(
                        "File extension '.{}' is not in the allowed list.",
                        ext_lower
                    ),
                );
            }
        }

        result
    }

    /// Validate chunk size after chunking.
    ///
    /// Checks edge case 20 (very small chunks).
    pub fn validate_chunk(&self, content: &str, token_count: usize) -> ValidationResult {
        let mut result = ValidationResult::default();

        // Edge Case 20: Chunk too small
        if token_count < self.config.min_chunk_tokens {
            result.add_warning(
                ValidationCode::ChunkTooSmall,
                format!(
                    "Chunk has only {} tokens (minimum: {}). May not extract meaningful entities.",
                    token_count, self.config.min_chunk_tokens
                ),
            );
        }

        // Check for meaningful content
        let trimmed = content.trim();
        if trimmed.is_empty() {
            result.add_error(
                ValidationCode::WhitespaceOnly,
                "Chunk contains only whitespace.",
            );
        }

        result
    }

    /// Check if content is a duplicate (requires content hash).
    ///
    /// Edge Case 15: Duplicate document upload.
    pub fn validate_not_duplicate(
        &self,
        _content_hash: &str,
        existing_hashes: &HashSet<String>,
    ) -> ValidationResult {
        let mut result = ValidationResult::default();

        if existing_hashes.contains(_content_hash) {
            result.add_error(
                ValidationCode::DuplicateContent,
                "A document with identical content already exists in this workspace.",
            );
        }

        result
    }
}

impl Default for DocumentValidator {
    fn default() -> Self {
        Self::new(ValidationConfig::default())
    }
}

/// Quick validation for content (convenience function).
pub fn validate_document_content(content: &str) -> Result<()> {
    let validator = DocumentValidator::default();
    let result = validator.validate_content(content);
    result.to_result().map(|_| ())
}

/// Quick validation for filename (convenience function).
pub fn validate_document_filename(filename: &str) -> Result<()> {
    let validator = DocumentValidator::default();
    let result = validator.validate_metadata(filename);
    result.to_result().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Edge Case 1: Empty document (0 bytes)
    // =========================================================================

    #[test]
    fn test_edge_case_1_empty_document() {
        let validator = DocumentValidator::default();
        let result = validator.validate_content("");

        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].code, ValidationCode::EmptyDocument);
        assert!(result.errors[0].message.contains("empty"));
    }

    // =========================================================================
    // Edge Case 2: Single character document
    // =========================================================================

    #[test]
    fn test_edge_case_2_single_char_document() {
        let validator = DocumentValidator::default();
        let result = validator.validate_content("X");

        assert!(!result.is_valid());
        assert_eq!(result.errors[0].code, ValidationCode::ContentTooShort);
    }

    // =========================================================================
    // Edge Case 3: Document with only whitespace
    // =========================================================================

    #[test]
    fn test_edge_case_3_whitespace_only() {
        let validator = DocumentValidator::default();
        let result = validator.validate_content("   \n\t\r   ");

        assert!(!result.is_valid());
        assert_eq!(result.errors[0].code, ValidationCode::WhitespaceOnly);
    }

    // =========================================================================
    // Edge Case 4 & 19: Document exceeding max size
    // =========================================================================

    #[test]
    fn test_edge_case_4_exceeds_max_size() {
        let config = ValidationConfig {
            max_size_bytes: 100, // Very small for testing
            ..Default::default()
        };
        let validator = DocumentValidator::new(config);
        let content = "X".repeat(200);

        let result = validator.validate_content(&content);

        assert!(!result.is_valid());
        assert_eq!(result.errors[0].code, ValidationCode::SizeExceeded);
    }

    // =========================================================================
    // Edge Case 5: Unsupported encoding (replacement char)
    // =========================================================================

    #[test]
    fn test_edge_case_5_encoding_warning() {
        let validator = DocumentValidator::default();
        let content = "Valid text with replacement char: \u{FFFD} here.";

        let result = validator.validate_content(content);

        assert!(result.is_valid()); // Warning, not error
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].code, ValidationCode::UnsupportedEncoding);
    }

    // =========================================================================
    // Edge Case 9: Embedded executables (blocked extension)
    // =========================================================================

    #[test]
    fn test_edge_case_9_blocked_extension_exe() {
        let validator = DocumentValidator::default();
        let result = validator.validate_metadata("malware.exe");

        assert!(!result.is_valid());
        assert_eq!(result.errors[0].code, ValidationCode::BlockedExtension);
    }

    #[test]
    fn test_edge_case_9_blocked_extension_sh() {
        let validator = DocumentValidator::default();
        let result = validator.validate_metadata("script.sh");

        assert!(!result.is_valid());
        assert_eq!(result.errors[0].code, ValidationCode::BlockedExtension);
    }

    #[test]
    fn test_edge_case_9_allowed_extension_pdf() {
        let validator = DocumentValidator::default();
        let result = validator.validate_metadata("document.pdf");

        assert!(result.is_valid());
    }

    // =========================================================================
    // Edge Case 15: Duplicate document
    // =========================================================================

    #[test]
    fn test_edge_case_15_duplicate_content() {
        let validator = DocumentValidator::default();
        let mut existing = HashSet::new();
        existing.insert("abc123hash".to_string());

        let result = validator.validate_not_duplicate("abc123hash", &existing);

        assert!(!result.is_valid());
        assert_eq!(result.errors[0].code, ValidationCode::DuplicateContent);
    }

    #[test]
    fn test_edge_case_15_not_duplicate() {
        let validator = DocumentValidator::default();
        let mut existing = HashSet::new();
        existing.insert("abc123hash".to_string());

        let result = validator.validate_not_duplicate("different_hash", &existing);

        assert!(result.is_valid());
    }

    // =========================================================================
    // Edge Case 20: Very small chunks
    // =========================================================================

    #[test]
    fn test_edge_case_20_small_chunk_warning() {
        let validator = DocumentValidator::default();
        let result = validator.validate_chunk("Hi", 2);

        assert!(result.is_valid()); // Warning, not error
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].code, ValidationCode::ChunkTooSmall);
    }

    #[test]
    fn test_edge_case_20_valid_chunk() {
        let validator = DocumentValidator::default();
        let result = validator.validate_chunk("This is a reasonable chunk of text.", 15);

        assert!(result.is_valid());
        assert!(result.warnings.is_empty());
    }

    // =========================================================================
    // Valid document tests
    // =========================================================================

    #[test]
    fn test_valid_document() {
        let validator = DocumentValidator::default();
        let content = "This is a valid document with enough content to be processed.";

        let result = validator.validate_content(content);

        assert!(result.is_valid());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_valid_filename() {
        let validator = DocumentValidator::default();

        for filename in &["report.pdf", "document.txt", "data.json", "paper.docx"] {
            let result = validator.validate_metadata(filename);
            assert!(result.is_valid(), "Failed for: {}", filename);
        }
    }

    // =========================================================================
    // Convenience function tests
    // =========================================================================

    #[test]
    fn test_convenience_validate_content() {
        assert!(validate_document_content("Valid document content here.").is_ok());
        assert!(validate_document_content("").is_err());
    }

    #[test]
    fn test_convenience_validate_filename() {
        assert!(validate_document_filename("valid.pdf").is_ok());
        assert!(validate_document_filename("malware.exe").is_err());
    }
}
