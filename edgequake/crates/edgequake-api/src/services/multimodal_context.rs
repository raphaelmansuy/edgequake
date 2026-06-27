//! Surrounding context for multimodal VLM prompts (LightRAG `multimodal_context.py` stub).
//!
//! Phase 4b: extract leading/trailing prose around a placeholder for future
//! prompt enrichment; images-only path uses this when drawing tags are present.

/// Extract leading and trailing text around a byte range in markdown.
pub fn surrounding_context(
    markdown: &str,
    start: usize,
    end: usize,
    max_chars: usize,
) -> (String, String) {
    let leading = markdown[..start.min(markdown.len())]
        .chars()
        .rev()
        .take(max_chars)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    let trailing = markdown[end.min(markdown.len())..]
        .chars()
        .take(max_chars)
        .collect::<String>();
    (leading.trim().to_string(), trailing.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_leading_and_trailing() {
        let md = "Before paragraph.\n<drawing id=\"x\" />\nAfter paragraph.";
        let tag = "<drawing id=\"x\" />";
        let start = md.find(tag).unwrap();
        let end = start + tag.len();
        let (lead, trail) = surrounding_context(md, start, end, 32);
        assert!(lead.contains("Before"));
        assert!(trail.contains("After"));
    }
}
