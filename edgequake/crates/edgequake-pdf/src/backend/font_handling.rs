//! Font handling and encoding resolution for PDF extraction.
//!
//! This module handles:
//! - Font information extraction from PDF dictionaries
//! - Bold/italic detection from font naming conventions
//! - Encoding resolution (ToUnicode, WinAnsi, MacRoman, etc.)

use lopdf::{Dictionary, Document as LopdfDocument, Object, Stream};
use super::encodings;
use super::encodings::Encoding;

/// Information about a font in the PDF
#[derive(Debug)]
pub struct FontInfo {
    /// Base font name (e.g., "Helvetica-Bold")
    pub base_font: String,
    /// Font encoding for text decoding
    pub encoding: Encoding,
    /// Detected font size from usage
    #[allow(dead_code)]
    pub size: f32,
    /// Is this font bold?
    pub is_bold: bool,
    /// Is this font italic?
    pub is_italic: bool,
}

impl FontInfo {
    /// Construct FontInfo from a PDF font dictionary.
    ///
    /// Extracts the base font name, detects bold/italic style from common naming
    /// conventions (including LaTeX fonts like SFBX/CMTI), and resolves encoding.
    pub fn from_dict(doc: &LopdfDocument, font_dict: &Dictionary) -> Self {
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

    /// Resolve the font encoding from the PDF dictionary.
    ///
    /// Priority order:
    /// 1. ToUnicode CMap (most reliable, handles any mapping)
    /// 2. Named encoding (WinAnsiEncoding, StandardEncoding, etc.)
    /// 3. Identity fallback (raw bytes as UTF-16BE)
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

    /// Resolve a PDF object to a Stream, following indirect references.
    ///
    /// Handles both direct Stream objects and Reference pointers.
    pub fn resolve_stream<'a>(doc: &'a LopdfDocument, obj: &'a Object) -> Option<&'a Stream> {
        match obj {
            Object::Reference(id) => doc.get_object(*id).ok()?.as_stream().ok(),
            Object::Stream(s) => Some(s),
            _ => None,
        }
    }

    /// Decode text bytes using this font's encoding.
    pub fn decode(&self, bytes: &[u8]) -> String {
        self.encoding.decode(bytes)
    }
}
