//! Chat request validation helpers.
//!
//! Centralises all validation logic so `completion` and `streaming` handlers
//! share a single source of truth (DRY) and each handler keeps a single
//! responsibility (SRP).
//!
//! @implements FEAT0801 (Image attachment validation)
//! @implements BR0574 (Image size and MIME-type constraints)

use crate::error::ApiError;
use crate::handlers::chat_types::ImageAttachment;

// ── Image constraints ────────────────────────────────────────────────────────

/// Maximum number of image attachments per request.
pub const MAX_IMAGES: usize = 4;

/// Maximum decoded byte size per image (20 MiB).
pub const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;

/// Accepted image MIME types.
pub const ACCEPTED_MIME: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];

// ── Validation functions ─────────────────────────────────────────────────────

/// Validate a slice of [`ImageAttachment`] values.
///
/// Checks:
/// 1. Count ≤ [`MAX_IMAGES`]
/// 2. Each MIME type is in [`ACCEPTED_MIME`]
/// 3. Each decoded byte estimate ≤ [`MAX_IMAGE_BYTES`]
///
/// # Errors
///
/// Returns [`ApiError::BadRequest`] on the first violated constraint.
pub fn validate_image_attachments(images: &[ImageAttachment]) -> Result<(), ApiError> {
    if images.len() > MAX_IMAGES {
        return Err(ApiError::BadRequest(format!(
            "Too many images: maximum {} allowed, got {}",
            MAX_IMAGES,
            images.len()
        )));
    }

    for (i, img) in images.iter().enumerate() {
        if !ACCEPTED_MIME.contains(&img.mime_type.as_str()) {
            return Err(ApiError::BadRequest(format!(
                "Image {}: unsupported MIME type '{}'; accepted: {}",
                i + 1,
                img.mime_type,
                ACCEPTED_MIME.join(", ")
            )));
        }

        // Base64 encoding inflates size by ~4/3; invert to estimate raw bytes.
        let estimated_bytes = img.data.len() * 3 / 4;
        if estimated_bytes > MAX_IMAGE_BYTES {
            return Err(ApiError::BadRequest(format!(
                "Image {} exceeds 20 MiB limit (estimated {} bytes)",
                i + 1,
                estimated_bytes
            )));
        }
    }

    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn img(mime: &str, data_len: usize) -> ImageAttachment {
        ImageAttachment {
            data: "A".repeat(data_len),
            mime_type: mime.to_string(),
        }
    }

    #[test]
    fn valid_single_jpeg() {
        let images = vec![img("image/jpeg", 100)];
        assert!(validate_image_attachments(&images).is_ok());
    }

    #[test]
    fn rejects_too_many() {
        let images: Vec<_> = (0..5).map(|_| img("image/png", 10)).collect();
        let err = validate_image_attachments(&images).unwrap_err();
        assert!(err.to_string().contains("Too many images"));
    }

    #[test]
    fn rejects_bad_mime() {
        let images = vec![img("application/pdf", 10)];
        let err = validate_image_attachments(&images).unwrap_err();
        assert!(err.to_string().contains("unsupported MIME type"));
    }

    #[test]
    fn rejects_oversized() {
        // MAX_IMAGE_BYTES = 20 MiB decoded → base64 len must be > 20*1024*1024*4/3
        let huge = MAX_IMAGE_BYTES * 4 / 3 + 10;
        let images = vec![img("image/webp", huge)];
        let err = validate_image_attachments(&images).unwrap_err();
        assert!(err.to_string().contains("exceeds 20 MiB"));
    }

    #[test]
    fn accepts_exactly_max_images() {
        let images: Vec<_> = (0..MAX_IMAGES).map(|_| img("image/png", 10)).collect();
        assert!(validate_image_attachments(&images).is_ok());
    }

    #[test]
    fn empty_slice_is_valid() {
        assert!(validate_image_attachments(&[]).is_ok());
    }
}
