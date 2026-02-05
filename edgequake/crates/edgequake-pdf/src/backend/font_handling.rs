//! Font handling and encoding resolution for PDF extraction.
//!
//! This module handles:
//! - Font information extraction from PDF dictionaries
//! - Bold/italic detection from font naming conventions
//! - Encoding resolution (ToUnicode, WinAnsi, MacRoman, /Differences, etc.)
//! - Embedded TrueType font parsing for subset fonts
//!
//! **WHY /Differences parsing is critical:**
//! Many legacy PDFs (especially from tools like Apple's Pages or early LaTeX)
//! use custom font encodings via /Differences arrays. Without parsing these,
//! text extraction produces garbled output like "!"#$%" instead of "Table".
//!
//! **WHY embedded TrueType parsing is critical:**
//! Subset TrueType fonts (like Calibri/Cambria from Microsoft Office) don't
//! have an explicit encoding. The glyph→Unicode mapping is in the font's
//! cmap table inside the /FontFile2 stream.

use super::encodings;
use super::encodings::Encoding;
use super::glyph_list::glyph_to_unicode;
use super::truetype_cmap;
use lopdf::{Dictionary, Document as LopdfDocument, Object, Stream};
use std::collections::HashMap;
use tracing::trace;

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
    /// Extracts the base font name, detects bold/italic style from:
    /// 1. FontDescriptor/Flags field (authoritative, like PyMuPDF)
    /// 2. Font name pattern matching (fallback for fonts without FontDescriptor)
    pub fn from_dict(doc: &LopdfDocument, font_dict: &Dictionary) -> Self {
        // Get base font name
        let base_font = font_dict
            .get(b"BaseFont")
            .ok()
            .and_then(|obj| obj.as_name().ok())
            .map(|n| String::from_utf8_lossy(n).to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        // Try to extract style flags from FontDescriptor first (authoritative)
        // WHY: Font name matching is unreliable. PyMuPDF uses font descriptor flags:
        // - Bit 7 (value 64) = Italic
        // - Weight >= 700 or ForceBold flag = Bold
        let (flags_italic, flags_bold) = Self::get_font_descriptor_flags(doc, font_dict);

        // Fallback to font name matching if FontDescriptor flags unavailable
        let lower_name = base_font.to_lowercase();

        // Bold detection: prefer FontDescriptor, fallback to name patterns
        let is_bold = if let Some(bold_flag) = flags_bold {
            bold_flag
        } else {
            // Check for common bold indicators:
            // - "bold", "black", "heavy" in name
            // - "sfbx" (SF Bold Extended) in arXiv/LaTeX fonts like TFFXIV+SFBX1200
            // - "cmbx" (Computer Modern Bold Extended)
            // - "-bold" suffix
            // - "medi" (medium weight in Nimbus fonts) - OODA-09: Re-enabled for abstract bold text
            lower_name.contains("bold")
                || lower_name.contains("black")
                || lower_name.contains("heavy")
                || lower_name.contains("sfbx")   // SF Bold Extended (arXiv/LaTeX)
                || lower_name.contains("cmbx")   // Computer Modern Bold Extended
                || lower_name.contains("medi")   // Medium weight in Nimbus fonts
                || lower_name.contains("-bold")
        };

        // Italic detection: prefer FontDescriptor, fallback to name patterns
        let is_italic = if let Some(italic_flag) = flags_italic {
            italic_flag
        } else {
            // OODA-05: Detect italic from font naming conventions.
            // Computer Modern fonts from LaTeX use these conventions:
            // - CMTI = CM Text Italic, CMMI = CM Math Italic, CMSY = CM Symbol (italic style)
            // - CMMIB = CM Math Italic Bold
            lower_name.contains("italic") 
                || lower_name.contains("oblique")
                || lower_name.contains("ital")   // Abbreviated form used by Nimbus fonts
                || lower_name.contains("sfti")   // SF Text Italic
                || lower_name.contains("cmti")   // Computer Modern Text Italic
                || lower_name.contains("cmmi")   // Computer Modern Math Italic
                || lower_name.contains("cmsy")   // Computer Modern Symbol
                || lower_name.contains("cmmib")  // Computer Modern Math Italic Bold
                || lower_name.contains("-italic")
        };

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

    /// Extract bold/italic flags from PDF FontDescriptor.
    ///
    /// Returns (Option<is_italic>, Option<is_bold>).
    /// None means the flag couldn't be determined from FontDescriptor.
    ///
    /// For italic detection, we check (in order):
    /// 1. Flags bit 7 (value 64) - PDF spec italic flag
    /// 2. ItalicAngle != 0 - Non-zero angle indicates italic/oblique
    ///
    /// WHY ItalicAngle is critical: Many PDFs (especially from LaTeX/scientific papers)
    /// don't set the Flags bit 7, but DO set ItalicAngle to a non-zero value like -12°.
    /// PyMuPDF uses ItalicAngle as the primary indicator for italic detection.
    ///
    /// For bold detection, we check:
    /// 1. Weight field >= 700 (CSS convention)
    /// 2. StemV > 120 (stem width heuristic)
    fn get_font_descriptor_flags(
        doc: &LopdfDocument,
        font_dict: &Dictionary,
    ) -> (Option<bool>, Option<bool>) {
        // Get FontDescriptor reference or dictionary
        let font_descriptor = match font_dict.get(b"FontDescriptor") {
            Ok(Object::Reference(id)) => doc.get_dictionary(*id).ok(),
            Ok(Object::Dictionary(d)) => Some(d),
            _ => None,
        };

        let fd = match font_descriptor {
            Some(fd) => fd,
            None => return (None, None), // No FontDescriptor, use name matching
        };

        // Extract Flags field (integer bitmask)
        let flags: i64 = match fd.get(b"Flags") {
            Ok(Object::Integer(i)) => *i,
            Ok(Object::Reference(id)) => doc
                .get_object(*id)
                .ok()
                .and_then(|obj| match obj {
                    Object::Integer(i) => Some(*i),
                    _ => None,
                })
                .unwrap_or(0),
            _ => 0,
        };

        // Check Flags bit 7 (0x40 = 64) for italic
        let flags_italic = (flags & 64) != 0;

        // Check ItalicAngle - non-zero means italic/oblique
        // WHY: This is the PRIMARY italic indicator in many PDFs where Flags bit 7 is not set.
        // Example: NimbusSanL-ReguItal has Flags=4 (no italic bit) but ItalicAngle=-12
        let italic_angle: f64 = match fd.get(b"ItalicAngle") {
            Ok(Object::Integer(i)) => *i as f64,
            Ok(Object::Real(r)) => *r as f64,
            Ok(Object::Reference(id)) => doc
                .get_object(*id)
                .ok()
                .and_then(|obj| match obj {
                    Object::Integer(i) => Some(*i as f64),
                    Object::Real(r) => Some(*r as f64),
                    _ => None,
                })
                .unwrap_or(0.0),
            _ => 0.0,
        };

        // Italic if Flags bit 7 is set OR ItalicAngle is non-zero
        let is_italic = Some(flags_italic || italic_angle != 0.0);

        // For bold, check Weight field first, then StemV as fallback
        // Weight >= 700 is typically considered bold (CSS convention)
        let weight: Option<i64> = match fd.get(b"Weight") {
            Ok(Object::Integer(i)) => Some(*i),
            Ok(Object::Reference(id)) => doc.get_object(*id).ok().and_then(|obj| match obj {
                Object::Integer(i) => Some(*i),
                _ => None,
            }),
            _ => None,
        };

        let is_bold = if let Some(w) = weight {
            Some(w >= 700)
        } else {
            // No Weight field - check StemV as heuristic
            // StemV > 120 typically indicates bold (normal is ~80-100)
            match fd.get(b"StemV") {
                Ok(Object::Integer(i)) => Some(*i > 120),
                Ok(Object::Real(r)) => Some(*r > 120.0),
                Ok(Object::Reference(id)) => doc.get_object(*id).ok().and_then(|obj| match obj {
                    Object::Integer(i) => Some(*i > 120),
                    Object::Real(r) => Some(*r > 120.0),
                    _ => None,
                }),
                _ => None, // No StemV, fall back to name matching
            }
        };

        (is_italic, is_bold)
    }

    /// Resolve the font encoding from the PDF dictionary.
    ///
    /// Priority order:
    /// 1. ToUnicode CMap (most reliable, handles any mapping)
    /// 2. Encoding dictionary with /Differences array (custom glyph mappings)
    /// 3. Named encoding (WinAnsiEncoding, StandardEncoding, etc.)
    /// 4. WinAnsi fallback (default for Type1/TrueType fonts)
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
                    // Encoding dictionary - check for /Differences array first
                    if let Ok(enc_dict) = doc.get_dictionary(*id) {
                        // Try to parse /Differences array
                        if let Some(diff_map) = Self::parse_differences(doc, enc_dict) {
                            return Encoding::DifferencesEncoding(diff_map);
                        }

                        // Fall back to BaseEncoding
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
                Object::Dictionary(enc_dict) => {
                    // Direct dictionary - check for /Differences array
                    if let Some(diff_map) = Self::parse_differences(doc, enc_dict) {
                        return Encoding::DifferencesEncoding(diff_map);
                    }
                }
                _ => {}
            }
        }

        // No explicit encoding - try embedded TrueType font
        // WHY: Subset TrueType fonts (like LHKJDD+Calibri-Bold) embed their
        // glyph→Unicode mapping in the font's cmap table, not in the PDF encoding.
        if let Some(encoding) = Self::try_embedded_truetype(doc, font_dict) {
            return encoding;
        }

        // Default to WinAnsi for Type1 and TrueType fonts
        Encoding::OneByteEncoding(&encodings::WIN_ANSI_ENCODING)
    }

    /// Try to parse embedded TrueType font from /FontFile2 stream.
    ///
    /// **WHY this method:**
    /// Subset TrueType fonts (e.g., "LHKJDD+Calibri-Bold") have no explicit
    /// encoding in the PDF. The glyph→Unicode mapping is in the embedded font's
    /// cmap table. This method:
    /// 1. Navigates FontDescriptor → FontFile2 → Stream
    /// 2. Decompresses the stream to get raw TrueType font bytes
    /// 3. Parses the font using ttf-parser to extract cmap table
    /// 4. Builds a glyph ID → Unicode mapping
    fn try_embedded_truetype(doc: &LopdfDocument, font_dict: &Dictionary) -> Option<Encoding> {
        trace!("try_embedded_truetype: Checking font dictionary for FontDescriptor");

        // Get FontDescriptor
        let fd_obj = match font_dict.get(b"FontDescriptor") {
            Ok(obj) => {
                trace!("try_embedded_truetype: Found FontDescriptor");
                obj
            }
            Err(_) => {
                trace!("try_embedded_truetype: No FontDescriptor found");
                return None;
            }
        };

        let fd = match fd_obj {
            Object::Reference(id) => {
                trace!(
                    "try_embedded_truetype: FontDescriptor is reference {:?}",
                    id
                );
                match doc.get_dictionary(*id) {
                    Ok(d) => d,
                    Err(_) => return None,
                }
            }
            Object::Dictionary(d) => d,
            _ => return None,
        };

        // Get FontFile2 (embedded TrueType font)
        let ff2_obj = match fd.get(b"FontFile2") {
            Ok(obj) => {
                trace!("try_embedded_truetype: Found FontFile2");
                obj
            }
            Err(_) => {
                trace!("try_embedded_truetype: No FontFile2 found");
                return None;
            }
        };

        let stream = Self::resolve_stream(doc, ff2_obj)?;

        // Decompress the font data
        let font_data = match stream.decompressed_content() {
            Ok(data) => {
                trace!(
                    "try_embedded_truetype: Decompressed {} bytes of font data",
                    data.len()
                );
                data
            }
            Err(e) => {
                trace!("try_embedded_truetype: Failed to decompress: {:?}", e);
                return None;
            }
        };

        // Parse the TrueType font and extract cmap
        let glyph_map = truetype_cmap::parse_embedded_truetype(&font_data)?;

        trace!(
            "try_embedded_truetype: Parsed {} glyph mappings from cmap",
            glyph_map.len()
        );

        Some(Encoding::EmbeddedTrueType(glyph_map))
    }

    /// Parse a /Differences array from an encoding dictionary.
    ///
    /// **WHY this is critical:**
    /// The /Differences array allows fonts to override specific byte codes
    /// with custom glyph names. Format: [firstCode glyphName glyphName ... nextCode glyphName ...]
    ///
    /// Example: `/Differences [33 /exclam /quotedbl /numbersign 65 /A /B /C]`
    /// This maps byte 33 → "!", 34 → "\"", 35 → "#", 65 → "A", 66 → "B", 67 → "C"
    ///
    /// # Returns
    /// - `Some(HashMap)` if /Differences was found and parsed successfully
    /// - `None` if no /Differences array or parsing failed
    fn parse_differences(doc: &LopdfDocument, enc_dict: &Dictionary) -> Option<HashMap<u8, char>> {
        let diffs = enc_dict.get(b"Differences").ok()?;

        // Resolve reference if needed
        let diffs_array = match diffs {
            Object::Array(arr) => arr,
            Object::Reference(id) => doc.get_object(*id).ok()?.as_array().ok()?,
            _ => return None,
        };

        let mut map = HashMap::new();
        let mut code: u8 = 0;

        for obj in diffs_array {
            match obj {
                Object::Integer(n) => {
                    // Integer sets the starting code for subsequent glyph names
                    code = (*n).clamp(0, 255) as u8;
                }
                Object::Name(name) => {
                    // Glyph name - look up Unicode equivalent
                    let glyph_name = String::from_utf8_lossy(name);
                    if let Some(unicode_char) = glyph_to_unicode(&glyph_name) {
                        map.insert(code, unicode_char);
                    }
                    // Always increment code for next glyph name
                    code = code.wrapping_add(1);
                }
                _ => {
                    // Unexpected object type, skip
                }
            }
        }

        // Only return if we found any mappings
        if map.is_empty() {
            None
        } else {
            Some(map)
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_info_decode_winansi() {
        let info = FontInfo {
            base_font: "Helvetica".to_string(),
            encoding: Encoding::OneByteEncoding(&encodings::WIN_ANSI_ENCODING),
            size: 12.0,
            is_bold: false,
            is_italic: false,
        };

        // Simple ASCII should decode correctly
        let result = info.decode(b"Hello");
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_detect_bold_from_name() {
        // Common bold indicators
        let bold_names = vec![
            "Helvetica-Bold",
            "ArialMT-Black",
            "TimesNewRomanPS-HeavyMT",
            "SFBX1200+Bold", // LaTeX bold
            "CMBX10",        // Computer Modern Bold
        ];

        for name in bold_names {
            let lower = name.to_lowercase();
            let is_bold = lower.contains("bold")
                || lower.contains("black")
                || lower.contains("heavy")
                || lower.contains("sfbx")
                || lower.contains("cmbx");
            assert!(is_bold, "Expected '{}' to be detected as bold", name);
        }
    }

    #[test]
    fn test_detect_italic_from_name() {
        let italic_names = vec![
            "Helvetica-Italic",
            "ArialMT-Oblique",
            "SFTI0900", // LaTeX italic
            "CMTI10",   // Computer Modern italic
            "CMMI10",   // Computer Modern math italic
        ];

        for name in italic_names {
            let lower = name.to_lowercase();
            let is_italic = lower.contains("italic")
                || lower.contains("oblique")
                || lower.contains("sfti")
                || lower.contains("cmti")
                || lower.contains("cmmi");
            assert!(is_italic, "Expected '{}' to be detected as italic", name);
        }
    }

    #[test]
    fn test_regular_font_not_bold_or_italic() {
        let regular_names = vec!["Helvetica", "ArialMT", "TimesNewRomanPSMT"];

        for name in regular_names {
            let lower = name.to_lowercase();
            let is_bold =
                lower.contains("bold") || lower.contains("black") || lower.contains("heavy");
            let is_italic = lower.contains("italic") || lower.contains("oblique");
            assert!(!is_bold, "Expected '{}' NOT to be bold", name);
            assert!(!is_italic, "Expected '{}' NOT to be italic", name);
        }
    }

    #[test]
    fn test_font_info_struct() {
        let info = FontInfo {
            base_font: "TestFont".to_string(),
            encoding: Encoding::Identity,
            size: 14.0,
            is_bold: true,
            is_italic: false,
        };

        assert_eq!(info.base_font, "TestFont");
        assert!(info.is_bold);
        assert!(!info.is_italic);
    }

    #[test]
    fn test_identity_encoding_decode() {
        let info = FontInfo {
            base_font: "Test".to_string(),
            encoding: Encoding::Identity,
            size: 12.0,
            is_bold: false,
            is_italic: false,
        };

        // Identity treats bytes as UTF-16BE
        let bytes = [0x00, 0x41]; // UTF-16BE for 'A'
        let result = info.decode(&bytes);
        assert_eq!(result, "A");
    }

    #[test]
    fn test_bold_italic_combined() {
        let name = "Helvetica-BoldOblique";
        let lower = name.to_lowercase();

        let is_bold = lower.contains("bold");
        let is_italic = lower.contains("oblique");

        assert!(is_bold);
        assert!(is_italic);
    }

    #[test]
    fn test_differences_encoding_decode() {
        // Test DifferencesEncoding with a custom mapping for "Table"
        // Simulates: /Differences [33 /T /a /b /l /e]
        // This maps byte 33='T', 34='a', 35='b', 36='l', 37='e'
        let mut diff_map = HashMap::new();
        diff_map.insert(33, 'T');
        diff_map.insert(34, 'a');
        diff_map.insert(35, 'b');
        diff_map.insert(36, 'l');
        diff_map.insert(37, 'e');

        let info = FontInfo {
            base_font: "CustomFont".to_string(),
            encoding: Encoding::DifferencesEncoding(diff_map),
            size: 12.0,
            is_bold: false,
            is_italic: false,
        };

        // bytes [33, 34, 35, 36, 37] should decode to "Table"
        let bytes = [33, 34, 35, 36, 37];
        let result = info.decode(&bytes);
        assert_eq!(result, "Table");
    }

    #[test]
    fn test_differences_encoding_fallback_to_winansi() {
        // DifferencesEncoding should fall back to WinAnsi for unmapped bytes
        let mut diff_map = HashMap::new();
        diff_map.insert(33, 'X'); // Map 33 to 'X' instead of '!'

        let info = FontInfo {
            base_font: "CustomFont".to_string(),
            encoding: Encoding::DifferencesEncoding(diff_map),
            size: 12.0,
            is_bold: false,
            is_italic: false,
        };

        // Byte 65 is not in diff_map, should fall back to WinAnsi ('A')
        let bytes = [33, 65]; // 33='X' (from diff), 65='A' (from WinAnsi fallback)
        let result = info.decode(&bytes);
        assert_eq!(result, "XA");
    }
}
