//! Show all elements at a specific Y range

use lopdf::content::Content;
use lopdf::{Document, Object};
use std::collections::BTreeMap;
use std::env;
use std::path::Path;

use edgequake_pdf::backend::font_handling::FontInfo;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_path>", args[0]);
        std::process::exit(1);
    }

    let pdf_path = Path::new(&args[1]);
    let doc = Document::load(pdf_path).expect("Failed to load PDF");

    // Get first page
    let page_ids = doc.get_pages();
    if let Some((_, &page_id)) = page_ids.iter().next() {
        let fonts = get_page_fonts(&doc, page_id);

        if let Ok(content_bytes) = doc.get_page_content(page_id) {
            let content = Content::decode(&content_bytes).expect("Failed to decode content");

            let mut current_font: Option<&FontInfo> = None;
            let mut text_matrix = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0];
            let mut line_matrix = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0];

            let mut elements_at_target = Vec::new();

            for op in &content.operations {
                match op.operator.as_str() {
                    "BT" => {
                        text_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                        line_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                    }
                    "Tf" => {
                        if !op.operands.is_empty() {
                            if let Object::Name(name) = &op.operands[0] {
                                current_font = fonts.get(name);
                            }
                        }
                    }
                    "Tm" => {
                        if op.operands.len() >= 6 {
                            for (i, operand) in op.operands.iter().enumerate().take(6) {
                                text_matrix[i] = get_number(operand).unwrap_or(0.0);
                            }
                            line_matrix = text_matrix;
                        }
                    }
                    "Td" | "TD" => {
                        if op.operands.len() >= 2 {
                            let tx = get_number(&op.operands[0]).unwrap_or(0.0);
                            let ty = get_number(&op.operands[1]).unwrap_or(0.0);
                            line_matrix[4] += tx;
                            line_matrix[5] += ty;
                            text_matrix = line_matrix;
                        }
                    }
                    "Tj" => {
                        if !op.operands.is_empty() {
                            if let Object::String(bytes, _) = &op.operands[0] {
                                let text = if let Some(font) = current_font {
                                    font.encoding.decode(bytes)
                                } else {
                                    String::from_utf8_lossy(bytes).to_string()
                                };

                                let y = text_matrix[5];

                                // Target Y=-6.27 ± 0.5
                                if (y - (-6.27)).abs() < 0.5 {
                                    elements_at_target.push((
                                        text_matrix[4],
                                        text.clone(),
                                        current_font
                                            .map(|f| f.base_font.clone())
                                            .unwrap_or_default(),
                                    ));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            println!("Elements at Y≈-6.27: {} total", elements_at_target.len());
            for (i, (x, text, font)) in elements_at_target.iter().enumerate() {
                println!("  [{:2}] X={:7.2} text='{}' font={}", i + 1, x, text, font);
            }
        }
    }
}

fn get_page_fonts(doc: &Document, page_id: lopdf::ObjectId) -> BTreeMap<Vec<u8>, FontInfo> {
    let mut fonts = BTreeMap::new();

    if let Ok((Some(resources), _)) = doc.get_page_resources(page_id) {
        if let Ok(font_obj) = resources.get(b"Font") {
            let font_dict = match font_obj {
                Object::Dictionary(d) => Some(d.clone()),
                Object::Reference(r) => doc.get_object(*r).ok().and_then(|o| match o {
                    Object::Dictionary(d) => Some(d.clone()),
                    _ => None,
                }),
                _ => None,
            };

            if let Some(font_dict) = font_dict {
                for (name, value) in font_dict.iter() {
                    if let Object::Reference(id) = value {
                        if let Ok(fd) = doc.get_dictionary(*id) {
                            fonts.insert(name.clone(), FontInfo::from_dict(doc, fd));
                        }
                    }
                }
            }
        }
    }

    fonts
}

fn get_number(obj: &Object) -> Option<f32> {
    match obj {
        Object::Integer(i) => Some(*i as f32),
        Object::Real(f) => Some(*f),
        _ => None,
    }
}
