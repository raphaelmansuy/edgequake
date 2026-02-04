//! Test extraction of agent PDF to find missing text
use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::PdfExtractor;
use std::fs;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let path = "test-data/real_dataset/agent_2510.09244v1.pdf";
    let pdf_bytes = fs::read(path).expect("Read failed");

    let extractor = PdfExtractor::new(Arc::new(MockProvider::new()));

    match extractor.extract_to_markdown(&pdf_bytes).await {
        Ok(md) => {
            fs::write("/tmp/agent_extracted.md", &md).expect("Write failed");
            println!("Wrote {} chars to /tmp/agent_extracted.md", md.len());
            if md.contains("Examine reasoning") {
                println!("✓ Found 'Examine reasoning'");
            } else {
                println!("✗ Missing 'Examine reasoning'");
            }
            if md.contains("Chain-of-Thought") || md.contains("Chain of Thought") {
                println!("✓ Found 'Chain-of-Thought'");
            } else {
                println!("✗ Missing 'Chain-of-Thought'");
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
