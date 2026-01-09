//! Document validation helpers.
//!
//! ## Implements
//!
//! - [`FEAT0420`]: Content size validation
//! - [`FEAT0421`]: Empty content detection
//! - [`FEAT0422`]: Content summary generation
//!
//! ## Use Cases
//!
//! - [`UC2020`]: System validates document before processing
//! - [`UC2021`]: System generates content preview
//!
//! ## Enforces
//!
//! - [`BR0420`]: Maximum document size limit
//! - [`BR0421`]: Non-empty content requirement
//!
//! # WHY: Single Responsibility
//!
//! Validation logic was duplicated across multiple handlers in documents.rs.
//! This module centralizes common validation patterns to:
//! - Reduce code duplication (DRY principle)
//! - Ensure consistent error messages
//! - Make validation rules easy to modify in one place
//!
//! # Usage
//!
//! ```rust,ignore
//! use edgequake_api::validation::{validate_content, generate_content_summary};
//!
//! fn upload_document(content: &str, max_size: usize) -> Result<(), ApiError> {
//!     validate_content(content, max_size)?;
//!     let summary = generate_content_summary(content);
//!     // ... process document
//!     Ok(())
//! }
//! ```

use crate::error::{ApiError, ApiResult};

/// Maximum length for content summary (200 characters + "...").
const SUMMARY_MAX_CHARS: usize = 200;

/// Validate document content for size and emptiness.
///
/// # Arguments
///
/// * `content` - The document content to validate
/// * `max_size` - Maximum allowed size in bytes
///
/// # Errors
///
/// * `ApiError::BadRequest` - If content exceeds max size
/// * `ApiError::ValidationError` - If content is empty or whitespace only
///
/// # Example
///
/// ```rust,ignore
/// validate_content(&request.content, state.config.max_document_size)?;
/// ```
pub fn validate_content(content: &str, max_size: usize) -> ApiResult<()> {
    if content.len() > max_size {
        return Err(ApiError::BadRequest(format!(
            "Document exceeds maximum size of {} bytes",
            max_size
        )));
    }

    if content.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Document content cannot be empty".to_string(),
        ));
    }

    Ok(())
}

/// Generate a content summary (first 200 characters with ellipsis if truncated).
///
/// # Arguments
///
/// * `content` - The full document content
///
/// # Returns
///
/// A summary string, either the full content if <= 200 chars,
/// or the first 200 characters followed by "..."
///
/// # Example
///
/// ```rust,ignore
/// let summary = generate_content_summary(&document_content);
/// ```
pub fn generate_content_summary(content: &str) -> String {
    if content.len() > SUMMARY_MAX_CHARS {
        format!(
            "{}...",
            content.chars().take(SUMMARY_MAX_CHARS).collect::<String>()
        )
    } else {
        content.to_string()
    }
}

/// Validate a query string (non-empty after trimming).
///
/// # Arguments
///
/// * `query` - The query string to validate
/// * `field_name` - Name of the field for error messages (e.g., "query", "message")
///
/// # Errors
///
/// * `ApiError::ValidationError` - If query is empty or whitespace only
pub fn validate_non_empty(query: &str, field_name: &str) -> ApiResult<()> {
    if query.trim().is_empty() {
        return Err(ApiError::ValidationError(format!(
            "{} cannot be empty",
            field_name
        )));
    }
    Ok(())
}

/// Validate a query string for emptiness and maximum length.
///
/// # Arguments
///
/// * `query` - The query string to validate
/// * `max_length` - Maximum allowed length in characters
///
/// # Errors
///
/// * `ApiError::ValidationError` - If query is empty or whitespace only
/// * `ApiError::BadRequest` - If query exceeds max length
pub fn validate_query(query: &str, max_length: usize) -> ApiResult<()> {
    if query.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Query cannot be empty".to_string(),
        ));
    }

    if query.len() > max_length {
        return Err(ApiError::BadRequest(format!(
            "Query exceeds maximum length of {} characters",
            max_length
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_content_success() {
        let result = validate_content("Hello, world!", 1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_content_too_large() {
        let result = validate_content("Hello, world!", 5);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn test_validate_content_empty() {
        let result = validate_content("", 1000);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ApiError::ValidationError(_)));
    }

    #[test]
    fn test_validate_content_whitespace_only() {
        let result = validate_content("   \n\t  ", 1000);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ApiError::ValidationError(_)));
    }

    #[test]
    fn test_generate_content_summary_short() {
        let content = "Short content";
        let summary = generate_content_summary(content);
        assert_eq!(summary, content);
        assert!(!summary.ends_with("..."));
    }

    #[test]
    fn test_generate_content_summary_exactly_200() {
        let content: String = (0..200).map(|_| 'a').collect();
        let summary = generate_content_summary(&content);
        assert_eq!(summary, content);
        assert!(!summary.ends_with("..."));
    }

    #[test]
    fn test_generate_content_summary_truncated() {
        let content: String = (0..300).map(|_| 'a').collect();
        let summary = generate_content_summary(&content);
        assert!(summary.ends_with("..."));
        assert_eq!(summary.len(), 203); // 200 chars + "..."
    }

    #[test]
    fn test_generate_content_summary_unicode() {
        // Test with unicode characters (emoji takes multiple bytes)
        let content = "🎉".repeat(100); // 100 emojis
        let summary = generate_content_summary(&content);
        // Should truncate by characters, not bytes
        assert!(summary.chars().count() <= 203);
    }

    #[test]
    fn test_validate_non_empty_success() {
        let result = validate_non_empty("hello", "query");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_non_empty_empty_string() {
        let result = validate_non_empty("", "message");
        assert!(result.is_err());
        if let Err(ApiError::ValidationError(msg)) = result {
            assert!(msg.contains("message"));
        }
    }

    #[test]
    fn test_validate_non_empty_whitespace() {
        let result = validate_non_empty("   ", "query");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_query_success() {
        let result = validate_query("What is the meaning of life?", 1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_query_empty() {
        let result = validate_query("", 1000);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ApiError::ValidationError(_)));
    }

    #[test]
    fn test_validate_query_too_long() {
        let long_query: String = (0..200).map(|_| 'a').collect();
        let result = validate_query(&long_query, 100);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn test_validate_content_exactly_max_size() {
        let content: String = (0..100).map(|_| 'a').collect();
        let result = validate_content(&content, 100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_content_one_over_max_size() {
        let content: String = (0..101).map(|_| 'a').collect();
        let result = validate_content(&content, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_content_error_includes_max_size() {
        let result = validate_content("Hello, world!", 5);
        if let Err(ApiError::BadRequest(msg)) = result {
            assert!(msg.contains("5"));
        } else {
            panic!("Expected BadRequest error");
        }
    }

    #[test]
    fn test_generate_content_summary_empty() {
        let summary = generate_content_summary("");
        assert_eq!(summary, "");
    }

    #[test]
    fn test_generate_content_summary_201_chars() {
        let content: String = (0..201).map(|_| 'a').collect();
        let summary = generate_content_summary(&content);
        assert!(summary.ends_with("..."));
        assert_eq!(summary.len(), 203);
    }

    #[test]
    fn test_validate_query_whitespace_only() {
        let result = validate_query("   \n\t   ", 1000);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ApiError::ValidationError(_)));
    }

    #[test]
    fn test_validate_query_exactly_max_length() {
        let query: String = (0..100).map(|_| 'a').collect();
        let result = validate_query(&query, 100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_query_one_over_max_length() {
        let query: String = (0..101).map(|_| 'a').collect();
        let result = validate_query(&query, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_non_empty_with_inner_whitespace() {
        let result = validate_non_empty("hello world", "field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_non_empty_error_includes_field_name() {
        let result = validate_non_empty("", "custom_field");
        if let Err(ApiError::ValidationError(msg)) = result {
            assert!(msg.contains("custom_field"));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[test]
    fn test_validate_content_with_newlines() {
        let content = "Line 1\nLine 2\nLine 3";
        let result = validate_content(content, 1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_content_only_newlines() {
        let content = "\n\n\n";
        let result = validate_content(content, 1000);
        assert!(result.is_err()); // Only whitespace
    }
}
