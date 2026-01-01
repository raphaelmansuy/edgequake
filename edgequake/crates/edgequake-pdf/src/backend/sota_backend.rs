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
    TextSpan, Point,
};
use crate::{DocumentMetadata, Result};

use lopdf::content::Content;
use lopdf::{Dictionary, Document as LopdfDocument, Object, ObjectId, Stream};

// Re-export encoding types we need
mod encodings {
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
        Some(0x0192), // Euro, ‚, ƒ
        Some(0x201E),
        Some(0x2026),
        Some(0x2020),
        Some(0x2021), // „, …, †, ‡
        Some(0x02C6),
        Some(0x2030),
        Some(0x0160),
        Some(0x2039), // ˆ, ‰, Š, ‹
        Some(0x0152),
        None,
        Some(0x017D),
        None, // Œ, Ž
        None,
        Some(0x2018),
        Some(0x2019),
        Some(0x201C), // ', ', "
        Some(0x201D),
        Some(0x2022),
        Some(0x2013),
        Some(0x2014), // ", •, –, —
        Some(0x02DC),
        Some(0x2122),
        Some(0x0161),
        Some(0x203A), // ˜, ™, š, ›
        Some(0x0153),
        None,
        Some(0x017E),
        Some(0x0178), // œ, ž, Ÿ
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
        Some(0x2019), // 0x27 = right quote
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
        Some(0x2018),
        Some(0x0061),
        Some(0x0062),
        Some(0x0063), // 0x60 = left quote
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
        Some(0x00A7), // 0xA4 = fraction slash
        Some(0x00A4),
        Some(0x0027),
        Some(0x201C),
        Some(0x00AB),
        Some(0x2039),
        Some(0x203A),
        Some(0xFB01),
        Some(0xFB02), // fi, fl ligatures
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

    /// Encoding types for PDF fonts
    #[derive(Debug)]
    pub enum Encoding {
        OneByteEncoding(&'static CodedCharacterSet),
        ToUnicodeMap(ToUnicodeMap),
        Identity,
    }

    impl Encoding {
        /// Decode bytes to string using this encoding
        pub fn decode(&self, bytes: &[u8]) -> String {
            match self {
                Encoding::OneByteEncoding(map) => bytes
                    .iter()
                    .filter_map(|&b| map[b as usize])
                    .map(|cp| char::from_u32(cp as u32).unwrap_or('\u{FFFD}'))
                    .collect(),
                Encoding::ToUnicodeMap(cmap) => cmap.decode(bytes),
                Encoding::Identity => {
                    // Identity-H: treat as UTF-16BE
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

    /// Simple ToUnicode CMap parser
    #[derive(Debug, Default)]
    pub struct ToUnicodeMap {
        /// Maps character codes to Unicode strings
        mappings: HashMap<u32, Vec<u16>>,
        /// Code space ranges (min, max)
        #[allow(dead_code)]
        code_spaces: Vec<(u32, u32)>,
    }

    impl ToUnicodeMap {
        /// Parse a ToUnicode CMap stream
        pub fn parse(data: &[u8]) -> Self {
            let mut map = ToUnicodeMap::default();
            let text = String::from_utf8_lossy(data);

            // Parse beginbfchar...endbfchar sections
            let mut in_bfchar = false;
            let lines: Vec<&str> = text.lines().collect();

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
                    // Format: <srcCode> <dstCode>
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
                    // Format: <start> <end> <startDest> or <start> <end> [<dest1> <dest2> ...]
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        if let (Some(start), Some(end)) = (
                            Self::parse_hex_code(parts[0]),
                            Self::parse_hex_code(parts[1]),
                        ) {
                            if parts[2].starts_with('[') {
                                // Array of destinations
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
                            } else {
                                // Single destination with increment
                                if let Some(start_dst) = Self::parse_hex_string(parts[2]) {
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
                // Try 2-byte code first, then 1-byte
                let mut found = false;

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
                        for &cp in chars {
                            if let Some(c) = char::from_u32(cp as u32) {
                                result.push(c);
                            }
                        }
                    } else {
                        // Fallback: treat as Latin-1
                        result.push(bytes[i] as char);
                    }
                    i += 1;
                }
            }

            result
        }
    }
}

use encodings::Encoding;

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
        let is_bold = lower_name.contains("bold")
            || lower_name.contains("black")
            || lower_name.contains("heavy");
        let is_italic = lower_name.contains("italic") || lower_name.contains("oblique");

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
                    return Encoding::ToUnicodeMap(encodings::ToUnicodeMap::parse(&data));
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

use super::elements::{TextElement, PdfLine};
use super::lattice::LatticeEngine;

/// SOTA PDF backend with proper encoding support
pub struct SotaBackend {
    config: PdfConfig,
    lattice_engine: LatticeEngine,
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
                        let a = ctm[0]; let b = ctm[1];
                        let c = ctm[2]; let d = ctm[3];
                        let e = ctm[4]; let f = ctm[5];
                        
                        let a_p = new_matrix[0]; let b_p = new_matrix[1];
                        let c_p = new_matrix[2]; let d_p = new_matrix[3];
                        let e_p = new_matrix[4]; let f_p = new_matrix[5];

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
                        let p1 = (x * ctm[0] + y * ctm[2] + ctm[4], x * ctm[1] + y * ctm[3] + ctm[5]);
                        let p2 = ((x+w) * ctm[0] + y * ctm[2] + ctm[4], (x+w) * ctm[1] + y * ctm[3] + ctm[5]);
                        let p3 = ((x+w) * ctm[0] + (y+h) * ctm[2] + ctm[4], (x+w) * ctm[1] + (y+h) * ctm[3] + ctm[5]);
                        let p4 = (x * ctm[0] + (y+h) * ctm[2] + ctm[4], x * ctm[1] + (y+h) * ctm[3] + ctm[5]);

                        line_elements.push(PdfLine { p1, p2, width: line_width });
                        line_elements.push(PdfLine { p1: p2, p2: p3, width: line_width });
                        line_elements.push(PdfLine { p1: p3, p2: p4, width: line_width });
                        line_elements.push(PdfLine { p1: p4, p2: p1, width: line_width });
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
                                        // Negative values > 100 indicate word space
                                        if *n < -100 {
                                            combined_text.push(' ');
                                        }
                                    }
                                    Object::Real(n) => {
                                        if *n < -100.0 {
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

                                text_elements.push(TextElement {
                                    text: combined_text.replace('\n', "").replace('\r', ""),
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
            Object::Real(f) => Some(*f as f32),
            _ => None,
        }
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
            for bin in start_bin..=end_bin.min(num_bins - 1) {
                proj[bin] += 1;
            }
        }
        proj
    }

    /// Find gaps (valleys) in the projection histogram
    /// Returns midpoint positions of significant gaps
    fn find_projection_gaps(&self, proj: &[usize], bin_size: f32, min_gap_bins: usize) -> Vec<f32> {
        let mut gaps = Vec::new();
        let mut low_start: Option<usize> = None;

        // Calculate mean density for adaptive thresholding
        let total: usize = proj.iter().sum();
        let avg_density = if proj.is_empty() {
            0
        } else {
            total / proj.len()
        };
        let low_threshold = (avg_density as f32 * 0.2) as usize; // 20% of average = "empty"

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

    /// Detect if page has two-column layout using projection histogram
    /// Returns Some(column_boundary_x) if two-column layout detected, None otherwise
    fn detect_columns(&self, elements: &[TextElement], page_width: f32) -> Option<f32> {
        if elements.len() < 10 {
            return None;
        }

        // Use projection histogram approach from spec_algo.md
        let bin_size = 5.0; // 5pt bins for fine granularity
        let proj = self.compute_vertical_projection(elements, page_width, bin_size);

        // Find gaps - minimum gap of 4 bins (20pt) for column separator
        let gaps = self.find_projection_gaps(&proj, bin_size, 4);

        // Look for a gap near the center of the page (column boundary)
        // In academic papers, the gutter is typically around 45-55% of page width
        let center = page_width / 2.0;
        let center_range = page_width * 0.15; // ±15% from center

        let center_gap = gaps
            .iter()
            .find(|&&gap| (gap - center).abs() < center_range);

        if let Some(&boundary) = center_gap {
            // Verify with element distribution
            let left_count = elements.iter().filter(|e| e.x < boundary - 10.0).count();
            let right_count = elements.iter().filter(|e| e.x > boundary + 10.0).count();

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

        // If global check failed, try checking only the bottom 75% of the page
        // This handles pages with full-width headers/abstracts but two-column body
        let max_y = elements.iter().map(|e| e.y).fold(f32::NEG_INFINITY, f32::max);
        let min_y = elements.iter().map(|e| e.y).fold(f32::INFINITY, f32::min);
        let page_height_content = max_y - min_y;
        
        // Only try this if we have enough vertical content
        if page_height_content > 200.0 {
            let threshold_y = max_y - (page_height_content * 0.25);
            let bottom_elements: Vec<TextElement> = elements.iter()
                .filter(|e| e.y < threshold_y)
                .cloned()
                .collect();
                
            if bottom_elements.len() > 20 {
                let proj_bottom = self.compute_vertical_projection(&bottom_elements, page_width, bin_size);
                let gaps_bottom = self.find_projection_gaps(&proj_bottom, bin_size, 4);
                
                let center_gap_bottom = gaps_bottom
                    .iter()
                    .find(|&&gap| (gap - center).abs() < center_range);
                    
                if let Some(&boundary) = center_gap_bottom {
                    // Verify with bottom element distribution
                    let left_count = bottom_elements.iter().filter(|e| e.x < boundary - 10.0).count();
                    let right_count = bottom_elements.iter().filter(|e| e.x > boundary + 10.0).count();
                    
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

        // Sort by Y (descending = top first)
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
        (self.group_single_column_layout(elements), Vec::new())
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

        // Define content regions (filter header/footer)
        // Academic papers typically have:
        // - Footer: Y < 60 (page numbers at ~52) - keep threshold low to avoid capturing body text
        // - Header: Y > 720 for 792pt page (running headers)
        // - Main body: 60 < Y < 720
        // Note: Page 1 may have affiliations at Y~106-126, but these should be processed
        // by the HeaderFooterDetectionProcessor later, not filtered here
        let footer_threshold = 60.0; // Only page numbers (body text can go down to Y~85)
        let header_threshold = 730.0; // Filter running headers
        let title_threshold = 650.0; // Title zone: Y > 650 for page titles
        
        // Threshold for affiliation/metadata region (above footer, below main body)
        // This captures institutional affiliations, email addresses, submission info
        let affiliation_threshold = 80.0; // Elements at Y < 80 but > footer are affiliations

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
            let is_header = elem.y > header_threshold && elem.font_size < 10.0;
            
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
            let is_large_font = elem.font_size > 11.0;
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
        result.extend(left_bottom);  // Bottom content after footer
        result.extend(right_bottom); // Bottom content after footer

        result
    }
    
    /// Split lines into main content and bottom-isolated content.
    /// If there's a vertical gap > threshold between content regions, the lower content is separated.
    fn split_by_vertical_gap(&self, lines: Vec<Vec<TextElement>>, gap_threshold: f32) -> (Vec<Vec<TextElement>>, Vec<Vec<TextElement>>) {
        if lines.len() < 2 {
            return (lines, Vec::new());
        }
        
        // Find lines' Y positions (use first element's Y as representative)
        let y_positions: Vec<f32> = lines.iter()
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
            debug!("Found vertical gap of {:.1}pt at line {}, splitting column content", max_gap, split_idx);
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

        // Sort by Y (descending = top first)
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

    /// Merge line elements into text with proper spacing
    fn merge_line(&self, elements: &[TextElement]) -> (String, f32, String, bool, bool) {
        if elements.is_empty() {
            return (String::new(), 12.0, String::new(), false, false);
        }

        let mut text = String::new();
        let avg_font_size =
            elements.iter().map(|e| e.font_size).sum::<f32>() / elements.len() as f32;
        let font_name = elements
            .first()
            .map(|e| e.font_name.clone())
            .unwrap_or_default();
        let is_bold = elements.iter().any(|e| e.is_bold);
        let is_italic = elements.iter().any(|e| e.is_italic);

        // Estimate average character width
        let avg_char_width = avg_font_size * 0.5;
        let space_threshold = avg_char_width * 1.5;

        for (i, elem) in elements.iter().enumerate() {
            if i > 0 {
                let prev = &elements[i - 1];
                // Estimate previous element's end position
                let prev_end = prev.x + (prev.text.len() as f32 * avg_char_width);
                let gap = elem.x - prev_end;

                if gap > space_threshold {
                    text.push(' ');
                }
            }
            text.push_str(&elem.text);
        }

        (text, avg_font_size, font_name, is_bold, is_italic)
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
            debug!(
                "  Block input line {}: Y={:.1} X={:.1} '{}'",
                i,
                y,
                x,
                &text[..text.len().min(40)]
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
        let body_size = font_size_counts
            .iter()
            .max_by_key(|&(_, count)| count)
            .map(|(&size, _)| size as f32 / 10.0)
            .unwrap_or(12.0);

        // Track text occurrences for running header detection
        let mut text_occurrences: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let line_texts: Vec<(String, f32, String, bool, bool)> =
            lines.iter().map(|line| self.merge_line(line)).collect();

        for (text, _, _, _, _) in &line_texts {
            let normalized = text.trim().to_lowercase();
            if !normalized.is_empty() && normalized.len() < 100 {
                *text_occurrences.entry(normalized).or_insert(0) += 1;
            }
        }

        // Section pattern regex
        let _section_pattern = regex::Regex::new(r"^(\d+\.)+\s+[A-Z]").ok();

        for (idx, line) in lines.iter().enumerate() {
            if line.is_empty() {
                continue;
            }

            let (text, font_size, font_name, is_bold, is_italic) = &line_texts[idx];
            let text = text.trim();
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

            let bbox = BoundingBox::new(min_x, y, max_x, y + *font_size);

            // Detect block type
            let normalized = text.trim().to_lowercase();
            let is_running_header = text_occurrences.get(&normalized).copied().unwrap_or(0) >= 3;

            let block_type = if is_running_header {
                BlockType::PageHeader
            } else if *font_size > body_size * 1.3 || *is_bold {
                // All headers use SectionHeader with level
                BlockType::SectionHeader
            } else if text.starts_with("• ")
                || text.starts_with("- ")
                || regex::Regex::new(r"^\d+[\.\)]\s")
                    .map(|r| r.is_match(text))
                    .unwrap_or(false)
            {
                BlockType::ListItem
            } else {
                BlockType::Text
            };

            if text.contains("thei") {
                debug!("Creating block with 'thei': '{}'", text);
            }

            let level = if block_type == BlockType::SectionHeader {
                Some(self.calculate_header_level(*font_size, body_size))
            } else {
                None
            };

            // Create span with style
            let span = TextSpan {
                text: text.to_string(),
                bbox: Some(bbox.clone()),
                style: FontStyle {
                    family: Some(font_name.clone()),
                    size: Some(*font_size),
                    weight: if *is_bold { Some(700) } else { Some(400) },
                    italic: *is_italic,
                    ..Default::default()
                },
            };

            let block = Block {
                id: BlockId::with_indices(0, blocks.len()),
                block_type,
                text: text.to_string(),
                bbox,
                page: 0,
                position: blocks.len(),
                level,
                spans: vec![span],
                ..Default::default()
            };

            blocks.push(block);
        }

        blocks
    }

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
        let (width, height) = self.get_page_dimensions(doc, page_id)?;

        // Get fonts
        let fonts = self.get_page_fonts(doc, page_id).unwrap_or_default();
        debug!("Page {} has {} fonts", page_num, fonts.len());

        // Get content
        let content_bytes = self.get_page_content(doc, page_id)?;

        // Extract text and graphical elements
        let (elements, pdf_lines) = self.extract_page_elements(&content_bytes, &fonts)?;
        debug!("Page {} has {} text elements and {} graphical lines", page_num, elements.len(), pdf_lines.len());

        // Detect tables
        let tables = self.lattice_engine.detect_tables(&pdf_lines, &elements, width, height);
        debug!("Page {} has {} detected tables", page_num, tables.len());

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
        let (lines, columns) = self.group_into_lines(non_table_elements, width, height);
        debug!(
            "Page {} has {} lines, {} columns detected",
            page_num,
            lines.len(),
            columns.len()
        );

        // Convert to blocks
        let mut blocks = self.lines_to_blocks(lines, width, height);
        
        // Add tables
        blocks.extend(tables);
        
        // Sort blocks by Y (top to bottom) - PDF Y is up, so sort descending by max Y (top)
        blocks.sort_by(|a, b| b.bbox.y2.partial_cmp(&a.bbox.y2).unwrap_or(std::cmp::Ordering::Equal));

        let char_count: usize = blocks.iter().map(|b| b.text.len()).sum();
        let word_count: usize = blocks
            .iter()
            .map(|b| b.text.split_whitespace().count())
            .sum();

        let mut page = Page::new(page_num, width, height);
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
            return Err(PdfError::PdfParse("PDF is encrypted and password-protected".to_string()));
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
            return Err(PdfError::PdfParse("PDF is encrypted and password-protected".to_string()));
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
