//! Debug tool to understand why certain words get incorrect spacing.
//! This dumps raw character data for specific text patterns.

use std::path::PathBuf;

#[cfg(feature = "pdfium")]
use pdfium_render::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(feature = "pdfium"))]
    {
        println!("Pdfium feature not enabled. Run with: cargo run --example debug_word_spacing --features pdfium");
        return Ok(());
    }

    #[cfg(feature = "pdfium")]
    {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let sample_pdf =
            manifest_dir.join("test-data/real_dataset/one_tool_2512.20957v2.pdf");

        println!("Analyzing character spacing in {}...\n", sample_pdf.display());

        // Load pdfium
        let libs_path = std::path::Path::new("libs/lib");
        let bindings = if libs_path.exists() {
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(libs_path))?
        } else {
            Pdfium::bind_to_system_library()?
        };
        let pdfium = Pdfium::new(bindings);

        let pdf_bytes = std::fs::read(&sample_pdf)?;
        let document = pdfium.load_pdf_from_byte_vec(pdf_bytes, None)?;

        // Search for specific patterns
        let search_patterns = ["jump", "ump", "arep", "arepository"];

        for (page_idx, page) in document.pages().iter().enumerate() {
            let text_page = page.text()?;
            let page_height = page.height().value;

            // Collect characters
            let mut chars: Vec<(String, f32, f32, f32, f32, f32)> = Vec::new();
            
            for char_info in text_page.chars().iter() {
                let bounds = match char_info.tight_bounds() {
                    Ok(b) => b,
                    Err(_) => continue,
                };

                let char_text = match char_info.unicode_string() {
                    Some(s) => s.replace("\r", "").replace("\n", ""),
                    None => continue,
                };

                if char_text.is_empty() {
                    continue;
                }

                let char_left = bounds.left().value;
                let char_right = bounds.right().value;
                let char_top = page_height - bounds.top().value;
                let char_bottom = page_height - bounds.bottom().value;
                let char_height = (char_bottom - char_top).abs();

                chars.push((char_text, char_left, char_right, char_top, char_bottom, char_height));
            }

            // Find windows containing search patterns
            for pattern in &search_patterns {
                // Look for character sequences that could form the pattern
                for i in 0..chars.len().saturating_sub(pattern.len()) {
                    let window: String = chars[i..i + pattern.len().min(chars.len() - i)]
                        .iter()
                        .map(|(c, _, _, _, _, _)| c.as_str())
                        .collect();

                    if window.to_lowercase().contains(&pattern.to_lowercase()) {
                        println!(
                            "\n=== Found '{}' near '{}' on page {} ===",
                            pattern, window, page_idx + 1
                        );
                        
                        // Print detailed char info for context (5 before, pattern, 5 after)
                        let start = i.saturating_sub(2);
                        let end = (i + pattern.len() + 5).min(chars.len());
                        
                        println!("Char | Left    | Right   | Height  | Gap from prev");
                        println!("-------------------------------------------------");
                        
                        let mut prev_right: Option<f32> = None;
                        for j in start..end {
                            let (text, left, right, _, _, height) = &chars[j];
                            let gap = prev_right.map(|pr| left - pr).unwrap_or(0.0);
                            let gap_ratio = gap / height;
                            
                            let threshold_met = gap > height * 0.3;
                            let marker = if j >= i && j < i + pattern.len() {
                                " <-- PATTERN"
                            } else {
                                ""
                            };
                            let space_marker = if threshold_met && prev_right.is_some() {
                                " [SPACE INSERTED]"
                            } else {
                                ""
                            };
                            
                            println!(
                                "'{}' | {:7.2} | {:7.2} | {:7.2} | {:7.2} ({:.2}x height){}{}",
                                text, left, right, height, gap, gap_ratio, space_marker, marker
                            );
                            
                            prev_right = Some(*right);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
