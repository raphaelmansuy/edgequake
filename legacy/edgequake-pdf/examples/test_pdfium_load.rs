//! Test PDFium loading and the pymupdf4llm-inspired pipeline.
//!
//! Usage:
//!   PDFIUM_DYNAMIC_LIB_PATH=/path/to/libpdfium.dylib cargo run --features pdfium --example test_pdfium_load [PDF_PATH]

fn main() {
    let lib_path = std::env::var("PDFIUM_DYNAMIC_LIB_PATH")
        .expect("Set PDFIUM_DYNAMIC_LIB_PATH environment variable");

    println!("Attempting to load PDFium from: {lib_path}");

    // Check file exists
    if !std::path::Path::new(&lib_path).exists() {
        eprintln!("ERROR: File does not exist!");
        std::process::exit(1);
    }
    println!("✓ File exists");

    #[cfg(feature = "pdfium")]
    {
        use edgequake_pdf::pipeline::PymupdfPipeline;

        println!("Creating PymupdfPipeline...");
        match PymupdfPipeline::with_library_path(&lib_path) {
            Ok(pipeline) => {
                println!("✓ Pipeline created successfully!");

                // Try to convert a PDF if provided
                if let Some(pdf_path) = std::env::args().nth(1) {
                    println!("\n=== Converting: {pdf_path} ===\n");
                    match pipeline.convert_file(&pdf_path) {
                        Ok(markdown) => {
                            // Print first 2000 characters
                            let preview_len = markdown.len().min(2000);
                            println!("{}", &markdown[..preview_len]);
                            if markdown.len() > 2000 {
                                println!("\n... [{} more characters]", markdown.len() - 2000);
                            }
                            println!("\n✓ Converted {} characters of Markdown", markdown.len());
                        }
                        Err(e) => eprintln!("ERROR converting: {e}"),
                    }
                } else {
                    println!("\nUsage: provide a PDF path as argument to convert it");
                }
            }
            Err(e) => {
                eprintln!("ERROR: Failed to create pipeline: {e}");
                std::process::exit(1);
            }
        }
    }

    #[cfg(not(feature = "pdfium"))]
    {
        eprintln!("ERROR: pdfium feature not enabled");
        std::process::exit(1);
    }
}
