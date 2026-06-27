//! Markdown placeholder scanning for virtual sidecar manifest (DRY SSOT).

use edgequake_pdf::inline_images::{scan_inline_image_refs, InlineImageRef};

use super::item_record::MultimodalItemRecord;
use super::manifest::ManifestItem;

/// Discover all multimodal manifest items in markdown (drawings, data-URIs, tables).
pub fn scan_manifest_items(markdown: &str) -> Vec<ManifestItem> {
    let mut items: Vec<ManifestItem> = scan_inline_image_refs(markdown)
        .into_iter()
        .map(manifest_item_from_image_ref)
        .collect();

    items.extend(scan_table_items(markdown));
    items.extend(scan_equation_items(markdown));
    items.sort_by_key(|i| i.start);
    items
}

fn scan_table_items(markdown: &str) -> Vec<ManifestItem> {
    let lower = markdown.to_ascii_lowercase();
    let mut items = Vec::new();
    let mut pos = 0usize;
    while let Some(rel) = lower[pos..].find("<table") {
        let start = pos + rel;
        let Some(close_rel) = lower[start..].find("</table>") else {
            break;
        };
        let end = start + close_rel + "</table>".len();
        let matched = &markdown[start..end];
        let open_end = matched.find('>').map(|i| start + i + 1).unwrap_or(start);
        let attrs = &markdown[start..open_end.min(end)];
        let body = &markdown[open_end.min(end)..end.saturating_sub("</table>".len())];
        let item_id = extract_attr(attrs, "id").unwrap_or_else(|| format!("table_{}", items.len()));
        let format = extract_attr(attrs, "format").unwrap_or_else(|| "html".into());
        items.push(ManifestItem {
            item_id,
            modality: "table".into(),
            start,
            end,
            matched: matched.to_string(),
            asset_path: None,
            mime_type: Some(format),
            body: Some(body.trim().to_string()),
            caption: extract_attr(attrs, "caption"),
            footnote: extract_attr(attrs, "footnote"),
            footnotes: Vec::new(),
            block_id: extract_attr(attrs, "blockid"),
            heading: None,
            analyze_result: None,
        });
        pos = end;
    }
    items
}

fn scan_equation_items(markdown: &str) -> Vec<ManifestItem> {
    let lower = markdown.to_ascii_lowercase();
    let mut items = Vec::new();
    let mut pos = 0usize;
    while let Some(rel) = lower[pos..].find("<equation") {
        let start = pos + rel;
        let open_end = lower[start..]
            .find('>')
            .map(|i| start + i + 1)
            .unwrap_or(start);
        let attrs = &markdown[start..open_end.min(markdown.len())];
        let item_id = match extract_attr(attrs, "id") {
            Some(id) if !id.is_empty() => id,
            _ => {
                pos = open_end;
                continue;
            }
        };
        let end = if let Some(close_rel) = lower[open_end..].find("</equation>") {
            open_end + close_rel + "</equation>".len()
        } else if attrs.ends_with("/>") || attrs.ends_with("/ >") {
            open_end
        } else {
            pos = open_end;
            continue;
        };
        let matched = &markdown[start..end];
        let body_start = if attrs.contains("/>") {
            String::new()
        } else {
            markdown[open_end..end.saturating_sub("</equation>".len())]
                .trim()
                .to_string()
        };
        items.push(ManifestItem {
            item_id,
            modality: "equation".into(),
            start,
            end,
            matched: matched.to_string(),
            asset_path: None,
            mime_type: Some("latex".into()),
            body: Some(body_start),
            caption: extract_attr(attrs, "caption"),
            footnote: extract_attr(attrs, "footnote"),
            footnotes: Vec::new(),
            block_id: extract_attr(attrs, "blockid"),
            heading: None,
            analyze_result: None,
        });
        pos = end.max(open_end + 1);
    }
    items
}

/// Locate item byte span by id (LightRAG `find_target_span`; DRY via manifest scan).
pub fn span_for_item(markdown: &str, item_id: &str, modality: &str) -> Option<(usize, usize)> {
    scan_manifest_items(markdown)
        .into_iter()
        .find(|i| i.item_id == item_id && i.modality == modality)
        .map(|i| (i.start, i.end))
        .or_else(|| {
            if modality == "table" {
                find_table_cite_span(markdown, item_id)
            } else {
                None
            }
        })
}

/// Table cite marker span (`<cite type="table" refid="…">`).
pub fn find_table_cite_span(markdown: &str, item_id: &str) -> Option<(usize, usize)> {
    let lower = markdown.to_ascii_lowercase();
    let mut pos = 0usize;
    while let Some(rel) = lower[pos..].find("<cite") {
        let start = pos + rel;
        let open_end = lower[start..]
            .find('>')
            .map(|i| start + i + 1)
            .unwrap_or(start);
        let attrs = &markdown[start..open_end.min(markdown.len())];
        let attrs_lower = attrs.to_ascii_lowercase();
        if !attrs_lower.contains("type=\"table\"") {
            pos = open_end;
            continue;
        }
        if extract_attr(attrs, "refid").as_deref() != Some(item_id) {
            pos = open_end;
            continue;
        }
        let end = lower[open_end..]
            .find("</cite>")
            .map(|i| open_end + i + "</cite>".len())?;
        return Some((start, end));
    }
    None
}

fn extract_attr(attrs: &str, name: &str) -> Option<String> {
    let needle = format!(r#"{name}=""#);
    let lower = attrs.to_ascii_lowercase();
    let start = lower.find(&needle)?;
    let rest = &attrs[start + needle.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn manifest_item_from_image_ref(image_ref: InlineImageRef) -> ManifestItem {
    ManifestItem {
        item_id: image_ref.item_id,
        modality: "drawing".into(),
        start: image_ref.start,
        end: image_ref.end,
        matched: image_ref.matched,
        asset_path: image_ref.asset_path,
        mime_type: if image_ref.mime_type.is_empty() {
            None
        } else {
            Some(image_ref.mime_type)
        },
        body: None,
        caption: image_ref.caption,
        footnote: image_ref.footnote,
        footnotes: Vec::new(),
        block_id: None,
        heading: None,
        analyze_result: None,
    }
}

/// Build a single-item manifest for standalone image upload.
pub fn standalone_image_manifest(
    record: MultimodalItemRecord,
) -> super::manifest::MultimodalManifest {
    super::manifest::MultimodalManifest {
        version: super::manifest::MultimodalManifest::CURRENT_VERSION,
        items: vec![ManifestItem {
            item_id: record.item_id.clone(),
            modality: record.modality.clone(),
            start: 0,
            end: 0,
            matched: String::new(),
            asset_path: None,
            mime_type: None,
            body: None,
            caption: None,
            footnote: None,
            footnotes: Vec::new(),
            block_id: None,
            heading: None,
            analyze_result: Some(record),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_finds_table_tag_with_id() {
        let md = r#"Before <table id="tb-1" format="html"><tr><td>A</td></tr></table> after"#;
        let items = scan_manifest_items(md);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].modality, "table");
        assert_eq!(items[0].item_id, "tb-1");
        assert!(items[0].body.as_ref().is_some_and(|b| b.contains("<td>")));
    }

    #[test]
    fn scan_finds_equation_with_id() {
        let md = r#"Text <equation id="eq-1">E=mc^2</equation> end"#;
        let items = scan_manifest_items(md);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].modality, "equation");
        assert_eq!(items[0].item_id, "eq-1");
    }

    #[test]
    fn scan_skips_equation_without_id() {
        let md = r#"<equation>E=mc^2</equation>"#;
        assert!(scan_manifest_items(md).is_empty());
    }
}
