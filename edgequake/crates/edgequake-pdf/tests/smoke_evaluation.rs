use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::PdfExtractor;

#[tokio::test]
async fn smoke_evaluate_first_available_doc() {
    let provider = Arc::new(MockProvider::new());
    let extractor = PdfExtractor::new(provider);

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data/gold");
    let pdf_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data/pdfs");

    // Find the first gold md that has a corresponding PDF (either in pdfs/ or root)
    for category in fs::read_dir(&test_dir).unwrap().flatten() {
        let cat_path = category.path();
        if !cat_path.is_dir() {
            continue;
        }
        for file in fs::read_dir(&cat_path).unwrap().flatten() {
            let md = file.path();
            if md.extension().map(|s| s == "md").unwrap_or(false) {
                let stem = md.file_stem().unwrap().to_string_lossy().to_string();
                // check for pdf in pdf_dir/category
                let candidate_pdf = pdf_dir
                    .join(cat_path.file_name().unwrap())
                    .join(md.with_extension("pdf").file_name().unwrap());
                if candidate_pdf.exists() {
                    let pdf_bytes = fs::read(candidate_pdf).expect("Failed to read pdf");
                    let res = extractor.extract_to_markdown(&pdf_bytes).await;
                    assert!(res.is_ok(), "Extraction should succeed");
                    return;
                }
            }
        }
    }

    // If none found, skip test gracefully
    println!("No PDFs found under test-data/pdfs. Generate PDFs first.");
}
