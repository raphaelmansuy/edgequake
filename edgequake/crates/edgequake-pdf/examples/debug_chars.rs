//! Debug tool to dump raw characters extracted by PDFium for a specific page area.
//!
//! Usage:
//!   PDFIUM_DYNAMIC_LIB_PATH=/path/to/libpdfium.dylib cargo run --features pdfium --example debug_chars <PDF_PATH> [PAGE] [Y_MIN] [Y_MAX]

fn main() {
    let lib_path = match std::env::var("PDFIUM_DYNAMIC_LIB_PATH") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("ERROR: Set PDFIUM_DYNAMIC_LIB_PATH environment variable");
            std::process::exit(1);
        }
    };

    let args: Vec<String> = std::env::args().collect();
    let pdf_path = match args.get(1) {
        Some(p) => p.clone(),
        None => {
            eprintln!("Usage: debug_chars <PDF_PATH> [PAGE] [Y_MIN] [Y_MAX]");
            std::process::exit(1);
        }
    };

    let page: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let y_min: f32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let y_max: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(f32::MAX);

    #[cfg(feature = "pdfium")]
    {
        use edgequake_pdf::backend::pdfium::PdfiumExtractor;

        match PdfiumExtractor::with_library_path(&lib_path) {
            Ok(extractor) => match extractor.extract_chars_from_file(&pdf_path) {
                Ok(chars) => {
                    // Filter to target page and Y range
                    let filtered: Vec<_> = chars
                        .iter()
                        .filter(|c| c.page_num == page && c.y0 >= y_min && c.y0 <= y_max)
                        .collect();

                    eprintln!(
                        "=== RAW CHARS: page={} y=[{:.0},{:.0}] found {} chars ===",
                        page,
                        y_min,
                        y_max,
                        filtered.len()
                    );

                    // Sort by y then x for readable output
                    let mut sorted = filtered.clone();
                    sorted.sort_by(|a, b| {
                        a.y0.partial_cmp(&b.y0)
                            .unwrap()
                            .then(a.x0.partial_cmp(&b.x0).unwrap())
                    });

                    let mut prev_y: Option<f32> = None;
                    let mut prev_x1: Option<f32> = None;
                    let mut prev_size: Option<f32> = None;

                    for ch in &sorted {
                        // Detect line change
                        let line_change = match prev_y {
                            Some(py) => (ch.y0 - py).abs() > 2.0,
                            None => false,
                        };
                        if line_change {
                            eprintln!("---");
                            prev_x1 = None;
                            prev_size = None;
                        }

                        // Calculate gap from previous char
                        let gap = match prev_x1 {
                            Some(px1) => ch.x0 - px1,
                            None => 0.0,
                        };
                        let space_thresh = match prev_size {
                            Some(sz) => sz * 0.25,
                            None => 0.0,
                        };

                        let is_space = ch.char.is_whitespace();
                        let marker = if is_space {
                            "SPC"
                        } else if gap > space_thresh && prev_x1.is_some() {
                            "GAP"
                        } else {
                            "   "
                        };

                        eprintln!(
                            "{} {:?} x=[{:6.1},{:6.1}] y=[{:6.1},{:6.1}] sz={:4.1} gap={:5.1} thresh={:4.1} font={:?} bold={} mono={}",
                            marker,
                            ch.char,
                            ch.x0,
                            ch.x1,
                            ch.y0,
                            ch.y1,
                            ch.font_size,
                            gap,
                            space_thresh,
                            ch.font_name.as_deref().unwrap_or("?"),
                            ch.is_bold,
                            ch.is_monospace,
                        );

                        prev_y = Some(ch.y0);
                        prev_x1 = Some(ch.x1);
                        prev_size = Some(ch.font_size);
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
