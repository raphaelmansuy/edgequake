//! Debug script to trace text extraction for page 2 (where numbered list is)
//!
//! Run with: cargo run --bin debug_page1

use std::path::Path;
use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::PdfExtractor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf_path = Path::new("test-data/real_dataset/agent_2510.09244v1.pdf");

    println!("=== Testing extraction of page 2 numbered list ===\n");

    // Use the full extractor
    let extractor = PdfExtractor::new(Arc::new(MockProvider::new()));
    let pdf_bytes = std::fs::read(pdf_path)?;

    // Run async extraction
    let rt = tokio::runtime::Runtime::new()?;
    let markdown = rt.block_on(extractor.extract_to_markdown(&pdf_bytes))?;

    // Search for the numbered list items
    println!("Searching for numbered list items in output...\n");

    let lines: Vec<&str> = markdown.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("1.")
            || line.starts_with("2.")
            || line.starts_with("3.")
            || line.starts_with("4.")
            || line.starts_with("5.")
        {
            println!("Line {}: {}", i, &line[..line.len().min(100)]);
            // Also print next 2 lines for context
            if i + 1 < lines.len() {
                println!(
                    "Line {}: {}",
                    i + 1,
                    &lines[i + 1][..lines[i + 1].len().min(80)]
                );
            }
            if i + 2 < lines.len() {
                println!(
                    "Line {}: {}",
                    i + 2,
                    &lines[i + 2][..lines[i + 2].len().min(80)]
                );
            }
            println!();
        }
    }

    // Search for "Examine"
    println!("\n=== Searching for 'Examine' ===");
    for (i, line) in lines.iter().enumerate() {
        if line.to_lowercase().contains("examine") {
            println!(
                "Found 'Examine' at line {}: {}",
                i,
                &line[..line.len().min(100)]
            );
        }
    }

    // Check if "Examine reasoning" phrase exists
    if markdown.contains("Examine reasoning") {
        println!("\n✅ 'Examine reasoning' FOUND in output");
    } else {
        println!("\n❌ 'Examine reasoning' NOT FOUND in output!");
    }

    // Check for "Chain-of-Thought"
    if markdown.contains("Chain-of-Thought") {
        println!("✅ 'Chain-of-Thought' FOUND in output");
    } else {
        println!("❌ 'Chain-of-Thought' NOT FOUND in output!");
    }

    Ok(())
}

// WHY: Allow dead_code - this utility function is kept for debugging sessions
#[allow(dead_code)]
fn extract_text_from_operand(operand: &lopdf::Object) -> String {
    match operand {
        lopdf::Object::String(bytes, _) => String::from_utf8_lossy(bytes).to_string(),
        lopdf::Object::Array(arr) => {
            let mut result = String::new();
            for item in arr {
                match item {
                    lopdf::Object::String(bytes, _) => {
                        result.push_str(&String::from_utf8_lossy(bytes));
                    }
                    lopdf::Object::Integer(_) | lopdf::Object::Real(_) => {
                        // These are kerning adjustments, not text
                    }
                    _ => {}
                }
            }
            result
        }
        _ => String::new(),
    }
}
