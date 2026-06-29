//! Inject multimodal entity nodes + association edges after LLM extraction.

use crate::chunker::TextChunk;
use crate::extractor::{ExtractedEntity, ExtractedRelationship, ExtractionResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

static MM_DISPLAY_NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\[(?:Image|Table|Equation) Name\](.+)$").expect("mm display name regex")
});

/// Parse friendly name from mm chunk content (LightRAG `_parse_mm_display_name`).
pub fn parse_mm_display_name(content: &str, fallback: &str) -> String {
    if let Some(cap) = MM_DISPLAY_NAME.captures(content) {
        let candidate = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        if !candidate.is_empty() {
            return candidate.to_string();
        }
    }
    fallback.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MmSidecarRef {
    #[serde(rename = "type")]
    pub ref_type: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MmSidecarBlock {
    #[serde(rename = "type")]
    pub sidecar_type: String,
    pub id: String,
    pub refs: Vec<MmSidecarRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MmHeadingBlock {
    pub level: u32,
    pub heading: String,
    #[serde(default)]
    pub parent_headings: Vec<String>,
}

/// Sidecar metadata persisted by EdgeQuake analyze stage (JSON-compatible with api `MultimodalChunk`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MmChunkSidecarMeta {
    pub item_id: String,
    pub modality: String,
    pub text: String,
    pub sidecar: MmSidecarBlock,
    #[serde(default)]
    pub heading: Option<MmHeadingBlock>,
    #[serde(default)]
    pub llm_cache_list: Vec<String>,
}

fn chunk_matches_mm(chunk_content: &str, mm: &MmChunkSidecarMeta) -> bool {
    chunk_content.contains(&mm.text)
        || (chunk_content.contains("[Image Name]") && mm.text.starts_with("[Image Name]"))
        || (chunk_content.contains("[Table Name]") && mm.text.starts_with("[Table Name]"))
        || (chunk_content.contains("[Equation Name]") && mm.text.starts_with("[Equation Name]"))
}

/// Augment extractions with mm entity + association edges (LightRAG operate L3622+).
pub fn inject_modality_relations(
    extractions: &mut [ExtractionResult],
    chunks: &[TextChunk],
    mm_chunks: &[MmChunkSidecarMeta],
    file_path: &str,
) {
    if mm_chunks.is_empty() {
        return;
    }
    for extraction in extractions.iter_mut() {
        let Some(chunk) = chunks.iter().find(|c| c.id == extraction.source_chunk_id) else {
            continue;
        };
        let Some(mm) = mm_chunks
            .iter()
            .find(|m| chunk_matches_mm(&chunk.content, m))
        else {
            continue;
        };
        inject_into_extraction(extraction, mm, &chunk.content, file_path, &chunk.id);
    }
}

fn inject_into_extraction(
    extraction: &mut ExtractionResult,
    mm: &MmChunkSidecarMeta,
    content: &str,
    file_path: &str,
    chunk_id: &str,
) {
    let sidecar_type = mm.sidecar.sidecar_type.as_str();
    if !matches!(sidecar_type, "drawing" | "table" | "equation") {
        return;
    }
    let entity_name = mm.sidecar.id.clone();

    if !extraction.entities.iter().any(|e| e.name == entity_name) {
        extraction.entities.push(
            ExtractedEntity::new(entity_name.clone(), sidecar_type, content)
                .with_source_chunk_id(chunk_id)
                .with_source_file_path(file_path),
        );
    }

    let heading_label = mm
        .heading
        .as_ref()
        .map(|h| h.heading.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("");
    let location = if heading_label.is_empty() {
        "of document".to_string()
    } else {
        format!("in section {heading_label} of document")
    };
    let display = parse_mm_display_name(content, &entity_name);

    let targets: Vec<String> = extraction
        .entities
        .iter()
        .map(|e| e.name.clone())
        .filter(|n| n != &entity_name)
        .collect();

    for tgt in targets {
        let already = extraction.relationships.iter().any(|r| {
            r.source == entity_name && r.target == tgt && r.relation_type == "associated with"
        });
        if already {
            continue;
        }
        let description =
            format!("{tgt} is associated with {sidecar_type} {display} {location} \"{file_path}\"");
        extraction.relationships.push(
            ExtractedRelationship::new(&entity_name, &tgt, "associated with")
                .with_description(description)
                .with_weight(1.0)
                .with_keywords(vec!["associated with".into(), "contained in".into()])
                .with_source_chunk_id(chunk_id)
                .with_source_file_path(file_path),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunker::TextChunk;

    #[test]
    fn parse_mm_display_name_reads_image_label() {
        let content = "[Image Name]系统架构图\n[Image Type]Chart\n\n模块交互关系";
        assert_eq!(parse_mm_display_name(content, "d1"), "系统架构图");
        assert_eq!(
            parse_mm_display_name("no marker", "fallback-id"),
            "fallback-id"
        );
    }

    #[test]
    fn inject_modality_relations_adds_entity_and_edges() {
        let mm = MmChunkSidecarMeta {
            item_id: "d1".into(),
            modality: "drawing".into(),
            text: "[Image Name]系统架构图\n[Image Type]Chart\n\n模块交互关系".into(),
            sidecar: MmSidecarBlock {
                sidecar_type: "drawing".into(),
                id: "d1".into(),
                refs: vec![MmSidecarRef {
                    ref_type: "drawing".into(),
                    id: "d1".into(),
                }],
            },
            heading: Some(MmHeadingBlock {
                level: 0,
                heading: "章节A".into(),
                parent_headings: vec![],
            }),
            llm_cache_list: vec!["default:analysis:abc123".into()],
        };
        let chunks = vec![TextChunk {
            id: "doc-mm-chunk-0".into(),
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
        }];
        let mut extractions = vec![ExtractionResult {
            entities: vec![ExtractedEntity::new(
                "OTHER_ENTITY",
                "CONCEPT",
                "related concept",
            )],
            relationships: vec![],
            source_chunk_id: "doc-mm-chunk-0".into(),
            metadata: Default::default(),
            input_tokens: 0,
            output_tokens: 0,
            extraction_time_ms: 0,
        }];
        inject_modality_relations(&mut extractions, &chunks, &[mm], "demo.pdf");
        assert!(extractions[0].entities.iter().any(|e| e.name == "d1"));
        assert_eq!(extractions[0].relationships.len(), 1);
        assert!(extractions[0].relationships[0]
            .description
            .contains("系统架构图"));
    }
}
