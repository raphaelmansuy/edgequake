//! PDF font encoding support for character-to-Unicode conversion.
//!
//! **Single Responsibility:** Map PDF font byte codes to Unicode code points.
//!
//! This module handles the complex world of PDF character encoding:
//! - WinAnsi, Standard, MacRoman encodings (fixed tables)
//! - ToUnicode CMap parsing (custom mappings per font)
//! - Identity-H encoding (direct UTF-16BE)
//! - Ligature expansion (fi, fl, ffi, ffl → character sequences)
//!
//! **WHY this matters:**
//! PDF fonts encode characters differently. Without proper decoding,
//! extracted text is garbled or contains replacement characters.

use std::collections::HashMap;

/// Character set mapping (byte -> Unicode code point)
pub type CodedCharacterSet = [Option<u16>; 256];

/// WinAnsi encoding - most common for PDF fonts
/// Maps bytes 0x00-0xFF to Unicode code points
pub static WIN_ANSI_ENCODING: CodedCharacterSet = [
    // 0x00-0x1F: Control characters (mostly None)
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    // 0x20-0x3F: Printable ASCII
    Some(0x0020),
    Some(0x0021),
    Some(0x0022),
    Some(0x0023),
    Some(0x0024),
    Some(0x0025),
    Some(0x0026),
    Some(0x0027),
    Some(0x0028),
    Some(0x0029),
    Some(0x002A),
    Some(0x002B),
    Some(0x002C),
    Some(0x002D),
    Some(0x002E),
    Some(0x002F),
    Some(0x0030),
    Some(0x0031),
    Some(0x0032),
    Some(0x0033),
    Some(0x0034),
    Some(0x0035),
    Some(0x0036),
    Some(0x0037),
    Some(0x0038),
    Some(0x0039),
    Some(0x003A),
    Some(0x003B),
    Some(0x003C),
    Some(0x003D),
    Some(0x003E),
    Some(0x003F),
    // 0x40-0x5F: Uppercase letters
    Some(0x0040),
    Some(0x0041),
    Some(0x0042),
    Some(0x0043),
    Some(0x0044),
    Some(0x0045),
    Some(0x0046),
    Some(0x0047),
    Some(0x0048),
    Some(0x0049),
    Some(0x004A),
    Some(0x004B),
    Some(0x004C),
    Some(0x004D),
    Some(0x004E),
    Some(0x004F),
    Some(0x0050),
    Some(0x0051),
    Some(0x0052),
    Some(0x0053),
    Some(0x0054),
    Some(0x0055),
    Some(0x0056),
    Some(0x0057),
    Some(0x0058),
    Some(0x0059),
    Some(0x005A),
    Some(0x005B),
    Some(0x005C),
    Some(0x005D),
    Some(0x005E),
    Some(0x005F),
    // 0x60-0x7F: Lowercase letters
    Some(0x0060),
    Some(0x0061),
    Some(0x0062),
    Some(0x0063),
    Some(0x0064),
    Some(0x0065),
    Some(0x0066),
    Some(0x0067),
    Some(0x0068),
    Some(0x0069),
    Some(0x006A),
    Some(0x006B),
    Some(0x006C),
    Some(0x006D),
    Some(0x006E),
    Some(0x006F),
    Some(0x0070),
    Some(0x0071),
    Some(0x0072),
    Some(0x0073),
    Some(0x0074),
    Some(0x0075),
    Some(0x0076),
    Some(0x0077),
    Some(0x0078),
    Some(0x0079),
    Some(0x007A),
    Some(0x007B),
    Some(0x007C),
    Some(0x007D),
    Some(0x007E),
    None,
    // 0x80-0x9F: Windows-1252 specific
    Some(0x20AC),
    None,
    Some(0x201A),
    Some(0x0192),
    Some(0x201E),
    Some(0x2026),
    Some(0x2020),
    Some(0x2021),
    Some(0x02C6),
    Some(0x2030),
    Some(0x0160),
    Some(0x2039),
    Some(0x0152),
    None,
    Some(0x017D),
    None,
    None,
    Some(0x2018),
    Some(0x2019),
    Some(0x201C),
    Some(0x201D),
    Some(0x2022),
    Some(0x2013),
    Some(0x2014),
    Some(0x02DC),
    Some(0x2122),
    Some(0x0161),
    Some(0x203A),
    Some(0x0153),
    None,
    Some(0x017E),
    Some(0x0178),
    // 0xA0-0xBF: Latin supplement
    Some(0x00A0),
    Some(0x00A1),
    Some(0x00A2),
    Some(0x00A3),
    Some(0x00A4),
    Some(0x00A5),
    Some(0x00A6),
    Some(0x00A7),
    Some(0x00A8),
    Some(0x00A9),
    Some(0x00AA),
    Some(0x00AB),
    Some(0x00AC),
    Some(0x00AD),
    Some(0x00AE),
    Some(0x00AF),
    Some(0x00B0),
    Some(0x00B1),
    Some(0x00B2),
    Some(0x00B3),
    Some(0x00B4),
    Some(0x00B5),
    Some(0x00B6),
    Some(0x00B7),
    Some(0x00B8),
    Some(0x00B9),
    Some(0x00BA),
    Some(0x00BB),
    Some(0x00BC),
    Some(0x00BD),
    Some(0x00BE),
    Some(0x00BF),
    // 0xC0-0xDF: Latin extended
    Some(0x00C0),
    Some(0x00C1),
    Some(0x00C2),
    Some(0x00C3),
    Some(0x00C4),
    Some(0x00C5),
    Some(0x00C6),
    Some(0x00C7),
    Some(0x00C8),
    Some(0x00C9),
    Some(0x00CA),
    Some(0x00CB),
    Some(0x00CC),
    Some(0x00CD),
    Some(0x00CE),
    Some(0x00CF),
    Some(0x00D0),
    Some(0x00D1),
    Some(0x00D2),
    Some(0x00D3),
    Some(0x00D4),
    Some(0x00D5),
    Some(0x00D6),
    Some(0x00D7),
    Some(0x00D8),
    Some(0x00D9),
    Some(0x00DA),
    Some(0x00DB),
    Some(0x00DC),
    Some(0x00DD),
    Some(0x00DE),
    Some(0x00DF),
    // 0xE0-0xFF: Lowercase Latin extended
    Some(0x00E0),
    Some(0x00E1),
    Some(0x00E2),
    Some(0x00E3),
    Some(0x00E4),
    Some(0x00E5),
    Some(0x00E6),
    Some(0x00E7),
    Some(0x00E8),
    Some(0x00E9),
    Some(0x00EA),
    Some(0x00EB),
    Some(0x00EC),
    Some(0x00ED),
    Some(0x00EE),
    Some(0x00EF),
    Some(0x00F0),
    Some(0x00F1),
    Some(0x00F2),
    Some(0x00F3),
    Some(0x00F4),
    Some(0x00F5),
    Some(0x00F6),
    Some(0x00F7),
    Some(0x00F8),
    Some(0x00F9),
    Some(0x00FA),
    Some(0x00FB),
    Some(0x00FC),
    Some(0x00FD),
    Some(0x00FE),
    Some(0x00FF),
];

/// Standard Encoding - Adobe Standard Encoding
pub static STANDARD_ENCODING: CodedCharacterSet = [
    // 0x00-0x1F: Control characters
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    // 0x20-0x3F: Printable ASCII
    Some(0x0020),
    Some(0x0021),
    Some(0x0022),
    Some(0x0023),
    Some(0x0024),
    Some(0x0025),
    Some(0x0026),
    Some(0x2019),
    Some(0x0028),
    Some(0x0029),
    Some(0x002A),
    Some(0x002B),
    Some(0x002C),
    Some(0x002D),
    Some(0x002E),
    Some(0x002F),
    Some(0x0030),
    Some(0x0031),
    Some(0x0032),
    Some(0x0033),
    Some(0x0034),
    Some(0x0035),
    Some(0x0036),
    Some(0x0037),
    Some(0x0038),
    Some(0x0039),
    Some(0x003A),
    Some(0x003B),
    Some(0x003C),
    Some(0x003D),
    Some(0x003E),
    Some(0x003F),
    // 0x40-0x5F: Uppercase letters
    Some(0x0040),
    Some(0x0041),
    Some(0x0042),
    Some(0x0043),
    Some(0x0044),
    Some(0x0045),
    Some(0x0046),
    Some(0x0047),
    Some(0x0048),
    Some(0x0049),
    Some(0x004A),
    Some(0x004B),
    Some(0x004C),
    Some(0x004D),
    Some(0x004E),
    Some(0x004F),
    Some(0x0050),
    Some(0x0051),
    Some(0x0052),
    Some(0x0053),
    Some(0x0054),
    Some(0x0055),
    Some(0x0056),
    Some(0x0057),
    Some(0x0058),
    Some(0x0059),
    Some(0x005A),
    Some(0x005B),
    Some(0x005C),
    Some(0x005D),
    Some(0x005E),
    Some(0x005F),
    // 0x60-0x7F: Lowercase letters (0x60 = left quote, 0x27 = right quote)
    Some(0x2018),
    Some(0x0061),
    Some(0x0062),
    Some(0x0063),
    Some(0x0064),
    Some(0x0065),
    Some(0x0066),
    Some(0x0067),
    Some(0x0068),
    Some(0x0069),
    Some(0x006A),
    Some(0x006B),
    Some(0x006C),
    Some(0x006D),
    Some(0x006E),
    Some(0x006F),
    Some(0x0070),
    Some(0x0071),
    Some(0x0072),
    Some(0x0073),
    Some(0x0074),
    Some(0x0075),
    Some(0x0076),
    Some(0x0077),
    Some(0x0078),
    Some(0x0079),
    Some(0x007A),
    Some(0x007B),
    Some(0x007C),
    Some(0x007D),
    Some(0x007E),
    None,
    // 0x80-0x9F: Undefined
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    // 0xA0-0xBF: Extended (fraction slash, fi/fl ligatures, etc.)
    None,
    Some(0x00A1),
    Some(0x00A2),
    Some(0x00A3),
    Some(0x2044),
    Some(0x00A5),
    Some(0x0192),
    Some(0x00A7),
    Some(0x00A4),
    Some(0x0027),
    Some(0x201C),
    Some(0x00AB),
    Some(0x2039),
    Some(0x203A),
    Some(0xFB01),
    Some(0xFB02),
    None,
    Some(0x2013),
    Some(0x2020),
    Some(0x2021),
    Some(0x00B7),
    None,
    Some(0x00B6),
    Some(0x2022),
    Some(0x201A),
    Some(0x201E),
    Some(0x201D),
    Some(0x00BB),
    Some(0x2026),
    Some(0x2030),
    None,
    Some(0x00BF),
    // 0xC0-0xDF: Accents
    None,
    Some(0x0060),
    Some(0x00B4),
    Some(0x02C6),
    Some(0x02DC),
    Some(0x00AF),
    Some(0x02D8),
    Some(0x02D9),
    Some(0x00A8),
    None,
    Some(0x02DA),
    Some(0x00B8),
    None,
    Some(0x02DD),
    Some(0x02DB),
    Some(0x02C7),
    Some(0x2014),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    // 0xE0-0xFF: Special characters
    None,
    Some(0x00C6),
    None,
    Some(0x00AA),
    None,
    None,
    None,
    None,
    Some(0x0141),
    Some(0x00D8),
    Some(0x0152),
    Some(0x00BA),
    None,
    None,
    None,
    None,
    None,
    Some(0x00E6),
    None,
    None,
    None,
    Some(0x0131),
    None,
    None,
    Some(0x0142),
    Some(0x00F8),
    Some(0x0153),
    Some(0x00DF),
    None,
    None,
    None,
    None,
];

/// Mac Roman encoding
pub static MAC_ROMAN_ENCODING: CodedCharacterSet = [
    // 0x00-0x1F: Control characters
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    // 0x20-0x7F: Standard ASCII
    Some(0x0020),
    Some(0x0021),
    Some(0x0022),
    Some(0x0023),
    Some(0x0024),
    Some(0x0025),
    Some(0x0026),
    Some(0x0027),
    Some(0x0028),
    Some(0x0029),
    Some(0x002A),
    Some(0x002B),
    Some(0x002C),
    Some(0x002D),
    Some(0x002E),
    Some(0x002F),
    Some(0x0030),
    Some(0x0031),
    Some(0x0032),
    Some(0x0033),
    Some(0x0034),
    Some(0x0035),
    Some(0x0036),
    Some(0x0037),
    Some(0x0038),
    Some(0x0039),
    Some(0x003A),
    Some(0x003B),
    Some(0x003C),
    Some(0x003D),
    Some(0x003E),
    Some(0x003F),
    Some(0x0040),
    Some(0x0041),
    Some(0x0042),
    Some(0x0043),
    Some(0x0044),
    Some(0x0045),
    Some(0x0046),
    Some(0x0047),
    Some(0x0048),
    Some(0x0049),
    Some(0x004A),
    Some(0x004B),
    Some(0x004C),
    Some(0x004D),
    Some(0x004E),
    Some(0x004F),
    Some(0x0050),
    Some(0x0051),
    Some(0x0052),
    Some(0x0053),
    Some(0x0054),
    Some(0x0055),
    Some(0x0056),
    Some(0x0057),
    Some(0x0058),
    Some(0x0059),
    Some(0x005A),
    Some(0x005B),
    Some(0x005C),
    Some(0x005D),
    Some(0x005E),
    Some(0x005F),
    Some(0x0060),
    Some(0x0061),
    Some(0x0062),
    Some(0x0063),
    Some(0x0064),
    Some(0x0065),
    Some(0x0066),
    Some(0x0067),
    Some(0x0068),
    Some(0x0069),
    Some(0x006A),
    Some(0x006B),
    Some(0x006C),
    Some(0x006D),
    Some(0x006E),
    Some(0x006F),
    Some(0x0070),
    Some(0x0071),
    Some(0x0072),
    Some(0x0073),
    Some(0x0074),
    Some(0x0075),
    Some(0x0076),
    Some(0x0077),
    Some(0x0078),
    Some(0x0079),
    Some(0x007A),
    Some(0x007B),
    Some(0x007C),
    Some(0x007D),
    Some(0x007E),
    None,
    // 0x80-0xFF: Mac extended
    Some(0x00C4),
    Some(0x00C5),
    Some(0x00C7),
    Some(0x00C9),
    Some(0x00D1),
    Some(0x00D6),
    Some(0x00DC),
    Some(0x00E1),
    Some(0x00E0),
    Some(0x00E2),
    Some(0x00E4),
    Some(0x00E3),
    Some(0x00E5),
    Some(0x00E7),
    Some(0x00E9),
    Some(0x00E8),
    Some(0x00EA),
    Some(0x00EB),
    Some(0x00ED),
    Some(0x00EC),
    Some(0x00EE),
    Some(0x00EF),
    Some(0x00F1),
    Some(0x00F3),
    Some(0x00F2),
    Some(0x00F4),
    Some(0x00F6),
    Some(0x00F5),
    Some(0x00FA),
    Some(0x00F9),
    Some(0x00FB),
    Some(0x00FC),
    Some(0x2020),
    Some(0x00B0),
    Some(0x00A2),
    Some(0x00A3),
    Some(0x00A7),
    Some(0x2022),
    Some(0x00B6),
    Some(0x00DF),
    Some(0x00AE),
    Some(0x00A9),
    Some(0x2122),
    Some(0x00B4),
    Some(0x00A8),
    Some(0x2260),
    Some(0x00C6),
    Some(0x00D8),
    Some(0x221E),
    Some(0x00B1),
    Some(0x2264),
    Some(0x2265),
    Some(0x00A5),
    Some(0x00B5),
    Some(0x2202),
    Some(0x2211),
    Some(0x220F),
    Some(0x03C0),
    Some(0x222B),
    Some(0x00AA),
    Some(0x00BA),
    Some(0x03A9),
    Some(0x00E6),
    Some(0x00F8),
    Some(0x00BF),
    Some(0x00A1),
    Some(0x00AC),
    Some(0x221A),
    Some(0x0192),
    Some(0x2248),
    Some(0x2206),
    Some(0x00AB),
    Some(0x00BB),
    Some(0x2026),
    Some(0x00A0),
    Some(0x00C0),
    Some(0x00C3),
    Some(0x00D5),
    Some(0x0152),
    Some(0x0153),
    Some(0x2013),
    Some(0x2014),
    Some(0x201C),
    Some(0x201D),
    Some(0x2018),
    Some(0x2019),
    Some(0x00F7),
    Some(0x25CA),
    Some(0x00FF),
    Some(0x0178),
    Some(0x2044),
    Some(0x20AC),
    Some(0x2039),
    Some(0x203A),
    Some(0xFB01),
    Some(0xFB02),
    Some(0x2021),
    Some(0x00B7),
    Some(0x201A),
    Some(0x201E),
    Some(0x2030),
    Some(0x00C2),
    Some(0x00CA),
    Some(0x00C1),
    Some(0x00CB),
    Some(0x00C8),
    Some(0x00CD),
    Some(0x00CE),
    Some(0x00CF),
    Some(0x00CC),
    Some(0x00D3),
    Some(0x00D4),
    Some(0xF8FF),
    Some(0x00D2),
    Some(0x00DA),
    Some(0x00DB),
    Some(0x00D9),
    Some(0x0131),
    Some(0x02C6),
    Some(0x02DC),
    Some(0x00AF),
    Some(0x02D8),
    Some(0x02D9),
    Some(0x02DA),
    Some(0x00B8),
    Some(0x02DD),
    Some(0x02DB),
    Some(0x02C7),
];

// =============================================================================
// Encoding Types
// =============================================================================

/// Encoding types for PDF fonts
#[derive(Debug)]
pub enum Encoding {
    OneByteEncoding(&'static CodedCharacterSet),
    ToUnicodeMap(ToUnicodeMap),
    Identity,
}

/// Common ligature byte mappings for fonts without ToUnicode CMaps.
fn get_ligature_expansion(byte: u8) -> Option<&'static str> {
    match byte {
        // PostScript Type 1 fonts typically use these positions
        0x02 => Some("fi"),
        0x03 => Some("fl"),
        0x04 => Some("ff"),
        0x05 => Some("ffi"),
        0x06 => Some("ffl"),
        // Windows/Adobe standard positions
        0x1B => Some("ffl"),
        0x1C => Some("ffi"),
        0x1D => Some("ff"),
        0x1E => Some("fl"),
        0x1F => Some("fi"),
        _ => None,
    }
}

impl Encoding {
    /// Decode bytes to string using this encoding
    pub fn decode(&self, bytes: &[u8]) -> String {
        match self {
            Encoding::OneByteEncoding(map) => {
                let mut result = String::new();
                for &b in bytes {
                    if let Some(cp) = map[b as usize] {
                        if let Some(c) = char::from_u32(cp as u32) {
                            result.push(c);
                        } else {
                            result.push('\u{FFFD}');
                        }
                    } else if let Some(ligature) = get_ligature_expansion(b) {
                        result.push_str(ligature);
                    }
                }
                result
            }
            Encoding::ToUnicodeMap(cmap) => cmap.decode(bytes),
            Encoding::Identity => {
                if bytes.len() >= 2 {
                    let utf16: Vec<u16> = bytes
                        .chunks(2)
                        .map(|c| {
                            if c.len() == 2 {
                                u16::from_be_bytes([c[0], c[1]])
                            } else {
                                0xFFFD
                            }
                        })
                        .collect();
                    String::from_utf16_lossy(&utf16)
                } else {
                    String::new()
                }
            }
        }
    }
}

// =============================================================================
// ToUnicode CMap Parser
// =============================================================================

/// Simple ToUnicode CMap parser
#[derive(Debug, Default)]
pub struct ToUnicodeMap {
    /// Maps character codes to Unicode strings
    pub mappings: HashMap<u32, Vec<u16>>,
    /// Code space ranges (min, max)
    #[allow(dead_code)]
    code_spaces: Vec<(u32, u32)>,
}

impl ToUnicodeMap {
    /// Parse a ToUnicode CMap stream
    pub fn parse(data: &[u8]) -> Self {
        let mut map = ToUnicodeMap::default();
        let text = String::from_utf8_lossy(data);
        let lines: Vec<&str> = text.lines().collect();

        // Parse beginbfchar...endbfchar sections
        let mut in_bfchar = false;
        for line in &lines {
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
                    if let (Some(src), Some(dst)) = (
                        Self::parse_hex_code(parts[0]),
                        Self::parse_hex_string(parts[1]),
                    ) {
                        map.mappings.insert(src, dst);
                    }
                }
            }
        }

        // Parse beginbfrange...endbfrange sections
        let mut in_bfrange = false;
        for line in &lines {
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
                    if let (Some(start), Some(end)) = (
                        Self::parse_hex_code(parts[0]),
                        Self::parse_hex_code(parts[1]),
                    ) {
                        if parts[2].starts_with('[') {
                            let array_str: String = parts[2..].join(" ");
                            let dest_codes: Vec<&str> = array_str
                                .trim_matches(|c| c == '[' || c == ']')
                                .split_whitespace()
                                .collect();
                            for (i, code) in (start..=end).enumerate() {
                                if i < dest_codes.len() {
                                    if let Some(dst) = Self::parse_hex_string(dest_codes[i]) {
                                        map.mappings.insert(code, dst);
                                    }
                                }
                            }
                        } else if let Some(start_dst) = Self::parse_hex_string(parts[2]) {
                            if !start_dst.is_empty() {
                                let base = start_dst[0] as u32;
                                for (i, code) in (start..=end).enumerate() {
                                    let dst_char = (base + i as u32) as u16;
                                    map.mappings.insert(code, vec![dst_char]);
                                }
                            }
                        }
                    }
                }
            }
        }

        map
    }

    fn parse_hex_code(s: &str) -> Option<u32> {
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

    /// Decode bytes using this CMap
    pub fn decode(&self, bytes: &[u8]) -> String {
        let mut result = String::new();
        let mut i = 0;

        while i < bytes.len() {
            let mut found = false;

            // Try 2-byte code first
            if i + 1 < bytes.len() {
                let code2 = ((bytes[i] as u32) << 8) | (bytes[i + 1] as u32);
                if let Some(chars) = self.mappings.get(&code2) {
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
                if let Some(chars) = self.mappings.get(&code1) {
                    // Check for corrupted ligature mappings
                    if chars.len() == 1 && chars[0] == 0x0066 {
                        if let Some(ligature) = get_ligature_expansion(bytes[i]) {
                            result.push_str(ligature);
                            i += 1;
                            continue;
                        }
                    }
                    for &cp in chars {
                        if let Some(c) = char::from_u32(cp as u32) {
                            result.push(c);
                        }
                    }
                } else if let Some(ligature) = get_ligature_expansion(bytes[i]) {
                    result.push_str(ligature);
                } else if bytes[i] >= 0x20 {
                    result.push(bytes[i] as char);
                }
                i += 1;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_winansi_decode_ascii() {
        let encoding = Encoding::OneByteEncoding(&WIN_ANSI_ENCODING);
        let text = encoding.decode(b"Hello World");
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_winansi_decode_extended() {
        let encoding = Encoding::OneByteEncoding(&WIN_ANSI_ENCODING);
        // 0x80 = Euro sign
        let text = encoding.decode(&[0x80]);
        assert_eq!(text, "€");
    }

    #[test]
    fn test_ligature_expansion() {
        let encoding = Encoding::OneByteEncoding(&WIN_ANSI_ENCODING);
        // 0x02 should expand to "fi"
        let text = encoding.decode(&[0x02]);
        assert_eq!(text, "fi");
    }
}
