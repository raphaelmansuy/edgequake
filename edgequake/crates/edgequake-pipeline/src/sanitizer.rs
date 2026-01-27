//! Text sanitization for document processing.
//!
//! @implements SPEC-001/Issue-9: Emoji and special character handling
//!
//! # Implements
//!
//! - **FEAT0903**: Text sanitization before LLM extraction
//! - **FEAT0904**: Unicode normalization (NFC)
//! - **FEAT0905**: Emoji handling (preserve or strip)
//! - **FEAT0906**: Control character sanitization
//!
//! # WHY Text Sanitization?
//!
//! LLM extraction can fail or produce corrupted output when input contains:
//! - Emojis (can confuse tokenizers)
//! - Control characters (invisible, break formatting)
//! - RTL markers (can flip text direction in output)
//! - Non-normalized Unicode (same character, different byte sequences)
//!
//! This module provides a configurable sanitization pipeline that runs
//! before text is sent to the LLM, ensuring consistent, clean input.
//!
//! # Example
//!
//! ```rust
//! use edgequake_pipeline::sanitizer::{Sanitizer, SanitizeConfig};
//!
//! let config = SanitizeConfig::default();
//! let sanitizer = Sanitizer::new(config);
//!
//! let dirty = "Hello 👋 World!\u{200B}";  // Contains emoji and zero-width space
//! let clean = sanitizer.sanitize(dirty);
//! assert_eq!(clean, "Hello 👋 World!");   // Zero-width space removed
//! ```

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

/// Configuration for text sanitization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizeConfig {
    /// Apply Unicode NFC normalization.
    ///
    /// WHY: Different byte sequences can represent the same character
    /// (e.g., é as single codepoint vs e + combining accent). NFC
    /// normalizes to composed form for consistent processing.
    pub normalize_unicode: bool,

    /// How to handle emojis.
    pub emoji_mode: EmojiMode,

    /// Remove control characters (U+0000-U+001F, U+007F-U+009F).
    ///
    /// WHY: Control chars are invisible and can break LLM output parsing.
    /// Exception: \n, \r, \t are preserved as they're meaningful whitespace.
    pub remove_control_chars: bool,

    /// Remove zero-width characters (ZWJ, ZWSP, etc.).
    ///
    /// WHY: Zero-width chars are invisible but affect text processing.
    /// Can cause unexpected tokenization or join/break behavior.
    pub remove_zero_width: bool,

    /// Remove RTL/LTR directional markers.
    ///
    /// WHY: Bidirectional markers can flip text direction in LLM output,
    /// causing garbled entity names.
    pub remove_directional_markers: bool,

    /// Replace multiple consecutive whitespace with single space.
    pub collapse_whitespace: bool,

    /// Maximum allowed consecutive newlines (0 = unlimited).
    ///
    /// WHY: Excessive blank lines waste tokens without adding meaning.
    pub max_consecutive_newlines: usize,
}

/// How to handle emojis in text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmojiMode {
    /// Keep emojis as-is.
    Preserve,
    /// Remove all emojis.
    Remove,
    /// Replace emojis with placeholder text [EMOJI].
    ReplaceWithPlaceholder,
}

impl Default for SanitizeConfig {
    fn default() -> Self {
        Self {
            normalize_unicode: true,
            emoji_mode: EmojiMode::Preserve,
            remove_control_chars: true,
            remove_zero_width: true,
            remove_directional_markers: true,
            collapse_whitespace: false,
            max_consecutive_newlines: 3,
        }
    }
}

/// Text sanitizer for preparing documents for LLM extraction.
#[derive(Debug, Clone)]
pub struct Sanitizer {
    config: SanitizeConfig,
}

impl Default for Sanitizer {
    fn default() -> Self {
        Self::new(SanitizeConfig::default())
    }
}

impl Sanitizer {
    /// Create a new sanitizer with the given configuration.
    pub fn new(config: SanitizeConfig) -> Self {
        Self { config }
    }

    /// Sanitize text according to configuration.
    ///
    /// Returns sanitized text and a log of changes made.
    pub fn sanitize(&self, input: &str) -> String {
        let mut text = input.to_string();

        // Step 1: Unicode normalization (NFC)
        if self.config.normalize_unicode {
            text = text.nfc().collect();
        }

        // Step 2: Handle emojis
        text = match self.config.emoji_mode {
            EmojiMode::Preserve => text,
            EmojiMode::Remove => remove_emojis(&text),
            EmojiMode::ReplaceWithPlaceholder => replace_emojis_with_placeholder(&text),
        };

        // Step 3: Remove control characters (except \n, \r, \t)
        if self.config.remove_control_chars {
            text = remove_control_chars(&text);
        }

        // Step 4: Remove zero-width characters
        if self.config.remove_zero_width {
            text = remove_zero_width_chars(&text);
        }

        // Step 5: Remove directional markers
        if self.config.remove_directional_markers {
            text = remove_directional_markers(&text);
        }

        // Step 6: Collapse whitespace
        if self.config.collapse_whitespace {
            text = collapse_whitespace(&text);
        }

        // Step 7: Limit consecutive newlines
        if self.config.max_consecutive_newlines > 0 {
            text = limit_newlines(&text, self.config.max_consecutive_newlines);
        }

        text
    }

    /// Sanitize text and return detailed report of changes.
    pub fn sanitize_with_report(&self, input: &str) -> SanitizeReport {
        let original_len = input.len();
        let sanitized = self.sanitize(input);
        let sanitized_len = sanitized.len();

        SanitizeReport {
            original_length: original_len,
            sanitized_length: sanitized_len,
            chars_removed: original_len.saturating_sub(sanitized_len),
            sanitized_text: sanitized,
        }
    }
}

/// Report of sanitization changes.
#[derive(Debug, Clone)]
pub struct SanitizeReport {
    /// Original text length in bytes.
    pub original_length: usize,
    /// Sanitized text length in bytes.
    pub sanitized_length: usize,
    /// Number of bytes removed.
    pub chars_removed: usize,
    /// The sanitized text.
    pub sanitized_text: String,
}

// ============================================
// Helper Functions
// ============================================

/// Check if a character is an emoji.
fn is_emoji(c: char) -> bool {
    // Emoji ranges (simplified - covers most common emojis)
    // Note: Using broader ranges to avoid unreachable pattern warnings
    matches!(c,
        '\u{1F600}'..='\u{1F64F}' |  // Emoticons
        '\u{1F300}'..='\u{1F5FF}' |  // Misc Symbols and Pictographs
        '\u{1F680}'..='\u{1F6FF}' |  // Transport and Map
        '\u{1F1E0}'..='\u{1F1FF}' |  // Flags
        '\u{2600}'..='\u{26FF}'   |  // Misc symbols (includes weather, zodiac, etc.)
        '\u{2700}'..='\u{27BF}'   |  // Dingbats
        '\u{FE00}'..='\u{FE0F}'   |  // Variation Selectors
        '\u{1F900}'..='\u{1F9FF}' |  // Supplemental Symbols and Pictographs
        '\u{1FA00}'..='\u{1FA6F}' |  // Chess Symbols
        '\u{1FA70}'..='\u{1FAFF}' |  // Symbols and Pictographs Extended-A
        '\u{231A}'..='\u{231B}'   |  // Watch, Hourglass
        '\u{23E9}'..='\u{23FA}'   |  // Media controls
        '\u{25AA}'..='\u{25FE}'   |  // Geometric shapes
        '\u{2934}'..='\u{2935}'   |  // Arrows
        '\u{2B05}'..='\u{2B07}'   |  // Arrows
        '\u{2B1B}'..='\u{2B1C}'   |  // Squares
        '\u{2B50}' | '\u{2B55}'   |  // Star, Circle
        '\u{3030}' | '\u{303D}'   |  // Wavy dash, Part alternation
        '\u{3297}' | '\u{3299}'      // Circled ideographs
    )
}

/// Remove all emojis from text.
fn remove_emojis(text: &str) -> String {
    text.chars().filter(|c| !is_emoji(*c)).collect()
}

/// Replace emojis with [EMOJI] placeholder.
fn replace_emojis_with_placeholder(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_emoji = false;

    for c in text.chars() {
        if is_emoji(c) {
            if !in_emoji {
                result.push_str("[EMOJI]");
                in_emoji = true;
            }
            // Skip emoji character
        } else {
            in_emoji = false;
            result.push(c);
        }
    }

    result
}

/// Remove control characters except \n, \r, \t.
fn remove_control_chars(text: &str) -> String {
    text.chars()
        .filter(|c| {
            // Keep meaningful whitespace
            if *c == '\n' || *c == '\r' || *c == '\t' {
                return true;
            }
            // Remove C0 control chars (U+0000-U+001F)
            if ('\u{0000}'..='\u{001F}').contains(c) {
                return false;
            }
            // Remove C1 control chars (U+007F-U+009F)
            if ('\u{007F}'..='\u{009F}').contains(c) {
                return false;
            }
            true
        })
        .collect()
}

/// Remove zero-width characters.
fn remove_zero_width_chars(text: &str) -> String {
    text.chars()
        .filter(|c| {
            !matches!(
                *c,
                '\u{200B}'  // Zero Width Space
                | '\u{200C}' // Zero Width Non-Joiner
                | '\u{200D}' // Zero Width Joiner
                | '\u{2060}' // Word Joiner
                | '\u{FEFF}' // BOM / Zero Width No-Break Space
            )
        })
        .collect()
}

/// Remove directional markers.
fn remove_directional_markers(text: &str) -> String {
    text.chars()
        .filter(|c| {
            !matches!(
                *c,
                '\u{200E}'  // Left-to-Right Mark
                | '\u{200F}' // Right-to-Left Mark
                | '\u{202A}' // Left-to-Right Embedding
                | '\u{202B}' // Right-to-Left Embedding
                | '\u{202C}' // Pop Directional Formatting
                | '\u{202D}' // Left-to-Right Override
                | '\u{202E}' // Right-to-Left Override
                | '\u{2066}' // Left-to-Right Isolate
                | '\u{2067}' // Right-to-Left Isolate
                | '\u{2068}' // First Strong Isolate
                | '\u{2069}' // Pop Directional Isolate
            )
        })
        .collect()
}

/// Collapse multiple whitespace into single space.
fn collapse_whitespace(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_space = false;

    for c in text.chars() {
        if c.is_whitespace() && c != '\n' {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(c);
            prev_space = false;
        }
    }

    result
}

/// Limit consecutive newlines.
fn limit_newlines(text: &str, max: usize) -> String {
    let mut result = String::with_capacity(text.len());
    let mut newline_count = 0;

    for c in text.chars() {
        if c == '\n' {
            newline_count += 1;
            if newline_count <= max {
                result.push(c);
            }
        } else {
            newline_count = 0;
            result.push(c);
        }
    }

    result
}

// ============================================
// Tests
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unicode_normalization() {
        let sanitizer = Sanitizer::new(SanitizeConfig {
            normalize_unicode: true,
            ..Default::default()
        });

        // Composed vs decomposed é
        let composed = "caf\u{00E9}"; // é as single codepoint
        let decomposed = "cafe\u{0301}"; // e + combining accent

        let result1 = sanitizer.sanitize(composed);
        let result2 = sanitizer.sanitize(decomposed);

        assert_eq!(result1, result2);
    }

    #[test]
    fn test_emoji_preserve() {
        let sanitizer = Sanitizer::new(SanitizeConfig {
            emoji_mode: EmojiMode::Preserve,
            ..Default::default()
        });

        let input = "Hello 👋 World!";
        let result = sanitizer.sanitize(input);

        assert!(result.contains("👋"));
    }

    #[test]
    fn test_emoji_remove() {
        let sanitizer = Sanitizer::new(SanitizeConfig {
            emoji_mode: EmojiMode::Remove,
            ..Default::default()
        });

        let input = "Hello 👋 World! 🌍";
        let result = sanitizer.sanitize(input);

        assert_eq!(result, "Hello  World! ");
    }

    #[test]
    fn test_emoji_replace() {
        let sanitizer = Sanitizer::new(SanitizeConfig {
            emoji_mode: EmojiMode::ReplaceWithPlaceholder,
            ..Default::default()
        });

        let input = "Hello 👋 World!";
        let result = sanitizer.sanitize(input);

        assert!(result.contains("[EMOJI]"));
        assert!(!result.contains("👋"));
    }

    #[test]
    fn test_control_chars_removed() {
        let sanitizer = Sanitizer::new(SanitizeConfig {
            remove_control_chars: true,
            ..Default::default()
        });

        let input = "Hello\u{0000}\u{001F}World\t\n";
        let result = sanitizer.sanitize(input);

        assert_eq!(result, "HelloWorld\t\n");
    }

    #[test]
    fn test_zero_width_removed() {
        let sanitizer = Sanitizer::new(SanitizeConfig {
            remove_zero_width: true,
            ..Default::default()
        });

        let input = "Hello\u{200B}World\u{FEFF}!";
        let result = sanitizer.sanitize(input);

        assert_eq!(result, "HelloWorld!");
    }

    #[test]
    fn test_directional_markers_removed() {
        let sanitizer = Sanitizer::new(SanitizeConfig {
            remove_directional_markers: true,
            ..Default::default()
        });

        let input = "Hello\u{200E}\u{200F}World";
        let result = sanitizer.sanitize(input);

        assert_eq!(result, "HelloWorld");
    }

    #[test]
    fn test_collapse_whitespace() {
        let sanitizer = Sanitizer::new(SanitizeConfig {
            collapse_whitespace: true,
            ..Default::default()
        });

        let input = "Hello    World  !";
        let result = sanitizer.sanitize(input);

        assert_eq!(result, "Hello World !");
    }

    #[test]
    fn test_limit_newlines() {
        let sanitizer = Sanitizer::new(SanitizeConfig {
            max_consecutive_newlines: 2,
            ..Default::default()
        });

        let input = "Hello\n\n\n\n\nWorld";
        let result = sanitizer.sanitize(input);

        assert_eq!(result, "Hello\n\nWorld");
    }

    #[test]
    fn test_default_config() {
        let sanitizer = Sanitizer::default();

        // Should preserve emojis but remove zero-width and control chars
        let input = "Hello 👋 World!\u{200B}\u{0000}";
        let result = sanitizer.sanitize(input);

        assert!(result.contains("👋"));
        assert!(!result.contains('\u{200B}'));
        assert!(!result.contains('\u{0000}'));
    }
}
