//! Markdown table preprocessor for RAG-friendly chunking.
//!
//! WHY: Excel exports and large markdown tables cause problems in the RAG pipeline:
//! 1. Chunks split mid-row, losing context
//! 2. Chunks lose the table header, making LLM extraction unreliable
//! 3. Highly repetitive rows create an explosion of similar entities
//! 4. Large tables (3000+ rows) generate 100+ chunks → expensive LLM calls
//!
//! This preprocessor detects markdown tables and restructures them into
//! semantically coherent sections grouped by the first column value.
//! Each section gets its own header, enabling the chunker to create
//! meaningful, self-contained chunks.
//!
//! @implements FIX-EXCEL-CHUNKING: Optimize large tabular document processing

use std::collections::BTreeMap;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for table preprocessing.
#[derive(Debug, Clone)]
pub struct TablePreprocessorConfig {
    /// Minimum ratio of table lines to non-empty lines to trigger preprocessing.
    /// Default: 0.5 (50%)
    pub table_detection_threshold: f64,
    /// Maximum number of rows per group before truncation.
    /// Groups larger than this get a summary note and only the first N rows.
    /// Default: 50
    pub max_rows_per_group: usize,
    /// Whether to deduplicate identical rows within a group.
    /// Default: true
    pub deduplicate_rows: bool,
    /// Optional document-level title for the restructured output.
    /// Default: None (auto-generates "Tabular Data Summary")
    pub document_title: Option<String>,
}

impl Default for TablePreprocessorConfig {
    fn default() -> Self {
        Self {
            table_detection_threshold: 0.5,
            max_rows_per_group: 50,
            deduplicate_rows: true,
            document_title: None,
        }
    }
}

// ============================================================================
// Result type
// ============================================================================

/// Result of preprocessing analysis.
#[derive(Debug)]
pub struct PreprocessResult {
    /// Preprocessed content (restructured into sections if tabular, original otherwise).
    pub content: String,
    /// Whether the content was detected as tabular and was restructured.
    pub was_restructured: bool,
    /// Number of data rows detected (excludes header and separator).
    pub table_rows: usize,
    /// Number of groups created from first-column values.
    pub groups: usize,
    /// Number of duplicate rows removed (only when `deduplicate_rows` is true).
    pub duplicates_removed: usize,
}

impl PreprocessResult {
    /// Create a pass-through result (content unchanged).
    fn passthrough(content: &str, table_rows: usize) -> Self {
        Self {
            content: content.to_string(),
            was_restructured: false,
            table_rows,
            groups: 0,
            duplicates_removed: 0,
        }
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Preprocess markdown content to optimize tabular data for RAG chunking.
///
/// If the content is predominantly a markdown table (above threshold ratio),
/// this function:
/// 1. Extracts the table header
/// 2. Groups data rows by the first column value
/// 3. Optionally deduplicates identical rows within each group
/// 4. Emits each group as a separate markdown section with the header repeated
///
/// Non-table content passes through unchanged.
pub fn preprocess_tabular_content(
    content: &str,
    config: &TablePreprocessorConfig,
) -> PreprocessResult {
    let lines: Vec<&str> = content.lines().collect();

    // Guard: empty or whitespace-only content
    let non_empty_lines = lines.iter().filter(|l| !l.trim().is_empty()).count();
    if non_empty_lines == 0 {
        return PreprocessResult::passthrough(content, 0);
    }

    // Detect table ratio
    let table_lines: Vec<&str> = lines
        .iter()
        .filter(|l| l.trim().starts_with('|'))
        .copied()
        .collect();

    let table_ratio = table_lines.len() as f64 / non_empty_lines as f64;
    if table_ratio < config.table_detection_threshold {
        return PreprocessResult::passthrough(content, table_lines.len());
    }

    // Parse table structure
    let parsed = ParsedTable::from_lines(&table_lines);

    // Need a header AND at least one data row to restructure
    let (header, separator) = match (parsed.header, parsed.separator) {
        (Some(h), sep) => (h, sep),
        (None, _) => return PreprocessResult::passthrough(content, table_lines.len()),
    };
    if parsed.data_rows.is_empty() {
        return PreprocessResult::passthrough(content, table_lines.len());
    }

    // Group and deduplicate
    let (groups, total_duplicates) =
        group_rows_by_first_column(&parsed.data_rows, config.deduplicate_rows);

    if groups.is_empty() {
        return PreprocessResult::passthrough(content, parsed.data_rows.len());
    }

    // Emit restructured output
    let output = emit_grouped_sections(
        &header,
        &separator,
        &groups,
        parsed.data_rows.len() - total_duplicates,
        config,
    );

    PreprocessResult {
        content: output,
        was_restructured: true,
        table_rows: parsed.data_rows.len(),
        groups: groups.len(),
        duplicates_removed: total_duplicates,
    }
}

// ============================================================================
// Internal: Table parsing
// ============================================================================

/// Parsed markdown table structure.
struct ParsedTable {
    header: Option<String>,
    separator: Option<String>,
    data_rows: Vec<String>,
}

impl ParsedTable {
    /// Parse table lines into header, separator, and data rows.
    fn from_lines(lines: &[&str]) -> Self {
        let mut header: Option<String> = None;
        let mut separator: Option<String> = None;
        let mut data_rows: Vec<String> = Vec::new();

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if is_separator_line(trimmed) {
                // Take only the first separator (multi-table edge case)
                if separator.is_none() {
                    separator = Some(trimmed.to_string());
                }
                continue;
            }

            if header.is_none() {
                header = Some(trimmed.to_string());
                continue;
            }

            data_rows.push(trimmed.to_string());
        }

        Self {
            header,
            separator,
            data_rows,
        }
    }
}

// ============================================================================
// Internal: Grouping
// ============================================================================

/// Group rows by their first column value, optionally deduplicating.
///
/// Returns (groups, duplicate_count).
fn group_rows_by_first_column(
    rows: &[String],
    deduplicate: bool,
) -> (BTreeMap<String, Vec<String>>, usize) {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut total_duplicates = 0usize;

    for row in rows {
        let group_key = extract_first_column(row);
        let entry = groups.entry(group_key).or_default();

        if deduplicate && entry.contains(row) {
            total_duplicates += 1;
        } else {
            entry.push(row.clone());
        }
    }

    (groups, total_duplicates)
}

// ============================================================================
// Internal: Output formatting
// ============================================================================

/// Emit the grouped table sections as markdown.
fn emit_grouped_sections(
    header: &str,
    separator: &Option<String>,
    groups: &BTreeMap<String, Vec<String>>,
    unique_row_count: usize,
    config: &TablePreprocessorConfig,
) -> String {
    let sep = separator.as_deref().unwrap_or("| --- | --- | --- |");

    let mut output = String::new();

    // Document-level header
    let title = config
        .document_title
        .as_deref()
        .unwrap_or("Tabular Data Summary");
    output.push_str(&format!("# {}\n\n", title));
    output.push_str(&format!(
        "> {} entries organized into {} categories.\n\n",
        unique_row_count,
        groups.len()
    ));

    for (group_name, rows) in groups {
        let display_name = if group_name.is_empty() {
            "General"
        } else {
            group_name.as_str()
        };

        // Section header creates a natural chunk boundary
        output.push_str(&format!("## {}\n\n", display_name));

        // Repeat table header + separator for each section
        output.push_str(header);
        output.push('\n');
        output.push_str(sep);
        output.push('\n');

        if rows.len() > config.max_rows_per_group {
            output.push_str(&format!(
                "| *({} entries — showing first {})* | | |\n",
                rows.len(),
                config.max_rows_per_group
            ));
            for row in rows.iter().take(config.max_rows_per_group) {
                output.push_str(row);
                output.push('\n');
            }
        } else {
            for row in rows {
                output.push_str(row);
                output.push('\n');
            }
        }

        output.push('\n');
    }

    output
}

// ============================================================================
// Internal: Line-level utilities
// ============================================================================

/// Check if a line is a markdown table separator (e.g., `| --- | --- |`).
fn is_separator_line(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return false;
    }
    // Collect non-empty cells — must have at least one
    let cells: Vec<&str> = trimmed
        .split('|')
        .filter(|s| !s.trim().is_empty())
        .collect();
    // WHY: `.all()` returns true on empty iterators, so guard against `| |`
    !cells.is_empty()
        && cells.iter().all(|cell| {
            let cell = cell.trim();
            !cell.is_empty()
                && cell
                    .chars()
                    .all(|c| c == '-' || c == ':' || c.is_whitespace())
        })
}

/// Extract the first column value from a markdown table row.
fn extract_first_column(row: &str) -> String {
    let trimmed = row.trim();
    if !trimmed.starts_with('|') {
        return String::new();
    }
    // Split by | → ["", "first col", "second col", ...]
    trimmed
        .split('|')
        .nth(1)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> TablePreprocessorConfig {
        TablePreprocessorConfig::default()
    }

    // ------------------------------------------------------------------
    // Pass-through cases (content should NOT be restructured)
    // ------------------------------------------------------------------

    #[test]
    fn passthrough_on_empty_string() {
        let result = preprocess_tabular_content("", &default_config());
        assert!(!result.was_restructured);
        assert_eq!(result.content, "");
        assert_eq!(result.table_rows, 0);
    }

    #[test]
    fn passthrough_on_whitespace_only() {
        let result = preprocess_tabular_content("   \n\n  \n", &default_config());
        assert!(!result.was_restructured);
        assert_eq!(result.table_rows, 0);
    }

    #[test]
    fn passthrough_on_plain_text() {
        let content = "# Hello World\n\nThis is a paragraph.\n\nAnother paragraph.";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(!result.was_restructured);
        assert_eq!(result.content, content);
    }

    #[test]
    fn passthrough_when_below_threshold() {
        // 4 non-table lines + 3 table lines = 3/7 ≈ 0.43 (below 0.5 threshold)
        let content =
            "# Title\n\nParagraph 1.\n\nParagraph 2.\n\nMore text.\n\n| A | B |\n| --- | --- |\n| 1 | 2 |";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(!result.was_restructured);
        assert_eq!(result.table_rows, 3);
    }

    #[test]
    fn passthrough_on_header_only_table() {
        // Header + separator but no data rows
        let content = "| A | B | C |\n| --- | --- | --- |";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(!result.was_restructured);
    }

    #[test]
    fn passthrough_when_no_header_detected() {
        // All separator lines
        let content = "| --- | --- |\n| --- | --- |";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(!result.was_restructured);
    }

    // ------------------------------------------------------------------
    // Restructuring cases
    // ------------------------------------------------------------------

    #[test]
    fn restructures_simple_table() {
        let content = "| Col1 | Col2 | Col3 |\n| --- | --- | --- |\n| A | B | C |\n| D | E | F |";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(result.was_restructured);
        assert_eq!(result.table_rows, 2);
        assert_eq!(result.groups, 2);
    }

    #[test]
    fn groups_by_first_column() {
        let content = "\
| Category | Name | Description |
| --- | --- | --- |
| Dashboard | Metric1 | Desc1 |
| Dashboard | Metric2 | Desc2 |
| Report | Metric3 | Desc3 |";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(result.was_restructured);
        assert_eq!(result.groups, 2);
        assert!(result.content.contains("## Dashboard"));
        assert!(result.content.contains("## Report"));
    }

    #[test]
    fn single_group_still_restructured() {
        let content = "\
| Sheet | KPI | Value |
| --- | --- | --- |
| Sales | Revenue | 100 |
| Sales | Profit | 50 |
| Sales | Margin | 50% |";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(result.was_restructured);
        assert_eq!(result.groups, 1);
        assert!(result.content.contains("## Sales"));
        assert!(result.content.contains("| Sheet | KPI | Value |"));
    }

    #[test]
    fn single_data_row_restructured() {
        let content = "| Cat | Name |\n| --- | --- |\n| Only | Row |";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(result.was_restructured);
        assert_eq!(result.table_rows, 1);
        assert_eq!(result.groups, 1);
    }

    #[test]
    fn header_repeated_per_group() {
        let content = "\
| Category | Name | Description |
| --- | --- | --- |
| A | Item1 | Desc1 |
| B | Item2 | Desc2 |";
        let result = preprocess_tabular_content(content, &default_config());
        let header_count = result
            .content
            .matches("| Category | Name | Description |")
            .count();
        assert_eq!(header_count, 2, "Header must appear once per group");
    }

    // ------------------------------------------------------------------
    // Deduplication
    // ------------------------------------------------------------------

    #[test]
    fn deduplication_removes_identical_rows() {
        let content = "\
| Cat | Name | Desc |
| --- | --- | --- |
| A | Same | Same desc |
| A | Same | Same desc |
| A | Different | Other |";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(result.was_restructured);
        assert_eq!(result.duplicates_removed, 1);
        assert_eq!(result.table_rows, 3);
    }

    #[test]
    fn deduplication_disabled_keeps_all_rows() {
        let content = "\
| Cat | Name |
| --- | --- |
| A | Same |
| A | Same |";
        let config = TablePreprocessorConfig {
            deduplicate_rows: false,
            ..default_config()
        };
        let result = preprocess_tabular_content(content, &config);
        assert!(result.was_restructured);
        assert_eq!(result.duplicates_removed, 0);
        assert_eq!(result.content.matches("| A | Same |").count(), 2);
    }

    // ------------------------------------------------------------------
    // Truncation
    // ------------------------------------------------------------------

    #[test]
    fn large_group_truncated() {
        let mut content = String::from("| Cat | Name | Desc |\n| --- | --- | --- |\n");
        for i in 0..100 {
            content.push_str(&format!("| BigGroup | Item{} | Desc{} |\n", i, i));
        }
        let config = TablePreprocessorConfig {
            max_rows_per_group: 50,
            ..default_config()
        };
        let result = preprocess_tabular_content(&content, &config);
        assert!(result.was_restructured);
        assert!(result.content.contains("100 entries"));
        assert!(result.content.contains("showing first 50"));
    }

    #[test]
    fn group_at_max_limit_not_truncated() {
        let mut content = String::from("| Cat | Name |\n| --- | --- |\n");
        for i in 0..50 {
            content.push_str(&format!("| G | Item{} |\n", i));
        }
        let config = TablePreprocessorConfig {
            max_rows_per_group: 50,
            ..default_config()
        };
        let result = preprocess_tabular_content(&content, &config);
        assert!(result.was_restructured);
        assert!(
            !result.content.contains("entries —"),
            "Exactly at limit should NOT be truncated"
        );
    }

    // ------------------------------------------------------------------
    // Configuration
    // ------------------------------------------------------------------

    #[test]
    fn custom_threshold_changes_detection() {
        let content = "Some text.\n\n| A | B |\n| --- | --- |\n| 1 | 2 |";
        let config = TablePreprocessorConfig {
            table_detection_threshold: 0.9,
            ..default_config()
        };
        let result = preprocess_tabular_content(content, &config);
        assert!(!result.was_restructured, "0.6 ratio < 0.9 threshold");
    }

    #[test]
    fn custom_title_used_in_output() {
        let content = "| A | B |\n| --- | --- |\n| 1 | 2 |";
        let config = TablePreprocessorConfig {
            document_title: Some("KPI Dictionary".to_string()),
            ..default_config()
        };
        let result = preprocess_tabular_content(content, &config);
        assert!(result.was_restructured);
        assert!(result.content.contains("# KPI Dictionary"));
    }

    #[test]
    fn default_title_when_none_configured() {
        let content = "| A | B |\n| --- | --- |\n| 1 | 2 |";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(result.content.contains("# Tabular Data Summary"));
    }

    // ------------------------------------------------------------------
    // Unicode and special characters
    // ------------------------------------------------------------------

    #[test]
    fn unicode_first_column_groups_correctly() {
        let content = "\
| Catégorie | Nom | Description |
| --- | --- | --- |
| Données | Item1 | Première |
| Données | Item2 | Deuxième |
| Résumé | Item3 | Troisième |";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(result.was_restructured);
        assert_eq!(result.groups, 2);
        assert!(result.content.contains("## Données"));
        assert!(result.content.contains("## Résumé"));
    }

    #[test]
    fn empty_first_column_grouped_as_general() {
        let content = "\
| Cat | Name |
| --- | --- |
|  | Orphan1 |
|  | Orphan2 |
| A | Named |";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(result.was_restructured);
        assert!(result.content.contains("## General"));
    }

    // ------------------------------------------------------------------
    // Separator line detection
    // ------------------------------------------------------------------

    #[test]
    fn separator_variants_detected() {
        assert!(is_separator_line("| --- | --- | --- |"));
        assert!(is_separator_line("| :--- | ---: | :---: |"));
        assert!(is_separator_line("|---|---|"));
        assert!(is_separator_line("| ---- | ----- |"));
    }

    #[test]
    fn non_separators_rejected() {
        assert!(!is_separator_line("| hello | world |"));
        assert!(!is_separator_line("not a table"));
        assert!(!is_separator_line(""));
        assert!(!is_separator_line("| |"));
    }

    // ------------------------------------------------------------------
    // First column extraction
    // ------------------------------------------------------------------

    #[test]
    fn extract_first_column_variants() {
        assert_eq!(extract_first_column("| Hello | World |"), "Hello");
        assert_eq!(extract_first_column("|  Spaces  | Value |"), "Spaces");
        assert_eq!(extract_first_column("no pipe"), "");
        assert_eq!(extract_first_column("|only_one_pipe"), "only_one_pipe");
        assert_eq!(extract_first_column("| | second |"), "");
    }

    // ------------------------------------------------------------------
    // Table without separator line
    // ------------------------------------------------------------------

    #[test]
    fn table_without_separator_still_restructured() {
        let content = "| Cat | Name |\n| A | Item1 |\n| B | Item2 |";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(result.was_restructured);
        assert_eq!(result.table_rows, 2);
        assert!(result.content.contains("| --- |"));
    }

    // ------------------------------------------------------------------
    // Exact threshold boundary
    // ------------------------------------------------------------------

    #[test]
    fn threshold_boundary_exact_triggers() {
        // 2 non-table lines + 2 table lines = 4 total, ratio = 2/4 = 0.5
        // Condition is `ratio < threshold` to skip; 0.5 < 0.5 is false → proceeds to restructure.
        // WHY: "at least 50%" means ratio >= threshold triggers
        let content = "Text line.\nAnother.\n| A | B |\n| 1 | 2 |";
        let config = TablePreprocessorConfig {
            table_detection_threshold: 0.5,
            ..default_config()
        };
        let result = preprocess_tabular_content(content, &config);
        assert!(
            result.was_restructured,
            "Ratio exactly at threshold should trigger (>= semantics)"
        );
    }

    #[test]
    fn above_threshold_triggers_restructuring() {
        // 1 non-table line + 3 table lines = 4 total, ratio = 3/4 = 0.75 > 0.5
        let content = "Text.\n| A | B |\n| --- | --- |\n| 1 | 2 |";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(result.was_restructured);
    }

    // ------------------------------------------------------------------
    // Ordered groups (BTreeMap guarantees alphabetical)
    // ------------------------------------------------------------------

    #[test]
    fn groups_are_alphabetically_ordered() {
        let content = "\
| Cat | Name |
| --- | --- |
| Zebra | Z1 |
| Alpha | A1 |
| Mid | M1 |";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(result.was_restructured);
        let alpha_pos = result.content.find("## Alpha").unwrap();
        let mid_pos = result.content.find("## Mid").unwrap();
        let zebra_pos = result.content.find("## Zebra").unwrap();
        assert!(
            alpha_pos < mid_pos && mid_pos < zebra_pos,
            "Groups must be alphabetically ordered"
        );
    }

    // ------------------------------------------------------------------
    // Summary statistics correctness
    // ------------------------------------------------------------------

    #[test]
    fn summary_line_reflects_deduplication() {
        let content = "\
| Cat | Name |
| --- | --- |
| A | Same |
| A | Same |
| A | Same |
| B | Other |";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(result.was_restructured);
        assert_eq!(result.duplicates_removed, 2);
        assert!(result.content.contains("2 entries"));
    }

    // ------------------------------------------------------------------
    // Mixed content: table ratio matters
    // ------------------------------------------------------------------

    #[test]
    fn mixed_content_above_threshold_restructured() {
        // 1 text line, 4 table lines → ratio = 4/5 = 0.8 > 0.5
        let content = "Some intro text.\n| A | B |\n| --- | --- |\n| X | 1 |\n| Y | 2 |";
        let result = preprocess_tabular_content(content, &default_config());
        assert!(result.was_restructured);
    }
}
