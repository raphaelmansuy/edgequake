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
/// Returns the input string with all PUA characters removed,
/// Unicode whitespace normalized to standard ASCII space,
/// smart quotes straightened, and ligatures decomposed.
/// OODA-15: Normalize non-breaking spaces, thin spaces, etc.
/// OODA-17: Decompose common ligatures (fi, fl, ff, ffi, ffl).
/// OODA-34: Collapse multiple consecutive spaces to single space.
pub fn filter_pua(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        if is_pua_char(c) {
            continue;
        }
        // OODA-17: Decompose ligatures before character normalization
        match c {
            '\u{FB00}' => result.push_str("ff"),
            '\u{FB01}' => result.push_str("fi"),
            '\u{FB02}' => result.push_str("fl"),
            '\u{FB03}' => result.push_str("ffi"),
            '\u{FB04}' => result.push_str("ffl"),
            '\u{FB05}' | '\u{FB06}' => result.push_str("st"),
            _ => {
                // OODA-18: Use normalize_char for extended normalization
                match normalize_char(c) {
                    Some(NormResult::Char(normalized)) => result.push(normalized),
                    Some(NormResult::Str(s)) => result.push_str(s),
                    Some(NormResult::Skip) => {}
                    None => result.push(c),
                }
            }
        }
    }
    // OODA-34: Collapse multiple consecutive spaces to a single space
    // WHY: After normalization, various Unicode spaces may produce runs of ASCII spaces.
    // This also handles original double-spaces from PDF text extraction.
    collapse_spaces(&mut result);
    result
}

/// OODA-34: Collapse runs of multiple spaces to a single space.
/// Preserves leading/trailing whitespace structure (for code block indentation).
fn collapse_spaces(text: &mut String) {
    if !text.contains("  ") {
        return; // Fast path: no double spaces
    }
    let mut collapsed = String::with_capacity(text.len());
    let mut prev_was_space = false;
    for c in text.chars() {
        if c == ' ' {
            if !prev_was_space {
                collapsed.push(' ');
            }
            prev_was_space = true;
        } else {
            prev_was_space = false;
            collapsed.push(c);
        }
    }
    *text = collapsed;
}

/// OODA-15: Normalize Unicode whitespace characters to ASCII space.
/// WHY: PDFs frequently use non-breaking spaces (U+00A0), thin spaces (U+2009),
/// and other Unicode space variants that cause comparison mismatches.
/// OODA-18: Also handles ellipsis, fraction chars, and zero-width joiners.
fn normalize_char(c: char) -> Option<NormResult> {
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
        => Some(NormResult::Char(' ')),
        // OODA-18: Strip zero-width joiners and soft hyphens
        '\u{200C}' // Zero-width non-joiner
        | '\u{200D}' // Zero-width joiner
        | '\u{00AD}' // Soft hyphen
        => Some(NormResult::Skip),
        // OODA-16: Normalize smart quotes to straight quotes
        '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => Some(NormResult::Char('\'')), // Single quotes
        '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => Some(NormResult::Char('"')),  // Double quotes
        // Normalize Unicode dashes to ASCII hyphen-minus
        '\u{2010}' | '\u{2011}' => Some(NormResult::Char('-')), // Hyphen and non-breaking hyphen
        '\u{2212}' => Some(NormResult::Char('-')), // Minus sign
        // OODA-18: Ellipsis → three dots
        '\u{2026}' => Some(NormResult::Str("...")),
        // OODA-18: Common fractions → text form
        '\u{00BC}' => Some(NormResult::Str("1/4")),
        '\u{00BD}' => Some(NormResult::Str("1/2")),
        '\u{00BE}' => Some(NormResult::Str("3/4")),
        '\u{2153}' => Some(NormResult::Str("1/3")),
        '\u{2154}' => Some(NormResult::Str("2/3")),
        // OODA-25: Copyright, trademark, registered symbols → ASCII equivalents
        '\u{00A9}' => Some(NormResult::Str("(c)")),
        '\u{00AE}' => Some(NormResult::Str("(R)")),
        '\u{2122}' => Some(NormResult::Str("(TM)")),
        // OODA-25: Superscript digits → regular digits (common in PDF footnote refs)
        '\u{00B2}' => Some(NormResult::Char('2')), // Superscript 2
        '\u{00B3}' => Some(NormResult::Char('3')), // Superscript 3
        '\u{00B9}' => Some(NormResult::Char('1')), // Superscript 1
        // OODA-25: Common mathematical symbols
        '\u{00D7}' => Some(NormResult::Char('x')), // Multiplication sign → x
        '\u{00F7}' => Some(NormResult::Char('/')), // Division sign → /
        '\u{2260}' => Some(NormResult::Str("!=")),  // Not equal
        '\u{2264}' => Some(NormResult::Str("<=")),   // Less than or equal
        '\u{2265}' => Some(NormResult::Str(">=")),   // Greater than or equal
        '\u{2248}' => Some(NormResult::Str("~=")),   // Almost equal
        _ => None,
    }
}

/// Result of character normalization.
enum NormResult {
    Char(char),
    Str(&'static str),
    Skip,
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

    /// OODA-16: Test smart quote and dash normalization
    #[test]
    fn test_normalize_quotes_dashes() {
        // Smart quotes → straight quotes
        assert_eq!(filter_pua("\u{201C}Hello\u{201D}"), "\"Hello\"");
        assert_eq!(filter_pua("\u{2018}world\u{2019}"), "'world'");
        // Unicode hyphens → ASCII hyphen
        assert_eq!(filter_pua("well\u{2010}known"), "well-known");
        assert_eq!(filter_pua("a \u{2212} b"), "a - b");
    }

    /// OODA-17: Test ligature decomposition
    #[test]
    fn test_decompose_ligatures() {
        assert_eq!(filter_pua("e\u{FB03}cient"), "efficient");
        assert_eq!(filter_pua("\u{FB01}rst"), "first");
        assert_eq!(filter_pua("\u{FB02}ow"), "flow");
        assert_eq!(filter_pua("o\u{FB00}er"), "offer");
        assert_eq!(filter_pua("mu\u{FB04}e"), "muffle");
    }

    /// OODA-18: Test ellipsis, fraction, and zero-width char normalization
    #[test]
    fn test_normalize_ellipsis_fractions() {
        // Horizontal ellipsis → three dots
        assert_eq!(filter_pua("Wait\u{2026}"), "Wait...");
        assert_eq!(filter_pua("a\u{2026}z"), "a...z");
        // Fractions → text form
        assert_eq!(filter_pua("\u{00BD} cup"), "1/2 cup");
        assert_eq!(filter_pua("\u{00BC}"), "1/4");
        assert_eq!(filter_pua("\u{00BE}"), "3/4");
        assert_eq!(filter_pua("\u{2153}"), "1/3");
        assert_eq!(filter_pua("\u{2154}"), "2/3");
    }

    /// OODA-18: Test zero-width and soft hyphen stripping
    #[test]
    fn test_strip_zero_width() {
        // Zero-width joiner stripped
        assert_eq!(filter_pua("ab\u{200D}cd"), "abcd");
        // Zero-width non-joiner stripped
        assert_eq!(filter_pua("ab\u{200C}cd"), "abcd");
        // Soft hyphen stripped
        assert_eq!(filter_pua("hyphen\u{00AD}ation"), "hyphenation");
    }

    /// OODA-25: Test copyright, trademark, and math symbol normalization
    #[test]
    fn test_normalize_symbols() {
        assert_eq!(filter_pua("\u{00A9} 2024"), "(c) 2024");
        assert_eq!(filter_pua("Brand\u{00AE}"), "Brand(R)");
        assert_eq!(filter_pua("Name\u{2122}"), "Name(TM)");
        // Superscript digits
        assert_eq!(filter_pua("x\u{00B2}"), "x2");
        assert_eq!(filter_pua("n\u{00B3}"), "n3");
        // Math symbols
        assert_eq!(filter_pua("a \u{2260} b"), "a != b");
        assert_eq!(filter_pua("x \u{2264} 5"), "x <= 5");
        assert_eq!(filter_pua("3 \u{00D7} 4"), "3 x 4");
    }

    /// OODA-34: Test multiple space collapsing
    #[test]
    fn test_collapse_multiple_spaces() {
        assert_eq!(filter_pua("hello  world"), "hello world");
        assert_eq!(filter_pua("a   b   c"), "a b c");
        // Multiple Unicode spaces → single space
        assert_eq!(filter_pua("a\u{00A0}\u{00A0}b"), "a b");
        // Single space unchanged
        assert_eq!(filter_pua("hello world"), "hello world");
        // No spaces at all
        assert_eq!(filter_pua("helloworld"), "helloworld");
    }
}
