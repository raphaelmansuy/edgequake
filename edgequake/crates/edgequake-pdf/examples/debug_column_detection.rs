//! Debug tool to trace column detection logic for a specific pattern.

use std::path::PathBuf;

#[cfg(feature = "pdfium")]
use pdfium_render::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(feature = "pdfium"))]
    {
        println!("Pdfium feature not enabled.");
        return Ok(());
    }

    #[cfg(feature = "pdfium")]
    {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let sample_pdf = manifest_dir.join("test-data/real_dataset/one_tool_2512.20957v2.pdf");

        println!("Tracing column detection in {}...\n", sample_pdf.display());

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

        // Only check page 1
        if let Some(page) = document.pages().iter().next() {
            let text_page = page.text()?;
            let page_height = page.height().value;

            // Collect characters
            #[derive(Clone, Debug)]
            struct CharData {
                text: String,
                left: f32,
                right: f32,
                top: f32,
                bottom: f32,
                height: f32,
            }

            let mut all_chars = Vec::new();
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

                all_chars.push(CharData {
                    text: char_text,
                    left: char_left,
                    right: char_right,
                    top: char_top,
                    bottom: char_bottom,
                    height: char_height,
                });
            }

            // Sort by top coordinate
            all_chars.sort_by(|a, b| {
                a.top.partial_cmp(&b.top).unwrap()
                    .then(a.left.partial_cmp(&b.left).unwrap())
            });

            // Group into lines
            let mut lines: Vec<Vec<CharData>> = Vec::new();
            for char_data in all_chars {
                let mut found_line = false;
                for line in lines.iter_mut().rev().take(10) {
                    let l_top = line.iter().map(|c| c.top).fold(f32::MAX, |a, b| a.min(b));
                    let l_bottom = line.iter().map(|c| c.bottom).fold(f32::MIN, |a, b| a.max(b));
                    let l_height = (l_bottom - l_top).max(1.0);
                    let overlap = (char_data.bottom.min(l_bottom) - char_data.top.max(l_top)).max(0.0);
                    if overlap > char_data.height * 0.3 || overlap > l_height * 0.3 {
                        line.push(char_data.clone());
                        found_line = true;
                        break;
                    }
                }
                if !found_line {
                    lines.push(vec![char_data]);
                }
            }

            // Find lines containing "arepository" pattern
            for (line_idx, mut line_chars) in lines.into_iter().enumerate() {
                // Sort by left coordinate
                line_chars.sort_by(|a, b| a.left.partial_cmp(&b.left).unwrap());
                
                // Calculate median char width (mimicking pdfium.rs)
                let mut char_widths: Vec<f32> = line_chars
                    .iter()
                    .map(|c| (c.right - c.left).abs())
                    .filter(|w| *w > 0.5)
                    .collect();
                char_widths.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let median_char_width = if char_widths.is_empty() {
                    5.0
                } else {
                    char_widths[char_widths.len() / 2]
                };
                
                // Reconstruct text with space insertion logic
                let mut result_text = String::new();
                let mut prev_right: Option<f32> = None;
                
                for c in &line_chars {
                    if let Some(pr) = prev_right {
                        let h_dist = c.left - pr;
                        let curr_char = c.text.chars().next().unwrap_or(' ');
                        let curr_is_punct = curr_char.is_ascii_punctuation();
                        
                        let threshold = if curr_is_punct {
                            median_char_width * 0.8
                        } else {
                            median_char_width * 0.5
                        };
                        
                        if h_dist > threshold && c.text != " " && !result_text.ends_with(' ') {
                            result_text.push(' ');
                        }
                    }
                    result_text.push_str(&c.text);
                    prev_right = Some(c.right);
                }
                
                // Look for problematic patterns
                if result_text.contains("arepository") || result_text.contains("repository") && line_idx < 100 {
                    println!("\n=== LINE {} ===", line_idx);
                    println!("Median char width: {:.2}", median_char_width);
                    println!("Reconstructed text: {}", result_text);
                    
                    // Show detail for chars around 'a' and 'r' of 'arepository'
                    if result_text.contains("arepository") {
                        println!("\n--- Character detail for 'as arepository' region ---");
                        for (i, c) in line_chars.iter().enumerate() {
                            if c.text == "a" || c.text == "r" || c.text == "s" || c.text == " " {
                                let gap = if i > 0 { 
                                    c.left - line_chars[i-1].right 
                                } else { 
                                    0.0 
                                };
                                let threshold = median_char_width * 0.5;
                                let space_inserted = gap > threshold && c.text != " ";
                                println!(
                                    "'{}' at x={:.1}-{:.1}, gap={:.2}, threshold={:.2}, space_inserted={}",
                                    c.text, c.left, c.right, gap, threshold, space_inserted
                                );
                            }
                        }
                    }
                    
                    if line_idx > 70 {
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}
