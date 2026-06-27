//! Surrounding context for multimodal prompts (LightRAG `multimodal_context.py`).

use std::collections::HashMap;

use super::blocks::content_for_item;
use super::chunk_budget::max_mm_chunk_tokens;
use super::manifest::ManifestItem;
use super::surrounding::{
    build_surrounding, char_trim_trailing, find_target_span, load_chunk_separators,
    row_trim_table_trailing, surrounding_leading_max_tokens, surrounding_trailing_max_tokens,
    SurroundingKind, SurroundingTokenCounter,
};

const CONTENT_TRUNCATION_MARKER: &str =
    "\n<!-- content truncated from {original} to {final} tokens, head preserved -->";

/// Leading/trailing prose around a multimodal item for VLM/Extract prompts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SurroundingContext {
    pub leading: String,
    pub trailing: String,
}

impl SurroundingContext {
    /// Build surrounding from markdown byte span (uses item modality when known).
    pub fn from_markdown(markdown: &str, start: usize, end: usize) -> Self {
        Self::from_span(markdown, (start, end), SurroundingKind::Drawings)
    }

    /// Build surrounding for a manifest item (section/block-scoped IR).
    pub fn from_item(markdown: &str, item: &ManifestItem) -> Self {
        let blocks = super::blocks::resolve_blocks_for_analyze(markdown, None);
        Self::from_item_with_blocks(markdown, item, &blocks)
    }

    /// Build surrounding using an explicit blocks map (LightRAG `blocks.jsonl` rows).
    pub fn from_item_with_blocks(
        fallback_markdown: &str,
        item: &ManifestItem,
        blocks: &HashMap<String, String>,
    ) -> Self {
        let kind = SurroundingKind::from_modality(&item.modality);
        let block_content = content_for_item(blocks, item.block_id.as_deref(), fallback_markdown);
        let span = find_target_span(kind, &item.item_id, block_content).unwrap_or_else(|| {
            if std::ptr::eq(block_content.as_ptr(), fallback_markdown.as_ptr()) {
                (
                    item.start.min(block_content.len()),
                    item.end.min(block_content.len()),
                )
            } else {
                let section_start = fallback_markdown.find(block_content).unwrap_or(0);
                (
                    item.start
                        .saturating_sub(section_start)
                        .min(block_content.len()),
                    item.end
                        .saturating_sub(section_start)
                        .min(block_content.len()),
                )
            }
        });
        Self::from_span(block_content, span, kind)
    }

    /// Token-budget surrounding at a span (LightRAG `build_surrounding`).
    pub fn from_span(markdown: &str, span: (usize, usize), kind: SurroundingKind) -> Self {
        let separators = load_chunk_separators();
        let counter = SurroundingTokenCounter::from_env();
        let lead_tokens = surrounding_leading_max_tokens();
        let trail_tokens = surrounding_trailing_max_tokens();

        build_surrounding(
            kind,
            markdown,
            span,
            lead_tokens,
            trail_tokens,
            &separators,
            counter,
        )
    }

    /// Build from item_id lookup inside block content (sidecar backfill parity).
    pub fn from_item_id(block_content: &str, item_id: &str, kind: SurroundingKind) -> Option<Self> {
        let span = find_target_span(kind, item_id, block_content)?;
        Some(Self::from_span(block_content, span, kind))
    }
}

/// Max tokens for table/equation body sent to Extract role (LightRAG `DEFAULT_MAX_EXTRACT_INPUT_TOKENS`).
pub fn max_extract_input_tokens() -> usize {
    max_mm_chunk_tokens()
}

/// Legacy char cap (approximation for callers still using char budgets).
pub fn max_extract_input_chars() -> usize {
    std::env::var("MAX_EXTRACT_INPUT_CHARS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n >= 256)
        .unwrap_or(max_extract_input_tokens() * 4)
}

fn unwrap_table_inner(wrapped: &str) -> Option<String> {
    let trimmed = wrapped.trim();
    if !trimmed.starts_with("<table") {
        return None;
    }
    let open_end = trimmed.find('>')? + 1;
    let close = trimmed.rfind("</table>")?;
    Some(trimmed[open_end..close].trim().to_string())
}

/// Trim oversized table/equation bodies; returns `(text, was_trimmed)`.
///
/// LightRAG `trim_content_to_budget` — row-aware for `<table>` wrappers, char fallback otherwise.
pub fn trim_content_to_budget(
    content: &str,
    max_tokens: usize,
    kind: SurroundingKind,
) -> (String, bool) {
    let counter = SurroundingTokenCounter::from_env();
    let trimmed = content.trim();
    if trimmed.is_empty() || max_tokens == 0 {
        return (trimmed.to_string(), false);
    }
    let original_tokens = counter.count(trimmed);
    if original_tokens <= max_tokens {
        return (trimmed.to_string(), false);
    }

    let marker_probe = CONTENT_TRUNCATION_MARKER
        .replace("{original}", &original_tokens.to_string())
        .replace("{final}", &max_tokens.to_string());
    let marker_tokens = counter.count(&marker_probe);
    let inner_budget = max_tokens.saturating_sub(marker_tokens);

    let unwrapped_input = !trimmed.starts_with("<table");
    let wrapped = if kind == SurroundingKind::Tables && unwrapped_input {
        format!("<table>{trimmed}</table>")
    } else {
        trimmed.to_string()
    };

    let trimmed_inner = if kind == SurroundingKind::Tables && wrapped.trim().starts_with("<table") {
        row_trim_table_trailing(&wrapped, inner_budget, counter)
            .unwrap_or_else(|| char_trim_trailing(&wrapped, inner_budget, counter))
    } else {
        char_trim_trailing(trimmed, inner_budget, counter)
    };

    let body = if kind == SurroundingKind::Tables && unwrapped_input {
        unwrap_table_inner(&trimmed_inner).unwrap_or(trimmed_inner)
    } else {
        trimmed_inner
    };

    let final_tokens = counter.count(&body);
    let marker = CONTENT_TRUNCATION_MARKER
        .replace("{original}", &original_tokens.to_string())
        .replace("{final}", &final_tokens.to_string());
    (format!("{body}{marker}"), true)
}

#[cfg(test)]
mod tests {
    use super::super::blocks::{enrich_items_with_block_ids, prepare_analyze_blocks};
    use super::super::manifest::ManifestItem;
    use super::super::scan::scan_manifest_items;
    use super::super::surrounding::{find_target_span, SurroundingKind};
    use super::*;

    #[test]
    fn from_item_strips_sibling_tables_for_table_modality() {
        let md = concat!(
            "<table id=\"tb-other\" format=\"json\">[[\"a\"]]</table> ",
            "narrative. ",
            "<table id=\"tb-target\" format=\"json\">[[\"x\"]]</table>",
            " tail."
        );
        let span = find_target_span(SurroundingKind::Tables, "tb-target", md).unwrap();
        let item = ManifestItem {
            item_id: "tb-target".into(),
            modality: "table".into(),
            start: span.0,
            end: span.1,
            matched: String::new(),
            asset_path: None,
            mime_type: Some("json".into()),
            body: None,
            caption: None,
            footnote: None,
            footnotes: Vec::new(),
            block_id: None,
            heading: None,
            analyze_result: None,
        };
        std::env::set_var("EDGEQUAKE_MM_SURROUNDING_TOKENS", "char");
        let ctx = SurroundingContext::from_item(md, &item);
        assert!(ctx.leading.contains("narrative"));
        assert!(!ctx.leading.contains("tb-other"));
        std::env::remove_var("EDGEQUAKE_MM_SURROUNDING_TOKENS");
    }

    #[test]
    fn enrich_does_not_cross_section_boundaries() {
        let md = concat!(
            "# Section A\n\n",
            "Intro A with unique marker ALPHA.\n\n",
            "# Section B\n\n",
            "Before table. ",
            "<table id=\"tb-target\" format=\"json\">[[\"x\"]]</table>",
            " tail."
        );
        std::env::set_var("EDGEQUAKE_MM_SURROUNDING_TOKENS", "char");
        let (blocks, sections) = prepare_analyze_blocks(md);
        let mut items = scan_manifest_items(md);
        enrich_items_with_block_ids(&mut items, &sections);
        let item = items
            .iter()
            .find(|i| i.item_id == "tb-target")
            .expect("table item");
        let ctx = SurroundingContext::from_item_with_blocks(md, item, &blocks);
        assert!(ctx.leading.contains("Before table"));
        assert!(!ctx.leading.contains("ALPHA"));
        assert!(!ctx.leading.contains("Intro A"));
        std::env::remove_var("EDGEQUAKE_MM_SURROUNDING_TOKENS");
    }

    #[test]
    fn trim_content_row_aware_for_table_wrapper() {
        std::env::set_var("EDGEQUAKE_MM_SURROUNDING_TOKENS", "char");
        let rows = (0..20)
            .map(|i| format!("<tr><td>{i}</td></tr>"))
            .collect::<Vec<_>>()
            .join("");
        let table = format!(r#"<table format="html">{rows}</table>"#);
        let (text, trimmed) = trim_content_to_budget(&table, 120, SurroundingKind::Tables);
        assert!(trimmed);
        assert!(text.contains("<tr>"));
        assert!(text.contains("truncated"));
        std::env::remove_var("EDGEQUAKE_MM_SURROUNDING_TOKENS");
    }

    #[test]
    fn trim_content_adds_marker_when_over_budget() {
        std::env::set_var("EDGEQUAKE_MM_SURROUNDING_TOKENS", "char");
        let (text, trimmed) = trim_content_to_budget("abcdef", 3, SurroundingKind::Equations);
        assert!(trimmed);
        assert!(text.contains("truncated"));
        std::env::remove_var("EDGEQUAKE_MM_SURROUNDING_TOKENS");
    }
}
