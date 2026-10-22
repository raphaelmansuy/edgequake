//! SPEC-083 D-51: stream multipart file fields to temp files + batch file cap.
//!
//! Multipart `field.bytes()` buffers each file fully in RAM and batch handlers
//! previously accumulated every file before processing. This module streams
//! chunks to a [`tempfile::NamedTempFile`] and enforces
//! [`edgequake_core::max_batch_upload_files`].

use std::io::Write;

use axum_extra::extract::multipart::Field;
use tempfile::NamedTempFile;
use tracing::debug;

use crate::error::{ApiError, ApiResult};

/// A multipart file streamed to disk (not held entirely in a `Vec` during parse).
pub struct StreamedUploadFile {
    pub filename: String,
    pub temp: NamedTempFile,
}

impl StreamedUploadFile {
    /// Read the temp file into memory for downstream validation / hashing.
    ///
    /// Call after the multipart parse loop so only one file's bytes are live
    /// during sequential processing.
    pub fn into_bytes(self) -> ApiResult<(String, Vec<u8>)> {
        let path = self.temp.path().to_path_buf();
        let bytes = std::fs::read(&path).map_err(|e| {
            ApiError::Internal(format!("Failed to read streamed upload temp file: {e}"))
        })?;
        Ok((self.filename, bytes))
    }
}

/// Stream a multipart file field to a temporary file (chunked; not `field.bytes()`).
pub async fn stream_field_to_tempfile(
    mut field: Field,
    filename: String,
) -> ApiResult<StreamedUploadFile> {
    let mut temp = NamedTempFile::new()
        .map_err(|e| ApiError::Internal(format!("Failed to create upload temp file: {e}")))?;

    let mut total: usize = 0;
    while let Some(chunk) = field.chunk().await.map_err(|e| {
        let mut detail = e.to_string();
        if let Some(source) = std::error::Error::source(&e) {
            detail.push_str(": ");
            detail.push_str(&source.to_string());
        }
        ApiError::BadRequest(format!("Failed to read file chunk: {detail}"))
    })? {
        total = total.saturating_add(chunk.len());
        temp.write_all(&chunk)
            .map_err(|e| ApiError::Internal(format!("Failed to write upload temp file: {e}")))?;
    }
    temp.flush()
        .map_err(|e| ApiError::Internal(format!("Failed to flush upload temp file: {e}")))?;

    debug!(
        filename = %filename,
        bytes = total,
        temp = %temp.path().display(),
        "SPEC-083 D-51: streamed multipart field to temp file"
    );

    Ok(StreamedUploadFile { filename, temp })
}

/// Ensure a batch has not exceeded the configured file-count cap.
pub fn ensure_batch_file_cap(current_count: usize) -> ApiResult<()> {
    let max = edgequake_core::max_batch_upload_files();
    if current_count >= max {
        return Err(ApiError::BadRequest(format!(
            "Batch upload exceeds max files ({max}). \
             Reduce the batch or raise EDGEQUAKE_MAX_BATCH_UPLOAD_FILES."
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e2e_batch_file_cap() {
        let max = edgequake_core::MAX_BATCH_UPLOAD_FILES;
        assert!(max >= 1);
        // At capacity, next file is rejected.
        let err = ensure_batch_file_cap(max).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("max files") || msg.contains("Batch upload"),
            "unexpected error: {msg}"
        );
        // Under capacity is ok.
        assert!(ensure_batch_file_cap(max.saturating_sub(1)).is_ok());
    }

    #[test]
    fn e2e_upload_streams_to_temp_contract() {
        // Source-level contract: helpers must stream via chunk() + NamedTempFile.
        let src = include_str!("multipart_upload.rs");
        assert!(src.contains("stream_field_to_tempfile"));
        assert!(src.contains("NamedTempFile"));
        assert!(src.contains(".chunk()"));
        // Implementation body (before tests) must stream chunks — not Field::bytes.
        let impl_src = src.split("#[cfg(test)]").next().unwrap_or(src);
        let live = impl_src
            .lines()
            .filter(|l| {
                let t = l.trim_start();
                !t.starts_with("//") && !t.starts_with("///") && !t.starts_with('*')
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !live.contains("field.bytes"),
            "stream helper must not call Field::bytes"
        );
    }
}
