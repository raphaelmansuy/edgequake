//! Debug tool to trace character extraction order from PDFium.
//!
//! Usage:
//!   PDFIUM_DYNAMIC_LIB_PATH=/path/to/libpdfium.dylib cargo run --features pdfium --example debug_chars <PDF_PATH>

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
            eprintln!("Usage: debug_chars <PDF_PATH>");
            std::process::exit(1);
        }
    };

    #[cfg(feature = "pdfium")]
    {
        use edgequake_pdf::backend::pdfium::PdfiumExtractor;

        match PdfiumExtractor::with_library_path(&lib_path) {
            Ok(extractor) => match extractor.extract_chars_from_file(&pdf_path) {
                Ok(chars) => {
                    // Print first 80 chars from page 0 with font info
                    let page0_chars: Vec<_> =
                        chars.iter().filter(|c| c.page_num == 0).take(80).collect();

                    for (i, ch) in page0_chars.iter().enumerate() {
                        let font = ch.font_name.as_deref().unwrap_or("?");
                        // Truncate font name for readability
                        let short_font = if font.len() > 20 { &font[..20] } else { font };
                        eprintln!(
                            "{:3}: '{}' y={:.1} sz={:.1} font={}",
                            i, ch.char, ch.y0, ch.font_size, short_font
                        );
                    }

                    // Show text reconstruction
                    let text: String = page0_chars.iter().map(|c| c.char).collect();
                    eprintln!("\nFirst 80 chars as text:\n{}", text);
                }
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
