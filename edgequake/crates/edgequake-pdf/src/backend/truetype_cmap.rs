//! TrueType cmap table parsing for embedded fonts.
//!
//! **WHY this module exists:**
//! Subset TrueType fonts (like Calibri, Cambria from Microsoft Office PDFs)
//! embed their glyph→Unicode mapping in the font's cmap table, not in the
//! PDF encoding dictionary. This module parses the /FontFile2 stream to
//! extract the glyph ID to Unicode character mapping.
//!
//! **The Problem:**
//! When a PDF uses a subset font like "LHKJDD+Calibri-Bold":
//! - The PDF stream contains raw glyph IDs (e.g., 33, 34, 35...)
//! - There's no /Encoding or /ToUnicode in the font dictionary
//! - The font embeds a TrueType font in /FontFile2 with a cmap table
//! - We need to parse that cmap to map glyph ID → Unicode character
//!
//! **Solution:**
//! Use the ttf-parser crate to:
//! 1. Parse the TrueType font from /FontFile2 stream bytes
//! 2. Iterate the cmap table to build a reverse map (glyphID → char)
//! 3. Use this map in the Encoding::EmbeddedTrueType variant
//!
//! **References:**
//! - Apple TrueType: https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6cmap.html
//! - Microsoft OpenType: https://learn.microsoft.com/en-us/typography/opentype/spec/cmap
//! - ttf-parser: https://docs.rs/ttf-parser/latest/ttf_parser/

use std::collections::HashMap;
use tracing::{debug, trace};
use ttf_parser::Face;

/// Parse an embedded TrueType font to extract glyph ID → Unicode mapping.
///
/// # Arguments
/// * `font_data` - Raw bytes of the TrueType font (from /FontFile2 stream)
///
/// # Returns
/// * `Some(HashMap<u16, char>)` - Mapping from glyph ID to Unicode character
/// * `None` - If parsing failed or no cmap table found
///
/// # How it works
/// The cmap table in TrueType fonts maps Unicode code points to glyph IDs.
/// We need the **inverse**: glyph ID → Unicode. So we iterate all Unicode
/// code points in the Basic Multilingual Plane (U+0000 to U+FFFF) and
/// build a reverse lookup table.
///
/// **Why iterate BMP?**
/// For subset fonts, only a small portion of glyphs are included (typically
/// < 256). Iterating the full BMP is O(65536) which is fast, and we stop
/// early once we've found all glyphs the font contains.
pub fn parse_embedded_truetype(font_data: &[u8]) -> Option<HashMap<u16, char>> {
    // Parse the TrueType font
    let face = match Face::parse(font_data, 0) {
        Ok(f) => f,
        Err(e) => {
            debug!("Failed to parse TrueType font: {:?}", e);
            return None;
        }
    };

    // Get number of glyphs to know when we're done
    let num_glyphs = face.number_of_glyphs();
    if num_glyphs == 0 {
        debug!("TrueType font has no glyphs");
        return None;
    }

    trace!("Parsing TrueType font with {} glyphs", num_glyphs);

    // Build reverse map: glyph ID → Unicode character
    // We iterate all BMP code points and look up their glyph IDs
    let mut glyph_to_char: HashMap<u16, char> = HashMap::with_capacity(num_glyphs as usize);

    // Track how many glyphs we've mapped (for early exit)
    let mut mapped_count = 0;

    // Iterate Unicode BMP (U+0000 to U+FFFF)
    // Skip control characters and surrogates
    for code_point in 0u32..=0xFFFF {
        // Skip surrogates (D800-DFFF)
        if (0xD800..=0xDFFF).contains(&code_point) {
            continue;
        }

        // Convert to char
        let Some(c) = char::from_u32(code_point) else {
            continue;
        };

        // Look up glyph ID for this character
        if let Some(glyph_id) = face.glyph_index(c) {
            let gid = glyph_id.0;

            // Only add if not already mapped (first wins for duplicates)
            // Skip glyph 0 (notdef/missing glyph)
            if gid != 0 && !glyph_to_char.contains_key(&gid) {
                glyph_to_char.insert(gid, c);
                mapped_count += 1;

                // Early exit if we've found all glyphs
                // (num_glyphs includes .notdef, so subtract 1)
                if mapped_count >= num_glyphs.saturating_sub(1) {
                    trace!("Mapped all {} glyphs, stopping early", mapped_count);
                    break;
                }
            }
        }
    }

    if glyph_to_char.is_empty() {
        debug!("No glyph mappings found in TrueType font");
        return None;
    }

    debug!(
        "Parsed {} glyph→char mappings from embedded TrueType font",
        glyph_to_char.len()
    );

    // Log some sample mappings for debugging
    if tracing::enabled!(tracing::Level::TRACE) {
        let samples: Vec<_> = glyph_to_char
            .iter()
            .take(10)
            .map(|(gid, c)| format!("{}→{:?}", gid, c))
            .collect();
        trace!("Sample mappings: {}", samples.join(", "));
    }

    Some(glyph_to_char)
}

/// Decode bytes using a glyph ID to character map.
///
/// For subset TrueType fonts, the raw bytes in the PDF stream are
/// glyph indices (not character codes). This function converts them
/// to the corresponding Unicode characters.
///
/// # Arguments
/// * `bytes` - Raw bytes from PDF text stream (interpreted as glyph IDs)
/// * `glyph_map` - Mapping from glyph ID to Unicode character
///
/// # Returns
/// Decoded string, with unmapped glyphs replaced by replacement char (U+FFFD)
pub fn decode_with_glyph_map(bytes: &[u8], glyph_map: &HashMap<u16, char>) -> String {
    let mut result = String::with_capacity(bytes.len());

    for &byte in bytes {
        let glyph_id = byte as u16;
        if let Some(&c) = glyph_map.get(&glyph_id) {
            result.push(c);
        } else {
            // Unknown glyph - use replacement character
            result.push('\u{FFFD}');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_with_glyph_map() {
        let mut map = HashMap::new();
        map.insert(33, 'T');
        map.insert(34, 'a');
        map.insert(35, 'b');
        map.insert(36, 'l');
        map.insert(37, 'e');

        // "Table" as glyph IDs: T=33, a=34, b=35, l=36, e=37
        let bytes = [33, 34, 35, 36, 37];
        let result = decode_with_glyph_map(&bytes, &map);
        assert_eq!(result, "Table");
    }

    #[test]
    fn test_decode_with_missing_glyphs() {
        let mut map = HashMap::new();
        map.insert(33, 'A');
        map.insert(34, 'B');

        // Byte 35 is not in the map
        let bytes = [33, 35, 34];
        let result = decode_with_glyph_map(&bytes, &map);
        assert_eq!(result, "A\u{FFFD}B");
    }

    #[test]
    fn test_empty_glyph_map() {
        let map = HashMap::new();
        let bytes = [33, 34, 35];
        let result = decode_with_glyph_map(&bytes, &map);
        assert_eq!(result, "\u{FFFD}\u{FFFD}\u{FFFD}");
    }

    // Note: We can't easily test parse_embedded_truetype without an actual
    // TrueType font file. Integration tests in fast_quality/ will cover this.
}
