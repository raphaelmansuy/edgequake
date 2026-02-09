//! OODA-47: Quality metrics for PDF to Markdown conversion.
//!
//! Implements unfalsifiable, automated metrics per the SOTA mission spec:
//! - CLF (Character-Level Fidelity): Levenshtein-based text similarity
//! - SPS (Structure Preservation Score): Markdown structural element matching
//! - ROA (Reading Order Accuracy): Paragraph ordering via LCS
//! - NR (Noise Ratio): Spurious content detection
//!
//! ## Usage
//!
//! ```rust,ignore
//! use edgequake_pdf::layout::quality_metrics::*;
//!
//! let clf = character_level_fidelity(&extracted, &gold);
//! let sps = structure_preservation_score(&extracted, &gold);
//! let roa = reading_order_accuracy(&extracted, &gold);
//! ```

/// Compute Character-Level Fidelity (CLF) between extracted and gold text.
///
/// OODA-49: Uses word-level Levenshtein for O(w^2) instead of O(n^2) performance.
/// OODA-51: Strips markdown formatting before comparison for fair content comparison.
///
/// CLF = 1 - (word_levenshtein_distance(a, b) / max(word_count(a), word_count(b)))
///
/// Returns a value in [0.0, 1.0] where 1.0 = perfect match.
pub fn character_level_fidelity(extracted: &str, gold: &str) -> f64 {
    // OODA-51: Strip markdown formatting before comparison
    let extracted_stripped = strip_markdown(extracted);
    let gold_stripped = strip_markdown(gold);

    let a_words = normalize_for_clf(&extracted_stripped);
    let b_words = normalize_for_clf(&gold_stripped);

    if a_words.is_empty() && b_words.is_empty() {
        return 1.0;
    }

    let max_len = a_words.len().max(b_words.len());
    if max_len == 0 {
        return 1.0;
    }

    let dist = word_levenshtein_distance(&a_words, &b_words);
    1.0 - (dist as f64 / max_len as f64)
}

/// Compute Structure Preservation Score (SPS).
///
/// Counts markdown structural elements in both extracted and gold:
/// - Headers (lines starting with #)
/// - List items (lines starting with - or N.)
/// - Code fences (``` lines)
/// - Bold markers (**)
///
/// SPS = matched_elements / total_gold_elements
pub fn structure_preservation_score(extracted: &str, gold: &str) -> f64 {
    let ext_structs = count_structural_elements(extracted);
    let gold_structs = count_structural_elements(gold);

    let total_gold: usize = gold_structs.values().sum();
    if total_gold == 0 {
        return 1.0; // No structure to preserve
    }

    let mut matched = 0usize;
    for (key, &gold_count) in &gold_structs {
        let ext_count = ext_structs.get(key).copied().unwrap_or(0);
        // Count matched as min(extracted, gold) for each element type
        matched += ext_count.min(gold_count);
    }

    matched as f64 / total_gold as f64
}

/// Compute Reading Order Accuracy (ROA).
///
/// Splits text into paragraph blocks and measures LCS-based ordering accuracy.
/// OODA-51: Strips markdown formatting before paragraph extraction for fair comparison.
/// ROA = LCS(extracted_paragraphs, gold_paragraphs) / len(gold_paragraphs)
pub fn reading_order_accuracy(extracted: &str, gold: &str) -> f64 {
    // OODA-51: Strip markdown formatting before extracting paragraphs
    let extracted_stripped = strip_markdown(extracted);
    let gold_stripped = strip_markdown(gold);

    let ext_paras = extract_paragraphs(&extracted_stripped);
    let gold_paras = extract_paragraphs(&gold_stripped);

    if gold_paras.is_empty() {
        return 1.0;
    }

    let lcs_len = lcs_length(&ext_paras, &gold_paras);
    lcs_len as f64 / gold_paras.len() as f64
}

/// Compute Noise Ratio (NR).
///
/// OODA-49: Improved to not count blank lines between paragraphs as noise.
/// NR = noise_lines / total_non_blank_lines
/// Noise lines: standalone page numbers, separator lines, very short orphan lines.
pub fn noise_ratio(text: &str) -> f64 {
    let lines: Vec<&str> = text.lines().collect();
    // Only count non-blank lines in total (blank lines between paragraphs are normal)
    let non_blank: Vec<&&str> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
    let total = non_blank.len();
    if total == 0 {
        return 0.0;
    }

    let noise_count = non_blank.iter().filter(|line| is_noise_line(line)).count();
    noise_count as f64 / total as f64
}

/// Full quality report for a single document.
#[derive(Debug, Clone)]
pub struct QualityReport {
    pub document_name: String,
    pub clf: f64,
    pub sps: f64,
    pub roa: f64,
    pub nr: f64,
    pub extracted_len: usize,
    pub gold_len: usize,
}

impl QualityReport {
    pub fn compute(name: &str, extracted: &str, gold: &str) -> Self {
        Self {
            document_name: name.to_string(),
            clf: character_level_fidelity(extracted, gold),
            sps: structure_preservation_score(extracted, gold),
            roa: reading_order_accuracy(extracted, gold),
            nr: noise_ratio(extracted),
            extracted_len: extracted.len(),
            gold_len: gold.len(),
        }
    }
}

impl std::fmt::Display for QualityReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: CLF={:.3} SPS={:.3} ROA={:.3} NR={:.3} ({}ch vs {}ch gold)",
            self.document_name,
            self.clf,
            self.sps,
            self.roa,
            self.nr,
            self.extracted_len,
            self.gold_len
        )
    }
}

// --- Internal helpers ---

/// Normalize text for CLF comparison.
/// OODA-49: Returns word vector instead of collapsed string for word-level Levenshtein.
/// OODA-51: Strip markdown formatting before comparison for fair content comparison.
///
/// Pipeline:
/// 1. Strip markdown formatting (bold, italic, headers, code fences, links)
/// 2. Normalize whitespace (collapse multiple spaces, newlines → space)
/// 3. Split by whitespace into words
/// 4. Filter empty tokens
fn normalize_for_clf(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

/// OODA-51: Strip markdown formatting markers from text for fair comparison.
/// OODA-62: Also strip pipe table markers and table separator rows.
/// Removes: **, *, _, ~~, #, ```, |, table separators, and other markdown syntax.
/// This ensures CLF/ROA measure content fidelity, not formatting differences.
fn strip_markdown(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    for line in text.lines() {
        let trimmed = line.trim();

        // OODA-62: Skip pipe-table separator rows like |---|---|---|
        if trimmed.starts_with('|')
            && trimmed.ends_with('|')
            && trimmed
                .chars()
                .all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
        {
            result.push('\n');
            continue;
        }

        let chars: Vec<char> = line.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            match chars[i] {
                // Skip code fences: ```
                '`' if i + 2 < len && chars[i + 1] == '`' && chars[i + 2] == '`' => {
                    i += 3;
                    // Skip optional language identifier on same line
                    while i < len && chars[i] != '\n' {
                        i += 1;
                    }
                }
                // Skip inline code backticks
                '`' => {
                    i += 1;
                }
                // Skip bold/italic markers: ** or *
                '*' => {
                    while i < len && chars[i] == '*' {
                        i += 1;
                    }
                }
                // Skip italic/underline markers: _
                '_' => {
                    i += 1;
                }
                // Skip strikethrough: ~~
                '~' => {
                    while i < len && chars[i] == '~' {
                        i += 1;
                    }
                }
                // OODA-62: Replace pipe chars with space (table cell separators)
                '|' => {
                    result.push(' ');
                    i += 1;
                }
                // Skip header markers: # at start of line
                '#' if i == 0 => {
                    while i < len && chars[i] == '#' {
                        i += 1;
                    }
                    if i < len && chars[i] == ' ' {
                        i += 1;
                    }
                }
                // Keep everything else
                c => {
                    result.push(c);
                    i += 1;
                }
            }
        }
        result.push('\n');
    }
    result
}

/// OODA-49: Word-level Levenshtein distance.
/// Compares word sequences instead of character sequences.
/// O(m*n) where m,n are word counts (~10K) instead of char counts (~70K).
fn word_levenshtein_distance(a: &[&str], b: &[&str]) -> usize {
    let m = a.len();
    let n = b.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    // Use shorter sequence for columns (memory optimization)
    if m < n {
        return word_levenshtein_distance(b, a);
    }

    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1].eq_ignore_ascii_case(b[j - 1]) {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// Character-level Levenshtein (kept for small string comparisons in tests).
#[cfg(test)]
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    // Ensure b is the shorter string for memory optimization
    if m < n {
        return levenshtein_distance(b, a);
    }

    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1) // deletion
                .min(curr[j - 1] + 1) // insertion
                .min(prev[j - 1] + cost); // substitution
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// Count structural markdown elements.
fn count_structural_elements(text: &str) -> std::collections::HashMap<&'static str, usize> {
    let mut counts = std::collections::HashMap::new();

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('#') {
            *counts.entry("headers").or_insert(0) += 1;
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            *counts.entry("list_items").or_insert(0) += 1;
        } else if trimmed.starts_with("```") {
            *counts.entry("code_fences").or_insert(0) += 1;
        } else if trimmed.contains("**") {
            *counts.entry("bold").or_insert(0) += 1;
        }

        // OODA-58: Numbered list: "1. ", "12. ", "123) ", etc.
        // WHY: Previous code consumed only one digit, missing multi-digit items
        // like "12. Item" which is common in papers with 10+ list items.
        if !trimmed.is_empty() {
            let num_end = trimmed
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(trimmed.len());
            if num_end > 0 {
                let rest = &trimmed[num_end..];
                if rest.starts_with(". ") || rest.starts_with(") ") {
                    *counts.entry("numbered_list").or_insert(0) += 1;
                }
            }
        }
    }

    counts
}

/// Extract meaningful paragraphs (non-empty, non-structural blocks).
/// OODA-63: Lowered minimum from 20 to 5 chars to include short section headers
/// and bullet items that pymupdf4llm produces as separate paragraphs.
fn extract_paragraphs(text: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut current = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current.is_empty() {
                // Normalize and store paragraph
                let normalized = current.split_whitespace().collect::<Vec<_>>().join(" ");
                if normalized.len() >= 5 {
                    // Include short paragraphs (headers, items)
                    paragraphs.push(normalized.to_lowercase());
                }
                current.clear();
            }
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(trimmed);
        }
    }

    if !current.is_empty() {
        let normalized = current.split_whitespace().collect::<Vec<_>>().join(" ");
        if normalized.len() >= 5 {
            paragraphs.push(normalized.to_lowercase());
        }
    }

    paragraphs
}

/// Longest Common Subsequence length between two string vectors.
/// Uses partial matching (Jaccard similarity > 0.5) for paragraph comparison.
fn lcs_length(a: &[String], b: &[String]) -> usize {
    let m = a.len();
    let n = b.len();

    if m == 0 || n == 0 {
        return 0;
    }

    // Build match matrix using word overlap (Jaccard similarity)
    // OODA-52: Lowered threshold from 0.3 to 0.2 for more forgiving paragraph matching.
    // WHY: Different line joining and formatting produces slightly different word sets
    // per paragraph (e.g., "ren" orphaned vs "rendering" joined). 0.2 allows matching
    // paragraphs that share 20%+ of unique words.
    let a_words: Vec<std::collections::HashSet<&str>> =
        a.iter().map(|s| s.split_whitespace().collect()).collect();
    let b_words: Vec<std::collections::HashSet<&str>> =
        b.iter().map(|s| s.split_whitespace().collect()).collect();

    // DP for LCS with fuzzy matching
    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for i in 1..=m {
        for j in 1..=n {
            // Check if paragraphs are similar enough (Jaccard)
            let intersection = a_words[i - 1].intersection(&b_words[j - 1]).count();
            let union = a_words[i - 1].union(&b_words[j - 1]).count();
            let jaccard = if union > 0 {
                intersection as f64 / union as f64
            } else {
                0.0
            };

            if jaccard > 0.2 {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    dp[m][n]
}

/// Check if a non-blank line is likely noise (page number, separator).
/// OODA-49: Blank lines pre-filtered by noise_ratio(), not counted here.
/// OODA-50: Extended with "Page N" patterns and more separator variants.
fn is_noise_line(line: &str) -> bool {
    let trimmed = line.trim();

    // Standalone page numbers
    if trimmed.chars().all(|c| c.is_ascii_digit()) && !trimmed.is_empty() && trimmed.len() <= 4 {
        return true;
    }

    // "Page N" or "page N" patterns
    if let Some(rest) = trimmed
        .strip_prefix("Page ")
        .or_else(|| trimmed.strip_prefix("page "))
    {
        if rest.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty() {
            return true;
        }
    }

    // Page separator markers (--- or ----- variants)
    if trimmed.starts_with("---") && trimmed.chars().all(|c| c == '-' || c.is_whitespace()) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("saturday", "sunday"), 3);
    }

    #[test]
    fn test_character_level_fidelity() {
        assert!((character_level_fidelity("hello world", "hello world") - 1.0).abs() < 0.001);
        assert!((character_level_fidelity("", "") - 1.0).abs() < 0.001);

        // Small difference (one word changed)
        let clf = character_level_fidelity("hello world foo", "hello world bar");
        assert!(clf > 0.5, "One word diff in 3 should be >0.5, got {}", clf);

        // Case insensitive word matching
        let clf = character_level_fidelity("Hello World", "hello world");
        assert!(
            (clf - 1.0).abs() < 0.001,
            "Case insensitive should match, got {}",
            clf
        );

        // OODA-49: Word-level distance test
        let clf = character_level_fidelity(
            "the quick brown fox jumps over the lazy dog",
            "the quick brown fox jumps over the lazy cat",
        );
        // 1 word differs out of 9 → CLF = 1 - 1/9 ≈ 0.889
        assert!(
            clf > 0.85,
            "One word diff should give high CLF, got {}",
            clf
        );
    }

    #[test]
    fn test_structure_preservation_score() {
        let gold = "# Header\n\n- Item 1\n- Item 2\n\n```python\ncode\n```\n";
        let extracted = "# Header\n\n- Item 1\n- Item 2\n\n```python\ncode\n```\n";
        let sps = structure_preservation_score(extracted, gold);
        assert!(
            (sps - 1.0).abs() < 0.001,
            "Perfect match should be 1.0, got {}",
            sps
        );

        // Missing a list item
        let extracted_partial = "# Header\n\n- Item 1\n\n```python\ncode\n```\n";
        let sps_partial = structure_preservation_score(extracted_partial, gold);
        assert!(sps_partial < 1.0, "Missing element should reduce SPS");
        assert!(
            sps_partial > 0.5,
            "Most elements preserved, got {}",
            sps_partial
        );
    }

    #[test]
    fn test_reading_order_accuracy() {
        let gold = "The quantum mechanics laboratory published groundbreaking results in February.\n\nMachine learning algorithms transformed the industrial automation sector completely.\n\nBiological research centers discovered novel protein folding configurations recently.\n";
        let extracted = "The quantum mechanics laboratory published groundbreaking results in February.\n\nMachine learning algorithms transformed the industrial automation sector completely.\n\nBiological research centers discovered novel protein folding configurations recently.\n";

        let roa = reading_order_accuracy(extracted, gold);
        assert!(
            (roa - 1.0).abs() < 0.001,
            "Same order should be 1.0, got {}",
            roa
        );

        // Swapped order: paragraph 1 and 2 swapped
        let swapped = "Machine learning algorithms transformed the industrial automation sector completely.\n\nThe quantum mechanics laboratory published groundbreaking results in February.\n\nBiological research centers discovered novel protein folding configurations recently.\n";
        let roa_swapped = reading_order_accuracy(swapped, gold);
        assert!(
            roa_swapped < 1.0,
            "Swapped order should reduce ROA, got {}",
            roa_swapped
        );
        assert!(
            roa_swapped >= 0.6,
            "Two of three in order, got {}",
            roa_swapped
        );
    }

    #[test]
    fn test_noise_ratio() {
        let clean = "This is a paragraph.\n\nAnother paragraph here.\n";
        let nr_clean = noise_ratio(clean);
        // Only the empty line between paragraphs is "noise"
        assert!(
            nr_clean < 0.5,
            "Clean text should have low noise, got {}",
            nr_clean
        );

        let noisy = "1\n\n---\n\n2\n\n---\n\nActual content here.\n";
        let nr_noisy = noise_ratio(noisy);
        assert!(nr_noisy > nr_clean, "Noisy text should have higher NR");
    }

    #[test]
    fn test_quality_report() {
        let gold = "# Introduction\n\nThis is a test document with real content for analysis.\n\n- Item one\n- Item two\n";
        let extracted = "# Introduction\n\nThis is a test document with real content for analysis.\n\n- Item one\n- Item two\n";

        let report = QualityReport::compute("test", extracted, gold);
        assert!(report.clf > 0.95);
        assert!(report.sps > 0.95);
        assert!(report.roa >= 1.0 - 0.01);
        println!("{}", report);
    }

    #[test]
    fn test_extract_paragraphs() {
        let text = "# Header\n\nFirst paragraph with enough text to count.\n\nSecond paragraph also with enough text.\n\nShort.\n\nHi.\n";
        let paras = extract_paragraphs(text);
        // OODA-63: "Short." (6 chars) now included, "Hi." (3 chars) excluded
        assert!(
            paras.len() >= 3,
            "Should extract at least 3 paragraphs (incl short), got {}",
            paras.len()
        );
    }

    /// OODA-50: Test noise detection includes "Page N" patterns
    #[test]
    fn test_is_noise_line_page_patterns() {
        assert!(is_noise_line("42"), "Standalone page number");
        assert!(is_noise_line("1"), "Single digit page number");
        assert!(is_noise_line("Page 1"), "Page N pattern");
        assert!(is_noise_line("Page 42"), "Page NN pattern");
        assert!(is_noise_line("page 5"), "Lowercase page pattern");
        assert!(is_noise_line("---"), "Triple dash separator");
        assert!(is_noise_line("-----"), "Five dash separator");
        assert!(!is_noise_line("Page title here"), "Not a page number");
        assert!(!is_noise_line("This is normal text"), "Normal text");
        assert!(!is_noise_line("12345"), "5+ digit number not noise");
    }

    /// OODA-50: Test noise ratio with page separator patterns
    #[test]
    fn test_noise_ratio_page_separators() {
        let with_separators = "Content paragraph one here.\n\n-----\n\nPage 2\n\nContent paragraph two here.\n\n-----\n\nPage 3\n\nContent paragraph three here.\n";
        let nr = noise_ratio(with_separators);
        // 3 content + 2 "-----" + 2 "Page N" = 7 non-blank, 4 noise → NR ≈ 0.571
        assert!(
            nr > 0.4,
            "Should detect page separators as noise, got {}",
            nr
        );

        let without_separators = "Content paragraph one here.\n\nContent paragraph two here.\n\nContent paragraph three here.\n";
        let nr_clean = noise_ratio(without_separators);
        assert!(
            nr_clean < 0.01,
            "Clean text should have no noise, got {}",
            nr_clean
        );
    }

    /// OODA-51: Test markdown stripping for CLF comparison
    /// OODA-62: Also tests pipe table stripping
    #[test]
    fn test_strip_markdown() {
        assert_eq!(strip_markdown("**bold**").trim(), "bold");
        assert_eq!(strip_markdown("*italic*").trim(), "italic");
        assert_eq!(strip_markdown("_underscore_").trim(), "underscore");
        assert_eq!(strip_markdown("## Header").trim(), "Header");
        assert_eq!(strip_markdown("~~struck~~").trim(), "struck");
        assert_eq!(strip_markdown("`code`").trim(), "code");
        assert_eq!(strip_markdown("normal text").trim(), "normal text");
        // Combined formatting
        assert_eq!(strip_markdown("**_bold italic_**").trim(), "bold italic");
        // Headers at line start
        let result = strip_markdown("# Title\n## Sub");
        assert!(result.contains("Title"), "Should contain Title: {}", result);
        assert!(result.contains("Sub"), "Should contain Sub: {}", result);
        // OODA-62: Pipe tables
        assert!(
            strip_markdown("|cell1|cell2|cell3|").contains("cell1"),
            "Should contain cell1"
        );
        assert!(
            strip_markdown("|---|---|---|").trim().is_empty(),
            "Separator row should be empty"
        );
    }

    /// OODA-51: Test CLF with markdown formatting differences
    #[test]
    fn test_clf_ignores_formatting() {
        // Same content with different formatting should have high CLF
        let text_a = "# Title\n\n**bold** and _italic_ text here";
        let text_b = "## Title\n\nbold and italic text here";
        let clf = character_level_fidelity(text_a, text_b);
        assert!(
            clf > 0.9,
            "Formatting differences should not hurt CLF much, got {}",
            clf
        );
    }
}
