use super::PdfBackend;
use crate::extractor::PdfInfo;
use crate::schema::Document;
use crate::Result;
use async_trait::async_trait;

/// Mock backend for testing.
pub struct MockBackend {
    document: Document,
}

impl MockBackend {
    pub fn new() -> Self {
        Self {
            document: Document::new(),
        }
    }

    pub fn with_document(document: Document) -> Self {
        Self { document }
    }
}

#[async_trait]
impl PdfBackend for MockBackend {
    async fn extract(&self, _pdf_bytes: &[u8]) -> Result<Document> {
        Ok(self.document.clone())
    }

    fn get_info(&self, pdf_bytes: &[u8]) -> Result<PdfInfo> {
        Ok(PdfInfo {
            page_count: self.document.pages.len(),
            pdf_version: "1.7".to_string(),
            has_images: false,
            image_count: 0,
            file_size: pdf_bytes.len(),
        })
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}
