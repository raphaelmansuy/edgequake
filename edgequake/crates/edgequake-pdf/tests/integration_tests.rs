//! Integration tests for PDF extraction functionality.

use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::{PdfConfig, PdfExtractor};

const SAMPLE_PDF_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/test-data/sample.pdf");

fn create_extractor() -> PdfExtractor {
    let provider = Arc::new(MockProvider::new());
    PdfExtractor::new(provider)
}

fn create_extractor_with_config(config: PdfConfig) -> PdfExtractor {
    let provider = Arc::new(MockProvider::new());
    PdfExtractor::with_config(provider, config)
}

fn load_sample_pdf() -> Vec<u8> {
    std::fs::read(SAMPLE_PDF_PATH).expect("Failed to load sample.pdf")
}

#[test]
fn test_get_pdf_info() {
    let extractor = create_extractor();
    let pdf_bytes = load_sample_pdf();

    let info = extractor
        .get_info(&pdf_bytes)
        .expect("Failed to get PDF info");

    assert!(info.page_count >= 1, "PDF should have at least 1 page");
    assert!(info.file_size > 0, "PDF should have non-zero size");
    assert!(!info.pdf_version.is_empty(), "PDF version should be set");

    println!("PDF Info:");
    println!("  - Pages: {}", info.page_count);
    println!("  - Version: {}", info.pdf_version);
    println!("  - Has images: {}", info.has_images);
    println!("  - Image count: {}", info.image_count);
    println!("  - File size: {} bytes", info.file_size);
}

#[tokio::test]
async fn test_extract_to_markdown() {
    let extractor = create_extractor();
    let pdf_bytes = load_sample_pdf();

    let markdown = extractor
        .extract_to_markdown(&pdf_bytes)
        .await
        .expect("Failed to extract markdown");

    assert!(!markdown.is_empty(), "Markdown should not be empty");

    // Should contain page indicator
    assert!(
        markdown.contains("Page 1") || markdown.contains("##"),
        "Markdown should contain page structure"
    );

    println!("Extracted Markdown ({} chars):", markdown.len());
    println!("{}", &markdown[..std::cmp::min(500, markdown.len())]);
}

#[tokio::test]
async fn test_extract_text() {
    let extractor = create_extractor();
    let pdf_bytes = load_sample_pdf();

    let text = extractor
        .extract_text(&pdf_bytes)
        .await
        .expect("Failed to extract text");

    assert!(!text.is_empty(), "Text should not be empty");

    println!("Extracted Text ({} chars):", text.len());
    println!("{}", &text[..std::cmp::min(300, text.len())]);
}

#[tokio::test]
async fn test_extract_full() {
    let extractor = create_extractor();
    let pdf_bytes = load_sample_pdf();

    let result = extractor
        .extract_full(&pdf_bytes)
        .await
        .expect("Failed to extract full document");

    assert!(result.page_count >= 1, "Should have at least 1 page");
    assert!(!result.pages.is_empty(), "Pages array should not be empty");
    assert!(
        !result.markdown.is_empty(),
        "Combined markdown should not be empty"
    );

    // Check metadata
    assert!(
        result.metadata.pdf_version.is_some(),
        "PDF version should be extracted"
    );

    println!("Full Extraction Result:");
    println!("  - Total pages: {}", result.page_count);
    println!("  - Pages extracted: {}", result.pages.len());
    println!("  - Images found: {}", result.images.len());
    println!("  - PDF version: {:?}", result.metadata.pdf_version);

    // Check individual pages
    for page in &result.pages {
        println!(
            "  - Page {} text length: {}",
            page.page_number,
            page.text.len()
        );
        assert!(
            !page.text.is_empty() || !page.markdown.is_empty(),
            "Page {} should have content",
            page.page_number
        );
    }
}

#[tokio::test]
async fn test_max_pages_config() {
    let config = PdfConfig::new().with_max_pages(1);
    let extractor = create_extractor_with_config(config);
    let pdf_bytes = load_sample_pdf();

    let result = extractor
        .extract_full(&pdf_bytes)
        .await
        .expect("Failed to extract");

    // Even if PDF has more pages, we should only process 1
    assert!(
        result.pages.len() <= 1,
        "Should process at most 1 page with max_pages=1"
    );
}

#[tokio::test]
async fn test_without_page_numbers() {
    let config = PdfConfig::new().with_page_numbers(false);
    let extractor = create_extractor_with_config(config);
    let pdf_bytes = load_sample_pdf();

    let markdown = extractor
        .extract_to_markdown(&pdf_bytes)
        .await
        .expect("Failed to extract");

    // Without page numbers, we shouldn't have "## Page" headers
    assert!(
        !markdown.contains("## Page"),
        "Should not contain page headers when disabled"
    );
}

#[tokio::test]
async fn test_without_image_extraction() {
    let config = PdfConfig::new().with_image_extraction(false);
    let extractor = create_extractor_with_config(config);
    let pdf_bytes = load_sample_pdf();

    let result = extractor
        .extract_full(&pdf_bytes)
        .await
        .expect("Failed to extract");

    // Without image extraction, images array should be empty
    assert!(
        result.images.is_empty(),
        "Should not extract images when disabled"
    );
}

#[test]
fn test_invalid_pdf() {
    let extractor = create_extractor();
    let invalid_bytes = b"This is not a PDF file at all!";

    let result = extractor.get_info(invalid_bytes);
    assert!(result.is_err(), "Should fail on invalid PDF");
}

#[tokio::test]
async fn test_empty_pdf_bytes() {
    let extractor = create_extractor();
    let empty_bytes: &[u8] = &[];

    let result = extractor.extract_to_markdown(empty_bytes).await;
    assert!(result.is_err(), "Should fail on empty bytes");
}

#[test]
fn test_config_builder() {
    let config = PdfConfig::new()
        .with_ocr_threshold(0.7)
        .with_max_pages(10)
        .with_page_numbers(true)
        .with_image_extraction(true)
        .with_table_enhancement(true)
        .with_ai_temperature(0.2);

    assert_eq!(config.ocr_threshold, 0.7);
    assert_eq!(config.max_pages, Some(10));
    assert!(config.include_page_numbers);
    assert!(config.extract_images);
    assert!(config.enhance_tables);
    assert_eq!(config.ai_temperature, 0.2);
}
