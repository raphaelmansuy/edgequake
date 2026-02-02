//! Diagnostic tool to trace content stream parsing
//!
//! Usage: cargo run --bin trace_content -- <pdf_path>

use lopdf::content::Content;
use lopdf::{Document as LopdfDocument, Object, ObjectId};
use std::collections::BTreeMap;
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

    let doc = LopdfDocument::load(pdf_path).expect("Failed to load PDF");
    let pages = doc.get_pages();

    println!("Document has {} pages\n", pages.len());

    for (page_num, &page_id) in pages.iter() {
        println!("=== Page {} (id: {:?}) ===", page_num, page_id);

        // Get fonts
        let fonts = get_page_fonts(&doc, page_id);
        println!("Found {} fonts", fonts.len());
        for (name, info) in &fonts {
            let count = if let FontEncoding::ToUnicodeMap(map) = &info.1.encoding {
                map.len()
            } else {
                0
            };
            println!(
                "  Font: {} ({} mappings)",
                String::from_utf8_lossy(name),
                count
            );
        }

        // Get content
        let content_bytes = match get_page_content(&doc, page_id) {
            Ok(bytes) => bytes,
            Err(e) => {
                println!("Error getting content: {}", e);
                continue;
            }
        };

        println!("Content stream: {} bytes", content_bytes.len());

        // Parse content using same logic as ContentParser
        let content = match Content::decode(&content_bytes) {
            Ok(c) => c,
            Err(e) => {
                println!("Error decoding content: {}", e);
                continue;
            }
        };

        println!("Operations: {}", content.operations.len());

        // Simulate ContentParser exactly
        let mut text_elements: Vec<TextElement> = Vec::new();
        let mut current_font: Option<&FontInfo> = None;
        let mut current_font_name = String::new();
        let mut font_size: f32 = 12.0;
        let mut text_matrix = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0];
        let mut line_matrix = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0];
        let mut ctm = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0];
        let mut graphics_stack: Vec<[f32; 6]> = Vec::new();

        for op in &content.operations {
            match op.operator.as_str() {
                "q" => {
                    graphics_stack.push(ctm);
                }
                "Q" => {
                    if let Some(saved) = graphics_stack.pop() {
                        ctm = saved;
                    }
                }
                "cm" => {
                    if op.operands.len() >= 6 {
                        let mut new_matrix = [0.0f32; 6];
                        for (i, operand) in op.operands.iter().enumerate().take(6) {
                            new_matrix[i] = get_number(operand).unwrap_or(0.0);
                        }
                        // Multiply ctm * new_matrix
                        let a = ctm[0];
                        let b = ctm[1];
                        let c = ctm[2];
                        let d = ctm[3];
                        let e = ctm[4];
                        let f = ctm[5];
                        let a_p = new_matrix[0];
                        let b_p = new_matrix[1];
                        let c_p = new_matrix[2];
                        let d_p = new_matrix[3];
                        let e_p = new_matrix[4];
                        let f_p = new_matrix[5];

                        ctm[0] = a * a_p + b * c_p;
                        ctm[1] = a * b_p + b * d_p;
                        ctm[2] = c * a_p + d * c_p;
                        ctm[3] = c * b_p + d * d_p;
                        ctm[4] = e * a_p + f * c_p + e_p;
                        ctm[5] = e * b_p + f * d_p + f_p;
                    }
                }
                "BT" => {
                    text_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                    line_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                }
                "Tf" => {
                    if op.operands.len() >= 2 {
                        if let Object::Name(name) = &op.operands[0] {
                            if let Some((_, info)) = fonts.get(name) {
                                current_font = Some(info);
                                current_font_name = String::from_utf8_lossy(name).to_string();
                            } else {
                                current_font = None;
                                current_font_name = String::from_utf8_lossy(name).to_string();
                            }
                        }
                        if let Some(size) = get_number(&op.operands[1]) {
                            font_size = size.abs();
                        }
                    }
                }
                "Tm" => {
                    if op.operands.len() >= 6 {
                        for (i, operand) in op.operands.iter().enumerate().take(6) {
                            if let Some(v) = get_number(operand) {
                                text_matrix[i] = v;
                            }
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
                                font.decode(bytes)
                            } else {
                                // Fallback
                                bytes.iter().map(|&b| b as char).collect()
                            };

                            let text = text.replace(['\n', '\r'], "");
                            if !text.is_empty() {
                                // Apply CTM
                                let raw_x = text_matrix[4];
                                let raw_y = text_matrix[5];
                                let visual_x = ctm[0] * raw_x + ctm[2] * raw_y + ctm[4];
                                let visual_y = ctm[1] * raw_x + ctm[3] * raw_y + ctm[5];

                                text_elements.push(TextElement {
                                    text: text.clone(),
                                    x: visual_x,
                                    y: visual_y,
                                    font_size,
                                });

                                // Advance
                                let char_count = text.chars().count() as f32;
                                let estimated_width = char_count * font_size * 0.55;
                                text_matrix[4] += estimated_width;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        println!("\nExtracted {} text elements", text_elements.len());

        // Print first 100 elements
        for (i, elem) in text_elements.iter().take(100).enumerate() {
            println!(
                "  [{}] '{}' at ({:.1}, {:.1}), size={:.1}",
                i, elem.text, elem.x, elem.y, elem.font_size
            );
        }

        // Print count of remaining elements
        if text_elements.len() > 100 {
            println!("  ... and {} more", text_elements.len() - 100);
        }

        // Calculate bounds
        if !text_elements.is_empty() {
            let min_x = text_elements
                .iter()
                .map(|e| e.x)
                .fold(f32::INFINITY, f32::min);
            let max_x = text_elements
                .iter()
                .map(|e| e.x)
                .fold(f32::NEG_INFINITY, f32::max);
            let min_y = text_elements
                .iter()
                .map(|e| e.y)
                .fold(f32::INFINITY, f32::min);
            let max_y = text_elements
                .iter()
                .map(|e| e.y)
                .fold(f32::NEG_INFINITY, f32::max);

            println!("\nElement bounds:");
            println!("  X: {:.1} to {:.1}", min_x, max_x);
            println!("  Y: {:.1} to {:.1}", min_y, max_y);
        }
    }
}

fn get_number(obj: &Object) -> Option<f32> {
    match obj {
        Object::Integer(i) => Some(*i as f32),
        Object::Real(f) => Some(*f),
        _ => None,
    }
}

struct TextElement {
    text: String,
    x: f32,
    y: f32,
    font_size: f32,
}

/// FontInfo for testing
struct FontInfo {
    encoding: FontEncoding,
}

enum FontEncoding {
    ToUnicodeMap(std::collections::HashMap<u32, Vec<u16>>),
    WinAnsi,
}

impl FontInfo {
    fn decode(&self, bytes: &[u8]) -> String {
        match &self.encoding {
            FontEncoding::ToUnicodeMap(map) => {
                let mut result = String::new();
                let mut i = 0;
                while i < bytes.len() {
                    let mut found = false;

                    // Try 2-byte first
                    if i + 1 < bytes.len() {
                        let code2 = ((bytes[i] as u32) << 8) | (bytes[i + 1] as u32);
                        if let Some(chars) = map.get(&code2) {
                            for &cp in chars {
                                if let Some(c) = char::from_u32(cp as u32) {
                                    result.push(c);
                                }
                            }
                            i += 2;
                            found = true;
                        }
                    }

                    if !found {
                        let code1 = bytes[i] as u32;
                        if let Some(chars) = map.get(&code1) {
                            for &cp in chars {
                                if let Some(c) = char::from_u32(cp as u32) {
                                    result.push(c);
                                }
                            }
                        } else {
                            result.push('?');
                        }
                        i += 1;
                    }
                }
                result
            }
            FontEncoding::WinAnsi => bytes.iter().map(|&b| b as char).collect(),
        }
    }
}

fn get_page_fonts(doc: &LopdfDocument, page_id: ObjectId) -> BTreeMap<Vec<u8>, (String, FontInfo)> {
    let mut fonts = BTreeMap::new();

    let page_dict = match doc.get_dictionary(page_id) {
        Ok(d) => d,
        Err(_) => return fonts,
    };

    // Get Resources
    let resources = match page_dict.get(b"Resources") {
        Ok(Object::Reference(id)) => doc.get_dictionary(*id).ok(),
        Ok(Object::Dictionary(d)) => Some(d),
        _ => None,
    };

    if let Some(resources) = resources {
        // Get Font dictionary
        let font_dict = match resources.get(b"Font") {
            Ok(Object::Reference(id)) => doc.get_dictionary(*id).ok(),
            Ok(Object::Dictionary(d)) => Some(d),
            _ => None,
        };

        if let Some(font_dict) = font_dict {
            for (name, value) in font_dict.iter() {
                let font = match value {
                    Object::Reference(id) => doc.get_dictionary(*id).ok(),
                    Object::Dictionary(d) => Some(d),
                    _ => None,
                };

                if let Some(font) = font {
                    let base_font = font
                        .get(b"BaseFont")
                        .ok()
                        .and_then(|obj| obj.as_name().ok())
                        .map(|n| String::from_utf8_lossy(n).to_string())
                        .unwrap_or_else(|| "Unknown".to_string());

                    // Get ToUnicode
                    let encoding = if let Ok(to_unicode) = font.get(b"ToUnicode") {
                        if let Some(cmap) = parse_tounicode(doc, to_unicode) {
                            FontEncoding::ToUnicodeMap(cmap)
                        } else {
                            FontEncoding::WinAnsi
                        }
                    } else {
                        FontEncoding::WinAnsi
                    };

                    fonts.insert(name.clone(), (base_font, FontInfo { encoding }));
                }
            }
        }
    }

    fonts
}

fn parse_tounicode(
    doc: &LopdfDocument,
    obj: &Object,
) -> Option<std::collections::HashMap<u32, Vec<u16>>> {
    let stream = match obj {
        Object::Reference(id) => doc.get_object(*id).ok()?.as_stream().ok()?,
        Object::Stream(s) => s,
        _ => return None,
    };

    let data = stream.decompressed_content().ok()?;
    let text = String::from_utf8_lossy(&data);

    let mut map = std::collections::HashMap::new();

    // Parse bfchar
    let mut in_bfchar = false;
    for line in text.lines() {
        let line = line.trim();
        if line.contains("beginbfchar") {
            in_bfchar = true;
            continue;
        }
        if line.contains("endbfchar") {
            in_bfchar = false;
            continue;
        }
        if in_bfchar {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let (Some(src), Some(dst)) = (parse_hex(parts[0]), parse_hex_string(parts[1])) {
                    map.insert(src, dst);
                }
            }
        }
    }

    // Parse bfrange
    let mut in_bfrange = false;
    for line in text.lines() {
        let line = line.trim();
        if line.contains("beginbfrange") {
            in_bfrange = true;
            continue;
        }
        if line.contains("endbfrange") {
            in_bfrange = false;
            continue;
        }
        if in_bfrange {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                if let (Some(start), Some(end)) = (parse_hex(parts[0]), parse_hex(parts[1])) {
                    if let Some(dst_start) = parse_hex_string(parts[2]) {
                        if !dst_start.is_empty() {
                            let base = dst_start[0] as u32;
                            for (i, code) in (start..=end).enumerate() {
                                map.insert(code, vec![(base + i as u32) as u16]);
                            }
                        }
                    }
                }
            }
        }
    }

    Some(map)
}

fn parse_hex(s: &str) -> Option<u32> {
    let s = s.trim_matches(|c| c == '<' || c == '>');
    u32::from_str_radix(s, 16).ok()
}

fn parse_hex_string(s: &str) -> Option<Vec<u16>> {
    let s = s.trim_matches(|c| c == '<' || c == '>');
    if s.is_empty() {
        return Some(Vec::new());
    }
    let mut result = Vec::new();
    for chunk in s.as_bytes().chunks(4) {
        if let Ok(hex_str) = std::str::from_utf8(chunk) {
            if let Ok(val) = u16::from_str_radix(hex_str, 16) {
                result.push(val);
            }
        }
    }
    Some(result)
}

fn get_page_content(doc: &LopdfDocument, page_id: ObjectId) -> Result<Vec<u8>, String> {
    let page_dict = doc
        .get_dictionary(page_id)
        .map_err(|e| format!("Failed to get page: {}", e))?;

    let contents = page_dict
        .get(b"Contents")
        .map_err(|_| "No Contents in page".to_string())?;

    match contents {
        Object::Reference(id) => {
            let stream = doc
                .get_object(*id)
                .map_err(|e| format!("Failed to get content: {}", e))?;
            if let Object::Stream(s) = stream {
                s.decompressed_content()
                    .map_err(|e| format!("Failed to decompress: {}", e))
            } else {
                Err("Content is not a stream".to_string())
            }
        }
        Object::Array(arr) => {
            let mut content = Vec::new();
            for obj in arr {
                if let Object::Reference(id) = obj {
                    if let Ok(Object::Stream(s)) = doc.get_object(*id) {
                        if let Ok(bytes) = s.decompressed_content() {
                            content.extend(bytes);
                            content.push(b'\n');
                        }
                    }
                }
            }
            Ok(content)
        }
        Object::Stream(s) => s
            .decompressed_content()
            .map_err(|e| format!("Failed to decompress: {}", e)),
        _ => Err("Invalid Contents type".to_string()),
    }
}
