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
/// - OODA-36: Lines containing URLs are never resolved (preserve URL integrity)
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

/// OODA-36: Check if a line contains a URL pattern.
/// OODA-46: Extended with DOI patterns and common academic URL domains.
/// WHY: URLs with hyphens (e.g., "https://example-site.com/path-") should not
/// be resolved across line breaks. The hyphen is part of the URL.
fn contains_url(text: &str) -> bool {
    text.contains("http://")
        || text.contains("https://")
        || text.contains("ftp://")
        || text.contains("www.")
        || text.contains("doi.org")
        || text.contains("arxiv.org")
        || text.contains("doi:")
        || text.contains("10.") && text.contains('/') // DOI pattern: 10.xxxx/yyyy
}

/// Try to resolve a hyphenated word break between two lines.
///
/// Returns Some(joined) if hyphenation was resolved, None if not.
fn try_resolve_hyphen(current: &str, next: &str) -> Option<String> {
    let trimmed_current = current.trim_end();

    // OODA-36: Never resolve hyphens in lines containing URLs
    if contains_url(trimmed_current) || contains_url(next) {
        return None;
    }

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

    // OODA-26: Don't resolve double hyphens (em dash substitutes)
    if trimmed_current.ends_with("--") {
        return None;
    }

    // Get the word fragment before the hyphen
    let prefix = &trimmed_current[..trimmed_current.len() - 1];

    // Must have actual text before the hyphen
    if prefix.is_empty() || prefix.ends_with(' ') {
        return None; // It's a list marker or standalone dash
    }

    // OODA-26: Don't resolve if the word before the hyphen is very short (likely compound)
    // e.g., "e-mail" split as "e-" / "mail" should NOT be resolved to "email"
    let word_before = prefix.rsplit_once(' ').map(|(_, w)| w).unwrap_or(prefix);
    if word_before.len() <= 2 && word_before.chars().all(|c| c.is_alphabetic()) {
        return None; // Short prefix like "e-", "x-", "re-" = likely compound
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

    /// OODA-26: Double hyphens should not be resolved
    #[test]
    fn test_double_hyphen_preserved() {
        let lines = vec!["hello--".to_string(), "world".to_string()];
        let resolved = resolve_hyphenation(&lines);
        assert_eq!(resolved, vec!["hello--".to_string(), "world".to_string()]);
    }

    /// OODA-26: Short compound prefixes should not be resolved
    #[test]
    fn test_short_compound_preserved() {
        // "e-" + "mail" = compound word, don't resolve
        let lines = vec!["e-".to_string(), "mail systems".to_string()];
        let resolved = resolve_hyphenation(&lines);
        assert_eq!(resolved, vec!["e-".to_string(), "mail systems".to_string()]);
    }

    /// OODA-36: URLs should not have hyphens resolved
    #[test]
    fn test_url_hyphen_preserved() {
        // URL with hyphen at line end
        let lines = vec![
            "See https://example-".to_string(),
            "site.com/path for details.".to_string(),
        ];
        let resolved = resolve_hyphenation(&lines);
        assert_eq!(
            resolved,
            vec![
                "See https://example-".to_string(),
                "site.com/path for details.".to_string()
            ]
        );
    }

    /// OODA-46: DOI patterns should not have hyphens resolved
    #[test]
    fn test_doi_hyphen_preserved() {
        let lines = vec!["doi: 10.1145/1234-".to_string(), "5678.2024".to_string()];
        let resolved = resolve_hyphenation(&lines);
        assert_eq!(
            resolved,
            vec!["doi: 10.1145/1234-".to_string(), "5678.2024".to_string()]
        );
    }
}
