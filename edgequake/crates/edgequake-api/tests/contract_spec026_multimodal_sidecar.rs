//! SPEC-026 Phase 4k — sidecar chunk schema + blocks.jsonl + modality relations.

use edgequake_api::services::{
    collect_mm_chunks_from_manifest, load_content_rows_by_blockid_jsonl, ManifestItem,
    MultimodalHeading, MultimodalItemRecord, MultimodalManifest, MultimodalProcessOptions,
};
use edgequake_pipeline::chunker::TextChunk;
use edgequake_pipeline::extractor::{ExtractedEntity, ExtractionResult};
use edgequake_pipeline::{inject_modality_relations, parse_mm_display_name, MmChunkSidecarMeta};

#[test]
fn mm_chunks_and_modality_relations_from_sidecars() {
    let mut record = MultimodalItemRecord::success_image(
        "d1",
        "系统架构图".into(),
        "Chart".into(),
        "模块交互关系".into(),
    );
    record.llm_cache_list = vec!["default:analysis:abc123".into()];
    let manifest = MultimodalManifest {
        version: 1,
        items: vec![ManifestItem {
            item_id: "d1".into(),
            modality: "drawing".into(),
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
            heading: Some(MultimodalHeading {
                level: 0,
                heading: "章节A".into(),
                parent_headings: vec![],
            }),
            analyze_result: Some(record),
        }],
    };
    let opts = MultimodalProcessOptions {
        images: true,
        ..Default::default()
    };
    let chunks = collect_mm_chunks_from_manifest(&manifest, &opts).unwrap();
    assert_eq!(chunks.len(), 1);
    let chunk = &chunks[0];
    assert!(chunk.text.starts_with("[Image Name]"));
    assert!(chunk.text.contains("[Image Type]Chart"));
    assert_eq!(chunk.sidecar.id, "d1");
    assert_eq!(chunk.sidecar.sidecar_type, "drawing");
    assert_eq!(
        chunk.heading.as_ref().map(|h| h.heading.as_str()),
        Some("章节A")
    );
    assert_eq!(chunk.llm_cache_list, vec!["default:analysis:abc123"]);

    let meta: MmChunkSidecarMeta =
        serde_json::from_value(serde_json::to_value(chunk).unwrap()).expect("pipeline meta");
    let text_chunks = vec![TextChunk {
        id: "doc-mm-0".into(),
        content: chunk.text.clone(),
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
        entities: vec![ExtractedEntity::new("OTHER", "CONCEPT", "ctx")],
        relationships: vec![],
        source_chunk_id: "doc-mm-0".into(),
        metadata: Default::default(),
        input_tokens: 0,
        output_tokens: 0,
        extraction_time_ms: 0,
    }];
    inject_modality_relations(&mut extractions, &text_chunks, &[meta], "demo.pdf");
    assert!(extractions[0].entities.iter().any(|e| e.name == "d1"));
    assert_eq!(extractions[0].relationships.len(), 1);
}

#[test]
fn parse_mm_display_name_matches_chunk_format() {
    let drawing = "[Image Name]系统架构图\n[Image Type]Chart\n\nbody";
    assert_eq!(parse_mm_display_name(drawing, "d1"), "系统架构图");
    let table = "[Table Name]性能对比表\n\nbody";
    assert_eq!(parse_mm_display_name(table, "t1"), "性能对比表");
    let equation = "E=mc^2\n[Equation Name]质能方程\n\nbody";
    assert_eq!(parse_mm_display_name(equation, "e1"), "质能方程");
}

#[test]
fn blocks_jsonl_loader_keeps_first_content_row() {
    let jsonl = r#"{"type":"content","blockid":"b1","content":"scoped block"}
{"type":"content","blockid":"b1","content":"duplicate"}"#;
    let rows = load_content_rows_by_blockid_jsonl(jsonl);
    assert_eq!(rows.get("b1").map(|s| s.as_str()), Some("scoped block"));
}
