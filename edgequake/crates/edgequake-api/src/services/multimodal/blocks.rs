//! LightRAG `blocks.jsonl` loader (`multimodal_context.load_content_rows_by_blockid`).

use std::collections::HashMap;
use std::path::Path;

/// Virtual block id when markdown is treated as a single content row (EdgeQuake sidecar).
pub const VIRTUAL_BLOCK_ID: &str = "edgequake-block-0";

fn stable_block_id(block_index: usize, heading: &str, content: &str) -> String {
    format!(
        "{:x}",
        md5::compute(format!("{block_index}:{heading}:{content}").as_bytes())
    )
}

/// Read `blocks.jsonl` text and return `{blockid: content}` for `type == "content"` rows.
///
/// When the same blockid appears multiple times, the first occurrence wins (LightRAG parity).
pub fn load_content_rows_by_blockid_jsonl(jsonl: &str) -> HashMap<String, String> {
    let mut rows = HashMap::new();
    for line in jsonl.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(map) = obj.as_object() else {
            continue;
        };
        if map.get("type").and_then(|v| v.as_str()) != Some("content") {
            continue;
        }
        let Some(blockid) = map.get("blockid").and_then(|v| v.as_str()) else {
            continue;
        };
        if blockid.is_empty() || rows.contains_key(blockid) {
            continue;
        }
        if let Some(content) = map.get("content").and_then(|v| v.as_str()) {
            rows.insert(blockid.to_string(), content.to_string());
        }
    }
    rows
}

/// Load from filesystem path; missing file yields empty map.
pub fn load_content_rows_by_blockid(path: impl AsRef<Path>) -> HashMap<String, String> {
    let path = path.as_ref();
    if !path.exists() {
        return HashMap::new();
    }
    std::fs::read_to_string(path)
        .map(|text| load_content_rows_by_blockid_jsonl(&text))
        .unwrap_or_default()
}

/// Treat flat markdown as one virtual block (virtual sidecar SSOT).
pub fn virtual_block_map(markdown: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert(VIRTUAL_BLOCK_ID.to_string(), markdown.to_string());
    map
}

/// Resolve block-scoped content for an item (`blockid` row or virtual fallback).
pub fn content_for_item<'a>(
    blocks: &'a HashMap<String, String>,
    block_id: Option<&str>,
    fallback_markdown: &'a str,
) -> &'a str {
    block_id
        .and_then(|id| blocks.get(id))
        .map(|s| s.as_str())
        .unwrap_or(fallback_markdown)
}

/// One markdown section block (LightRAG `blocks.jsonl` content row subset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownSection {
    pub block_id: String,
    pub heading: String,
    pub start: usize,
    pub end: usize,
    pub content: String,
}

/// Split markdown on ATX headings into section blocks (virtual IR when no `blocks.jsonl`).
pub fn split_markdown_sections(markdown: &str) -> Vec<MarkdownSection> {
    let mut heading_starts: Vec<(usize, String)> = Vec::new();
    let mut byte_offset = 0usize;
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        let hashes = trimmed.chars().take_while(|c| *c == '#').count();
        if hashes > 0 && hashes <= 6 {
            let rest = trimmed[hashes..].trim_start();
            if !rest.is_empty() {
                heading_starts.push((byte_offset, rest.to_string()));
            }
        }
        byte_offset += line.len() + 1;
    }

    if heading_starts.is_empty() {
        return vec![MarkdownSection {
            block_id: VIRTUAL_BLOCK_ID.to_string(),
            heading: String::new(),
            start: 0,
            end: markdown.len(),
            content: markdown.to_string(),
        }];
    }

    heading_starts
        .iter()
        .enumerate()
        .map(|(idx, (start, heading))| {
            let end = heading_starts
                .get(idx + 1)
                .map(|(s, _)| *s)
                .unwrap_or(markdown.len());
            let content = markdown[*start..end].to_string();
            MarkdownSection {
                block_id: stable_block_id(idx, heading, &content),
                heading: heading.clone(),
                start: *start,
                end,
                content,
            }
        })
        .collect()
}

/// `{blockid → content}` from section split.
pub fn blocks_map_from_sections(sections: &[MarkdownSection]) -> HashMap<String, String> {
    sections
        .iter()
        .map(|s| (s.block_id.clone(), s.content.clone()))
        .collect()
}

/// Resolve block id for a byte offset inside markdown sections.
pub fn block_id_for_offset(sections: &[MarkdownSection], offset: usize) -> Option<String> {
    sections
        .iter()
        .find(|s| offset >= s.start && offset < s.end)
        .map(|s| s.block_id.clone())
        .or_else(|| sections.last().map(|s| s.block_id.clone()))
}

/// Fill missing `block_id` on manifest items from section spans.
pub fn enrich_items_with_block_ids(
    items: &mut [super::manifest::ManifestItem],
    sections: &[MarkdownSection],
) {
    for item in items.iter_mut() {
        if item.block_id.is_none() {
            item.block_id = block_id_for_offset(sections, item.start);
        }
    }
}

/// Blocks map for analyze: prefer KV/jsonl rows, else heading-based virtual IR.
pub fn resolve_blocks_for_analyze(
    markdown: &str,
    blocks_jsonl: Option<&str>,
) -> HashMap<String, String> {
    if let Some(jsonl) = blocks_jsonl {
        let loaded = load_content_rows_by_blockid_jsonl(jsonl);
        if !loaded.is_empty() {
            return loaded;
        }
    }
    blocks_map_from_sections(&split_markdown_sections(markdown))
}

/// Prepare section list + blocks map for the analyze stage.
pub fn prepare_analyze_blocks(markdown: &str) -> (HashMap<String, String>, Vec<MarkdownSection>) {
    let sections = split_markdown_sections(markdown);
    let map = blocks_map_from_sections(&sections);
    (map, sections)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_content_rows_first_wins() {
        let jsonl = r#"{"type":"meta","format":"lightrag"}
{"type":"content","blockid":"b1","content":"block one"}
{"type":"content","blockid":"b1","content":"ignored duplicate"}
{"type":"content","blockid":"b2","content":"block two"}"#;
        let rows = load_content_rows_by_blockid_jsonl(jsonl);
        assert_eq!(rows.get("b1").map(|s| s.as_str()), Some("block one"));
        assert_eq!(rows.get("b2").map(|s| s.as_str()), Some("block two"));
    }

    #[test]
    fn split_sections_assigns_distinct_block_ids() {
        let md = "# Alpha\n\nText A.\n\n## Beta\n\n<table id=\"t1\"></table>";
        let sections = split_markdown_sections(md);
        assert_eq!(sections.len(), 2);
        assert_ne!(sections[0].block_id, sections[1].block_id);
        let map = blocks_map_from_sections(&sections);
        assert!(map.contains_key(&sections[1].block_id));
        assert!(map[&sections[1].block_id].contains("t1"));
    }

    #[test]
    fn block_id_for_offset_picks_containing_section() {
        let md = "# A\n\naaa\n\n# B\n\n<table id=\"tb\"></table>";
        let sections = split_markdown_sections(md);
        let table_start = md.find("<table").unwrap();
        let id = block_id_for_offset(&sections, table_start).unwrap();
        assert_eq!(id, sections[1].block_id);
    }
}
