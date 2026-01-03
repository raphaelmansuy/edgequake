//! SOTA PDF extraction backend using lopdf with proper character encoding.
//!
//! This module provides production-quality PDF text extraction with:
//! - Proper font encoding support (WinAnsi, ToUnicode CMap, etc.)
//! - Font size tracking for header detection
//! - Bold/italic detection from font names
//! - Section pattern detection
//! - Running header removal

#![cfg(feature = "lopdf")]

use async_trait::async_trait;
use std::collections::BTreeMap;
use tracing::{debug, info, warn};

use super::PdfBackend;
use crate::config::PdfConfig;
use crate::error::PdfError;
use crate::extractor::PdfInfo;
use crate::schema::{
    Block, BlockId, BlockType, BoundingBox, Document, ExtractionMethod, FontStyle, Page, PageStats,
    Point, TextSpan,
};
use crate::{DocumentMetadata, Result};

use lopdf::content::Content;
use lopdf::{Dictionary, Document as LopdfDocument, Object, ObjectId, Stream};

// Import encoding types from the extracted encodings module
use super::encodings;
use super::encodings::Encoding;

/// Information about a font in the PDF
#[derive(Debug)]
struct FontInfo {
    /// Base font name (e.g., "Helvetica-Bold")
    base_font: String,
    /// Font encoding for text decoding
    encoding: Encoding,
    /// Detected font size from usage
    #[allow(dead_code)]
    size: f32,
    /// Is this font bold?
    is_bold: bool,
    /// Is this font italic?
    is_italic: bool,
}

impl FontInfo {
    fn from_dict(doc: &LopdfDocument, font_dict: &Dictionary) -> Self {
        // Get base font name
        let base_font = font_dict
            .get(b"BaseFont")
            .ok()
            .and_then(|obj| obj.as_name().ok())
            .map(|n| String::from_utf8_lossy(n).to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        // Detect bold/italic from font name
        let lower_name = base_font.to_lowercase();
        // Check for common bold indicators:
        // - "bold", "black", "heavy" in name
        // - "sfbx" (SF Bold Extended) in arXiv/LaTeX fonts like TFFXIV+SFBX1200
        // - "cmbx" (Computer Modern Bold Extended)
        // - "-bold" suffix
        let is_bold = lower_name.contains("bold")
            || lower_name.contains("black")
            || lower_name.contains("heavy")
            || lower_name.contains("sfbx")   // SF Bold Extended (arXiv/LaTeX) - TFFXIV+SFBX1200
            || lower_name.contains("cmbx")   // Computer Modern Bold Extended
            || lower_name.contains("-bold");

        let is_italic = lower_name.contains("italic") 
            || lower_name.contains("oblique")
            || lower_name.contains("sfti")   // SF Text Italic - e.g., TXAXLJ+SFTI0900
            || lower_name.contains("cmti")   // Computer Modern Text Italic
            || lower_name.contains("cmmi")   // Computer Modern Math Italic
            || lower_name.contains("cmmib")  // Computer Modern Math Italic Bold
            || lower_name.contains("-italic");

        // Get encoding
        let encoding = Self::get_encoding(doc, font_dict);

        FontInfo {
            base_font,
            encoding,
            size: 12.0,
            is_bold,
            is_italic,
        }
    }

    fn get_encoding(doc: &LopdfDocument, font_dict: &Dictionary) -> Encoding {
        // Check for ToUnicode CMap first (most reliable)
        if let Ok(to_unicode) = font_dict.get(b"ToUnicode") {
            if let Some(stream) = Self::resolve_stream(doc, to_unicode) {
                if let Ok(data) = stream.decompressed_content() {
                    let cmap = encodings::ToUnicodeMap::parse(&data);
                    return Encoding::ToUnicodeMap(cmap);
                }
            }
        }

        // Check Encoding entry
        if let Ok(enc) = font_dict.get(b"Encoding") {
            match enc {
                Object::Name(name) => {
                    let name_str = String::from_utf8_lossy(name);
                    return match name_str.as_ref() {
                        "WinAnsiEncoding" => {
                            Encoding::OneByteEncoding(&encodings::WIN_ANSI_ENCODING)
                        }
                        "MacRomanEncoding" => {
                            Encoding::OneByteEncoding(&encodings::MAC_ROMAN_ENCODING)
                        }
                        "StandardEncoding" => {
                            Encoding::OneByteEncoding(&encodings::STANDARD_ENCODING)
                        }
                        "Identity-H" | "Identity-V" => Encoding::Identity,
                        _ => Encoding::OneByteEncoding(&encodings::WIN_ANSI_ENCODING),
                    };
                }
                Object::Reference(id) => {
                    // Encoding dictionary - check for BaseEncoding
                    if let Ok(enc_dict) = doc.get_dictionary(*id) {
                        if let Ok(base) = enc_dict.get(b"BaseEncoding") {
                            if let Ok(name) = base.as_name() {
                                let name_str = String::from_utf8_lossy(name);
                                return match name_str.as_ref() {
                                    "WinAnsiEncoding" => {
                                        Encoding::OneByteEncoding(&encodings::WIN_ANSI_ENCODING)
                                    }
                                    "MacRomanEncoding" => {
                                        Encoding::OneByteEncoding(&encodings::MAC_ROMAN_ENCODING)
                                    }
                                    _ => Encoding::OneByteEncoding(&encodings::STANDARD_ENCODING),
                                };
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Default to WinAnsi for Type1 and TrueType fonts
        Encoding::OneByteEncoding(&encodings::WIN_ANSI_ENCODING)
    }

    fn resolve_stream<'a>(doc: &'a LopdfDocument, obj: &'a Object) -> Option<&'a Stream> {
        match obj {
            Object::Reference(id) => doc.get_object(*id).ok()?.as_stream().ok(),
            Object::Stream(s) => Some(s),
            _ => None,
        }
    }
}

use super::elements::{PdfLine, TextElement};
use super::lattice::LatticeEngine;

/// SOTA PDF backend with proper encoding support
pub struct SotaBackend {
    config: PdfConfig,
    lattice_engine: LatticeEngine,
}

#[derive(Debug, Clone)]
struct MergedLine {
    text: String,
    avg_font_size: f32,
    spans: Vec<TextSpan>,
}

impl SotaBackend {
    pub fn new() -> Self {
        Self::with_config(PdfConfig::default())
    }

    pub fn with_config(config: PdfConfig) -> Self {
        Self {
            config,
            lattice_engine: LatticeEngine::new(),
        }
    }

    /// Get fonts from page resources
    fn get_page_fonts(
        &self,
        doc: &LopdfDocument,
        page_id: ObjectId,
    ) -> Result<BTreeMap<Vec<u8>, FontInfo>> {
        let mut fonts = BTreeMap::new();

        let page_dict = doc
            .get_dictionary(page_id)
            .map_err(|e| PdfError::PdfParse(format!("Failed to get page: {}", e)))?;

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
                        fonts.insert(name.clone(), FontInfo::from_dict(doc, font));
                    }
                }
            }
        }

        Ok(fonts)
    }

    /// Get page content stream
    fn get_page_content(&self, doc: &LopdfDocument, page_id: ObjectId) -> Result<Vec<u8>> {
        let page_dict = doc
            .get_dictionary(page_id)
            .map_err(|e| PdfError::PdfParse(format!("Failed to get page: {}", e)))?;

        let contents = page_dict
            .get(b"Contents")
            .map_err(|_| PdfError::PdfParse("No Contents in page".to_string()))?;

        match contents {
            Object::Reference(id) => {
                let stream = doc
                    .get_object(*id)
                    .map_err(|e| PdfError::PdfParse(format!("Failed to get content: {}", e)))?;
                if let Object::Stream(s) = stream {
                    s.decompressed_content()
                        .map_err(|e| PdfError::PdfParse(format!("Failed to decompress: {}", e)))
                } else {
                    Err(PdfError::PdfParse("Content is not a stream".to_string()))
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
                .map_err(|e| PdfError::PdfParse(format!("Failed to decompress: {}", e))),
            _ => Err(PdfError::PdfParse("Invalid Contents type".to_string())),
        }
    }

    /// Extract text and graphical elements from content stream
    fn extract_page_elements(
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
                        for i in 0..6 {
                            new_matrix[i] = Self::get_number(&op.operands[i]).unwrap_or(0.0);
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
                        for i in 0..6 {
                            if let Some(v) = Self::get_number(&op.operands[i]) {
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
                            let text = text.replace('\n', "").replace('\r', "");
                            if !text.is_empty() {
                                let (is_bold, is_italic) = current_font
                                    .map(|f| (f.is_bold, f.is_italic))
                                    .unwrap_or((false, false));

                                text_elements.push(TextElement {
                                    text,
                                    x: text_matrix[4],
                                    y: text_matrix[5],
                                    font_size,
                                    font_name: current_font_name.clone(),
                                    is_bold,
                                    is_italic,
                                });
                            }
                        }
                    }
                }
                // Show text with spacing: [...] TJ
                "TJ" => {
                    if !op.operands.is_empty() {
                        if let Object::Array(arr) = &op.operands[0] {
                            let mut combined_text = String::new();

                            for item in arr {
                                match item {
                                    Object::String(_, _) => {
                                        if let Some(text) =
                                            self.decode_text_operand(item, current_font)
                                        {
                                            combined_text.push_str(&text);
                                        }
                                    }
                                    Object::Integer(n) => {
                                        // In TJ arrays, negative kerning values often encode word spaces.
                                        // Be more permissive to avoid missing spaces in real-world PDFs.
                                        if *n < -50 {
                                            combined_text.push(' ');
                                        }
                                    }
                                    Object::Real(n) => {
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

                                text_elements.push(TextElement {
                                    text: cleaned,
                                    x: text_matrix[4],
                                    y: text_matrix[5],
                                    font_size,
                                    font_name: current_font_name.clone(),
                                    is_bold,
                                    is_italic,
                                });
                            }
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

                                text_elements.push(TextElement {
                                    text: cleaned,
                                    x: text_matrix[4],
                                    y: text_matrix[5],
                                    font_size,
                                    font_name: current_font_name.clone(),
                                    is_bold,
                                    is_italic,
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok((text_elements, line_elements))
    }

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

    fn get_number(obj: &Object) -> Option<f32> {
        match obj {
            Object::Integer(i) => Some(*i as f32),
            Object::Real(f) => Some(*f),
            _ => None,
        }
    }

    /// Deduplicate text elements that are identical and at the same position.
    ///
    /// **WHY deduplication is critical:**
    /// - PDF files often contain invisible text layers (e.g., OCR layer + visible layer)
    /// - Without dedup, we get doubled text like "TheThe ProblemProblem"
    /// - Position tolerance of 2pt handles slight rendering variations
    /// - Keep element with more text if one is prefix of another (OCR sometimes partial)
    fn deduplicate_elements(&self, elements: Vec<TextElement>) -> Vec<TextElement> {
        if elements.is_empty() {
            return Vec::new();
        }

        // Sort by Y (descending), then X (ascending)
        let mut sorted = elements;
        sorted.sort_by(|a, b| {
            b.y.partial_cmp(&a.y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
        });

        let mut unique = Vec::new();
        unique.push(sorted[0].clone());

        for elem in sorted.into_iter().skip(1) {
            let prev = unique.last().unwrap();

            // Check for overlap
            let same_pos = (elem.x - prev.x).abs() < 2.0 && (elem.y - prev.y).abs() < 2.0;

            if same_pos {
                // If text is identical, skip
                if elem.text == prev.text {
                    continue;
                }
                // If one contains the other, keep the longer one
                if elem.text.contains(&prev.text) {
                    unique.pop(); // Remove shorter prev
                    unique.push(elem);
                    continue;
                }
                if prev.text.contains(&elem.text) {
                    continue; // Skip shorter elem
                }
            }

            unique.push(elem);
        }

        unique
    }

    /// Merge text elements that are physically adjacent on the same line.
    ///
    /// **WHY merging is essential:**
    /// - PDF operators (Tj, TJ) emit individual words or even characters
    /// - "Hello World" might come as ["Hello", " ", "World"] at different positions
    /// - Merge threshold uses font size to estimate character width
    /// - Result: contiguous text runs for proper word/sentence extraction
    fn merge_text_elements(&self, elements: Vec<TextElement>) -> Vec<TextElement> {
        if elements.is_empty() {
            return Vec::new();
        }

        // Sort by Y (descending), then X (ascending)
        let mut sorted = elements;
        sorted.sort_by(|a, b| {
            b.y.partial_cmp(&a.y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
        });

        let mut merged = Vec::new();
        let mut current = sorted[0].clone();

        for next in sorted.into_iter().skip(1) {
            // Check if on same line
            if (next.y - current.y).abs() < 2.0 {
                // Check horizontal distance
                // Use font size from current element to estimate char width
                let char_width = if current.font_size > 0.0 {
                    current.font_size * 0.4 // Conservative estimate
                } else {
                    4.0
                };

                let current_width = current.text.len() as f32 * char_width;
                let current_end = current.x + current_width;
                let gap = next.x - current_end;

                // If gap is small (e.g. < 2 chars), merge
                // Allow slight negative gap (overlap) due to kerning
                if gap > -char_width && gap < char_width * 2.5 {
                    // Merge!
                    // Add space if gap is significant (> 0.3 char width)
                    if gap > char_width * 0.3 {
                        current.text.push(' ');
                    }
                    current.text.push_str(&next.text);
                    continue;
                }
            }

            // Push current and start new
            merged.push(current);
            current = next;
        }
        merged.push(current);

        merged
    }

    // ============================================================================
    // SOTA Column Detection using Vertical Projection Histograms (XY-Cut approach)
    // Based on spec_algo.md: Enhanced XY-Cut with Adaptive Thresholds
    // ============================================================================

    /// Compute vertical projection histogram from text elements
    /// Returns a vector where each bin contains the density (count) of elements
    fn compute_vertical_projection(
        &self,
        elements: &[TextElement],
        page_width: f32,
        bin_size: f32,
    ) -> Vec<usize> {
        let num_bins = (page_width / bin_size).ceil() as usize;
        let mut proj = vec![0; num_bins];

        for elem in elements {
            // Count each element's contribution to bins it spans
            let start_bin = (elem.x / bin_size) as usize;
            let end_bin = ((elem.x + 20.0) / bin_size) as usize; // Approximate text width
            for i in start_bin..=end_bin.min(num_bins - 1) {
                proj[i] += 1;
            }
        }
        proj
    }

    /// Find gaps (valleys) in the projection histogram
    /// Returns midpoint positions of significant gaps
    fn find_projection_gaps(&self, proj: &[usize], bin_size: f32, min_gap_bins: usize) -> Vec<f32> {
        let mut gaps = Vec::new();
        let mut low_start: Option<usize> = None;

        // Calculate adaptive threshold based on content distribution
        // This is a first-principles approach that adapts to document density
        let total: usize = proj.iter().sum();
        let _avg_density = if proj.is_empty() {
            0
        } else {
            total / proj.len()
        };

        // Use 20th percentile instead of fixed 20% of average
        // This adapts to skewed distributions better
        let mut sorted_proj = proj.to_vec();
        sorted_proj.sort();
        let percentile_idx = (sorted_proj.len() as f32 * 0.20) as usize;
        let low_threshold = sorted_proj.get(percentile_idx).copied().unwrap_or(0);

        for (i, &count) in proj.iter().enumerate() {
            if count <= low_threshold {
                // Low density region
                if low_start.is_none() {
                    low_start = Some(i);
                }
            } else if let Some(start) = low_start {
                // End of low density region
                let gap_width = i - start;
                if gap_width >= min_gap_bins {
                    // Significant gap found - record midpoint
                    let midpoint = ((start + i) as f32 / 2.0) * bin_size;
                    gaps.push(midpoint);
                    debug!("Found gap at X={:.1} (width={} bins)", midpoint, gap_width);
                }
                low_start = None;
            }
        }
        gaps
    }

    /// Detect if page has two-column layout using projection histogram.
    /// Returns Some(column_boundary_x) if two-column layout detected, None otherwise.
    ///
    /// **WHY projection histogram approach:**
    /// - Academic papers commonly use two-column layout with a "gutter" in the middle
    /// - Vertical projection histograms count text elements per horizontal bin
    /// - A gap in the histogram (near page center) indicates the column separator
    /// - This is more robust than x-position clustering for varying column widths
    fn detect_columns(&self, elements: &[TextElement], page_width: f32) -> Option<f32> {
        if elements.len() < 10 {
            return None;
        }

        // Use projection histogram approach from spec_algo.md
        let bin_size = 5.0; // 5pt bins for fine granularity
        let proj = self.compute_vertical_projection(elements, page_width, bin_size);

        // Find gaps - minimum gap of 4 bins (20pt) for column separator
        let gaps = self.find_projection_gaps(&proj, bin_size, 4);
        debug!("Projection gaps found: {:?}", gaps);

        // Look for a gap near the center of the page (column boundary)
        // In academic papers, the gutter is typically around 45-55% of page width
        let center = page_width / 2.0;
        let center_range = page_width * 0.15; // ±15% from center

        let center_gap = gaps
            .iter()
            .find(|&&gap| (gap - center).abs() < center_range);

        if let Some(&boundary) = center_gap {
            // Verify with element distribution
            // The gap position is the START of the gap (whitespace column).
            // Text in the right column starts AFTER the gap, not at the gap.
            // Use asymmetric thresholds: elements ending before gap = left, elements starting after gap = right
            // For now, use boundary as the rough separation point with wider margin

            // Find elements that are clearly in left column (well before boundary)
            // and elements that are in right column (at or after boundary)
            // A gap at X means content is sparse there - left column ends before X, right column starts at or after X
            let left_count = elements.iter().filter(|e| e.x < boundary).count();
            let right_count = elements.iter().filter(|e| e.x >= boundary).count();

            // Both columns should have significant content and be somewhat balanced
            let balance = if left_count > right_count {
                right_count as f32 / left_count as f32
            } else {
                left_count as f32 / right_count as f32
            };

            debug!(
                "Projection gap at X={:.1}: left={}, right={}, balance={:.2}",
                boundary, left_count, right_count, balance
            );

            if left_count >= 5 && right_count >= 5 && balance > 0.25 {
                debug!(
                    "Detected TWO-COLUMN layout with boundary at {:.1}",
                    boundary
                );
                return Some(boundary);
            }
        }

        // If global check failed, try checking only the bottom portion of the page
        // This handles pages with full-width headers/abstracts but two-column body
        // Use adaptive threshold based on content distribution instead of fixed 75%
        let max_y = elements
            .iter()
            .map(|e| e.y)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = elements.iter().map(|e| e.y).fold(f32::INFINITY, f32::min);
        let page_height_content = max_y - min_y;

        // Only try this if we have enough vertical content
        if page_height_content > 200.0 {
            // Calculate adaptive threshold based on content density
            // Use 20th percentile of y-coordinates to find natural content boundary
            let mut y_coords: Vec<f32> = elements.iter().map(|e| e.y).collect();
            y_coords.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let percentile_idx = (y_coords.len() as f32 * 0.20) as usize;
            let threshold_y = y_coords
                .get(percentile_idx)
                .copied()
                .unwrap_or(max_y - page_height_content * 0.25);
            let bottom_elements: Vec<TextElement> = elements
                .iter()
                .filter(|e| e.y < threshold_y)
                .cloned()
                .collect();

            if bottom_elements.len() > 20 {
                let proj_bottom =
                    self.compute_vertical_projection(&bottom_elements, page_width, bin_size);
                let gaps_bottom = self.find_projection_gaps(&proj_bottom, bin_size, 4);

                let center_gap_bottom = gaps_bottom
                    .iter()
                    .find(|&&gap| (gap - center).abs() < center_range);

                if let Some(&boundary) = center_gap_bottom {
                    // Verify with bottom element distribution
                    let left_count = bottom_elements
                        .iter()
                        .filter(|e| e.x < boundary - 10.0)
                        .count();
                    let right_count = bottom_elements
                        .iter()
                        .filter(|e| e.x > boundary + 10.0)
                        .count();

                    let balance = if left_count > right_count {
                        right_count as f32 / left_count as f32
                    } else {
                        left_count as f32 / right_count as f32
                    };

                    debug!(
                        "Bottom-only Projection gap at X={:.1}: left={}, right={}, balance={:.2}",
                        boundary, left_count, right_count, balance
                    );

                    if left_count >= 5 && right_count >= 5 && balance > 0.25 {
                        debug!(
                            "Detected TWO-COLUMN layout (bottom-only) with boundary at {:.1}",
                            boundary
                        );
                        return Some(boundary);
                    }
                }
            }
        }

        // Fallback: simple zone-based detection for papers with unusual layout
        let column_boundary = page_width * 0.49;
        let left_zone_end = page_width * 0.45;
        let right_zone_start = page_width * 0.50;

        let mut left_starts = 0;
        let mut right_starts = 0;

        for elem in elements {
            if elem.x < left_zone_end {
                left_starts += 1;
            } else if elem.x > right_zone_start {
                right_starts += 1;
            }
        }

        let balance_ratio = if left_starts > 0 && right_starts > 0 {
            let (min_col, max_col) = if left_starts < right_starts {
                (left_starts, right_starts)
            } else {
                (right_starts, left_starts)
            };
            min_col as f32 / max_col as f32
        } else {
            0.0
        };

        debug!(
            "Column fallback detection: left_starts={}, right_starts={}, balance={:.2}",
            left_starts, right_starts, balance_ratio
        );

        // Two-column layout if:
        // 1. Most elements start clearly in left or right zones
        // 2. Both columns have significant content
        // 3. Columns are somewhat balanced
        if left_starts >= 5 && right_starts >= 5 && balance_ratio > 0.3 {
            debug!(
                "Detected TWO-COLUMN layout with boundary at {:.1}",
                column_boundary
            );
            Some(column_boundary)
        } else {
            debug!("Detected SINGLE-COLUMN layout");
            None
        }
    }

    /// Group text elements into lines with proper column handling
    /// For two-column layouts: reads left column top-to-bottom, then right column
    /// Returns (lines, detected_columns) where detected_columns are BoundingBoxes for each column
    fn group_into_lines(
        &self,
        elements: Vec<TextElement>,
        page_width: f32,
        page_height: f32,
    ) -> (Vec<Vec<TextElement>>, Vec<BoundingBox>) {
        if elements.is_empty() {
            return (Vec::new(), Vec::new());
        }

        // First, detect if this is a two-column layout
        let column_boundary = self.detect_columns(&elements, page_width);

        // Sort by Y descending (higher Y = top of page in PDF coordinates)
        // This puts content that appears at the top of the page first
        let mut elements = elements;
        elements.sort_by(|a, b| b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal));

        // If two-column layout detected, separate columns first
        if let Some(boundary) = column_boundary {
            let lines = self.group_two_column_layout(elements, boundary, page_width);
            // Create column bounding boxes
            let left_column = BoundingBox::new(0.0, 0.0, boundary, page_height);
            let right_column = BoundingBox::new(boundary, 0.0, page_width, page_height);
            return (lines, vec![left_column, right_column]);
        }

        // Single-column layout: group into Y-bands
        let lines = self.group_single_column_layout(elements);
        (lines, Vec::new())
    }

    /// Handle two-column layout: separate left and right columns, then process each
    /// Uses footer filtering and handles spanning elements
    fn group_two_column_layout(
        &self,
        elements: Vec<TextElement>,
        column_boundary: f32,
        _page_width: f32,
    ) -> Vec<Vec<TextElement>> {
        let mut left_column: Vec<TextElement> = Vec::new();
        let mut right_column: Vec<TextElement> = Vec::new();
        let mut spanning_elements: Vec<TextElement> = Vec::new();
        let mut footer_elements: Vec<TextElement> = Vec::new();

        // Calculate adaptive thresholds based on actual content distribution
        // This is a first-principles approach that adapts to different document layouts
        let (
            footer_threshold,
            header_threshold,
            title_threshold,
            affiliation_threshold,
            large_font_threshold,
        ) = self.calculate_adaptive_region_thresholds(&elements);

        // Margin around column boundary for classification
        let margin = 15.0;

        for elem in elements {
            // Skip very small elements (likely artifacts)
            if elem.text.trim().is_empty() {
                continue;
            }

            // Check if element is in footer region
            let is_footer = elem.y < footer_threshold;

            // Check if element is in header region (running header)
            let is_header = elem.y > header_threshold && elem.font_size < large_font_threshold;

            // Check if element is affiliation/metadata (between body and footer)
            // These include: university names, emails, conference submission lines
            let is_affiliation_zone = elem.y < affiliation_threshold && elem.y >= footer_threshold;
            let looks_like_affiliation = elem.text.contains('@')
                || elem.text.contains("University")
                || elem.text.contains("School of")
                || elem.text.contains("Department")
                || elem.text.contains("Correspondence")
                || elem.text.contains("Submitted to")
                || elem.text.contains("Conference")
                || elem.text.starts_with("1") && elem.text.len() < 5  // Affiliation numbers
                || elem.text.starts_with("2") && elem.text.len() < 5;
            let is_affiliation = is_affiliation_zone || looks_like_affiliation;

            // Handle spanning elements (titles):
            // - In title zone (near top of page)
            // - Larger font size (typically > 11pt for titles)
            // - Not a header/footer
            let is_title_zone = elem.y > title_threshold;
            let is_large_font = elem.font_size > large_font_threshold;
            let is_spanning = is_title_zone && is_large_font && !is_footer && !is_header;

            if is_spanning {
                // Spanning elements go to beginning (will be processed first)
                spanning_elements.push(elem);
            } else if is_footer || is_header || is_affiliation {
                // Footer/header/affiliation: add to separate collection (will appear at end)
                debug!(
                    "Footer/affiliation element: Y={:.1} X={:.1} affil={} '{}'",
                    elem.y,
                    elem.x,
                    is_affiliation,
                    &elem.text[..elem.text.len().min(40)]
                );
                footer_elements.push(elem);
            } else if elem.x < column_boundary - margin {
                // Clearly in left column
                left_column.push(elem);
            } else if elem.x > column_boundary + margin {
                // Clearly in right column
                right_column.push(elem);
            } else {
                // In the gap - use element width to decide
                // If it's a continuation of left column text, it belongs to left
                // Short elements in gap likely belong to whichever column has more content at this Y
                if elem.text.starts_with(|c: char| c.is_lowercase()) {
                    // Starts with lowercase = likely continuation
                    left_column.push(elem);
                } else {
                    right_column.push(elem);
                }
            }
        }

        debug!(
            "Two-column separation: spanning={}, left={}, right={}, footer={}",
            spanning_elements.len(),
            left_column.len(),
            right_column.len(),
            footer_elements.len()
        );

        // Process spanning elements first (titles, etc.)
        let spanning_lines = self.group_single_column_layout(spanning_elements);

        // Process each column into lines
        let left_lines = self.group_single_column_layout(left_column);
        let right_lines = self.group_single_column_layout(right_column);

        // Process footer/header elements last
        let footer_lines = self.group_single_column_layout(footer_elements);

        debug!(
            "Grouped: spanning={}, left={} lines, right={} lines, footer={} lines",
            spanning_lines.len(),
            left_lines.len(),
            right_lines.len(),
            footer_lines.len()
        );

        // Log first few lines of each section
        for (i, line) in spanning_lines.iter().take(2).enumerate() {
            let text: String = line
                .iter()
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join("");
            debug!("Spanning line {}: '{}'", i, &text[..text.len().min(50)]);
        }
        for (i, line) in left_lines.iter().take(3).enumerate() {
            let text: String = line
                .iter()
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join("");
            debug!("Left line {}: '{}'", i, &text[..text.len().min(50)]);
        }
        for (i, line) in right_lines.iter().take(3).enumerate() {
            let text: String = line
                .iter()
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join("");
            debug!("Right line {}: '{}'", i, &text[..text.len().min(50)]);
        }

        // Combine: spanning elements first, then left column, then right column, then footer
        // This ensures titles/headers appear before column content
        let mut result = Vec::new();
        result.extend(spanning_lines);

        // Before adding left/right columns, detect and move isolated bottom content to footer
        // This handles affiliations, figure captions that are below the main column content
        let (left_main, left_bottom) = self.split_by_vertical_gap(left_lines, 30.0);
        let (right_main, right_bottom) = self.split_by_vertical_gap(right_lines, 30.0);

        result.extend(left_main);
        result.extend(right_main);
        result.extend(footer_lines); // Footer at the end
        result.extend(left_bottom); // Bottom content after footer
        result.extend(right_bottom); // Bottom content after footer

        result
    }

    /// Split lines into main content and bottom-isolated content.
    /// If there's a vertical gap > threshold between content regions, the lower content is separated.
    fn split_by_vertical_gap(
        &self,
        lines: Vec<Vec<TextElement>>,
        gap_threshold: f32,
    ) -> (Vec<Vec<TextElement>>, Vec<Vec<TextElement>>) {
        if lines.len() < 2 {
            return (lines, Vec::new());
        }

        // Find lines' Y positions (use first element's Y as representative)
        let y_positions: Vec<f32> = lines
            .iter()
            .map(|line| line.first().map(|e| e.y).unwrap_or(0.0))
            .collect();

        // Find largest gap in Y (remember: sorted by Y descending, so gaps are when Y suddenly drops more)
        let mut max_gap = 0.0f32;
        let mut split_idx = lines.len();

        for i in 1..y_positions.len() {
            let gap = y_positions[i - 1] - y_positions[i]; // Previous Y minus current Y (should be positive)
            if gap > max_gap && gap > gap_threshold {
                max_gap = gap;
                split_idx = i;
            }
        }

        if max_gap > gap_threshold {
            debug!(
                "Found vertical gap of {:.1}pt at line {}, splitting column content",
                max_gap, split_idx
            );
            let (main, bottom) = lines.split_at(split_idx);
            (main.to_vec(), bottom.to_vec())
        } else {
            (lines, Vec::new())
        }
    }

    /// Group elements into lines for single-column layout
    fn group_single_column_layout(&self, mut elements: Vec<TextElement>) -> Vec<Vec<TextElement>> {
        if elements.is_empty() {
            return Vec::new();
        }

        // Sort by Y descending (higher Y = top of page in PDF coordinates)
        elements.sort_by(|a, b| b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal));

        // Group into Y-bands
        let mut lines: Vec<Vec<TextElement>> = Vec::new();
        let mut current_line: Vec<TextElement> = Vec::new();
        let mut current_y: Option<f32> = None;

        for elem in elements {
            let y_tolerance = elem.font_size * 0.5;

            if let Some(y) = current_y {
                if (elem.y - y).abs() > y_tolerance {
                    // New line - save current and start new
                    if !current_line.is_empty() {
                        current_line.sort_by(|a, b| {
                            a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
                        });
                        lines.push(std::mem::take(&mut current_line));
                    }
                    current_y = Some(elem.y);
                }
            } else {
                current_y = Some(elem.y);
            }
            current_line.push(elem);
        }

        if !current_line.is_empty() {
            current_line.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
            lines.push(current_line);
        }

        lines
    }

    /// Merge line elements into text with proper spacing while preserving style runs as spans.
    fn merge_line(&self, elements: &[TextElement]) -> MergedLine {
        if elements.is_empty() {
            return MergedLine {
                text: String::new(),
                avg_font_size: 12.0,
                spans: Vec::new(),
            };
        }

        let avg_font_size =
            elements.iter().map(|e| e.font_size).sum::<f32>() / elements.len() as f32;

        // Estimate average character width.
        // We bias toward inserting spaces (missing spaces are worse than extra spaces).
        // Using a low threshold (0.3x) to be more aggressive about space insertion.
        // Post-processing can clean up extra spaces, but missing spaces cause word concatenation.
        let avg_char_width = avg_font_size * 0.5;
        let space_threshold = avg_char_width * 0.3;

        let mut text = String::new();
        let mut spans: Vec<TextSpan> = Vec::new();

        let push_to_spans = |spans: &mut Vec<TextSpan>, chunk: &str, style: FontStyle| {
            if chunk.is_empty() {
                return;
            }
            if let Some(last) = spans.last_mut() {
                if last.style == style {
                    last.text.push_str(chunk);
                    return;
                }
            }
            spans.push(TextSpan {
                text: chunk.to_string(),
                bbox: None,
                style,
            });
        };

        for (i, elem) in elements.iter().enumerate() {
            if i > 0 {
                let prev = &elements[i - 1];
                // Estimate previous element's end position using its own font size and Unicode-safe length.
                let prev_char_width = prev.font_size * 0.5;
                let prev_len = prev.text.chars().count() as f32;
                let prev_end = prev.x + (prev_len * prev_char_width);
                let gap = elem.x - prev_end;

                // Avoid inserting spaces before punctuation.
                let starts_with_punct = elem
                    .text
                    .chars()
                    .next()
                    .map(|c| matches!(c, ',' | '.' | ':' | ';' | ')' | ']' | '}' | '?' | '!'))
                    .unwrap_or(false);

                if gap > space_threshold && !starts_with_punct {
                    text.push(' ');
                    if let Some(last) = spans.last_mut() {
                        last.text.push(' ');
                    } else {
                        spans.push(TextSpan::plain(" "));
                    }
                }
            }

            text.push_str(&elem.text);
            // FontStyle with weight and italic are used - this flows to output correctly!
            let style = FontStyle {
                family: Some(elem.font_name.clone()),
                size: Some(elem.font_size),
                weight: Some(if elem.is_bold { 700 } else { 400 }),
                italic: elem.is_italic,
                ..Default::default()
            };
            push_to_spans(&mut spans, &elem.text, style);
        }

        MergedLine {
            text,
            avg_font_size,
            spans,
        }
    }

    /// Convert lines to blocks with type detection
    fn lines_to_blocks(
        &self,
        lines: Vec<Vec<TextElement>>,
        page_width: f32,
        _page_height: f32,
    ) -> Vec<Block> {
        let mut blocks = Vec::new();

        // Debug: Log first 10 lines being processed
        debug!("Converting {} lines to blocks", lines.len());
        for (i, line) in lines.iter().take(10).enumerate() {
            let text: String = line
                .iter()
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join("");
            let y = line.first().map(|e| e.y).unwrap_or(0.0);
            let x = line.first().map(|e| e.x).unwrap_or(0.0);
            let preview: String = text.chars().take(40).collect();
            debug!(
                "  Block input line {}: Y={:.1} X={:.1} '{}'",
                i, y, x, preview
            );
        }

        // Calculate body font size (most common)
        let mut font_size_counts: BTreeMap<i32, usize> = BTreeMap::new();
        for line in &lines {
            for elem in line {
                let key = (elem.font_size * 10.0) as i32;
                *font_size_counts.entry(key).or_insert(0) += 1;
            }
        }
        let _body_size = font_size_counts
            .iter()
            .max_by_key(|&(_, count)| count)
            .map(|(&size, _)| size as f32 / 10.0)
            .unwrap_or(12.0);

        // Track text occurrences for running header detection
        let mut text_occurrences: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let line_texts: Vec<MergedLine> = lines.iter().map(|line| self.merge_line(line)).collect();

        for merged in &line_texts {
            let normalized = merged.text.trim().to_lowercase();
            if !normalized.is_empty() && normalized.len() < 100 {
                *text_occurrences.entry(normalized).or_insert(0) += 1;
            }
        }

        // Section pattern regex
        let _section_pattern = regex::Regex::new(r"^(\d+\.)+\s+[A-Z]").ok();

        let mut last_bbox: Option<BoundingBox> = None;
        let mut last_text: String = String::new();

        for (idx, line) in lines.iter().enumerate() {
            if line.is_empty() {
                continue;
            }

            let merged = &line_texts[idx];
            let text = merged.text.trim();
            if text.is_empty() {
                continue;
            }

            // Get bounding box
            let min_x = line
                .iter()
                .map(|e| e.x)
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(0.0);
            let max_x = line
                .iter()
                .map(|e| e.x)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(page_width);
            let y = line.first().map(|e| e.y).unwrap_or(0.0);

            let bbox = BoundingBox::new(min_x, y, max_x, y + merged.avg_font_size);

            // Deduplication: Check if this block is a duplicate of the previous one
            // (e.g. hidden OCR layer overlapping with visible text)
            if let Some(prev_bbox) = &last_bbox {
                // Check vertical overlap (lines are sorted by Y, so duplicates should be adjacent)
                let overlap_y = prev_bbox.y2.min(bbox.y2) - prev_bbox.y1.max(bbox.y1);
                let min_h = (prev_bbox.y2 - prev_bbox.y1).min(bbox.y2 - bbox.y1);

                if overlap_y > min_h * 0.5 {
                    // Significant vertical overlap (>50%). Check text similarity.
                    // We check for exact match or containment to handle slight OCR variations
                    if text == last_text
                        || (text.len() > 5
                            && (text.contains(&last_text) || last_text.contains(text)))
                    {
                        // tracing::debug!("Skipping duplicate block: '{}'", text);
                        continue;
                    }
                }
            }

            last_bbox = Some(bbox);
            last_text = text.to_string();

            // Detect block type
            let normalized = text.to_lowercase();
            let is_running_header = text_occurrences.get(&normalized).copied().unwrap_or(0) >= 3;

            let block_type = if is_running_header {
                BlockType::PageHeader
            } else {
                BlockType::Text
            };

            let spans = merged
                .spans
                .iter()
                .cloned()
                .map(|mut s| {
                    s.bbox = Some(bbox);
                    s
                })
                .collect::<Vec<_>>();

            let block = Block {
                id: BlockId::with_indices(0, blocks.len()),
                block_type,
                text: text.to_string(),
                bbox,
                page: 0,
                position: blocks.len(),
                level: None,
                spans,
                ..Default::default()
            };

            blocks.push(block);
        }

        blocks
    }

    #[allow(dead_code)]
    fn calculate_header_level(&self, font_size: f32, body_size: f32) -> u8 {
        let ratio = font_size / body_size;
        if ratio >= 2.0 {
            1
        } else if ratio >= 1.5 {
            2
        } else if ratio >= 1.3 {
            3
        } else {
            4
        }
    }

    /// Calculate adaptive region thresholds based on actual content distribution.
    ///
    /// This is a first-principles approach that analyzes the document's
    /// actual layout to determine appropriate thresholds for header/footer/title
    /// detection, instead of using hardcoded magic numbers.
    ///
    /// # Arguments
    /// * `elements` - Text elements to analyze
    ///
    /// # Returns
    /// Tuple of (footer_threshold, header_threshold, title_threshold, affiliation_threshold, large_font_threshold)
    fn calculate_adaptive_region_thresholds(
        &self,
        elements: &[TextElement],
    ) -> (f32, f32, f32, f32, f32) {
        if elements.is_empty() {
            // Fallback to reasonable defaults for empty documents
            return (60.0, 730.0, 650.0, 80.0, 11.0);
        }

        // Calculate page height from elements
        let page_height = elements.iter().map(|e| e.y).fold(f32::MIN, f32::max);
        let page_bottom = elements.iter().map(|e| e.y).fold(f32::MAX, f32::min);

        // Calculate font size distribution
        let font_sizes: Vec<f32> = elements.iter().map(|e| e.font_size).collect();
        let avg_font_size = if font_sizes.is_empty() {
            10.0
        } else {
            font_sizes.iter().sum::<f32>() / font_sizes.len() as f32
        };

        // Calculate adaptive thresholds based on page dimensions and content
        let footer_threshold = page_bottom + (page_height - page_bottom) * 0.08; // Bottom 8% of page
        let header_threshold = page_height - (page_height - page_bottom) * 0.08; // Top 8% of page
        let title_threshold = page_bottom + (page_height - page_bottom) * 0.15; // Top 15% of page
        let affiliation_threshold = page_bottom + (page_height - page_bottom) * 0.12; // Bottom 12% of page
        let large_font_threshold = avg_font_size * 1.2; // 20% larger than average

        // Clamp to reasonable ranges - ensure min <= max to avoid panic
        let footer_threshold = footer_threshold.clamp(40.0, 100.0);
        let header_min = (page_height - 100.0).max(0.0);
        let header_max = (page_height - 20.0).max(header_min);
        let header_threshold = header_threshold.clamp(header_min, header_max);

        let title_min = page_bottom + 100.0;
        let title_max = (page_height - 50.0).max(title_min);
        let title_threshold = title_threshold.clamp(title_min, title_max);

        let affiliation_threshold = affiliation_threshold.clamp(60.0, 120.0);
        let large_font_threshold = large_font_threshold.clamp(10.0, 14.0);

        (
            footer_threshold,
            header_threshold,
            title_threshold,
            affiliation_threshold,
            large_font_threshold,
        )
    }

    /// Get page dimensions
    fn get_page_dimensions(&self, doc: &LopdfDocument, page_id: ObjectId) -> Result<(f32, f32)> {
        let page_dict = doc
            .get_dictionary(page_id)
            .map_err(|e| PdfError::PdfParse(format!("Failed to get page: {}", e)))?;

        // Try MediaBox
        if let Ok(media_box) = page_dict.get(b"MediaBox") {
            if let Object::Array(arr) = media_box {
                if arr.len() >= 4 {
                    let width = Self::get_number(&arr[2]).unwrap_or(612.0);
                    let height = Self::get_number(&arr[3]).unwrap_or(792.0);
                    return Ok((width, height));
                }
            } else if let Object::Reference(id) = media_box {
                if let Ok(Object::Array(arr)) = doc.get_object(*id) {
                    if arr.len() >= 4 {
                        let width = Self::get_number(&arr[2]).unwrap_or(612.0);
                        let height = Self::get_number(&arr[3]).unwrap_or(792.0);
                        return Ok((width, height));
                    }
                }
            }
        }

        Ok((612.0, 792.0))
    }

    /// Extract a single page
    fn extract_page(
        &self,
        doc: &LopdfDocument,
        page_id: ObjectId,
        page_num: usize,
    ) -> Result<Page> {
        let (page_width, page_height) = self.get_page_dimensions(doc, page_id)?;

        // Get fonts
        let fonts = self.get_page_fonts(doc, page_id).unwrap_or_default();
        debug!("Page {} has {} fonts", page_num, fonts.len());

        // Get content
        let content_bytes = self.get_page_content(doc, page_id)?;

        // Extract text and graphical elements
        let (elements, pdf_lines) = self.extract_page_elements(&content_bytes, &fonts)?;

        // Deduplicate elements (OCR layers)
        let elements = self.deduplicate_elements(elements);

        // Merge fragmented text elements
        let elements = self.merge_text_elements(elements);

        debug!(
            "Page {} has {} text elements and {} graphical lines",
            page_num,
            elements.len(),
            pdf_lines.len()
        );

        // Detect tables using lattice-based line detection
        let tables: Vec<Block> = self
            .lattice_engine
            .detect_tables(&pdf_lines, &elements, page_width, page_height)
            .into_iter()
            .filter(|table| {
                // Exclude tables that are too small (< 50x50 points)
                // This filters out small decorative boxes
                let min_size = 50.0;
                if table.bbox.width() < min_size || table.bbox.height() < min_size {
                    debug!(
                        "Filtered out table: too small ({:.1}x{:.1})",
                        table.bbox.width(),
                        table.bbox.height()
                    );
                    return false;
                }

                // Exclude tables that are too large (> 80% of page)
                // This filters out page borders and full-page elements
                // First principles: tables typically have margins on all sides
                let max_width = page_width * 0.8;
                let max_height = page_height * 0.8;
                if table.bbox.width() > max_width || table.bbox.height() > max_height {
                    debug!(
                        "Filtered out table: too large ({:.1}x{:.1})",
                        table.bbox.width(),
                        table.bbox.height()
                    );
                    return false;
                }

                // Exclude tables that are too close to page edges (likely page borders)
                // First principles: tables are typically centered with margins
                let margin_threshold = 20.0; // 20 points from edge
                if table.bbox.x1 < margin_threshold
                    || table.bbox.y1 < margin_threshold
                    || table.bbox.x2 > page_width - margin_threshold
                    || table.bbox.y2 > page_height - margin_threshold
                {
                    debug!("Filtered out table: too close to page edges");
                    return false;
                }

                // Exclude empty tables (no text content)
                if table.text.trim().is_empty() {
                    debug!("Filtered out table: empty");
                    return false;
                }

                // Exclude tables with very low text density (likely decorative boxes)
                // First principles: tables contain data, not just whitespace
                let text_len = table.text.trim().len();
                let table_area = table.bbox.width() * table.bbox.height();
                let text_density = text_len as f32 / table_area;
                if text_density < 0.0001 {
                    // Less than 1 char per 10000 points²
                    debug!("Filtered out table: low text density ({:.6})", text_density);
                    return false;
                }

                true
            })
            .collect();

        // Filter out text elements that are inside tables
        let mut non_table_elements = Vec::new();
        for elem in &elements {
            let mut inside_table = false;
            for table in &tables {
                // Check if element center is inside table bbox
                let cx = elem.x;
                let cy = elem.y;
                if table.bbox.contains_point(&Point::new(cx, cy)) {
                    inside_table = true;
                    break;
                }
            }
            if !inside_table {
                non_table_elements.push(elem.clone());
            }
        }

        // Safety check: if we filtered everything, maybe the table detection was too aggressive (e.g. page border)
        if non_table_elements.is_empty() && !elements.is_empty() {
            warn!("Table detection filtered all text elements on page {}. Ignoring table detection for text filtering.", page_num);
            non_table_elements = elements;
        }

        // Group into lines (handles two-column layouts) and get column bounding boxes
        let (lines, columns) = self.group_into_lines(non_table_elements, page_width, page_height);
        debug!(
            "Page {} has {} lines, {} columns detected",
            page_num,
            lines.len(),
            columns.len()
        );

        // Convert to blocks
        let mut blocks = self.lines_to_blocks(lines, page_width, page_height);

        // Insert detected tables back into the existing reading order.
        // We intentionally do NOT re-sort `blocks` globally (that can break multi-column reading
        // order), but we also do not want tables to be appended at the end of the page.
        // Instead, place each table at the first position where subsequent blocks appear below
        // the table on the page.
        let mut tables = tables;
        tables.sort_by(|a, b| {
            // Top-to-bottom insertion (higher Y first).
            b.bbox
                .y2
                .partial_cmp(&a.bbox.y2)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for table in tables {
            let table_y = (table.bbox.y1 + table.bbox.y2) * 0.5;
            let mut insert_idx = blocks.len();
            for (idx, blk) in blocks.iter().enumerate() {
                let blk_y = (blk.bbox.y1 + blk.bbox.y2) * 0.5;
                if blk_y < table_y {
                    insert_idx = idx;
                    break;
                }
            }
            blocks.insert(insert_idx, table);
        }

        // NOTE: Do NOT sort blocks here! The reading order has already been established by
        // group_into_lines() -> group_two_column_layout() or group_single_column_layout().
        // Sorting by Y would destroy the correct column-based reading order.

        let char_count: usize = blocks.iter().map(|b| b.text.len()).sum();
        let word_count: usize = blocks
            .iter()
            .map(|b| b.text.split_whitespace().count())
            .sum();

        let mut page = Page::new(page_num, page_width, page_height);
        page.blocks = blocks;
        page.columns = columns; // Set detected columns to prevent LayoutProcessor re-analysis
        page.method = ExtractionMethod::Native;
        page.stats = PageStats {
            text_blocks: page.blocks.len(),
            tables: page
                .blocks
                .iter()
                .filter(|b| b.block_type == BlockType::Table)
                .count(),
            figures: 0,
            headers: page
                .blocks
                .iter()
                .filter(|b| b.block_type == BlockType::SectionHeader)
                .count(),
            code_blocks: 0,
            equations: 0,
            char_count,
            word_count,
            avg_confidence: 1.0,
            ocr_used: false,
            processing_time_ms: 0,
        };

        Ok(page)
    }
}

impl Default for SotaBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PdfBackend for SotaBackend {
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document> {
        info!("Extracting PDF with SOTA backend");

        let lopdf_doc = LopdfDocument::load_mem(pdf_bytes)
            .map_err(|e| PdfError::PdfParse(format!("Failed to load PDF: {}", e)))?;

        if lopdf_doc.is_encrypted() {
            return Err(PdfError::PdfParse(
                "PDF is encrypted and password-protected".to_string(),
            ));
        }

        let pages = lopdf_doc.get_pages();
        let page_count = pages.len();
        info!("PDF has {} pages", page_count);

        let max_pages = self.config.max_pages.unwrap_or(page_count);
        let pages_to_process = page_count.min(max_pages);

        let mut document = Document::new();
        document.metadata = DocumentMetadata {
            pdf_version: Some(lopdf_doc.version.clone()),
            ..Default::default()
        };

        for (page_num, page_id) in pages.iter().take(pages_to_process) {
            debug!("Processing page {}", page_num);

            match self.extract_page(&lopdf_doc, *page_id, *page_num as usize) {
                Ok(page) => {
                    document.add_page(page);
                }
                Err(e) => {
                    warn!("Failed to extract page {}: {}", page_num, e);
                }
            }
        }

        info!("Extracted {} pages with SOTA backend", document.pages.len());
        Ok(document)
    }

    fn get_info(&self, pdf_bytes: &[u8]) -> Result<PdfInfo> {
        let lopdf_doc = LopdfDocument::load_mem(pdf_bytes)
            .map_err(|e| PdfError::PdfParse(format!("Failed to load PDF: {}", e)))?;

        if lopdf_doc.is_encrypted() {
            return Err(PdfError::PdfParse(
                "PDF is encrypted and password-protected".to_string(),
            ));
        }

        let pages = lopdf_doc.get_pages();

        Ok(PdfInfo {
            page_count: pages.len(),
            pdf_version: lopdf_doc.version.clone(),
            has_images: false,
            image_count: 0,
            file_size: pdf_bytes.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_line_preserves_style_runs_as_spans() {
        let backend = SotaBackend::new();

        let elems = vec![
            TextElement {
                text: "Hello".to_string(),
                x: 10.0,
                y: 700.0,
                font_size: 12.0,
                font_name: "Times-Roman".to_string(),
                is_bold: false,
                is_italic: false,
            },
            TextElement {
                text: "World".to_string(),
                x: 60.0,
                y: 700.0,
                font_size: 12.0,
                font_name: "Times-Bold".to_string(),
                is_bold: true,
                is_italic: false,
            },
        ];

        let merged = backend.merge_line(&elems);
        assert_eq!(merged.text, "Hello World");
        assert!(merged.spans.len() >= 2);
        assert_eq!(merged.spans[0].text, "Hello ");
        assert_eq!(merged.spans[0].style.weight, Some(400));
        assert_eq!(merged.spans[1].text, "World");
        assert_eq!(merged.spans[1].style.weight, Some(700));
    }
}
