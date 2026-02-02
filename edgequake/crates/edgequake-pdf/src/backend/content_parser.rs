//! PDF content stream parsing.
//!
//! This module handles parsing PDF content streams and extracting:
//! - Text elements (Tj, TJ, ' operators)
//! - Line graphics (m, l, re operators)
//! - Graphics state tracking (q, Q, cm, w)
//! - Text positioning (BT, Tm, Td, TD, T*)
//!
//! # WHY a separate content parser?
//!
//! PDF content streams use a complex stack-based operator language. Isolating
//! this parsing logic makes the extraction engine cleaner and allows independent
//! testing of operator handling.

use std::collections::BTreeMap;

use lopdf::content::Content;
use lopdf::Object;
use tracing::debug;

use super::elements::{PdfLine, TextElement};
use super::font_handling::FontInfo;
use crate::error::PdfError;
use crate::Result;

/// Parser for PDF content streams.
///
/// Handles graphics state, text positioning, and element extraction.
pub struct ContentParser {
    // Could add configuration in the future
}

impl ContentParser {
    pub fn new() -> Self {
        Self {}
    }

    /// Extract text and graphical elements from a content stream.
    ///
    /// # Arguments
    /// * `content_bytes` - Raw (decompressed) content stream bytes
    /// * `fonts` - Font dictionary mapping font names to FontInfo
    ///
    /// # Returns
    /// Tuple of (text_elements, line_elements)
    pub fn parse(
        &self,
        content_bytes: &[u8],
        fonts: &BTreeMap<Vec<u8>, FontInfo>,
    ) -> Result<(Vec<TextElement>, Vec<PdfLine>)> {
        let content = Content::decode(content_bytes)
            .map_err(|e| PdfError::PdfParse(format!("Failed to decode content: {}", e)))?;

        let mut text_elements = Vec::new();
        let mut line_elements = Vec::new();

        let mut current_font: Option<&FontInfo> = None;
        let mut current_font_name = String::new();
        let mut font_size: f32 = 12.0;

        // Text matrices
        let mut text_matrix = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0];
        let mut line_matrix = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0];

        // Graphics state
        let mut ctm = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0];
        let mut line_width = 1.0;
        let mut current_point = (0.0, 0.0);
        let mut graphics_stack = Vec::new(); // Stack for q/Q

        for op in &content.operations {
            match op.operator.as_str() {
                // --- Graphics State ---
                "q" => {
                    graphics_stack.push((ctm, line_width));
                }
                "Q" => {
                    if let Some((saved_ctm, saved_width)) = graphics_stack.pop() {
                        ctm = saved_ctm;
                        line_width = saved_width;
                    }
                }
                "cm" => {
                    if op.operands.len() >= 6 {
                        let mut new_matrix = [0.0; 6];
                        for (i, operand) in op.operands.iter().enumerate().take(6) {
                            new_matrix[i] = Self::get_number(operand).unwrap_or(0.0);
                        }
                        // Multiply ctm * new_matrix
                        // [a b 0]   [a' b' 0]
                        // [c d 0] * [c' d' 0]
                        // [e f 1]   [e' f' 1]
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
                "w" => {
                    if let Some(w) = Self::get_number(&op.operands[0]) {
                        // Scale line width by CTM expansion factor (approx)
                        let scale = (ctm[0].abs() + ctm[3].abs()) / 2.0;
                        line_width = w * scale;
                    }
                }

                // --- Path Construction ---
                "m" => {
                    if op.operands.len() >= 2 {
                        let x = Self::get_number(&op.operands[0]).unwrap_or(0.0);
                        let y = Self::get_number(&op.operands[1]).unwrap_or(0.0);
                        // Transform point
                        let tx = x * ctm[0] + y * ctm[2] + ctm[4];
                        let ty = x * ctm[1] + y * ctm[3] + ctm[5];
                        current_point = (tx, ty);
                    }
                }
                "l" => {
                    if op.operands.len() >= 2 {
                        let x = Self::get_number(&op.operands[0]).unwrap_or(0.0);
                        let y = Self::get_number(&op.operands[1]).unwrap_or(0.0);
                        // Transform point
                        let tx = x * ctm[0] + y * ctm[2] + ctm[4];
                        let ty = x * ctm[1] + y * ctm[3] + ctm[5];

                        line_elements.push(PdfLine {
                            p1: current_point,
                            p2: (tx, ty),
                            width: line_width,
                        });
                        current_point = (tx, ty);
                    }
                }
                "re" => {
                    if op.operands.len() >= 4 {
                        let x = Self::get_number(&op.operands[0]).unwrap_or(0.0);
                        let y = Self::get_number(&op.operands[1]).unwrap_or(0.0);
                        let w = Self::get_number(&op.operands[2]).unwrap_or(0.0);
                        let h = Self::get_number(&op.operands[3]).unwrap_or(0.0);

                        // Transform all 4 corners
                        let p1 = (
                            x * ctm[0] + y * ctm[2] + ctm[4],
                            x * ctm[1] + y * ctm[3] + ctm[5],
                        );
                        let p2 = (
                            (x + w) * ctm[0] + y * ctm[2] + ctm[4],
                            (x + w) * ctm[1] + y * ctm[3] + ctm[5],
                        );
                        let p3 = (
                            (x + w) * ctm[0] + (y + h) * ctm[2] + ctm[4],
                            (x + w) * ctm[1] + (y + h) * ctm[3] + ctm[5],
                        );
                        let p4 = (
                            x * ctm[0] + (y + h) * ctm[2] + ctm[4],
                            x * ctm[1] + (y + h) * ctm[3] + ctm[5],
                        );

                        line_elements.push(PdfLine {
                            p1,
                            p2,
                            width: line_width,
                        });
                        line_elements.push(PdfLine {
                            p1: p2,
                            p2: p3,
                            width: line_width,
                        });
                        line_elements.push(PdfLine {
                            p1: p3,
                            p2: p4,
                            width: line_width,
                        });
                        line_elements.push(PdfLine {
                            p1: p4,
                            p2: p1,
                            width: line_width,
                        });
                    }
                }

                // --- Text Objects ---
                // Begin text block - reset matrices
                "BT" => {
                    text_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                    line_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                }
                // Set font: /FontName Size Tf
                "Tf" => {
                    if op.operands.len() >= 2 {
                        if let Object::Name(name) = &op.operands[0] {
                            current_font = fonts.get(name);
                            current_font_name = current_font
                                .map(|f| f.base_font.clone())
                                .unwrap_or_else(|| String::from_utf8_lossy(name).to_string());
                        }
                        if let Some(size) = Self::get_number(&op.operands[1]) {
                            font_size = size.abs();
                        }
                    }
                }
                // Text matrix: a b c d e f Tm
                "Tm" => {
                    if op.operands.len() >= 6 {
                        for (i, operand) in op.operands.iter().enumerate().take(6) {
                            if let Some(v) = Self::get_number(operand) {
                                text_matrix[i] = v;
                            }
                        }
                        line_matrix = text_matrix;
                    }
                }
                // Move text position: tx ty Td
                "Td" => {
                    if op.operands.len() >= 2 {
                        let tx = Self::get_number(&op.operands[0]).unwrap_or(0.0);
                        let ty = Self::get_number(&op.operands[1]).unwrap_or(0.0);
                        line_matrix[4] += tx;
                        line_matrix[5] += ty;
                        text_matrix = line_matrix;
                    }
                }
                // Move text position and set leading: tx ty TD
                "TD" => {
                    if op.operands.len() >= 2 {
                        let tx = Self::get_number(&op.operands[0]).unwrap_or(0.0);
                        let ty = Self::get_number(&op.operands[1]).unwrap_or(0.0);
                        line_matrix[4] += tx;
                        line_matrix[5] += ty;
                        text_matrix = line_matrix;
                    }
                }
                // Move to next line: T*
                "T*" => {
                    // Use default leading (we don't track TL operator)
                    line_matrix[5] -= font_size;
                    text_matrix = line_matrix;
                }
                // Show text: (string) Tj
                "Tj" => {
                    if !op.operands.is_empty() {
                        if let Some(text) = self.decode_text_operand(&op.operands[0], current_font)
                        {
                            let text = text.replace(['\n', '\r'], "");
                            if !text.is_empty() {
                                let (is_bold, is_italic) = current_font
                                    .map(|f| (f.is_bold, f.is_italic))
                                    .unwrap_or((false, false));

                                // Apply CTM transformation to get visual coordinates
                                // visual_pos = CTM * text_pos
                                let raw_x = text_matrix[4];
                                let raw_y = text_matrix[5];
                                let visual_x = ctm[0] * raw_x + ctm[2] * raw_y + ctm[4];
                                let visual_y = ctm[1] * raw_x + ctm[3] * raw_y + ctm[5];

                                text_elements.push(TextElement {
                                    text: text.clone(),
                                    x: visual_x,
                                    y: visual_y,
                                    font_size,
                                    font_name: current_font_name.clone(),
                                    is_bold,
                                    is_italic,
                                });
                                
                                // Advance text matrix by estimated text width.
                                // WHY: PDF text showing operators advance the cursor.
                                // Average char width is ~55% of font size for proportional fonts.
                                // Without this, consecutive text operators appear at same position.
                                let char_count = text.chars().count() as f32;
                                let estimated_width = char_count * font_size * 0.55;
                                text_matrix[4] += estimated_width;
                            }
                        }
                    }
                }
                // Show text with spacing: [...] TJ
                "TJ" => {
                    if !op.operands.is_empty() {
                        if let Object::Array(arr) = &op.operands[0] {
                            let mut combined_text = String::new();
                            // Track total width displacement from TJ array.
                            // Negative values = advance right, positive = move left (in thousandths of em).
                            let mut total_displacement: f32 = 0.0;

                            for item in arr {
                                match item {
                                    Object::String(_, _) => {
                                        if let Some(text) =
                                            self.decode_text_operand(item, current_font)
                                        {
                                            combined_text.push_str(&text);
                                            // Add estimated width of this text run
                                            let char_count = text.chars().count() as f32;
                                            total_displacement += char_count * font_size * 0.55;
                                        }
                                    }
                                    Object::Integer(n) => {
                                        // Negative values move right (kerning adjustment).
                                        // Values in thousandths of em-space.
                                        // Convert to font units: -n/1000 * font_size
                                        let displacement = -*n as f32 / 1000.0 * font_size;
                                        total_displacement += displacement;
                                        
                                        // In TJ arrays, negative kerning values often encode word spaces.
                                        // Be more permissive to avoid missing spaces in real-world PDFs.
                                        if *n < -50 {
                                            combined_text.push(' ');
                                        }
                                    }
                                    Object::Real(n) => {
                                        let displacement = -n / 1000.0 * font_size;
                                        total_displacement += displacement;
                                        
                                        if *n < -50.0 {
                                            combined_text.push(' ');
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            if !combined_text.is_empty() {
                                let (is_bold, is_italic) = current_font
                                    .map(|f| (f.is_bold, f.is_italic))
                                    .unwrap_or((false, false));

                                // Remove CR/LF which can appear in PDF strings
                                let cleaned: String = combined_text
                                    .chars()
                                    .filter(|&c| c != '\n' && c != '\r')
                                    .collect();

                                // Apply CTM transformation to get visual coordinates
                                let raw_x = text_matrix[4];
                                let raw_y = text_matrix[5];
                                let visual_x = ctm[0] * raw_x + ctm[2] * raw_y + ctm[4];
                                let visual_y = ctm[1] * raw_x + ctm[3] * raw_y + ctm[5];

                                text_elements.push(TextElement {
                                    text: cleaned,
                                    x: visual_x,
                                    y: visual_y,
                                    font_size,
                                    font_name: current_font_name.clone(),
                                    is_bold,
                                    is_italic,
                                });
                            }
                            
                            // Advance text matrix by total displacement.
                            // WHY: TJ positions text correctly; we must track the cursor.
                            text_matrix[4] += total_displacement;
                        }
                    }
                }
                // Show text and go to next line: (string) '
                "'" => {
                    line_matrix[5] -= font_size;
                    text_matrix = line_matrix;

                    if !op.operands.is_empty() {
                        if let Some(text) = self.decode_text_operand(&op.operands[0], current_font)
                        {
                            // Remove CR/LF which can appear in PDF strings
                            let cleaned: String =
                                text.chars().filter(|&c| c != '\n' && c != '\r').collect();

                            if !cleaned.is_empty() {
                                let (is_bold, is_italic) = current_font
                                    .map(|f| (f.is_bold, f.is_italic))
                                    .unwrap_or((false, false));

                                // Apply CTM transformation to get visual coordinates
                                let raw_x = text_matrix[4];
                                let raw_y = text_matrix[5];
                                let visual_x = ctm[0] * raw_x + ctm[2] * raw_y + ctm[4];
                                let visual_y = ctm[1] * raw_x + ctm[3] * raw_y + ctm[5];

                                text_elements.push(TextElement {
                                    text: cleaned.clone(),
                                    x: visual_x,
                                    y: visual_y,
                                    font_size,
                                    font_name: current_font_name.clone(),
                                    is_bold,
                                    is_italic,
                                });
                                
                                // Advance text matrix by estimated text width
                                let char_count = cleaned.chars().count() as f32;
                                let estimated_width = char_count * font_size * 0.55;
                                text_matrix[4] += estimated_width;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        debug!(
            "ContentParser: extracted {} text elements, {} line elements",
            text_elements.len(),
            line_elements.len()
        );

        Ok((text_elements, line_elements))
    }

    /// Decode a PDF text operand to a Unicode string.
    ///
    /// Uses the font's encoding (ToUnicode CMap, WinAnsi, etc.) if available.
    /// Falls back to UTF-16BE with BOM detection, then Latin-1.
    fn decode_text_operand(&self, obj: &Object, font: Option<&FontInfo>) -> Option<String> {
        if let Object::String(bytes, _) = obj {
            if let Some(font) = font {
                Some(font.encoding.decode(bytes))
            } else {
                // Fallback: try UTF-16BE then Latin-1
                if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
                    let utf16: Vec<u16> = bytes[2..]
                        .chunks(2)
                        .map(|c| {
                            if c.len() == 2 {
                                u16::from_be_bytes([c[0], c[1]])
                            } else {
                                0xFFFD
                            }
                        })
                        .collect();
                    Some(String::from_utf16_lossy(&utf16))
                } else {
                    Some(bytes.iter().map(|&b| b as char).collect())
                }
            }
        } else {
            None
        }
    }

    /// Extract a number from a PDF object.
    pub fn get_number(obj: &Object) -> Option<f32> {
        match obj {
            Object::Integer(i) => Some(*i as f32),
            Object::Real(f) => Some(*f),
            _ => None,
        }
    }
}

impl Default for ContentParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_parser_creation() {
        let parser = ContentParser::new();
        // Just verify it can be created
        let _ = parser;
    }

    #[test]
    fn test_content_parser_default() {
        let parser = ContentParser::default();
        let _ = parser;
    }

    #[test]
    fn test_get_number_integer() {
        assert_eq!(ContentParser::get_number(&Object::Integer(42)), Some(42.0));
        assert_eq!(
            ContentParser::get_number(&Object::Integer(-10)),
            Some(-10.0)
        );
        assert_eq!(ContentParser::get_number(&Object::Integer(0)), Some(0.0));
    }

    #[test]
    fn test_get_number_real() {
        assert_eq!(ContentParser::get_number(&Object::Real(3.14)), Some(3.14));
        assert_eq!(ContentParser::get_number(&Object::Real(-2.5)), Some(-2.5));
        assert_eq!(ContentParser::get_number(&Object::Real(0.0)), Some(0.0));
    }

    #[test]
    fn test_get_number_non_numeric() {
        assert_eq!(
            ContentParser::get_number(&Object::Name(b"foo".to_vec())),
            None
        );
        assert_eq!(
            ContentParser::get_number(&Object::String(
                b"bar".to_vec(),
                lopdf::StringFormat::Literal
            )),
            None
        );
        assert_eq!(ContentParser::get_number(&Object::Boolean(true)), None);
        assert_eq!(ContentParser::get_number(&Object::Null), None);
    }

    #[test]
    fn test_empty_content() {
        let parser = ContentParser::new();
        let fonts = BTreeMap::new();
        let result = parser.parse(b"", &fonts);
        assert!(result.is_ok());
        let (text, lines) = result.unwrap();
        assert!(text.is_empty());
        assert!(lines.is_empty());
    }

    #[test]
    fn test_parse_simple_text_operator() {
        let parser = ContentParser::new();
        let fonts = BTreeMap::new();
        // Simple content stream: BT (Hello) Tj ET
        let content = b"BT (Hello) Tj ET";
        let result = parser.parse(content, &fonts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_line_operators() {
        let parser = ContentParser::new();
        let fonts = BTreeMap::new();
        // Simple line: move to and line to
        let content = b"100 200 m 300 400 l S";
        let result = parser.parse(content, &fonts);
        assert!(result.is_ok());
        let (_, lines) = result.unwrap();
        // Should have at least one line
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_parse_rectangle_operator() {
        let parser = ContentParser::new();
        let fonts = BTreeMap::new();
        // Rectangle operator: x y width height re
        let content = b"50 50 100 100 re S";
        let result = parser.parse(content, &fonts);
        assert!(result.is_ok());
        let (_, lines) = result.unwrap();
        // Rectangle should produce 4 lines
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn test_parse_graphics_state_save_restore() {
        let parser = ContentParser::new();
        let fonts = BTreeMap::new();
        // Save and restore graphics state
        let content = b"q 2 w 100 200 m 300 400 l S Q 100 200 m 300 400 l S";
        let result = parser.parse(content, &fonts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_line_width_operator() {
        let parser = ContentParser::new();
        let fonts = BTreeMap::new();
        // Set line width and draw
        let content = b"3 w 100 200 m 300 200 l S";
        let result = parser.parse(content, &fonts);
        assert!(result.is_ok());
        let (_, lines) = result.unwrap();
        assert!(!lines.is_empty());
        // First line should have width 3.0
        assert_eq!(lines[0].width, 3.0);
    }

    #[test]
    fn test_parse_ctm_transform() {
        let parser = ContentParser::new();
        let fonts = BTreeMap::new();
        // Apply CTM transformation
        let content = b"1 0 0 1 50 50 cm 100 200 m 300 400 l S";
        let result = parser.parse(content, &fonts);
        assert!(result.is_ok());
    }
}
