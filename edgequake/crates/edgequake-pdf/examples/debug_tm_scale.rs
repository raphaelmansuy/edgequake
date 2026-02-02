//! Debug Tm operator to understand text matrix scaling
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

    println!("=== Text Matrix (Tm) and Font (Tf) operators ===\n");
    
    let mut cur_font_size = 0.0f64;
    let mut cur_font = String::new();
    let mut tm: [f64; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut op_count = 0;

    for op in &ops.operations {
        match op.operator.as_str() {
            "BT" => {
                tm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
            }
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
                if op_count < 20 {
                    println!("Tf: font={} size={:.1}", cur_font, cur_font_size);
                }
            }
            "Tm" => {
                if op.operands.len() >= 6 {
                    for (i, obj) in op.operands.iter().take(6).enumerate() {
                        tm[i] = match obj {
                            lopdf::Object::Real(v) => *v as f64,
                            lopdf::Object::Integer(v) => *v as f64,
                            _ => tm[i],
                        };
                    }
                    if op_count < 20 {
                        let scale = (tm[0]*tm[0] + tm[1]*tm[1]).sqrt();
                        let effective_size = cur_font_size * scale;
                        println!("Tm: [{:.2} {:.2} {:.2} {:.2} {:.1} {:.1}] scale={:.2} effective_size={:.1}", 
                            tm[0], tm[1], tm[2], tm[3], tm[4], tm[5], scale, effective_size);
                    }
                }
            }
            "Td" | "TD" | "T*" => {
                if op_count < 20 && op.operands.len() >= 2 {
                    println!("{}: operands={:?}", op.operator, op.operands);
                }
            }
            "Tj" | "TJ" => {
                op_count += 1;
                if op_count <= 5 {
                    println!("  -> Text op #{} with font_size={:.1}", op_count, cur_font_size);
                }
            }
            _ => {}
        }
    }
    println!("\nTotal text ops: {}", op_count);
}
