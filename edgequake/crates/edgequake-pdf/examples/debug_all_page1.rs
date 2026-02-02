//! Debug all elements extracted from page 1 of hotmess PDF
use lopdf::Document;
use std::path::Path;

fn main() {
    let pdf_path = Path::new(
        "/Users/raphaelmansuy/Github/03-working/edgequake/zz_test_docs/hotmess_2601.23045v1.pdf",
    );

    let doc = Document::load(pdf_path).expect("Failed to load PDF");
    let pages = doc.get_pages();
    let page_id = *pages.get(&1).expect("No page 1");
    let content = doc.get_page_content(page_id).expect("No content");
    let ops = lopdf::content::Content::decode(&content).expect("Decode failed");

    println!("=== ALL text from page 1 of hotmess ===\n");

    let mut cur_font_size = 0.0f64;
    let mut cur_font = String::new();
    let mut text_count = 0;

    for op in &ops.operations {
        match op.operator.as_str() {
            "BT" => {}
            "Tf" => {
                if let Some(lopdf::Object::Name(name)) = op.operands.first() {
                    cur_font = String::from_utf8_lossy(name).to_string();
                }
                if let Some(size_obj) = op.operands.get(1) {
                    cur_font_size = match size_obj {
                        lopdf::Object::Real(s) => *s as f64,
                        lopdf::Object::Integer(s) => *s as f64,
                        _ => cur_font_size,
                    };
                }
            }
            "Tj" => {
                if let Some(lopdf::Object::String(bytes, _)) = op.operands.first() {
                    let text = String::from_utf8_lossy(bytes);
                    text_count += 1;
                    println!(
                        "{:3}  font={:.1}  {:10}  '{}'",
                        text_count, cur_font_size, cur_font, text
                    );
                }
            }
            "TJ" => {
                if let Some(lopdf::Object::Array(arr)) = op.operands.first() {
                    let mut combined = String::new();
                    for item in arr {
                        if let lopdf::Object::String(bytes, _) = item {
                            combined.push_str(&String::from_utf8_lossy(bytes));
                        }
                    }
                    if !combined.trim().is_empty() {
                        text_count += 1;
                        println!(
                            "{:3}  font={:.1}  {:10}  '{}'",
                            text_count, cur_font_size, cur_font, combined
                        );
                    }
                }
            }
            _ => {}
        }
    }

    println!("\nTotal text operations: {}", text_count);
}
