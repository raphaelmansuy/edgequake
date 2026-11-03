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

/// Allowed file extensions for image uploads (processed via vision LLM).
pub const ALLOWED_IMAGE_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "gif", "webp"];

/// Returns the MIME type for a given image extension.
///
/// # Returns
/// MIME type string (e.g., `"image/png"`), or `None` if not an image extension.
pub fn image_mime_type(extension: &str) -> Option<&'static str> {
    match extension {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

/// Returns `true` if the given lowercase extension is a supported image type.
pub fn is_image_extension(extension: &str) -> bool {
    ALLOWED_IMAGE_EXTENSIONS.contains(&extension)
}

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

/// Sanitize an upload filename (SPEC-083 S-12).
///
/// Strips path components and replaces unsafe characters so callers never
/// persist or echo path traversal / control characters from client input.
pub fn sanitize_filename(filename: &str) -> String {
    let base = filename
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(filename)
        .trim();

    let mut sanitized: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();

    while sanitized.starts_with('.') {
        sanitized.remove(0);
    }

    if sanitized.is_empty() || sanitized == "." || sanitized == ".." {
        "upload.bin".to_string()
    } else {
        // Cap length to avoid pathological storage keys.
        sanitized.chars().take(255).collect()
    }
}

/// Human-readable document title from a client-supplied filename.
///
/// WHY this is not `sanitize_filename`: that function protects storage keys, so it must
/// strip path separators and replace every character outside `[A-Za-z0-9._-]` with `_`.
/// Applying it to the *title* turned "A beagle with its tongue out.jpg" into
/// "A_beagle_with_its_tongue_out.jpg", which reads like a filename in the documents list.
///
/// This keeps the words, drops the executable extension, collapses whitespace, strips
/// control characters that would break a JSON list row, and caps the length. It returns
/// `None` when nothing usable is left, so the caller can fall back to the sanitized name.
pub fn display_title_from_filename(filename: &str) -> Option<String> {
    let base = filename
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(filename)
        .trim();
    // Drop a trailing extension, but only when it really looks like one. WHY the guards:
    // `rsplit_once('.')` alone turned "v1.2" into "v1" and left ".jpg" intact (empty head,
    // so the guard skipped the strip and the dot survived). A version number is a title; a
    // leading dot is a hidden file, not an extension.
    let stem = match base.rsplit_once('.') {
        Some((head, tail))
            if !head.is_empty() && tail.len() <= 5 && !tail.chars().all(|c| c.is_ascii_digit()) =>
        {
            head
        }
        _ => base.trim_start_matches('.'),
    };
    let cleaned: String = stem
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| if c == '_' { ' ' } else { c })
        .collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }
    Some(collapsed.chars().take(200).collect())
}

/// Build a clear BadRequest message for extensions rejected on text upload.
///
/// SPEC-121: PDF must use `POST /api/v1/documents/pdf`; Office formats are
/// product-unsupported (not a transient upload bug).
pub fn unsupported_text_upload_extension_message(extension: &str) -> String {
    match extension {
        "pdf" => "Unsupported file type: .pdf on /documents/upload. \
             Upload PDFs via POST /api/v1/documents/pdf (multipart). \
             Text upload allows: txt, md, json, csv, html, htm, xml, yaml, yml \
             (images: png, jpg, jpeg, gif, webp)."
            .to_string(),
        "docx" | "doc" | "docm" => format!(
            "Unsupported file type: .{extension}. Word documents are not supported. \
             Export to PDF or Markdown. See SPEC-121."
        ),
        "xlsx" | "xls" | "xlsm" => format!(
            "Unsupported file type: .{extension}. Excel spreadsheets are not supported. \
             Export to CSV, PDF, or Markdown. See SPEC-121."
        ),
        "" => format!(
            "Unsupported file type: missing extension. Allowed text types: {:?}",
            ALLOWED_EXTENSIONS
        ),
        other => format!(
            "Unsupported file type: .{other}. Allowed text types: {:?}. \
             PDFs: POST /api/v1/documents/pdf. Images: png/jpg/gif/webp on this endpoint. \
             DOCX/Excel: not supported (SPEC-121).",
            ALLOWED_EXTENSIONS
        ),
    }
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
    let filename = sanitize_filename(filename);
    let extension = filename.rsplit('.').next().unwrap_or("").to_lowercase();

    if !ALLOWED_EXTENSIONS.contains(&extension.as_str()) {
        return Err(ApiError::BadRequest(
            unsupported_text_upload_extension_message(&extension),
        ));
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
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

/// Magic-byte MIME sniff for common binary types (SPEC-083 S-12).
///
/// Returns `None` when content has no recognized binary signature (typical for
/// UTF-8 text uploads). Does not depend on an external MIME database.
pub fn sniff_magic_mime(content: &[u8]) -> Option<&'static str> {
    if content.starts_with(b"%PDF-") {
        return Some("application/pdf");
    }
    if content.len() >= 8 && content.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if content.len() >= 3 && content.starts_with(b"\xff\xd8\xff") {
        return Some("image/jpeg");
    }
    if content.len() >= 6 && (content.starts_with(b"GIF87a") || content.starts_with(b"GIF89a")) {
        return Some("image/gif");
    }
    if content.len() >= 12 && content.starts_with(b"RIFF") && &content[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    // PE / MZ executable
    if content.len() >= 2 && content.starts_with(b"MZ") {
        return Some("application/vnd.microsoft.portable-executable");
    }
    // ZIP / Office Open XML / JAR
    if content.len() >= 4 && content.starts_with(b"PK\x03\x04") {
        return Some("application/zip");
    }
    None
}

/// Reject extension/MIME mismatch when magic bytes identify a different type (SPEC-083 S-12).
///
/// Text uploads (txt/md/…) must not carry binary magic. Binary uploads (pdf/images)
/// must match their declared extension.
pub fn validate_magic_matches_extension(extension: &str, content: &[u8]) -> ApiResult<()> {
    let sniffed = sniff_magic_mime(content);
    let declared = get_mime_type(extension);

    match sniffed {
        None => {
            // No binary magic — OK for text types; reject for binary extensions.
            if matches!(extension, "pdf" | "png" | "jpg" | "jpeg" | "gif" | "webp") {
                return Err(ApiError::BadRequest(format!(
                    "File content does not match .{extension} magic bytes"
                )));
            }
            Ok(())
        }
        Some(magic_mime) => {
            let text_ext = ALLOWED_EXTENSIONS.contains(&extension);
            if text_ext {
                return Err(ApiError::BadRequest(format!(
                    "File content looks like {magic_mime} but extension is .{extension}"
                )));
            }
            // Normalize jpeg aliases.
            let declared_norm = if declared == "image/jpeg" {
                "image/jpeg"
            } else {
                declared
            };
            let magic_norm = if magic_mime == "image/jpeg" {
                "image/jpeg"
            } else {
                magic_mime
            };
            if declared_norm != magic_norm && declared != "application/octet-stream" {
                return Err(ApiError::BadRequest(format!(
                    "File content MIME {magic_mime} does not match extension .{extension} ({declared})"
                )));
            }
            Ok(())
        }
    }
}

/// Comprehensive file validation combining size, extension, magic MIME, and UTF-8 checks.
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
    let _safe_name = sanitize_filename(filename);
    let extension = validate_extension(filename)?;
    validate_magic_matches_extension(&extension, content)?;
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
    fn a_caption_filename_becomes_a_readable_title() {
        // The regression this function exists for: the sanitizer ran on the title and the
        // documents list showed "A_beagle_with_its_tongue_out__outdoors".
        assert_eq!(
            display_title_from_filename("A beagle with its tongue out, outdoors.jpg").as_deref(),
            Some("A beagle with its tongue out, outdoors")
        );
        assert_eq!(
            display_title_from_filename("20-beagle-portrait.jpg").as_deref(),
            Some("20-beagle-portrait")
        );
    }

    #[test]
    fn a_title_keeps_dots_that_are_not_an_extension() {
        // WHY: "v1.2" and "README" are titles, not extensions. Stripping the last dot group
        // from every name would silently rename them.
        assert_eq!(display_title_from_filename("v1.2").as_deref(), Some("v1.2"));
        assert_eq!(
            display_title_from_filename("README").as_deref(),
            Some("README")
        );
        assert_eq!(
            display_title_from_filename("report.final.draft.md").as_deref(),
            Some("report.final.draft")
        );
    }

    #[test]
    fn a_title_with_no_words_falls_back() {
        // WHY the dotfile case is asserted as Some("jpg") and not None: the leading dot is
        // stripped and what remains is a usable word. The caller's fallback (the sanitized
        // filename) produces exactly the same string, so either path names the document
        // "jpg" — the guarantee that matters is that a dotfile never yields an empty title.
        assert_eq!(display_title_from_filename(".jpg").as_deref(), Some("jpg"));
        // Nothing but separators and control characters: the caller must fall back.
        assert_eq!(display_title_from_filename("   "), None);
        assert_eq!(display_title_from_filename("..."), None);
        assert_eq!(display_title_from_filename("\u{0}\u{1}"), None);
    }

    #[test]
    fn underscores_become_spaces_in_a_title_but_never_in_a_storage_key() {
        assert_eq!(
            display_title_from_filename("two_cups_of_coffee.jpg").as_deref(),
            Some("two cups of coffee")
        );
        assert_eq!(
            sanitize_filename("two cups of coffee.jpg"),
            "two_cups_of_coffee.jpg"
        );
    }

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
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("SPEC-121") || msg.contains("Allowed text types"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn test_validate_extension_no_extension() {
        let result = validate_extension("README");
        assert!(result.is_err());
    }

    /// SPEC-121 T4 — PDF on text-upload whitelist must hint the PDF route.
    #[test]
    fn spec121_pdf_on_text_upload_hints_pdf_route() {
        let err = validate_extension("report.pdf").unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("/documents/pdf"), "missing route hint: {msg}");
        assert!(msg.contains(".pdf"), "missing extension: {msg}");
    }

    /// SPEC-121 T6/T7 — Office formats fail closed with product-honest copy.
    #[test]
    fn spec121_office_extensions_rejected_with_clear_message() {
        for name in ["memo.docx", "sheet.xlsx", "legacy.xls", "old.doc"] {
            let err = validate_extension(name).unwrap_err();
            let msg = format!("{err:?}");
            assert!(
                msg.contains("not supported") && msg.contains("SPEC-121"),
                "{name}: {msg}"
            );
        }
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

    #[test]
    fn contract_filename_strips_path() {
        assert_eq!(sanitize_filename("../../etc/passwd.txt"), "passwd.txt");
        assert_eq!(sanitize_filename("report (1).md"), "report__1_.md");
        assert_eq!(sanitize_filename(""), "upload.bin");
        assert_eq!(sanitize_filename(".."), "upload.bin");
    }

    #[test]
    fn test_sniff_magic_mime_pdf_png_exe() {
        assert_eq!(sniff_magic_mime(b"%PDF-1.4\n"), Some("application/pdf"));
        assert_eq!(
            sniff_magic_mime(b"\x89PNG\r\n\x1a\nxxxx"),
            Some("image/png")
        );
        assert_eq!(
            sniff_magic_mime(b"MZ\x90\x00"),
            Some("application/vnd.microsoft.portable-executable")
        );
        assert_eq!(sniff_magic_mime(b"hello text"), None);
    }

    #[test]
    fn test_exe_renamed_as_txt_rejected() {
        let exe = b"MZ\x90\x00fake-pe";
        let err = validate_magic_matches_extension("txt", exe).unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("portable-executable") || msg.contains("looks like"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn e2e_exe_as_pdf_rejected() {
        let exe = b"MZ\x90\x00fake-pe";
        let err = validate_magic_matches_extension("pdf", exe).unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("portable-executable")
                || msg.contains("looks like")
                || msg.contains("magic"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn test_pdf_magic_mismatch_rejected() {
        let err = validate_magic_matches_extension("pdf", b"not a pdf").unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("magic"), "unexpected error: {msg}");
    }
}
