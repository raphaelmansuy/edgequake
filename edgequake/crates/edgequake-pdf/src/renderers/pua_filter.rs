//! Private Use Area (PUA) character filtering.
//!
//! PDFs often use Unicode Private Use Area code points for custom symbols
//! like bullets, ornaments, or font-specific glyphs. These appear as
//! garbage characters in text output and should be filtered.
//!
//! ## Algorithm
//!
//! Check if character is in any PUA range:
//! - BMP PUA: U+E000..U+F8FF
//! - Supplementary PUA-A: U+F0000..U+FFFFD
//! - Supplementary PUA-B: U+100000..U+10FFFD
//!
//! REF: pymupdf4llm document_layout.py:83-94 (omit_if_pua_char)

/// Check if a character is in the Unicode Private Use Area (PUA).
///
/// PUA characters are used by PDFs for custom glyphs (e.g., Wingdings bullets)
/// and should be filtered from text output to prevent garbage symbols.
pub fn is_pua_char(c: char) -> bool {
    let code_point = c as u32;
    matches!(
        code_point,
        0xE000..=0xF8FF       // BMP PUA
        | 0xF0000..=0xFFFFD   // Supplementary PUA-A
        | 0x100000..=0x10FFFD // Supplementary PUA-B
    )
}

/// Filter PUA characters from a text string.
///
/// Returns the input string with all PUA characters removed and
/// Unicode whitespace normalized to standard ASCII space.
/// OODA-15: Normalize non-breaking spaces, thin spaces, etc.
pub fn filter_pua(text: &str) -> String {
    text.chars()
        .filter(|&c| !is_pua_char(c))
        .map(|c| normalize_whitespace(c))
        .collect()
}

/// OODA-15: Normalize Unicode whitespace characters to ASCII space.
/// WHY: PDFs frequently use non-breaking spaces (U+00A0), thin spaces (U+2009),
/// and other Unicode space variants that cause comparison mismatches.
fn normalize_whitespace(c: char) -> char {
    match c {
        '\u{00A0}' // Non-breaking space
        | '\u{2000}' // En quad
        | '\u{2001}' // Em quad
        | '\u{2002}' // En space
        | '\u{2003}' // Em space
        | '\u{2004}' // Three-per-em space
        | '\u{2005}' // Four-per-em space
        | '\u{2006}' // Six-per-em space
        | '\u{2007}' // Figure space
        | '\u{2008}' // Punctuation space
        | '\u{2009}' // Thin space
        | '\u{200A}' // Hair space
        | '\u{200B}' // Zero-width space
        | '\u{202F}' // Narrow no-break space
        | '\u{205F}' // Medium mathematical space
        | '\u{3000}' // Ideographic space
        | '\u{FEFF}' // BOM / zero-width no-break space
        => ' ',
        _ => c,
    }
}

/// Filter PUA characters, returning None if the result is empty.
///
/// Useful for span rendering where empty text should be skipped entirely.
pub fn filter_pua_opt(text: &str) -> Option<String> {
    let filtered = filter_pua(text);
    if filtered.is_empty() {
        None
    } else {
        Some(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pua_detection_bmp() {
        assert!(is_pua_char('\u{E000}'));
        assert!(is_pua_char('\u{F000}'));
        assert!(is_pua_char('\u{F8FF}'));
    }

    #[test]
    fn test_pua_detection_supplementary_a() {
        assert!(is_pua_char('\u{F0000}'));
        assert!(is_pua_char('\u{F5555}'));
        assert!(is_pua_char('\u{FFFFD}'));
    }

    #[test]
    fn test_pua_detection_supplementary_b() {
        assert!(is_pua_char('\u{100000}'));
        assert!(is_pua_char('\u{105555}'));
        assert!(is_pua_char('\u{10FFFD}'));
    }

    #[test]
    fn test_non_pua_characters() {
        assert!(!is_pua_char('A'));
        assert!(!is_pua_char('z'));
        assert!(!is_pua_char('0'));
        assert!(!is_pua_char('\u{2022}')); // BULLET
        assert!(!is_pua_char('\u{00A9}')); // COPYRIGHT
        assert!(!is_pua_char('\u{2192}')); // RIGHTWARDS ARROW
        assert!(!is_pua_char('\u{20AC}')); // EURO SIGN
    }

    #[test]
    fn test_boundary_cases() {
        // Just before BMP PUA range (U+D7FF is last valid BMP char before surrogates)
        assert!(!is_pua_char('\u{D7FF}'));
        // Just after BMP PUA range
        assert!(!is_pua_char('\u{F900}')); // CJK Compatibility
                                           // Before supplementary PUA ranges
        assert!(!is_pua_char('\u{EFFFF}'));
    }

    #[test]
    fn test_filter_empty() {
        assert_eq!(filter_pua(""), "");
    }

    #[test]
    fn test_filter_no_pua() {
        assert_eq!(filter_pua("Hello World 123"), "Hello World 123");
    }

    #[test]
    fn test_filter_all_pua() {
        assert_eq!(filter_pua("\u{E000}\u{E001}\u{E002}"), "");
    }

    #[test]
    fn test_filter_mixed() {
        assert_eq!(
            filter_pua("Hello\u{E001}World\u{F000}Test"),
            "HelloWorldTest"
        );
    }

    #[test]
    fn test_filter_preserves_emoji() {
        assert_eq!(filter_pua("Hello World"), "Hello World");
    }

    #[test]
    fn test_common_pdf_pua_bullets() {
        // Wingdings bullets commonly used in PDFs
        let bullets = "\u{F0B7}\u{F0A7}\u{F0D8}";
        assert!(bullets.chars().all(is_pua_char));
        assert_eq!(filter_pua(bullets), "");
    }

    #[test]
    fn test_filter_pua_opt_empty() {
        assert_eq!(filter_pua_opt("\u{E000}\u{E001}"), None);
    }

    #[test]
    fn test_filter_pua_opt_some() {
        assert_eq!(filter_pua_opt("Hello\u{E001}"), Some("Hello".to_string()));
    }

    /// OODA-15: Test Unicode whitespace normalization
    #[test]
    fn test_normalize_whitespace() {
        // Non-breaking space → regular space
        assert_eq!(filter_pua("Hello\u{00A0}World"), "Hello World");
        // Thin space → regular space
        assert_eq!(filter_pua("Hello\u{2009}World"), "Hello World");
        // Em space → regular space
        assert_eq!(filter_pua("Hello\u{2003}World"), "Hello World");
        // BOM → space
        assert_eq!(filter_pua("Hello\u{FEFF}World"), "Hello World");
        // Regular space unchanged
        assert_eq!(filter_pua("Hello World"), "Hello World");
    }
}
