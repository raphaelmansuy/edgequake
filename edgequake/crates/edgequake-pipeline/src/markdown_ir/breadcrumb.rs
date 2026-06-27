//! Heading breadcrumb formatting (LightRAG chunk_schema.py parity).

pub const HEADING_BREADCRUMB_SEP: &str = " → ";
pub const DEFAULT_HEADING_LEVEL_MAX_CHARS: usize = 80;

/// Join parent headings + current heading into breadcrumb path.
pub fn format_breadcrumb(parents: &[String], heading: &str) -> String {
    let mut chain: Vec<String> = parents
        .iter()
        .map(|h| cap_heading(h))
        .filter(|h| !h.is_empty())
        .collect();
    let current = cap_heading(heading);
    if !current.is_empty() && heading != super::parse::PREFACE_HEADING {
        chain.push(current);
    }
    chain.join(HEADING_BREADCRUMB_SEP)
}

fn cap_heading(s: &str) -> String {
    let cleaned = s
        .replace('→', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if cleaned.chars().count() <= DEFAULT_HEADING_LEVEL_MAX_CHARS {
        cleaned
    } else {
        let truncated: String = cleaned
            .chars()
            .take(DEFAULT_HEADING_LEVEL_MAX_CHARS)
            .collect();
        format!("{truncated}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breadcrumb_joins_parents_and_heading() {
        assert_eq!(
            format_breadcrumb(&["Install".into()], "Prerequisites"),
            "Install → Prerequisites"
        );
    }
}
