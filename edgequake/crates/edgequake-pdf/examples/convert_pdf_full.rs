//! Convert PDF to Markdown using the pymupdf4llm-inspired pipeline.
//!
//! Usage:
//!   PDFIUM_DYNAMIC_LIB_PATH=/path/to/libpdfium.dylib cargo run --features pdfium --example convert_pdf_full <PDF_PATH>
//!
//! Output is written to stdout (Markdown only, no debug info).

fn main() {
    let lib_path = match std::env::var("PDFIUM_DYNAMIC_LIB_PATH") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("ERROR: Set PDFIUM_DYNAMIC_LIB_PATH environment variable");
            std::process::exit(1);
        }
    };

    let pdf_path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("Usage: convert_pdf_full <PDF_PATH>");
            std::process::exit(1);
        }
    };

    #[cfg(feature = "pdfium")]
    {
        use edgequake_pdf::pipeline::PymupdfPipeline;

        match PymupdfPipeline::with_library_path(&lib_path) {
            Ok(pipeline) => match pipeline.convert_file(&pdf_path) {
                Ok(markdown) => print!("{}", markdown),
                Err(e) => {
                    eprintln!("ERROR: {e}");
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("ERROR: {e}");
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
