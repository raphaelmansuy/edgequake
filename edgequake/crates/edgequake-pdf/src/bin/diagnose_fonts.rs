//! Diagnostic tool to dump font and encoding information from a PDF
//!
//! Usage: cargo run --bin diagnose_fonts -- <pdf_path>

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
    println!("PDF loaded successfully");

    // Get first page
    let page_ids = doc.get_pages();
    if let Some((&page_num, &page_id)) = page_ids.iter().next() {
        println!("\n=== Page {} (id: {:?}) ===", page_num, page_id);

        if let Ok((Some(resources), _)) = doc.get_page_resources(page_id) {
            // Get fonts dictionary
            if let Ok(fonts_dict) = resources.get(b"Font") {
                let fonts_dict = match fonts_dict {
                    Object::Dictionary(d) => d,
                    Object::Reference(r) => {
                        if let Ok(Object::Dictionary(d)) = doc.get_object(*r) {
                            d
                        } else {
                            println!("Could not resolve Font dictionary");
                            return;
                        }
                    }
                    _ => {
                        println!("Font is not a dictionary");
                        return;
                    }
                };

                println!("Found {} fonts", fonts_dict.len());

                for (font_name, font_obj) in fonts_dict.iter() {
                    let font_name = String::from_utf8_lossy(font_name);
                    println!("\n--- Font: {} ---", font_name);

                    let font_id = match font_obj {
                        Object::Reference(r) => *r,
                        _ => continue,
                    };

                    if let Ok(font_dict) = doc.get_dictionary(font_id) {
                        // Print font type
                        if let Ok(subtype) = font_dict.get(b"Subtype") {
                            println!("  Subtype: {:?}", subtype);
                        }

                        // Print base font
                        if let Ok(base_font) = font_dict.get(b"BaseFont") {
                            println!("  BaseFont: {:?}", base_font);
                        }

                        // Print encoding
                        if let Ok(encoding) = font_dict.get(b"Encoding") {
                            println!("  Encoding: {:?}", encoding);
                        }

                        // Check for ToUnicode
                        if let Ok(to_unicode) = font_dict.get(b"ToUnicode") {
                            println!("  ToUnicode reference: {:?}", to_unicode);

                            // Resolve the stream
                            if let Object::Reference(ref_id) = to_unicode {
                                if let Ok(stream) = doc.get_object(*ref_id) {
                                    if let Object::Stream(s) = stream {
                                        if let Ok(data) = s.decompressed_content() {
                                            let text = String::from_utf8_lossy(&data);
                                            println!("  ToUnicode CMap ({} bytes):", data.len());
                                            println!("  --- CMap content (first 80 lines) ---");
                                            for line in text.lines().take(80) {
                                                println!("    {}", line);
                                            }
                                            println!("  --- End CMap ---");

                                            // Count mappings
                                            let bfchar_count =
                                                text.matches("beginbfchar").count();
                                            let bfrange_count =
                                                text.matches("beginbfrange").count();
                                            println!(
                                                "  bfchar sections: {}, bfrange sections: {}",
                                                bfchar_count, bfrange_count
                                            );
                                        }
                                    }
                                }
                            }
                        } else {
                            println!("  No ToUnicode CMap!");
                        }

                        // Check for descendant fonts (for Type0 fonts)
                        if let Ok(descendants) = font_dict.get(b"DescendantFonts") {
                            println!("  DescendantFonts: {:?}", descendants);
                        }
                    }
                }
            } else {
                println!("No Font dictionary in resources");
            }
        } else {
            println!("No resources for page");
        }
    }

    // Also dump raw content stream for first page to see text operators
    println!("\n=== Content stream analysis ===");
    if let Some((_, &page_id)) = page_ids.iter().next() {
        if let Ok(content) = doc.get_page_content(page_id) {
            let text = String::from_utf8_lossy(&content);
            println!("First 1500 chars of content stream:");
            println!("{}", &text[..text.len().min(1500)]);
        }
    }
}
