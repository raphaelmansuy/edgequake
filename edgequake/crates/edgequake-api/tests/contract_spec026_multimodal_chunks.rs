//! SPEC-026 Phase 4g — mm chunk builder contract (LightRAG `_build_mm_chunks` parity).

use edgequake_api::services::{
    collect_mm_chunks_from_manifest, render_mm_chunk, ManifestItem, MultimodalItemRecord,
    MultimodalManifest, MultimodalProcessOptions,
};

fn sample_manifest() -> MultimodalManifest {
    MultimodalManifest {
        version: 1,
        items: vec![
            ManifestItem {
                item_id: "im-1".into(),
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
                heading: None,
                analyze_result: Some(MultimodalItemRecord::success_image(
                    "im-1",
                    "zurich_lab".into(),
                    "Photo".into(),
                    "EdgeQuake research in Zurich.".into(),
                )),
            },
            ManifestItem {
                item_id: "tb-1".into(),
                modality: "table".into(),
                start: 0,
                end: 0,
                matched: String::new(),
                asset_path: None,
                mime_type: Some("html".into()),
                body: None,
                caption: None,
                footnote: None,
                footnotes: Vec::new(),
                block_id: None,
                heading: None,
                analyze_result: Some(MultimodalItemRecord::success_modality(
                    "tb-1",
                    "table",
                    "sales_q4".into(),
                    "Table".into(),
                    "Quarterly sales.".into(),
                )),
            },
        ],
    }
}

#[test]
fn build_mm_chunks_respects_process_options_filter() {
    let manifest = sample_manifest();
    let opts = MultimodalProcessOptions {
        images: true,
        tables: false,
        equations: false,
    };
    let chunks = collect_mm_chunks_from_manifest(&manifest, &opts).unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].item_id, "im-1");
    assert!(chunks[0].text.contains("[Image Name]zurich_lab"));
}

#[test]
fn build_mm_chunks_rejects_failed_enabled_modality() {
    let manifest = MultimodalManifest {
        version: 1,
        items: vec![ManifestItem {
            item_id: "im-1".into(),
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
            heading: None,
            analyze_result: Some(MultimodalItemRecord::failed(
                "im-1",
                "drawing",
                "json invalid",
            )),
        }],
    };
    let opts = MultimodalProcessOptions {
        images: true,
        ..Default::default()
    };
    let err = collect_mm_chunks_from_manifest(&manifest, &opts).unwrap_err();
    assert_eq!(err.item_id, "im-1");
    assert_eq!(err.modality, "drawing");
}

#[test]
fn mm_chunk_labels_match_lightrag_contract() {
    let record = MultimodalItemRecord::success_image(
        "im-1",
        "figure_a".into(),
        "Chart".into(),
        "Distinctive multimodal retrieval phrase.".into(),
    );
    let text = render_mm_chunk(&record, "drawing", &[]);
    assert!(text.contains("[Image Name]figure_a"));
    assert!(text.contains("[Image Type]Chart"));
}
