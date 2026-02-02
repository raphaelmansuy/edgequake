//! Diagnostic tool to test ToUnicode CMap decoding
//!
//! Usage: cargo run --bin test_decode -- <pdf_path>

use edgequake_pdf::backend::encodings::ToUnicodeMap;
use lopdf::{Document, Object};
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_path>", args[0]);
        std::process::exit(1);
    }

    let pdf_path = Path::new(&args[1]);
    println!("Loading PDF: {}", pdf_path.display());

    let doc = Document::load(pdf_path).expect("Failed to load PDF");

    // Get first page
    let page_ids = doc.get_pages();
    if let Some((_, &page_id)) = page_ids.iter().next() {
        if let Ok((Some(resources), _)) = doc.get_page_resources(page_id) {
            if let Ok(fonts_dict) = resources.get(b"Font") {
                let fonts_dict = match fonts_dict {
                    Object::Dictionary(d) => d,
                    Object::Reference(r) => {
                        if let Ok(Object::Dictionary(d)) = doc.get_object(*r) {
                            d
                        } else {
                            return;
                        }
                    }
                    _ => return,
                };

                // Get F4 font (the first one used)
                for (font_name, font_obj) in fonts_dict.iter() {
                    let fname = String::from_utf8_lossy(font_name);
                    if fname != "F4" {
                        continue;
                    }
                    
                    let font_id = match font_obj {
                        Object::Reference(r) => *r,
                        _ => continue,
                    };
                    
                    if let Ok(font_dict) = doc.get_dictionary(font_id) {
                        if let Ok(to_unicode) = font_dict.get(b"ToUnicode") {
                            if let Object::Reference(ref_id) = to_unicode {
                                if let Ok(Object::Stream(s)) = doc.get_object(*ref_id) {
                                    if let Ok(data) = s.decompressed_content() {
                                        println!("=== Parsing ToUnicode CMap ===");
                                        let cmap = ToUnicodeMap::parse(&data);

                                        // Print all mappings
                                        println!("\n=== Mappings ({} total) ===", cmap.mappings.len());
                                        let mut keys: Vec<_> = cmap.mappings.keys().collect();
                                        keys.sort();
                                        for key in keys {
                                            let val = &cmap.mappings[key];
                                            let chars: String = val
                                                .iter()
                                                .filter_map(|&c| char::from_u32(c as u32))
                                                .collect();
                                            println!(
                                                "  0x{:04X} -> {:?} = '{}'",
                                                key, val, chars
                                            );
                                        }

                                        // Test decoding the actual bytes from the PDF
                                        println!("\n=== Test decoding ===");

                                        // From the content stream: <0037> should be 'T'
                                        let bytes1 = [0x00, 0x37];
                                        let result1 = cmap.decode(&bytes1);
                                        println!(
                                            "decode([0x00, 0x37]) = '{}' (expected 'T')",
                                            result1
                                        );

                                        // <004C0057004F0048001D> should be "itle:"
                                        let bytes2 = [0x00, 0x4C, 0x00, 0x57, 0x00, 0x4F, 0x00, 0x48, 0x00, 0x1D];
                                        let result2 = cmap.decode(&bytes2);
                                        println!(
                                            "decode([0x00,0x4C,...]) = '{}' (expected 'itle:')",
                                            result2
                                        );

                                        // <0003> should be space
                                        let bytes3 = [0x00, 0x03];
                                        let result3 = cmap.decode(&bytes3);
                                        println!(
                                            "decode([0x00, 0x03]) = '{}' (expected ' ')",
                                            result3
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}