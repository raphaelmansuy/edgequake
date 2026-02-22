//! PDF Extraction Quality Tests
//!
//! These tests verify that PDF extraction produces high-quality markdown output
//! by checking for key content, proper reading order, and structure.
//!
//! Run with: cargo test --package edgequake-pdf quality_extraction --no-fail-fast -- --nocapture

use std::fs;
use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::PdfExtractor;

const TEST_DOCS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../zz_test_docs/");

/// Helper to create extractor
fn make_extractor() -> PdfExtractor {
    let provider = Arc::new(MockProvider::new());
    PdfExtractor::new(provider)
}

/// Helper to get test file path
fn test_file(name: &str) -> String {
    format!("{}{}", TEST_DOCS_PATH, name)
}

// =============================================================================
// Qwen.pdf - Type3 fonts with flipped coordinates
// =============================================================================

#[tokio::test]
async fn test_qwen_reading_order() {
    // WHY: Qwen.pdf uses negative CTM transform causing Y-flip
    // Title "Pushing" should appear before "Beyond" in reading order
    let path = test_file("Qwen.pdf");
    if !std::path::Path::new(&path).exists() {
        println!("Skipping: {} not found", path);
        return;
    }

    let pdf_bytes = fs::read(&path).expect("Failed to read PDF");
    let extractor = make_extractor();
    let markdown = extractor
        .extract_to_markdown(&pdf_bytes)
        .await
        .expect("Extraction failed");

    println!(
        "Qwen.pdf output ({} bytes):\n{}",
        markdown.len(),
        &markdown[..markdown.len().min(500)]
    );

    // Find positions of key phrases
    let markdown_lower = markdown.to_lowercase();
    let pushing_pos = markdown_lower.find("pushing");
    let beyond_pos = markdown_lower.find("beyond");

    assert!(pushing_pos.is_some(), "Should find 'Pushing' in output");
    assert!(beyond_pos.is_some(), "Should find 'Beyond' in output");

    // Verify reading order: Pushing should come before Beyond
    // WHY: After fix for flipped coordinates, title line should be in correct order
    assert!(
        pushing_pos.unwrap() < beyond_pos.unwrap(),
        "Reading order wrong: 'Pushing' ({:?}) should appear before 'Beyond' ({:?})",
        pushing_pos,
        beyond_pos
    );
}

#[tokio::test]
async fn test_qwen_key_content() {
    // WHY: Verifies Type3 font ToUnicode decoding works correctly
    let path = test_file("Qwen.pdf");
    if !std::path::Path::new(&path).exists() {
        return;
    }

    let pdf_bytes = fs::read(&path).expect("Failed to read PDF");
    let extractor = make_extractor();
    let markdown = extractor
        .extract_to_markdown(&pdf_bytes)
        .await
        .expect("Extraction failed");

    let markdown_lower = markdown.to_lowercase();

    // Key phrases that should be in the document
    let required_phrases = ["qwen", "thinking", "reasoning"];

    for phrase in required_phrases {
        assert!(
            markdown_lower.contains(phrase),
            "Missing expected phrase '{}' in Qwen.pdf extraction",
            phrase
        );
    }
}

// =============================================================================
// Beyond Transformer - Standard PDF with academic content
// =============================================================================

#[tokio::test]
async fn test_beyond_transformer_content() {
    let path = test_file("001-BEYONG-TRANFORMER-OUTLINE-V1_1.pdf");
    if !std::path::Path::new(&path).exists() {
        println!("Skipping: {} not found", path);
        return;
    }

    let pdf_bytes = fs::read(&path).expect("Failed to read PDF");
    let extractor = make_extractor();
    let markdown = extractor
        .extract_to_markdown(&pdf_bytes)
        .await
        .expect("Extraction failed");

    println!("Beyond Transformer output: {} bytes", markdown.len());

    // Should extract substantial content (this is a multi-page academic PDF)
    assert!(
        markdown.len() >= 10000,
        "Beyond Transformer should extract at least 10KB, got {} bytes",
        markdown.len()
    );

    let markdown_lower = markdown.to_lowercase();

    // Key phrases expected in academic PDF about transformers
    let expected_phrases = ["transformer", "attention", "model"];

    for phrase in expected_phrases {
        assert!(
            markdown_lower.contains(phrase),
            "Missing expected phrase '{}' in Beyond Transformer extraction",
            phrase
        );
    }
}

#[tokio::test]
async fn test_beyond_transformer_structure() {
    let path = test_file("001-BEYONG-TRANFORMER-OUTLINE-V1_1.pdf");
    if !std::path::Path::new(&path).exists() {
        return;
    }

    let pdf_bytes = fs::read(&path).expect("Failed to read PDF");
    let extractor = make_extractor();
    let doc = extractor
        .extract_document(&pdf_bytes)
        .await
        .expect("Extraction failed");

    // Multi-page PDF should have multiple pages
    assert!(
        doc.pages.len() >= 5,
        "Beyond Transformer should have at least 5 pages, got {}",
        doc.pages.len()
    );

    // Each page should have content
    for (i, page) in doc.pages.iter().enumerate() {
        assert!(!page.blocks.is_empty(), "Page {} should have blocks", i + 1);
    }
}

// =============================================================================
// Agentic Platform - Complex architecture document with diagrams
// =============================================================================

#[tokio::test]
async fn test_agentic_platform_content() {
    let path = test_file("AgenticPlatformReference Architecture.pdf");
    if !std::path::Path::new(&path).exists() {
        println!("Skipping: {} not found", path);
        return;
    }

    let pdf_bytes = fs::read(&path).expect("Failed to read PDF");
    let extractor = make_extractor();
    let markdown = extractor
        .extract_to_markdown(&pdf_bytes)
        .await
        .expect("Extraction failed");

    println!("Agentic Platform output: {} bytes", markdown.len());

    // Should extract substantial content (architecture document)
    assert!(
        markdown.len() >= 50000,
        "Agentic Platform should extract at least 50KB, got {} bytes",
        markdown.len()
    );

    let markdown_lower = markdown.to_lowercase();

    // Key phrases from the architecture document
    let expected_phrases = [
        "agentic",
        "platform",
        "architecture",
        "operational reliability",
        "security",
    ];

    for phrase in expected_phrases {
        assert!(
            markdown_lower.contains(phrase),
            "Missing expected phrase '{}' in Agentic Platform extraction",
            phrase
        );
    }
}

#[tokio::test]
async fn test_agentic_platform_headings() {
    // WHY: Validates that heading detection works for structured documents
    let path = test_file("AgenticPlatformReference Architecture.pdf");
    if !std::path::Path::new(&path).exists() {
        return;
    }

    let pdf_bytes = fs::read(&path).expect("Failed to read PDF");
    let extractor = make_extractor();
    let markdown = extractor
        .extract_to_markdown(&pdf_bytes)
        .await
        .expect("Extraction failed");

    // Count markdown headings
    let h1_count =
        markdown.matches("\n# ").count() + if markdown.starts_with("# ") { 1 } else { 0 };
    let h2_count = markdown.matches("\n## ").count();

    println!("Headings: H1={}, H2={}", h1_count, h2_count);

    // Architecture document should have multiple sections
    assert!(
        h1_count + h2_count >= 5,
        "Should detect at least 5 headings, found H1={} + H2={}",
        h1_count,
        h2_count
    );
}

#[tokio::test]
async fn test_agentic_platform_code_blocks() {
    // WHY: Document contains ASCII art diagrams that may be detected as code
    let path = test_file("AgenticPlatformReference Architecture.pdf");
    if !std::path::Path::new(&path).exists() {
        return;
    }

    let pdf_bytes = fs::read(&path).expect("Failed to read PDF");
    let extractor = make_extractor();
    let markdown = extractor
        .extract_to_markdown(&pdf_bytes)
        .await
        .expect("Extraction failed");

    // Check for diagram-like content (boxes, ASCII art)
    let has_box_chars = markdown.contains("┌") || markdown.contains("│") || markdown.contains("└");

    println!("Contains box drawing chars: {}", has_box_chars);

    // The document has ASCII diagrams, so we should see box characters
    // This validates that special characters are preserved
    assert!(
        has_box_chars,
        "Should preserve box-drawing characters from diagrams"
    );
}

// =============================================================================
// Metrics and Summary Test
// =============================================================================

#[tokio::test]
async fn test_all_pdfs_extraction_summary() {
    let test_files = [
        ("Qwen.pdf", 500, vec!["qwen"]),
        (
            "001-BEYONG-TRANFORMER-OUTLINE-V1_1.pdf",
            10000,
            vec!["transformer"],
        ),
        (
            "AgenticPlatformReference Architecture.pdf",
            50000,
            vec!["agentic"],
        ),
    ];

    let extractor = make_extractor();
    let mut results = Vec::new();

    for (name, min_bytes, required_words) in test_files {
        let path = test_file(name);
        if !std::path::Path::new(&path).exists() {
            println!("SKIP: {} not found", name);
            continue;
        }

        let pdf_bytes = fs::read(&path).expect("Failed to read PDF");
        let result = extractor.extract_to_markdown(&pdf_bytes).await;

        match result {
            Ok(markdown) => {
                let markdown_lower = markdown.to_lowercase();
                let has_required = required_words.iter().all(|w| markdown_lower.contains(*w));

                let status = if markdown.len() >= min_bytes && has_required {
                    "PASS"
                } else {
                    "FAIL"
                };

                results.push((name, status, markdown.len(), min_bytes));
                println!(
                    "{}: {} - {} bytes (min: {}), has_keywords: {}",
                    status,
                    name,
                    markdown.len(),
                    min_bytes,
                    has_required
                );
            }
            Err(e) => {
                results.push((name, "ERROR", 0, min_bytes));
                println!("ERROR: {} - {:?}", name, e);
            }
        }
    }

    println!("\n=== EXTRACTION QUALITY SUMMARY ===");
    for (name, status, actual, expected) in &results {
        println!(
            "{}: {} ({} bytes, expected >= {})",
            status, name, actual, expected
        );
    }

    // All tests should pass
    let failures: Vec<_> = results.iter().filter(|(_, s, _, _)| *s != "PASS").collect();
    assert!(
        failures.is_empty(),
        "Some PDFs failed extraction: {:?}",
        failures
    );
}
