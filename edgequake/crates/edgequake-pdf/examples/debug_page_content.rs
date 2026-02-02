//! Debug page content extraction
//!
//! Run with: cargo run --example debug_page_content
//!
//! This example dumps the content operators from page 1 of the hotmess PDF
//! to identify where text extraction is failing.

use lopdf::Document;
use std::path::Path;

fn main() {
    let pdf_path = Path::new(
        "/Users/raphaelmansuy/Github/03-working/edgequake/zz_test_docs/hotmess_2601.23045v1.pdf",
    );

    println!("=== DEBUG: Hotmess PDF Page 1 ===\n");

    let doc = Document::load(pdf_path).expect("Failed to load PDF");

    // Get pages
    let pages = doc.get_pages();
    println!("Total pages: {}", pages.len());

    // Get first page
    let page_id = *pages.get(&1).expect("No page 1");
    println!("Page 1 object ID: {:?}\n", page_id);

    // Get page dictionary
    let page = doc.get_dictionary(page_id).expect("No page dict");

    // Print MediaBox
    println!("=== Page Attributes ===");
    if let Ok(mbox) = page.get(b"MediaBox") {
        println!("MediaBox: {:?}", mbox);
    }
    if let Ok(cbox) = page.get(b"CropBox") {
        println!("CropBox: {:?}", cbox);
    }

    // Get Resources
    println!("\n=== Resources ===");
    let resources = match page.get(b"Resources") {
        Ok(lopdf::Object::Reference(id)) => {
            println!("Resources -> ref {:?}", id);
            doc.get_dictionary(*id).ok()
        }
        Ok(lopdf::Object::Dictionary(d)) => {
            println!("Resources -> inline dict");
            Some(d)
        }
        _ => {
            println!("Resources -> MISSING!");
            None
        }
    };

    if let Some(res) = resources {
        // Print all resource keys
        println!(
            "Resource keys: {:?}",
            res.iter()
                .map(|(k, _)| String::from_utf8_lossy(k).to_string())
                .collect::<Vec<_>>()
        );

        // Print Font
        if let Ok(fonts) = res.get(b"Font") {
            match fonts {
                lopdf::Object::Reference(id) => {
                    println!("\nFont -> ref {:?}", id);
                    if let Ok(font_dict) = doc.get_dictionary(*id) {
                        for (name, val) in font_dict.iter() {
                            println!(
                                "  /{}: {:?}",
                                String::from_utf8_lossy(name),
                                match val {
                                    lopdf::Object::Reference(r) => format!("ref {:?}", r),
                                    _ => format!("{:?}", val),
                                }
                            );
                        }
                    }
                }
                lopdf::Object::Dictionary(d) => {
                    println!("\nFont -> inline dict");
                    for (name, val) in d.iter() {
                        println!(
                            "  /{}: {:?}",
                            String::from_utf8_lossy(name),
                            match val {
                                lopdf::Object::Reference(r) => format!("ref {:?}", r),
                                _ => format!("{:?}", val),
                            }
                        );
                    }
                }
                _ => println!("\nFont -> {:?}", fonts),
            }
        }

        // Print XObject (may contain image with text overlay)
        if let Ok(xobj) = res.get(b"XObject") {
            match xobj {
                lopdf::Object::Reference(id) => {
                    println!("\nXObject -> ref {:?}", id);
                    if let Ok(xobj_dict) = doc.get_dictionary(*id) {
                        for (name, val) in xobj_dict.iter() {
                            println!(
                                "  /{}: {:?}",
                                String::from_utf8_lossy(name),
                                match val {
                                    lopdf::Object::Reference(r) => format!("ref {:?}", r),
                                    _ => format!("{:?}", val),
                                }
                            );
                        }
                    }
                }
                lopdf::Object::Dictionary(d) => {
                    println!("\nXObject -> inline dict");
                    for (name, _) in d.iter() {
                        println!("  /{}", String::from_utf8_lossy(name));
                    }
                }
                _ => println!("\nXObject -> {:?}", xobj),
            }
        }
    }

    // Get content stream
    println!("\n=== Content Stream ===");
    match doc.get_page_content(page_id) {
        Ok(content) => {
            println!("Content bytes: {}", content.len());

            match lopdf::content::Content::decode(&content) {
                Ok(ops) => {
                    println!("Total operations: {}", ops.operations.len());

                    // Find text operators
                    let text_ops: Vec<_> = ops
                        .operations
                        .iter()
                        .filter(|op| matches!(op.operator.as_str(), "Tj" | "TJ" | "'" | "\""))
                        .collect();
                    println!("Text operators: {}", text_ops.len());

                    // Find Tf (font selection) operators
                    let font_ops: Vec<_> = ops
                        .operations
                        .iter()
                        .filter(|op| op.operator == "Tf")
                        .collect();
                    println!("Font selection (Tf) operators: {}", font_ops.len());

                    // Print first 20 text ops with surrounding context
                    println!("\n=== First 20 Text Operations ===");
                    for (i, op) in text_ops.iter().take(20).enumerate() {
                        // Decode text from operand
                        let text = match op.operands.first() {
                            Some(lopdf::Object::String(s, _)) => {
                                String::from_utf8_lossy(s).to_string()
                            }
                            Some(lopdf::Object::Array(arr)) => {
                                // TJ array
                                let mut result = String::new();
                                for item in arr {
                                    if let lopdf::Object::String(s, _) = item {
                                        result.push_str(&String::from_utf8_lossy(s));
                                    }
                                }
                                result
                            }
                            _ => format!("{:?}", op.operands),
                        };
                        println!("{}: {} -> \"{}\"", i, op.operator, text);
                    }

                    // Look for "HOT" or "MESS" in any text
                    println!("\n=== Searching for title text ===");
                    for (i, op) in ops.operations.iter().enumerate() {
                        if matches!(op.operator.as_str(), "Tj" | "TJ") {
                            let text = match op.operands.first() {
                                Some(lopdf::Object::String(s, _)) => {
                                    String::from_utf8_lossy(s).to_string()
                                }
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

                            let upper = text.to_uppercase();
                            if upper.contains("THE")
                                || upper.contains("HOT")
                                || upper.contains("MESS")
                                || upper.contains("AI")
                            {
                                println!("Op #{}: {} -> \"{}\"", i, op.operator, text);
                            }
                        }
                    }
                }
                Err(e) => println!("Failed to decode content: {:?}", e),
            }
        }
        Err(e) => println!("Failed to get content: {:?}", e),
    }
}
