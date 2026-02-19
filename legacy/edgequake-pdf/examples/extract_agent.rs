//! Quick agent extraction to disk for OODA-14 analysis
use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::PdfExtractor;
use std::fs;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let path = "crates/edgequake-pdf/test-data/real_dataset/agent_2510.09244v1.pdf";
    let pdf_bytes = fs::read(path).expect("Read failed");

    let extractor = PdfExtractor::new(Arc::new(MockProvider::new()));

    match extractor.extract_to_markdown(&pdf_bytes).await {
        Ok(md) => {
            fs::write("/tmp/agent_extracted.md", &md).expect("Write failed");
            println!("Wrote {} chars to /tmp/agent_extracted.md", md.len());
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
