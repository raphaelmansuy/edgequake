//! Debug TJ kerning values in hotmess PDF
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

    println!("=== TJ kerning values in hotmess title ===\n");

    let mut title_region = false;
    let mut tj_count = 0;

    for op in &ops.operations {
        match op.operator.as_str() {
            "Tf" => {
                if let Some(size_obj) = op.operands.get(1) {
                    let size = match size_obj {
                        lopdf::Object::Real(s) => *s as f64,
                        lopdf::Object::Integer(s) => *s as f64,
                        _ => 0.0,
                    };
                    // Title uses 17.2 and 13.8 font sizes
                    title_region = size > 13.0 && size < 18.0;
                }
            }
            "TJ" => {
                if title_region && tj_count < 30 {
                    tj_count += 1;
                    if let Some(lopdf::Object::Array(arr)) = op.operands.first() {
                        print!("TJ#{}: ", tj_count);
                        for item in arr {
                            match item {
                                lopdf::Object::String(bytes, _) => {
                                    let text = String::from_utf8_lossy(bytes);
                                    print!("\"{}\" ", text);
                                }
                                lopdf::Object::Integer(n) => {
                                    print!("[{}] ", n);
                                }
                                lopdf::Object::Real(n) => {
                                    print!("[{:.1}] ", n);
                                }
                                _ => {}
                            }
                        }
                        println!();
                    }
                }
            }
            _ => {}
        }
    }
}
