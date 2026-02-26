//! Markdown table preprocessor for RAG-friendly chunking.
//!
//! WHY: Excel exports and large markdown tables cause problems in the RAG pipeline:
//! 1. Chunks split mid-row, losing context
//! 2. Chunks lose the table header, making LLM extraction unreliable
//! 3. Highly repetitive rows create explosion of similar entities
//! 4. Large tables (3000+ rows) generate 100+ chunks → expensive LLM calls
//!
//! This preprocessor detects markdown tables and restructures them into
//! semantically coherent sections grouped by the first column value.
//! Each section gets its own header, enabling the chunker to create
//! meaningful, self-contained chunks.
//!
//! @implements FIX-EXCEL-CHUNKING: Optimize large tabular document processing

use std::collections::BTreeMap;

/// Configuration for table preprocessing.
#[derive(Debug, Clone)]
pub struct TablePreprocessorConfig {
    /// Minimum percentage of lines that must be table rows to trigger preprocessing.
    /// Default: 0.5 (50%)
    pub table_detection_threshold: f64,
    /// Maximum number of rows per group before summarization.
    /// Groups larger than this get a summary prefix instead of all rows.
    /// Default: 50
    pub max_rows_per_group: usize,
    /// Whether to deduplicate identical rows within a group.
    /// Default: true
    pub deduplicate_rows: bool,
}

impl Default for TablePreprocessorConfig {
    fn default() -> Self {
        Self {
            table_detection_threshold: 0.5,
            max_rows_per_group: 50,
            deduplicate_rows: true,
        }
    }
}

/// Result of preprocessing analysis.
#[derive(Debug)]
pub struct PreprocessResult {
    /// Preprocessed content (restructured into sections).
    pub content: String,
    /// Whether the content was detected as tabular and was restructured.
    pub was_restructured: bool,
    /// Number of table rows detected.
    pub table_rows: usize,
    /// Number of groups created.
    pub groups: usize,
    /// Number of duplicate rows removed.
    pub duplicates_removed: usize,
}

/// Preprocess markdown content to optimize tabular data for RAG chunking.
///
/// If the content is predominantly a markdown table (>50% of lines start with `|`),
/// this function:
/// 1. Extracts the table header
/// 2. Groups rows by the first column value
/// 3. Deduplicates identical rows within each group
/// 4. Emits each group as a separate markdown section with the header repeated
///
/// This creates natural paragraph boundaries that the chunker can use,
/// resulting in semantically coherent chunks.
pub fn preprocess_tabular_content(
    content: &str,
    config: &TablePreprocessorConfig,
) -> PreprocessResult {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return PreprocessResult {
            content: content.to_string(),
            was_restructured: false,
            table_rows: 0,
            groups: 0,
            duplicates_removed: 0,
        };
    }

    // Detect if content is predominantly a markdown table
    let table_lines: Vec<&str> = lines
        .iter()
        .filter(|l| l.trim().starts_with('|'))
        .copied()
        .collect();

    let non_empty_lines = lines.iter().filter(|l| !l.trim().is_empty()).count();
    if non_empty_lines == 0 {
        return PreprocessResult {
            content: content.to_string(),
            was_restructured: false,
            table_rows: 0,
            groups: 0,
            duplicates_removed: 0,
        };
    }

    let table_ratio = table_lines.len() as f64 / non_empty_lines as f64;

    if table_ratio < config.table_detection_threshold {
        return PreprocessResult {
            content: content.to_string(),
            was_restructured: false,
            table_rows: table_lines.len(),
            groups: 0,
            duplicates_removed: 0,
        };
    }

    // Parse table structure
    let (header, separator, data_rows) = parse_table_structure(&table_lines);

    if header.is_none() || data_rows.is_empty() {
        return PreprocessResult {
            content: content.to_string(),
            was_restructured: false,
            table_rows: table_lines.len(),
            groups: 0,
            duplicates_removed: 0,
        };
    }

    let header = header.unwrap();
    let separator = separator.unwrap_or_else(|| "| --- | --- | --- |".to_string());

    // Group rows by first column value
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut total_duplicates = 0usize;

    for row in &data_rows {
        let first_col = extract_first_column(row);
        let group_key = first_col.trim().to_string();
        let entry = groups.entry(group_key).or_default();

        if config.deduplicate_rows {
            if !entry.contains(row) {
                entry.push(row.clone());
            } else {
                total_duplicates += 1;
            }
        } else {
            entry.push(row.clone());
        }
    }

    // Emit restructured content
    let mut output = String::with_capacity(content.len());
    let group_count = groups.len();

    // Add document-level header
    output.push_str("# Glossary / Data Dictionary\n\n");
    output.push_str(&format!(
        "> This document contains {} entries organized into {} categories.\n\n",
        data_rows.len() - total_duplicates,
        group_count
    ));

    for (group_name, rows) in &groups {
        let display_name = if group_name.is_empty() {
            "General"
        } else {
            group_name
        };

        // Section header for natural chunk boundary
        output.push_str(&format!("## {}\n\n", display_name));

        // Repeat table header for each section
        output.push_str(&header);
        output.push('\n');
        output.push_str(&separator);
        output.push('\n');

        // If group is too large, add a note and limit rows
        if rows.len() > config.max_rows_per_group {
            output.push_str(&format!(
                "| *({} entries in this category — showing first {})* | | |\n",
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

    PreprocessResult {
        content: output,
        was_restructured: true,
        table_rows: data_rows.len(),
        groups: group_count,
        duplicates_removed: total_duplicates,
    }
}

/// Parse the table structure: header, separator line, and data rows.
fn parse_table_structure(lines: &[&str]) -> (Option<String>, Option<String>, Vec<String>) {
    if lines.is_empty() {
        return (None, None, Vec::new());
    }

    let mut header: Option<String> = None;
    let mut separator: Option<String> = None;
    let mut data_rows: Vec<String> = Vec::new();

    for line in lines {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Detect separator line (| --- | --- | --- |)
        if is_separator_line(trimmed) {
            separator = Some(trimmed.to_string());
            continue;
        }

        // First non-separator line is the header
        if header.is_none() {
            header = Some(trimmed.to_string());
            continue;
        }

        // Everything else is a data row
        data_rows.push(trimmed.to_string());
    }

    (header, separator, data_rows)
}

/// Check if a line is a markdown table separator (e.g., `| --- | --- |`).
fn is_separator_line(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return false;
    }
    // All cells should be dashes (with optional colons for alignment)
    trimmed
        .split('|')
        .filter(|s| !s.trim().is_empty())
        .all(|cell| {
            let cell = cell.trim();
            cell.chars()
                .all(|c| c == '-' || c == ':' || c.is_whitespace())
        })
}

/// Extract the first column value from a markdown table row.
fn extract_first_column(row: &str) -> String {
    let trimmed = row.trim();
    if !trimmed.starts_with('|') {
        return String::new();
    }

    // Split by | and get first non-empty cell
    let cells: Vec<&str> = trimmed.split('|').collect();
    // cells[0] is empty (before first |), cells[1] is first column
    if cells.len() > 1 {
        cells[1].trim().to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_table_content_passes_through() {
        let content = "# Hello World\n\nThis is a paragraph.\n\nAnother paragraph.";
        let result = preprocess_tabular_content(content, &TablePreprocessorConfig::default());
        assert!(!result.was_restructured);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_table_detection() {
        let content = "| Col1 | Col2 | Col3 |\n| --- | --- | --- |\n| A | B | C |\n| D | E | F |";
        let result = preprocess_tabular_content(content, &TablePreprocessorConfig::default());
        assert!(result.was_restructured);
        assert_eq!(result.table_rows, 2);
        assert_eq!(result.groups, 2); // A and D groups
    }

    #[test]
    fn test_grouping_by_first_column() {
        let content = "\
| Category | Name | Description |
| --- | --- | --- |
| Dashboard | Metric1 | Desc1 |
| Dashboard | Metric2 | Desc2 |
| Report | Metric3 | Desc3 |";
        let result = preprocess_tabular_content(content, &TablePreprocessorConfig::default());
        assert!(result.was_restructured);
        assert_eq!(result.groups, 2); // Dashboard and Report
        assert!(result.content.contains("## Dashboard"));
        assert!(result.content.contains("## Report"));
    }

    #[test]
    fn test_deduplication() {
        let content = "\
| Category | Name | Description |
| --- | --- | --- |
| A | Same | Same desc |
| A | Same | Same desc |
| A | Different | Other desc |";
        let result = preprocess_tabular_content(content, &TablePreprocessorConfig::default());
        assert!(result.was_restructured);
        assert_eq!(result.duplicates_removed, 1);
    }

    #[test]
    fn test_separator_detection() {
        assert!(is_separator_line("| --- | --- | --- |"));
        assert!(is_separator_line("| :--- | ---: | :---: |"));
        assert!(!is_separator_line("| hello | world |"));
        assert!(!is_separator_line("not a table"));
    }

    #[test]
    fn test_extract_first_column() {
        assert_eq!(extract_first_column("| Hello | World |"), "Hello");
        assert_eq!(extract_first_column("|  Spaces  | Value |"), "Spaces");
        assert_eq!(extract_first_column("no pipe"), "");
    }

    #[test]
    fn test_header_repeated_per_group() {
        let content = "\
| Category | Name | Description |
| --- | --- | --- |
| A | Item1 | Desc1 |
| B | Item2 | Desc2 |";
        let result = preprocess_tabular_content(content, &TablePreprocessorConfig::default());
        // Header "| Category | Name | Description |" should appear twice (once per group)
        let header_count = result
            .content
            .matches("| Category | Name | Description |")
            .count();
        assert_eq!(header_count, 2);
    }

    #[test]
    fn test_large_group_truncation() {
        let mut content = String::from("| Cat | Name | Desc |\n| --- | --- | --- |\n");
        for i in 0..100 {
            content.push_str(&format!("| BigGroup | Item{} | Desc{} |\n", i, i));
        }
        let config = TablePreprocessorConfig {
            max_rows_per_group: 50,
            ..Default::default()
        };
        let result = preprocess_tabular_content(&content, &config);
        assert!(result.was_restructured);
        // Should contain truncation note
        assert!(result.content.contains("100 entries"));
    }

    #[test]
    fn test_below_threshold_not_restructured() {
        // 4 non-table lines + 3 table lines = 3/7 ≈ 0.43 which is below 0.5 threshold
        let content = "# Title\n\nParagraph 1.\n\nParagraph 2.\n\nMore text here.\n\n| A | B |\n| --- | --- |\n| 1 | 2 |";
        let result = preprocess_tabular_content(content, &TablePreprocessorConfig::default());
        assert!(!result.was_restructured);
    }
}
