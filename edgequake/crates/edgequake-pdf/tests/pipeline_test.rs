use async_trait::async_trait;
use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::{Document, PdfBackend, PdfConfig, PdfExtractor, PdfInfo};
use std::sync::Arc;

// Mock backend for testing
struct TestBackend {
    content: String,
}

#[async_trait]
impl PdfBackend for TestBackend {
    async fn extract(&self, _pdf_bytes: &[u8]) -> edgequake_pdf::Result<Document> {
        let mut doc = Document::new();
        // Add a dummy page with text
        // We need to construct a Page and Block manually, but for now let's just return empty doc
        // or check if we can use the MockBackend from the crate if it was public?
        // MockBackend is in `backend::mock`, let's see if it's public.
        // It is `pub mod mock` in `backend/mod.rs` and `pub mod backend` in `lib.rs`.
        // So `edgequake_pdf::backend::mock::MockBackend` should be available.
        Ok(doc)
    }

    fn get_info(&self, pdf_bytes: &[u8]) -> edgequake_pdf::Result<PdfInfo> {
        Ok(PdfInfo {
            page_count: 1,
            pdf_version: "1.7".to_string(),
            has_images: false,
            image_count: 0,
            file_size: pdf_bytes.len(),
        })
    }
}

#[tokio::test]
async fn test_pipeline_flow() {
    let provider = Arc::new(MockProvider::new());
    let config = PdfConfig::default();

    // Use the built-in MockBackend
    // We need to make sure we can access it.
    // In lib.rs: pub mod backend; -> pub mod mock; -> pub struct MockBackend
    // So edgequake_pdf::backend::mock::MockBackend

    let backend = Box::new(edgequake_pdf::backend::mock::MockBackend::new());
    let extractor = PdfExtractor::with_backend(backend, provider, config);

    let pdf_bytes = b"fake pdf content";
    let result = extractor.extract_to_markdown(pdf_bytes).await;

    assert!(result.is_ok());
    let markdown = result.unwrap();
    println!("Markdown: {}", markdown);
}
