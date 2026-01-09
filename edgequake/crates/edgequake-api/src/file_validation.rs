//! File validation utilities for document handling.
//!
//! ## Implements
//!
//! - [`FEAT0430`]: File size validation
//! - [`FEAT0431`]: Extension whitelist validation
//! - [`FEAT0432`]: UTF-8 content validation
//!
//! ## Use Cases
//!
//! - [`UC2030`]: System validates file upload
//! - [`UC2031`]: System rejects unsupported file types
//!
//! ## Enforces
//!
//! - [`BR0430`]: Maximum file size limit
//! - [`BR0431`]: Extension whitelist enforcement
//!
//! This module provides reusable file validation functions to ensure DRY
//! compliance across document upload handlers.

use crate::error::{ApiError, ApiResult};

/// Allowed file extensions for text-based uploads.
pub const ALLOWED_EXTENSIONS: [&str; 9] = [
    "txt", "md", "json", "csv", "html", "htm", "xml", "yaml", "yml",
];

/// Validate file size against a maximum limit.
///
/// # Arguments
/// * `size` - The file size in bytes
/// * `max_size` - Maximum allowed size in bytes
///
/// # Returns
/// * `Ok(())` if size is within limit
/// * `Err(ApiError::BadRequest)` if size exceeds limit
pub fn validate_file_size(size: usize, max_size: usize) -> ApiResult<()> {
    if size > max_size {
        return Err(ApiError::BadRequest(format!(
            "File exceeds maximum size of {} bytes",
            max_size
        )));
    }
    Ok(())
}

/// Extract and validate file extension.
///
/// # Arguments
/// * `filename` - The filename to extract extension from
///
/// # Returns
/// * `Ok(extension)` - Lowercased extension string if valid
/// * `Err(ApiError::BadRequest)` if extension is not in allowed list
pub fn validate_extension(filename: &str) -> ApiResult<String> {
    let extension = filename.rsplit('.').next().unwrap_or("").to_lowercase();

    if !ALLOWED_EXTENSIONS.contains(&extension.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Unsupported file type: .{}. Allowed types: {:?}",
            extension, ALLOWED_EXTENSIONS
        )));
    }

    Ok(extension)
}

/// Convert file content to UTF-8 string with validation.
///
/// # Arguments
/// * `content` - Raw bytes of file content
///
/// # Returns
/// * `Ok(text)` - UTF-8 string if valid
/// * `Err(ApiError::BadRequest)` if not valid UTF-8
pub fn validate_utf8(content: &[u8]) -> ApiResult<String> {
    String::from_utf8(content.to_vec())
        .map_err(|e| ApiError::BadRequest(format!("File is not valid UTF-8: {}", e)))
}

/// Get MIME type from file extension.
///
/// # Arguments
/// * `extension` - Lowercased file extension
///
/// # Returns
/// MIME type string corresponding to the extension
pub fn get_mime_type(extension: &str) -> &'static str {
    match extension {
        "txt" => "text/plain",
        "md" => "text/markdown",
        "json" => "application/json",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "xml" => "application/xml",
        "yaml" | "yml" => "application/x-yaml",
        _ => "application/octet-stream",
    }
}

/// Comprehensive file validation combining size, extension, and UTF-8 checks.
///
/// # Arguments
/// * `filename` - Name of the file
/// * `content` - Raw file content bytes
/// * `max_size` - Maximum allowed file size
///
/// # Returns
/// * `Ok((extension, text_content, mime_type))` - Validated file info
/// * `Err(ApiError)` - If any validation fails
pub fn validate_file(
    filename: &str,
    content: &[u8],
    max_size: usize,
) -> ApiResult<(String, String, &'static str)> {
    validate_file_size(content.len(), max_size)?;
    let extension = validate_extension(filename)?;
    let text_content = validate_utf8(content)?;

    if text_content.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "File content cannot be empty".to_string(),
        ));
    }

    let mime_type = get_mime_type(&extension);

    Ok((extension, text_content, mime_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_file_size_ok() {
        assert!(validate_file_size(100, 1000).is_ok());
    }

    #[test]
    fn test_validate_file_size_exact() {
        assert!(validate_file_size(1000, 1000).is_ok());
    }

    #[test]
    fn test_validate_file_size_exceeded() {
        let result = validate_file_size(1001, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_extension_txt() {
        assert_eq!(validate_extension("test.txt").unwrap(), "txt");
    }

    #[test]
    fn test_validate_extension_md() {
        assert_eq!(validate_extension("readme.MD").unwrap(), "md");
    }

    #[test]
    fn test_validate_extension_invalid() {
        let result = validate_extension("test.exe");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_extension_no_extension() {
        let result = validate_extension("README");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_utf8_valid() {
        let content = "Hello, world!".as_bytes();
        assert_eq!(validate_utf8(content).unwrap(), "Hello, world!");
    }

    #[test]
    fn test_validate_utf8_invalid() {
        let content = vec![0xff, 0xfe]; // Invalid UTF-8
        assert!(validate_utf8(&content).is_err());
    }

    #[test]
    fn test_get_mime_type() {
        assert_eq!(get_mime_type("txt"), "text/plain");
        assert_eq!(get_mime_type("md"), "text/markdown");
        assert_eq!(get_mime_type("json"), "application/json");
        assert_eq!(get_mime_type("csv"), "text/csv");
        assert_eq!(get_mime_type("html"), "text/html");
        assert_eq!(get_mime_type("htm"), "text/html");
        assert_eq!(get_mime_type("xml"), "application/xml");
        assert_eq!(get_mime_type("yaml"), "application/x-yaml");
        assert_eq!(get_mime_type("yml"), "application/x-yaml");
        assert_eq!(get_mime_type("unknown"), "application/octet-stream");
    }

    #[test]
    fn test_validate_file_success() {
        let content = "Hello, world!".as_bytes();
        let result = validate_file("test.txt", content, 1000);
        assert!(result.is_ok());
        let (ext, text, mime) = result.unwrap();
        assert_eq!(ext, "txt");
        assert_eq!(text, "Hello, world!");
        assert_eq!(mime, "text/plain");
    }

    #[test]
    fn test_validate_file_empty() {
        let content = "   ".as_bytes();
        let result = validate_file("test.txt", content, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_file_too_large() {
        let content = "x".repeat(1001);
        let result = validate_file("test.txt", content.as_bytes(), 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_file_bad_extension() {
        let content = "Hello".as_bytes();
        let result = validate_file("test.exe", content, 1000);
        assert!(result.is_err());
    }
}
