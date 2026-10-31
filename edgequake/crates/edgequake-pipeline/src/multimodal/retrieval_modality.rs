//! SPEC-047 MV-23 — retrieval modality on vector/KV chunk metadata.
//!
//! First principle: chart numeric answers must be retrievable via filtered search
//! (`modality=chart|figure|table|equation`) without re-parsing chunk bodies at query time.

use std::sync::LazyLock;

use regex::Regex;

use crate::chunker::TextChunk;

use super::injection::MmChunkSidecarMeta;

static IMAGE_TYPE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\[Image Type\](.+)$").expect("image type regex"));

static VLM_TYPE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\*\*Type:\*\*\s*(.+)$").expect("vlm type regex"));

/// Retrieval filter values persisted on chunk vector metadata (MV-23 / MV-32).
pub const MODALITY_CHART: &str = "chart";
pub const MODALITY_FIGURE: &str = "figure";
pub const MODALITY_TABLE: &str = "table";
pub const MODALITY_EQUATION: &str = "equation";
/// SPEC-180: the chunk is an image embedded as pixels (`chunks.source_modality = 'image'`).
pub const MODALITY_IMAGE: &str = "image";

/// True when `chunk_content` contains the mm sidecar text (shared with KG injection).
pub fn chunk_matches_mm_sidecar(chunk_content: &str, mm: &MmChunkSidecarMeta) -> bool {
    chunk_content.contains(&mm.text)
        || (chunk_content.contains("[Image Name]") && mm.text.starts_with("[Image Name]"))
        || (chunk_content.contains("[Chart Name]") && mm.text.starts_with("[Chart Name]"))
        || (chunk_content.contains("[Figure Name]") && mm.text.starts_with("[Figure Name]"))
        || (chunk_content.contains("[Table Name]") && mm.text.starts_with("[Table Name]"))
        || (chunk_content.contains("[Equation Name]") && mm.text.starts_with("[Equation Name]"))
}

fn parse_image_type_label(content: &str) -> Option<String> {
    IMAGE_TYPE_RE
        .captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_vlm_type_label(content: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(cap) = VLM_TYPE_RE.captures(line.trim()) {
            let label = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            if !label.is_empty() {
                return Some(label.to_string());
            }
        }
    }
    None
}

/// Map VLM / LightRAG image type label to retrieval modality (None = plain prose image).
pub fn map_image_type_to_retrieval_modality(image_type: &str) -> Option<&'static str> {
    match image_type.trim() {
        "Chart" | "Infographic" => Some(MODALITY_CHART),
        "Illustration" | "Flowchart" | "Wireframe" => Some(MODALITY_FIGURE),
        "Table" => Some(MODALITY_TABLE),
        _ => None,
    }
}

/// Infer retrieval modality from chunk body (mm labels, VLM inline, chart specialize prose).
pub fn resolve_retrieval_modality_from_content(content: &str) -> Option<&'static str> {
    if content.contains("[Table Name]") {
        return Some(MODALITY_TABLE);
    }
    if content.contains("[Equation Name]") {
        return Some(MODALITY_EQUATION);
    }
    if content.contains("[Chart Name]") {
        return Some(MODALITY_CHART);
    }
    if content.contains("[Figure Name]") {
        return Some(MODALITY_FIGURE);
    }
    if content.contains("[Image Name]") {
        if let Some(t) = parse_image_type_label(content) {
            if let Some(m) = map_image_type_to_retrieval_modality(&t) {
                return Some(m);
            }
        }
    }
    if let Some(t) = parse_vlm_type_label(content) {
        if let Some(m) = map_image_type_to_retrieval_modality(&t) {
            return Some(m);
        }
    }
    // Chart specialize body without repeating **Type:** in a split chunk.
    if content.contains("**Chart kind:**") || content.contains("**Key values:**") {
        return Some(MODALITY_CHART);
    }
    None
}

/// Resolve from persisted mm sidecar row (uses modality + rendered text).
pub fn resolve_retrieval_modality_from_mm(mm: &MmChunkSidecarMeta) -> Option<&'static str> {
    // Consts are already the filter strings ("table" / "equation"); no literal aliases.
    match mm.modality.as_str() {
        MODALITY_TABLE => Some(MODALITY_TABLE),
        MODALITY_EQUATION => Some(MODALITY_EQUATION),
        "drawing" => resolve_retrieval_modality_from_content(&mm.text),
        _ => resolve_retrieval_modality_from_content(&mm.text),
    }
}

/// Stamp `TextChunk.modality` for filtered vector retrieve (MV-23).
pub fn stamp_retrieval_modality_on_chunks(
    chunks: &mut [TextChunk],
    mm_chunks: &[MmChunkSidecarMeta],
) {
    for chunk in chunks.iter_mut() {
        if chunk.modality.is_some() {
            continue;
        }
        if let Some(mm) = mm_chunks
            .iter()
            .find(|m| chunk_matches_mm_sidecar(&chunk.content, m))
        {
            if let Some(modality) = resolve_retrieval_modality_from_mm(mm) {
                chunk.modality = Some(modality.to_string());
                continue;
            }
        }
        if let Some(modality) = resolve_retrieval_modality_from_content(&chunk.content) {
            chunk.modality = Some(modality.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::injection::{MmSidecarBlock, MmSidecarRef};
    use super::*;
    use crate::chunker::TextChunk;

    fn chart_mm_text() -> String {
        "[Chart Name]rev_q4\n[Image Type]Chart\n\n**Key values:**\n- Q4: 42".into()
    }

    fn chart_mm_meta() -> MmChunkSidecarMeta {
        MmChunkSidecarMeta {
            item_id: "d1".into(),
            modality: "drawing".into(),
            text: chart_mm_text(),
            sidecar: MmSidecarBlock {
                sidecar_type: "drawing".into(),
                id: "d1".into(),
                refs: vec![MmSidecarRef {
                    ref_type: "drawing".into(),
                    id: "d1".into(),
                }],
            },
            heading: None,
            llm_cache_list: vec![],
        }
    }

    #[test]
    fn resolves_chart_from_mm_labels() {
        assert_eq!(
            resolve_retrieval_modality_from_content(&chart_mm_text()),
            Some(MODALITY_CHART)
        );
    }

    #[test]
    fn resolves_table_and_equation_labels() {
        assert_eq!(
            resolve_retrieval_modality_from_content("[Table Name]t1\n\n| A |"),
            Some(MODALITY_TABLE)
        );
        assert_eq!(
            resolve_retrieval_modality_from_content("E=mc^2\n[Equation Name]eq1"),
            Some(MODALITY_EQUATION)
        );
    }

    #[test]
    fn resolves_vlm_inline_chart_block() {
        let body = "# rev q4\n\n**Type:** Chart\n\n**Key values:**\n- Q4: 42";
        assert_eq!(
            resolve_retrieval_modality_from_content(body),
            Some(MODALITY_CHART)
        );
    }

    #[test]
    fn stamp_from_sidecar_and_content() {
        let mm = chart_mm_meta();
        let mut chunks = vec![TextChunk {
            id: "c0".into(),
            content: mm.text.clone(),
            index: 0,
            start_offset: 0,
            end_offset: 0,
            start_line: 1,
            end_line: 1,
            token_count: 10,
            embedding: None,
            section: None,
            page_start: None,
            page_end: None,
            modality: None,
        }];
        stamp_retrieval_modality_on_chunks(&mut chunks, &[mm]);
        assert_eq!(chunks[0].modality.as_deref(), Some(MODALITY_CHART));
    }

    #[test]
    fn stamp_vlm_inline_without_sidecar() {
        let body = "# rev q4\n\n**Type:** Chart\n\n- Q4: 42";
        let mut chunks = vec![TextChunk {
            id: "c0".into(),
            content: body.into(),
            index: 0,
            start_offset: 0,
            end_offset: 0,
            start_line: 1,
            end_line: 1,
            token_count: 10,
            embedding: None,
            section: None,
            page_start: None,
            page_end: None,
            modality: None,
        }];
        stamp_retrieval_modality_on_chunks(&mut chunks, &[]);
        assert_eq!(chunks[0].modality.as_deref(), Some(MODALITY_CHART));
    }
}
