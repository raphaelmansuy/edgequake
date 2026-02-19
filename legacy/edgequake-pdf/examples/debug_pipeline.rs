//! Debug tool to trace the complete pipeline steps.
//!
//! Usage:
//!   PDFIUM_DYNAMIC_LIB_PATH=/path/to/libpdfium.dylib cargo run --features pdfium --example debug_pipeline <PDF_PATH>

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
            eprintln!("Usage: debug_pipeline <PDF_PATH>");
            std::process::exit(1);
        }
    };

    #[cfg(feature = "pdfium")]
    {
        use edgequake_pdf::backend::pdfium::PdfiumExtractor;
        use edgequake_pdf::layout::{Line, Span, TextBlock, TextGrouper};

        match PdfiumExtractor::with_library_path(&lib_path) {
            Ok(extractor) => match extractor.extract_chars_from_file(&pdf_path) {
                Ok(chars) => {
                    let grouper = TextGrouper::new();

                    // Step 1: chars to spans
                    let spans = grouper.chars_to_spans(&chars);
                    let page0_spans: Vec<&Span> =
                        spans.iter().filter(|s| s.page_num == 0).take(20).collect();

                    eprintln!(
                        "=== STEP 1: SPANS ({} total, showing first 20 from page 0) ===",
                        spans.len()
                    );
                    for (i, span) in page0_spans.iter().enumerate() {
                        let font = span.font_name.as_deref().unwrap_or("?");
                        let short_font = if font.len() > 15 { &font[..15] } else { font };
                        let text_display = if span.text.len() > 30 {
                            format!("{}...", &span.text[..30])
                        } else {
                            span.text.clone()
                        };
                        eprintln!(
                            "{:3}: y={:.0} x={:.0}-{:.0} sz={:.0} font={} text={:?}",
                            i, span.y0, span.x0, span.x1, span.font_size, short_font, text_display
                        );
                    }

                    // Step 2: spans to lines
                    let lines = grouper.spans_to_lines(spans);
                    let page0_lines: Vec<&Line> =
                        lines.iter().filter(|l| l.page_num == 0).take(15).collect();

                    eprintln!(
                        "\n=== STEP 2: LINES ({} total, showing first 15 from page 0) ===",
                        lines.len()
                    );
                    for (i, line) in page0_lines.iter().enumerate() {
                        let text: String = line
                            .spans
                            .iter()
                            .map(|s| s.text.as_str())
                            .collect::<Vec<_>>()
                            .join(" ");
                        let short_text = if text.len() > 60 {
                            format!("{}...", &text[..60])
                        } else {
                            text
                        };
                        eprintln!(
                            "{:3}: y={:.0}-{:.0} x={:.0}-{:.0} spans={} text={:?}",
                            i,
                            line.y0,
                            line.y1,
                            line.x0,
                            line.x1,
                            line.spans.len(),
                            short_text
                        );
                    }

                    // Step 3: lines to blocks
                    let blocks = grouper.group(&chars);
                    let page0_blocks: Vec<&TextBlock> =
                        blocks.iter().filter(|b| b.page_num == 0).take(15).collect();

                    eprintln!(
                        "\n=== STEP 3: BLOCKS ({} total, showing first 15 from page 0) ===",
                        blocks.len()
                    );
                    for (i, block) in page0_blocks.iter().enumerate() {
                        let text: String = block
                            .lines
                            .iter()
                            .flat_map(|l| l.spans.iter())
                            .map(|s| s.text.as_str())
                            .collect::<Vec<_>>()
                            .join(" ");
                        let short_text = if text.len() > 50 {
                            format!("{}...", &text[..50])
                        } else {
                            text
                        };
                        eprintln!(
                            "{:3}: y={:.0}-{:.0} x={:.0}-{:.0} lines={} type={:?} text={:?}",
                            i,
                            block.y0,
                            block.y1,
                            block.x0,
                            block.x1,
                            block.lines.len(),
                            block.block_type,
                            short_text
                        );
                    }
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
