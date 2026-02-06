//! Hyphenation resolution across line breaks.
//!
//! PDFs often break words across lines with hyphens, e.g.:
//! ```text
//! "computa-"
//! "tion"
//! ```
//! should become "computation".
//!
//! ## Algorithm
//!
//! 1. Scan consecutive lines within a block
//! 2. If a line ends with a soft hyphen (U+00AD) or ASCII hyphen at word boundary:
//!    a. Check if the next line starts with a lowercase letter
//!    b. If so, join the fragments and remove the hyphen
//! 3. Preserve intentional hyphens (e.g., "state-of-the-art")
//!
//! ## Heuristics for distinguishing soft vs hard hyphens:
//!
//! - Soft hyphen: line ends with `foo-` and next line starts with lowercase `bar` → "foobar"
//! - Hard hyphen: `state-of-the-art`, `well-known`, `GPU-accelerated` → preserved

/// Resolve hyphenated words across consecutive lines.
///
/// Input: vector of line texts (from Block lines).
/// Output: vector of processed line texts with hyphenated breaks resolved.
///
/// Rules:
/// - Line ending with `-` followed by next line starting with lowercase: join without hyphen
/// - Line ending with `-` followed by uppercase, digit, or special: keep hyphen (hard hyphen)
/// - Soft hyphens (U+00AD) are always resolved
pub fn resolve_hyphenation(lines: &[String]) -> Vec<String> {
    if lines.len() <= 1 {
        return lines.to_vec();
    }

    let mut result: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;

    while i < lines.len() {
        let current = &lines[i];

        if i + 1 < lines.len() {
            let next = &lines[i + 1];

            if let Some(joined) = try_resolve_hyphen(current, next) {
                result.push(joined);
                i += 2; // Skip the next line (already merged)
                continue;
            }
        }

        result.push(current.clone());
        i += 1;
    }

    result
}

/// Try to resolve a hyphenated word break between two lines.
///
/// Returns Some(joined) if hyphenation was resolved, None if not.
fn try_resolve_hyphen(current: &str, next: &str) -> Option<String> {
    let trimmed_current = current.trim_end();

    // Check for soft hyphen (U+00AD) - always resolve
    if trimmed_current.ends_with('\u{00AD}') {
        let prefix = &trimmed_current[..trimmed_current.len() - '\u{00AD}'.len_utf8()];
        let next_trimmed = next.trim_start();
        return Some(format!("{}{}", prefix, next_trimmed));
    }

    // Check for ASCII hyphen at end of line
    if !trimmed_current.ends_with('-') {
        return None;
    }

    // Get the word fragment before the hyphen
    let prefix = &trimmed_current[..trimmed_current.len() - 1];

    // Must have actual text before the hyphen
    if prefix.is_empty() || prefix.ends_with(' ') {
        return None; // It's a list marker or standalone dash
    }

    let next_trimmed = next.trim_start();
    if next_trimmed.is_empty() {
        return None;
    }

    let first_next_char = next_trimmed.chars().next()?;

    // If next line starts with lowercase letter: soft hyphen (resolve it)
    if first_next_char.is_lowercase() {
        // Additional check: the prefix should end with a letter
        let last_prefix_char = prefix.chars().last()?;
        if last_prefix_char.is_alphabetic() {
            return Some(format!("{}{}", prefix, next_trimmed));
        }
    }

    // Otherwise: hard hyphen (state-of-the-art, GPU-accelerated, etc.)
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_hyphenation() {
        let lines = vec!["computa-".to_string(), "tion of results".to_string()];
        let resolved = resolve_hyphenation(&lines);
        assert_eq!(resolved, vec!["computation of results"]);
    }

    #[test]
    fn test_soft_hyphen() {
        let lines = vec!["computa\u{00AD}".to_string(), "tion of results".to_string()];
        let resolved = resolve_hyphenation(&lines);
        assert_eq!(resolved, vec!["computation of results"]);
    }

    #[test]
    fn test_hard_hyphen_preserved() {
        // "state-of-the-art" should not be resolved
        let lines = vec![
            "state-of-the-".to_string(),
            "Art model".to_string(), // Uppercase A = hard hyphen
        ];
        let resolved = resolve_hyphenation(&lines);
        assert_eq!(
            resolved,
            vec!["state-of-the-".to_string(), "Art model".to_string()]
        );
    }

    #[test]
    fn test_list_marker_not_resolved() {
        // "- item" should not be treated as hyphenation
        let lines = vec!["- ".to_string(), "item text".to_string()];
        let resolved = resolve_hyphenation(&lines);
        assert_eq!(resolved, vec!["- ".to_string(), "item text".to_string()]);
    }

    #[test]
    fn test_multiple_hyphenations() {
        let lines = vec![
            "implemen-".to_string(),
            "tation of the algo-".to_string(),
            "rithm is complex".to_string(),
        ];
        let resolved = resolve_hyphenation(&lines);
        assert_eq!(
            resolved,
            vec![
                "implementation of the algo-".to_string(),
                "rithm is complex".to_string()
            ]
        );
    }

    #[test]
    fn test_no_hyphenation() {
        let lines = vec![
            "This is a normal line.".to_string(),
            "This is another line.".to_string(),
        ];
        let resolved = resolve_hyphenation(&lines);
        assert_eq!(resolved, lines);
    }

    #[test]
    fn test_single_line() {
        let lines = vec!["Just one line.".to_string()];
        let resolved = resolve_hyphenation(&lines);
        assert_eq!(resolved, vec!["Just one line."]);
    }

    #[test]
    fn test_empty_lines() {
        let lines: Vec<String> = vec![];
        let resolved = resolve_hyphenation(&lines);
        assert!(resolved.is_empty());
    }

    #[test]
    fn test_number_after_hyphen_preserved() {
        // "Figure 3-" followed by "2" should not be resolved
        let lines = vec!["Figure 3-".to_string(), "2 shows the results".to_string()];
        let resolved = resolve_hyphenation(&lines);
        assert_eq!(
            resolved,
            vec!["Figure 3-".to_string(), "2 shows the results".to_string()]
        );
    }

    #[test]
    fn test_gpu_hyphen_preserved() {
        // "GPU-" followed by "Accelerated" should keep hyphen (uppercase)
        let lines = vec!["GPU-".to_string(), "Accelerated training".to_string()];
        let resolved = resolve_hyphenation(&lines);
        assert_eq!(
            resolved,
            vec!["GPU-".to_string(), "Accelerated training".to_string()]
        );
    }
}
