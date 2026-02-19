//! Debug test for text loss investigation
//!
//! This test checks if specific text is being extracted correctly from PDFs

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::PdfExtractor;
use std::path::PathBuf;
use std::sync::Arc;

fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data")
}

fn create_extractor() -> PdfExtractor {
    PdfExtractor::new(Arc::new(MockProvider::new()))
}

/// Test extraction from the agent PDF to find text loss
#[tokio::test]
async fn test_agent_pdf_text_completeness() {
    let pdf_path = test_data_dir().join("real_dataset/agent_2510.09244v1.pdf");

    if !pdf_path.exists() {
        println!("⚠️  Skipping: PDF not found at {:?}", pdf_path);
        return;
    }

    let extractor = create_extractor();
    let pdf_bytes = std::fs::read(&pdf_path).expect("Failed to read PDF");

    let markdown = extractor
        .extract_to_markdown(&pdf_bytes)
        .await
        .expect("Failed to extract markdown");

    println!("Extracted {} characters", markdown.len());
    println!(
        "\n=== First 1000 chars ===\n{}",
        &markdown[..markdown.len().min(1000)]
    );

    // Check for expected phrases (these should be in the PDF)
    let expected_phrases = vec!["LLM", "agent", "reasoning", "Chain-of-Thought"];

    for phrase in &expected_phrases {
        if markdown.contains(phrase) {
            println!("✅ Found: {}", phrase);
        } else {
            println!("❌ Missing: {}", phrase);
        }
    }

    // Write output to file for analysis
    std::fs::write("debug_agent_extraction.md", &markdown).ok();
    println!("\nOutput saved to debug_agent_extraction.md");

    // Verify we got meaningful output
    assert!(
        markdown.len() > 100,
        "Output too short: {} chars",
        markdown.len()
    );
}

/// Test extraction from hotmess PDF (slowest in benchmarks)
#[tokio::test]
async fn test_hotmess_pdf_text_completeness() {
    let pdf_path = test_data_dir().join("real_dataset/hotmess_2601.23045v1.pdf");

    if !pdf_path.exists() {
        println!("⚠️  Skipping: PDF not found at {:?}", pdf_path);
        return;
    }

    let extractor = create_extractor();
    let pdf_bytes = std::fs::read(&pdf_path).expect("Failed to read PDF");

    let markdown = extractor
        .extract_to_markdown(&pdf_bytes)
        .await
        .expect("Failed to extract markdown");

    println!("Extracted {} characters from hotmess", markdown.len());

    // Write output for analysis
    std::fs::write("debug_hotmess_extraction.md", &markdown).ok();
    println!("Output saved to debug_hotmess_extraction.md");

    assert!(
        markdown.len() > 100,
        "Output too short: {} chars",
        markdown.len()
    );
}

/// Compare our extraction against pdftotext output
#[tokio::test]
async fn test_compare_with_pdftotext() {
    let pdf_path = test_data_dir().join("real_dataset/agent_2510.09244v1.pdf");

    if !pdf_path.exists() {
        println!("⚠️  Skipping: PDF not found");
        return;
    }

    let extractor = create_extractor();
    let pdf_bytes = std::fs::read(&pdf_path).expect("Failed to read PDF");

    let our_output = extractor.extract_to_markdown(&pdf_bytes).await.unwrap();

    // Run pdftotext for comparison
    let pdftotext_output = std::process::Command::new("pdftotext")
        .args(["-layout", pdf_path.to_str().unwrap(), "-"])
        .output();

    if let Ok(output) = pdftotext_output {
        let pdftotext_text = String::from_utf8_lossy(&output.stdout);

        println!("Our extraction: {} chars", our_output.len());
        println!("pdftotext: {} chars", pdftotext_text.len());

        // Find text in pdftotext but not in ours
        let pdftotext_words: std::collections::HashSet<&str> =
            pdftotext_text.split_whitespace().collect();
        let our_words: std::collections::HashSet<&str> = our_output.split_whitespace().collect();

        let missing: Vec<_> = pdftotext_words.difference(&our_words).take(50).collect();

        if !missing.is_empty() {
            println!("\nFirst 50 words in pdftotext but not in our output:");
            for word in &missing {
                println!("  - {}", word);
            }
        }

        // Also check coverage ratio
        let our_word_count = our_words.len();
        let pdftotext_word_count = pdftotext_words.len();
        let overlap = our_words.intersection(&pdftotext_words).count();

        println!("\n=== Coverage Analysis ===");
        println!("Our unique words: {}", our_word_count);
        println!("pdftotext unique words: {}", pdftotext_word_count);
        println!("Overlap: {}", overlap);
        println!(
            "Coverage: {:.1}%",
            100.0 * overlap as f64 / pdftotext_word_count as f64
        );
    }
}
