//! Debug page content extraction with coordinates
//!
//! Run with: cargo run --example debug_page_coords
//!
//! This example tracks text position to understand why title fragments
//! are not being assembled correctly.

use lopdf::Document;
use std::path::Path;

fn main() {
    let pdf_path = Path::new(
        "/Users/raphaelmansuy/Github/03-working/edgequake/zz_test_docs/hotmess_2601.23045v1.pdf",
    );

    println!("=== DEBUG: Hotmess PDF Page 1 Coordinates ===\n");

    let doc = Document::load(pdf_path).expect("Failed to load PDF");
    let pages = doc.get_pages();
    let page_id = *pages.get(&1).expect("No page 1");

    let content = doc.get_page_content(page_id).expect("No content");
    let ops = lopdf::content::Content::decode(&content).expect("Decode failed");

    // Track graphics state
    let mut cur_font = String::new();
    let mut font_size = 0.0f64;
    let mut tm: [f64; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]; // Text matrix
    let mut lm: [f64; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]; // Line matrix
    let mut text_op_index = 0;

    println!(
        "{:4} {:8} {:8} {:8} {:10} {}",
        "Op#", "X", "Y", "Size", "Font", "Text"
    );
    println!("{}", "-".repeat(80));

    for op in &ops.operations {
        match op.operator.as_str() {
            "BT" => {
                // Begin text - reset matrices
                tm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                lm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
            }
            "Tf" => {
                // Font selection: /FontName size Tf
                if let Some(lopdf::Object::Name(name)) = op.operands.first() {
                    cur_font = String::from_utf8_lossy(name).to_string();
                }
                if let Some(size_obj) = op.operands.get(1) {
                    font_size = match size_obj {
                        lopdf::Object::Real(s) => *s as f64,
                        lopdf::Object::Integer(s) => *s as f64,
                        _ => font_size,
                    };
                }
            }
            "Tm" => {
                // Text matrix: a b c d e f Tm
                if op.operands.len() >= 6 {
                    for (i, obj) in op.operands.iter().take(6).enumerate() {
                        tm[i] = match obj {
                            lopdf::Object::Real(v) => *v as f64,
                            lopdf::Object::Integer(v) => *v as f64,
                            _ => tm[i],
                        };
                    }
                    lm = tm; // Tm also sets line matrix
                }
            }
            "Td" => {
                // Move: tx ty Td
                let (tx, ty) = extract_pair(&op.operands);
                // Translate both matrices
                tm[4] += tx * tm[0] + ty * tm[2];
                tm[5] += tx * tm[1] + ty * tm[3];
                lm = tm;
            }
            "TD" => {
                // Move and set leading: tx ty TD (equivalent to -ty TL, tx ty Td)
                let (tx, ty) = extract_pair(&op.operands);
                tm[4] += tx * tm[0] + ty * tm[2];
                tm[5] += tx * tm[1] + ty * tm[3];
                lm = tm;
            }
            "T*" => {
                // Move to next line (using TL)
                // Simplified: just use line matrix as is for now
            }
            "TJ" | "Tj" => {
                let text = match op.operands.first() {
                    Some(lopdf::Object::String(s, _)) => String::from_utf8_lossy(s).to_string(),
                    Some(lopdf::Object::Array(arr)) => {
                        let mut result = String::new();
                        for item in arr {
                            if let lopdf::Object::String(s, _) = item {
                                result.push_str(&String::from_utf8_lossy(s));
                            }
                        }
                        result
                    }
                    _ => String::new(),
                };

                // Get effective position (e, f of text matrix)
                let x = tm[4];
                let y = tm[5];

                // Print first 40 text operations
                if text_op_index < 40 {
                    println!(
                        "{:4} {:8.2} {:8.2} {:8.1} {:10} \"{}\"",
                        text_op_index,
                        x,
                        y,
                        font_size,
                        if cur_font.len() > 10 {
                            &cur_font[..10]
                        } else {
                            &cur_font
                        },
                        if text.len() > 40 {
                            format!("{}...", &text[..40])
                        } else {
                            text.clone()
                        }
                    );
                }
                text_op_index += 1;
            }
            _ => {}
        }
    }

    println!("\nTotal text operations: {}", text_op_index);
}

fn extract_pair(operands: &[lopdf::Object]) -> (f64, f64) {
    let x = match operands.first() {
        Some(lopdf::Object::Real(v)) => *v as f64,
        Some(lopdf::Object::Integer(v)) => *v as f64,
        _ => 0.0,
    };
    let y = match operands.get(1) {
        Some(lopdf::Object::Real(v)) => *v as f64,
        Some(lopdf::Object::Integer(v)) => *v as f64,
        _ => 0.0,
    };
    (x, y)
}
