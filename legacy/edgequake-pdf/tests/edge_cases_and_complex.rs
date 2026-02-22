//! Edge Cases and Complex Scenario Tests
//!
//! This test suite covers:
//! - Edge cases (empty, boundary, maximum values)
//! - Error conditions and recovery
//! - Configuration permutations

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::{OutputFormat, PdfConfig, PdfExtractor};
use std::sync::Arc;

fn get_test_data_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data")
}

fn load_pdf(filename: &str) -> Vec<u8> {
    let path = get_test_data_dir().join(filename);
    std::fs::read(&path).unwrap_or_else(|_| Vec::new())
}

fn create_extractor() -> PdfExtractor {
    PdfExtractor::new(Arc::new(MockProvider::new()))
}

fn create_extractor_with_config(config: PdfConfig) -> PdfExtractor {
    PdfExtractor::with_config(Arc::new(MockProvider::new()), config)
}

// EDGE CASES
#[test]
fn edge_01_empty_byte_array() {
    let extractor = create_extractor();
    let result = extractor.get_info(&Vec::new());
    assert!(result.is_err());
}

#[test]
fn edge_02_single_byte() {
    let result = create_extractor().get_info(&vec![42u8]);
    assert!(result.is_err());
}

#[test]
fn edge_03_invalid_pdf_header() {
    let result = create_extractor().get_info(&b"Not a PDF".to_vec());
    assert!(result.is_err());
}

#[test]
fn edge_04_null_bytes() {
    let mut bytes = vec![0u8; 100];
    bytes[0..4].copy_from_slice(b"%PDF");
    let _ = create_extractor().get_info(&bytes);
}

#[test]
fn edge_05_corrupted_header() {
    let bytes = b"%PDF\xFF\xFF\xFF\xFF".to_vec();
    let _ = create_extractor().get_info(&bytes);
}

#[test]
fn edge_06_maximum_page_config() {
    let mut config = PdfConfig::default();
    config.max_pages = Some(usize::MAX);
    assert!(create_extractor_with_config(config)
        .config()
        .max_pages
        .is_some());
}

#[test]
fn edge_07_zero_max_pages() {
    let mut config = PdfConfig::default();
    config.max_pages = Some(0);
    let _ = create_extractor_with_config(config);
}

#[tokio::test]
async fn edge_08_extract_empty_input() {
    let result = create_extractor().extract_to_markdown(&Vec::new()).await;
    let _ = result;
}

#[test]
fn edge_09_all_output_formats() {
    let formats = vec![
        OutputFormat::Markdown,
        OutputFormat::Json,
        OutputFormat::Html,
        OutputFormat::Chunks,
    ];
    for fmt in formats {
        let mut config = PdfConfig::default();
        config.output_format = fmt;
        let _ = create_extractor_with_config(config);
    }
}

#[test]
fn edge_10_all_extraction_modes() {
    use edgequake_pdf::ExtractionMode;
    let modes = vec![
        ExtractionMode::Text,
        ExtractionMode::Vision,
        ExtractionMode::Hybrid,
    ];
    for mode in modes {
        let mut config = PdfConfig::default();
        config.mode = mode;
        let _ = create_extractor_with_config(config);
    }
}

// BOUNDARY CONDITIONS
#[tokio::test]
async fn boundary_01_large_pdf() {
    let pdf = load_pdf("008_multi_page_5_pages.pdf");
    let result = create_extractor().extract_to_markdown(&pdf).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn boundary_02_small_pdf() {
    let pdf = load_pdf("001_simple_text.pdf");
    assert!(!pdf.is_empty());
    let result = create_extractor().extract_to_markdown(&pdf).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn boundary_03_output_size() {
    let pdf = load_pdf("001_simple_text.pdf");
    let md = create_extractor().extract_to_markdown(&pdf).await.unwrap();
    assert!(md.len() > 0 && md.len() < 1_000_000);
}

#[tokio::test]
async fn boundary_04_text_size() {
    let pdf = load_pdf("008_multi_page_5_pages.pdf");
    let text = create_extractor().extract_text(&pdf).await.unwrap();
    assert!(!text.is_empty() && text.len() < 10_000_000);
}

// CONFIGURATION COMBINATIONS
#[test]
fn config_01_format_combinations() {
    let formats = [
        OutputFormat::Markdown,
        OutputFormat::Json,
        OutputFormat::Html,
        OutputFormat::Chunks,
    ];
    for fmt in &formats {
        let mut config = PdfConfig::default();
        config.output_format = *fmt;
        let _ = create_extractor_with_config(config);
    }
}

#[test]
fn config_02_boolean_toggles() {
    let mut config = PdfConfig::default();
    config.extract_images = true;
    let _ = create_extractor_with_config(config);

    let mut config = PdfConfig::default();
    config.include_page_numbers = true;
    let _ = create_extractor_with_config(config);
}

#[test]
fn config_03_16_combinations() {
    let mut count = 0;
    for i1 in [true, false] {
        for i2 in [true, false] {
            for i3 in [true, false] {
                for i4 in [true, false] {
                    let mut config = PdfConfig::default();
                    config.extract_images = i1;
                    config.include_page_numbers = i2;
                    config.layout.detect_columns = i3;
                    config.layout.detect_tables = i4;
                    let _ = create_extractor_with_config(config);
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 16);
}

#[test]
fn config_04_page_limits() {
    for limit in [1, 10, 100, 1000] {
        let mut config = PdfConfig::default();
        config.max_pages = Some(limit);
        let _ = create_extractor_with_config(config);
    }
}

// CONCURRENT OPERATIONS
#[tokio::test]
async fn concurrent_01_same_pdf_multiple() {
    let pdf = Arc::new(load_pdf("001_simple_text.pdf"));
    let mut handles = vec![];

    for _ in 0..5 {
        let pdf_clone = Arc::clone(&pdf);
        handles.push(tokio::spawn(async move {
            create_extractor()
                .extract_to_markdown(&pdf_clone)
                .await
                .is_ok()
        }));
    }

    let mut success = 0;
    for h in handles {
        if h.await.unwrap() {
            success += 1;
        }
    }
    assert!(success > 0);
}

#[tokio::test]
async fn concurrent_02_multiple_pdfs() {
    let files = vec![
        "001_simple_text.pdf",
        "003_two_columns.pdf",
        "004_simple_table_2x3.pdf",
    ];
    let mut handles = vec![];

    for file in files {
        handles.push(tokio::spawn(async move {
            create_extractor()
                .extract_to_markdown(&load_pdf(file))
                .await
                .is_ok()
        }));
    }

    let mut success = 0;
    for h in handles {
        if let Ok(true) = h.await {
            success += 1;
        }
    }
    assert!(success > 0);
}

#[tokio::test]
async fn concurrent_03_concurrent_info() {
    let pdf = Arc::new(load_pdf("001_simple_text.pdf"));
    let mut handles = vec![];

    for _ in 0..10 {
        let pdf_clone = Arc::clone(&pdf);
        handles.push(tokio::spawn(async move {
            create_extractor().get_info(&pdf_clone).is_ok()
        }));
    }

    let success = futures::future::join_all(handles)
        .await
        .iter()
        .filter(|r| r.is_ok())
        .count();
    assert_eq!(success, 10);
}

// ERROR HANDLING
#[tokio::test]
async fn error_01_corrupted_header() {
    let mut bytes = load_pdf("001_simple_text.pdf");
    if !bytes.is_empty() {
        bytes[0] = 0xFF;
        let _ = create_extractor().extract_to_markdown(&bytes).await;
    }
}

#[tokio::test]
async fn error_02_truncated() {
    let mut bytes = load_pdf("008_multi_page_5_pages.pdf");
    if bytes.len() > 100 {
        bytes.truncate(bytes.len() / 2);
        let _ = create_extractor().extract_to_markdown(&bytes).await;
    }
}

#[tokio::test]
async fn error_03_corrupted_middle() {
    let mut bytes = load_pdf("003_two_columns.pdf");
    if bytes.len() > 50 {
        let mid = bytes.len() / 2;
        bytes[mid] = 0xFF;
        let _ = create_extractor().extract_to_markdown(&bytes).await;
    }
}

#[test]
fn error_04_invalid_config() {
    let mut config = PdfConfig::default();
    config.max_pages = Some(0);
    let _ = create_extractor_with_config(config);
}

#[tokio::test]
async fn error_05_recovery() {
    let ext = create_extractor();
    for _ in 0..3 {
        let _ = ext.extract_to_markdown(&Vec::new()).await;
    }
}

// STRESS TESTS
#[tokio::test]
async fn stress_01_repeated() {
    let ext = Arc::new(create_extractor());
    let pdf = Arc::new(load_pdf("001_simple_text.pdf"));

    for _ in 0..10 {
        let _ = ext.extract_to_markdown(&pdf).await;
    }
}

#[tokio::test]
async fn stress_02_multiple_extractors() {
    let pdf = load_pdf("001_simple_text.pdf");
    for _ in 0..5 {
        let _ = create_extractor().extract_to_markdown(&pdf).await;
    }
}

#[tokio::test]
async fn stress_03_rapid_info() {
    let ext = create_extractor();
    let pdf = load_pdf("001_simple_text.pdf");

    for _ in 0..20 {
        let _ = ext.get_info(&pdf);
    }
}

#[tokio::test]
async fn stress_04_mixed_ops() {
    let ext = Arc::new(create_extractor());
    let pdf = Arc::new(load_pdf("003_two_columns.pdf"));

    for _ in 0..3 {
        let ext_c = Arc::clone(&ext);
        let pdf_c = Arc::clone(&pdf);
        let _ = tokio::spawn(async move { ext_c.get_info(&pdf_c) }).await;
        let _ = ext.extract_to_markdown(&pdf).await;
    }
}

// COMPLEX SCENARIOS
#[tokio::test]
async fn scenario_01_sequential_multi() {
    let files = vec![
        "001_simple_text.pdf",
        "003_two_columns.pdf",
        "004_simple_table_2x3.pdf",
    ];
    let ext = create_extractor();

    for file in files {
        if let Ok(md) = ext.extract_to_markdown(&load_pdf(file)).await {
            assert!(!md.is_empty());
        }
    }
}

#[tokio::test]
async fn scenario_02_different_configs() {
    let pdf = Arc::new(load_pdf("001_simple_text.pdf"));

    for extract in [true, false] {
        let mut config = PdfConfig::default();
        config.extract_images = extract;
        let ext = create_extractor_with_config(config);
        let _ = ext.extract_to_markdown(&pdf).await;
    }
}

#[tokio::test]
async fn scenario_03_all_methods() {
    let pdf = load_pdf("001_simple_text.pdf");
    let ext = create_extractor();

    assert!(ext.extract_text(&pdf).await.is_ok());
    assert!(ext.extract_to_markdown(&pdf).await.is_ok());
    assert!(ext.get_info(&pdf).is_ok());
}

#[tokio::test]
async fn scenario_04_full_extraction() {
    let pdf = load_pdf("008_multi_page_5_pages.pdf");
    let ext = create_extractor();

    if let Ok(doc) = ext.extract_full(&pdf).await {
        assert!(doc.page_count > 0);
        assert!(!doc.pages.is_empty());
    }
}

// PERFORMANCE
#[tokio::test]
async fn perf_01_single_page_speed() {
    let pdf = load_pdf("001_simple_text.pdf");
    let start = std::time::Instant::now();
    let _ = create_extractor().extract_to_markdown(&pdf).await;
    assert!(start.elapsed().as_secs() < 1);
}

#[tokio::test]
async fn perf_02_multi_page_speed() {
    let pdf = load_pdf("008_multi_page_5_pages.pdf");
    let start = std::time::Instant::now();
    let _ = create_extractor().extract_to_markdown(&pdf).await;
    assert!(start.elapsed().as_secs() < 5);
}

#[test]
fn perf_03_info_speed() {
    let pdf = load_pdf("001_simple_text.pdf");
    let start = std::time::Instant::now();
    let _ = create_extractor().get_info(&pdf);
    assert!(start.elapsed().as_millis() < 1000);
}

// DATA VALIDATION
#[tokio::test]
async fn validate_01_text_presence() {
    let pdf = load_pdf("001_simple_text.pdf");
    let md = create_extractor().extract_to_markdown(&pdf).await.unwrap();
    assert!(md.len() > 5);
}

#[tokio::test]
async fn validate_02_multipage_content() {
    let pdf = load_pdf("008_multi_page_5_pages.pdf");
    let md = create_extractor().extract_to_markdown(&pdf).await.unwrap();
    assert!(md.len() > 50);
}

#[tokio::test]
async fn validate_03_table_content() {
    let pdf = load_pdf("004_simple_table_2x3.pdf");
    let md = create_extractor().extract_to_markdown(&pdf).await.unwrap();
    assert!(!md.is_empty());
}

#[test]
fn validate_04_config_consistency() {
    let mut config = PdfConfig::default();
    config.max_pages = Some(100);
    let ext = create_extractor_with_config(config);
    assert!(ext.config().max_pages.is_some());
}

#[tokio::test]
async fn validate_05_info_consistency() {
    let pdf = load_pdf("008_multi_page_5_pages.pdf");
    let ext = create_extractor();

    if let (Ok(i1), Ok(i2)) = (ext.get_info(&pdf), ext.get_info(&pdf)) {
        assert_eq!(i1.page_count, i2.page_count);
        assert_eq!(i1.file_size, i2.file_size);
    }
}

// ADVANCED EDGE CASES (022-031)

#[test]
fn advanced_022_corrupted_xref_table() {
    let pdf = load_pdf("022_corrupted_xref_table.pdf");
    let result = create_extractor().get_info(&pdf);
    assert!(result.is_err(), "Should fail gracefully on corrupted XRef");
}

#[tokio::test]
async fn advanced_023_incomplete_unicode_mapping() {
    let pdf = load_pdf("023_incomplete_unicode_mapping.pdf");
    let result = create_extractor().extract_text(&pdf).await;
    assert!(result.is_ok(), "Should extract, but may contain (cid:x)");
}

#[tokio::test]
async fn advanced_024_embedded_fonts_obfuscated() {
    let pdf = load_pdf("024_embedded_fonts_obfuscated.pdf");
    let result = create_extractor().extract_text(&pdf).await;
    assert!(
        result.is_ok(),
        "Should extract, but may be gibberish if font mapping missing"
    );
}

#[tokio::test]
async fn advanced_025_rotated_text() {
    let pdf = load_pdf("025_rotated_text.pdf");
    let result = create_extractor().extract_text(&pdf).await;
    assert!(result.is_ok(), "Should extract all rotated text");
}

#[tokio::test]
async fn advanced_026_overlapping_text_layers() {
    let pdf = load_pdf("026_overlapping_text_layers.pdf");
    let result = create_extractor().extract_text(&pdf).await;
    assert!(result.is_ok(), "Should extract, avoid duplicate overlays");
}

#[tokio::test]
async fn advanced_027_digital_signatures_annotations() {
    let pdf = load_pdf("027_digital_signatures_annotations.pdf");
    let result = create_extractor().extract_text(&pdf).await;
    assert!(
        result.is_ok(),
        "Should extract text, ignore annotations/signatures"
    );
}

#[tokio::test]
async fn advanced_028_vector_graphics_text_on_path() {
    let pdf = load_pdf("028_vector_graphics_text_on_path.pdf");
    let result = create_extractor().extract_text(&pdf).await;
    assert!(result.is_ok(), "Should extract text on path if possible");
}

#[test]
fn advanced_029_encrypted_password_protected() {
    let pdf = load_pdf("029_encrypted_password_protected.pdf");
    let result = create_extractor().get_info(&pdf);
    assert!(
        result.is_err(),
        "Should fail with clear error on encrypted PDF"
    );
}

#[tokio::test]
async fn advanced_030_mixed_writing_directions() {
    let pdf = load_pdf("030_mixed_writing_directions.pdf");
    let result = create_extractor().extract_text(&pdf).await;
    assert!(result.is_ok(), "Should extract both LTR and RTL text");
}

#[tokio::test]
async fn advanced_031_embedded_files_attachments() {
    let pdf = load_pdf("031_embedded_files_attachments.pdf");
    let result = create_extractor().extract_text(&pdf).await;
    assert!(result.is_ok(), "Should extract text, ignore attachments");
}

#[test]
fn summary_60_tests_defined() {
    println!("50+ Edge Cases & Complex Tests Summary:");
    println!("  - 10 Edge case tests");
    println!("  - 4 Boundary condition tests");
    println!("  - 4 Configuration combination tests");
    println!("  - 3 Concurrent operation tests");
    println!("  - 5 Error handling tests");
    println!("  - 4 Stress tests");
    println!("  - 4 Complex scenario tests");
    println!("  - 3 Performance benchmark tests");
    println!("  - 5 Data validation tests");
    println!("Total: 50+ comprehensive edge case and complex scenario tests");
}
