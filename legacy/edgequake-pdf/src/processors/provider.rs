//! PDF document providers.
//!
//! Providers are responsible for loading PDF data from various sources.

use crate::error::PdfError;
use crate::Result;
use std::path::Path;

/// Trait for PDF data providers.
pub trait PdfProvider: Send + Sync {
    /// Get the PDF bytes.
    fn get_bytes(&self) -> Result<Vec<u8>>;

    /// Get the source identifier (path, URL, etc.).
    fn source(&self) -> Option<String>;
}

/// Provider for in-memory PDF bytes.
#[derive(Debug, Clone)]
pub struct ByteProvider {
    bytes: Vec<u8>,
    source: Option<String>,
}

impl ByteProvider {
    /// Create a new byte provider.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            source: None,
        }
    }

    /// Create with a source identifier.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

impl PdfProvider for ByteProvider {
    fn get_bytes(&self) -> Result<Vec<u8>> {
        Ok(self.bytes.clone())
    }

    fn source(&self) -> Option<String> {
        self.source.clone()
    }
}

/// Provider for PDF files on disk.
#[derive(Debug, Clone)]
pub struct FileProvider {
    path: String,
}

impl FileProvider {
    /// Create a new file provider.
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    /// Create from a Path.
    pub fn from_path(path: &Path) -> Self {
        Self {
            path: path.to_string_lossy().to_string(),
        }
    }
}

impl PdfProvider for FileProvider {
    fn get_bytes(&self) -> Result<Vec<u8>> {
        std::fs::read(&self.path).map_err(|e| PdfError::Io(e.to_string()))
    }

    fn source(&self) -> Option<String> {
        Some(self.path.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_provider() {
        let bytes = vec![0x25, 0x50, 0x44, 0x46]; // %PDF
        let provider = ByteProvider::new(bytes.clone()).with_source("test.pdf");

        assert_eq!(provider.get_bytes().unwrap(), bytes);
        assert_eq!(provider.source(), Some("test.pdf".to_string()));
    }

    #[test]
    fn test_file_provider_source() {
        let provider = FileProvider::new("/path/to/doc.pdf");
        assert_eq!(provider.source(), Some("/path/to/doc.pdf".to_string()));
    }
}
