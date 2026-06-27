//! Multimodal chunk builder (LightRAG `_build_mm_chunks_from_sidecars`, Phase 4g/4k).

use super::super::vision_content::MultimodalProcessOptions;
use super::item_record::{MultimodalItemRecord, MultimodalItemStatus};
use super::manifest::{ManifestItem, MultimodalManifest};
use super::manifest_store::load_manifest;
use super::metadata::resolve_process_options_from_metadata;
use super::sanitize::sanitize_text_for_encoding;
use super::sidecar::{build_sidecar_block, MultimodalHeading, MultimodalSidecar};
use edgequake_storage::traits::KVStorage;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// Defensive error when a enabled-modality item has `status=Failed` (LightRAG `_build_mm_chunks`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MmChunkBuildError {
    pub item_id: String,
    pub modality: String,
    pub message: Option<String>,
}

impl fmt::Display for MmChunkBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "multimodal item {}/{} failed: {}",
            self.modality,
            self.item_id,
            self.message.as_deref().unwrap_or("no message")
        )
    }
}

impl std::error::Error for MmChunkBuildError {}

fn modality_enabled(modality: &str, opts: &MultimodalProcessOptions) -> bool {
    match modality {
        "drawing" => opts.images,
        "table" => opts.tables,
        "equation" => opts.equations,
        _ => false,
    }
}

/// LightRAG defensive check: fail if any enabled-modality item has analyze failure.
pub fn validate_manifest_for_mm_chunks(
    manifest: &MultimodalManifest,
    opts: &MultimodalProcessOptions,
) -> Result<(), MmChunkBuildError> {
    for item in &manifest.items {
        if !modality_enabled(&item.modality, opts) {
            continue;
        }
        let Some(record) = item.analyze_result.as_ref() else {
            continue;
        };
        if record.status == MultimodalItemStatus::Failed {
            return Err(MmChunkBuildError {
                item_id: item.item_id.clone(),
                modality: item.modality.clone(),
                message: record.message.clone(),
            });
        }
    }
    Ok(())
}

/// Feature flag for injecting multimodal chunks into the ingestion pipeline.
///
/// Default **on** (LightRAG always builds mm chunks). Opt out with `EDGEQUAKE_MM_CHUNKS=0`.
pub fn mm_chunks_enabled() -> bool {
    match std::env::var("EDGEQUAKE_MM_CHUNKS")
        .ok()
        .as_deref()
        .map(str::trim)
    {
        Some("0") | Some("false") | Some("no") | Some("off") => false,
        Some(_) => true,
        None => true,
    }
}

/// Indexed chunk derived from a successful manifest item (LightRAG nested schema subset).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultimodalChunk {
    pub item_id: String,
    pub modality: String,
    pub text: String,
    pub sidecar: MultimodalSidecar,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<MultimodalHeading>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub llm_cache_list: Vec<String>,
    #[serde(default)]
    pub chunk_order_index: u32,
}

fn item_footnotes(item: &ManifestItem) -> Vec<String> {
    let mut notes: Vec<String> = item.footnotes.clone();
    if let Some(single) = item.footnote.as_ref().filter(|s| !s.is_empty()) {
        if !notes.iter().any(|n| n == single) {
            notes.push(single.clone());
        }
    }
    notes
        .into_iter()
        .map(|n| sanitize_text_for_encoding(&n))
        .filter(|n| !n.is_empty())
        .collect()
}

fn heading_for_item(item: &ManifestItem) -> Option<MultimodalHeading> {
    item.heading
        .clone()
        .or_else(|| MultimodalHeading::from_legacy(item.caption.as_deref(), 0, Vec::new()))
}

/// Collect mm chunks from manifest (pure; no env gate — for unit tests).
pub fn collect_mm_chunks_from_manifest(
    manifest: &MultimodalManifest,
    opts: &MultimodalProcessOptions,
) -> Result<Vec<MultimodalChunk>, MmChunkBuildError> {
    validate_manifest_for_mm_chunks(manifest, opts)?;
    let mut order = 0u32;
    let mut chunks = Vec::new();
    for item in &manifest.items {
        let Some(record) = item.analyze_result.as_ref() else {
            continue;
        };
        if record.status != MultimodalItemStatus::Success {
            continue;
        }
        if !modality_enabled(&item.modality, opts) {
            continue;
        }
        let footnotes = item_footnotes(item);
        let text =
            super::chunk_budget::render_mm_chunk_with_budget(record, &item.modality, &footnotes)
                .map_err(|msg| MmChunkBuildError {
                    item_id: item.item_id.clone(),
                    modality: item.modality.clone(),
                    message: Some(msg),
                })?;
        chunks.push(MultimodalChunk {
            item_id: item.item_id.clone(),
            modality: item.modality.clone(),
            text,
            sidecar: build_sidecar_block(&item.modality, &item.item_id),
            heading: heading_for_item(item),
            llm_cache_list: record.llm_cache_list.clone(),
            chunk_order_index: order,
        });
        order += 1;
    }
    Ok(chunks)
}

/// Build mm chunks from manifest items that succeeded and match process_options.
pub fn build_mm_chunks_from_manifest(
    manifest: &MultimodalManifest,
    opts: &MultimodalProcessOptions,
) -> Result<Vec<MultimodalChunk>, MmChunkBuildError> {
    if !mm_chunks_enabled() {
        return Ok(Vec::new());
    }
    collect_mm_chunks_from_manifest(manifest, opts)
}

/// LightRAG chunk label contract (`_render` in `pipeline.py`).
pub fn render_mm_chunk(
    record: &MultimodalItemRecord,
    modality: &str,
    footnotes: &[String],
) -> String {
    render_mm_chunk_with_description(record, modality, footnotes, record.description.as_deref())
}

/// Render with an explicit description override (used by token-budget truncation).
pub fn render_mm_chunk_with_description(
    record: &MultimodalItemRecord,
    modality: &str,
    footnotes: &[String],
    description: Option<&str>,
) -> String {
    let name = sanitize_text_for_encoding(record.name.as_deref().unwrap_or("item"));
    let description = sanitize_text_for_encoding(description.unwrap_or(""));
    let footnotes_joined = footnotes.join("; ");

    let (head, footnote_label) = match modality {
        "drawing" => {
            let image_type =
                sanitize_text_for_encoding(record.item_type.as_deref().unwrap_or("Other"));
            (
                format!("[Image Name]{name}\n[Image Type]{image_type}"),
                "Image Footnotes",
            )
        }
        "table" => (format!("[Table Name]{name}"), "Table Footnotes"),
        "equation" => {
            let equation_body =
                sanitize_text_for_encoding(record.equation.as_deref().unwrap_or(""));
            (
                format!("{equation_body}\n[Equation Name]{name}"),
                "Equation Footnotes",
            )
        }
        _ => (format!("[Item Name]{name}"), "Footnotes"),
    };

    let mut sections = vec![head, description];
    if !footnotes_joined.is_empty() {
        sections.push(format!("[{footnote_label}]{footnotes_joined}"));
    }
    sections
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Append multimodal chunks as markdown sections for pipeline indexing (Phase 4g).
pub fn append_mm_chunks_to_text(text: &str, chunks: &[MultimodalChunk]) -> String {
    if chunks.is_empty() {
        return text.to_string();
    }
    let mut out = text.to_string();
    out.push_str("\n\n<!-- multimodal-chunks -->\n");
    for chunk in chunks {
        out.push_str("\n\n");
        out.push_str(&chunk.text);
    }
    out
}

/// Load KV manifest, persist structured mm chunks, append text sections when enabled.
pub async fn enrich_processed_text_with_mm_chunks(
    kv: &dyn KVStorage,
    document_id: &str,
    metadata: Option<&Value>,
    text: String,
) -> String {
    if !mm_chunks_enabled() {
        return text;
    }
    let Some(manifest) = load_manifest(kv, document_id).await else {
        return text;
    };
    let opts = metadata
        .and_then(resolve_process_options_from_metadata)
        .map(|s| MultimodalProcessOptions::from_option_str(&s))
        .unwrap_or_default();
    let chunks = match build_mm_chunks_from_manifest(&manifest, &opts) {
        Ok(chunks) => chunks,
        Err(e) => {
            tracing::warn!(document_id = %document_id, error = %e, "skipping mm chunk injection due to failed analyze item");
            return text;
        }
    };
    if !chunks.is_empty() {
        if let Err(e) = super::chunks_store::persist_mm_chunks(kv, document_id, &chunks).await {
            tracing::warn!(document_id = %document_id, error = %e, "failed to persist multimodal chunk sidecar metadata");
        }
    }
    append_mm_chunks_to_text(&text, &chunks)
}

#[cfg(test)]
mod tests {
    use super::super::item_record::MultimodalItemRecord;
    use super::super::manifest::ManifestItem;
    use super::super::sidecar::MultimodalHeading;
    use super::*;

    fn manifest_with(records: Vec<(String, &str, MultimodalItemRecord)>) -> MultimodalManifest {
        MultimodalManifest {
            version: 1,
            items: records
                .into_iter()
                .map(|(id, modality, record)| ManifestItem {
                    item_id: id.clone(),
                    modality: modality.to_string(),
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
                        parent_headings: Vec::new(),
                    }),
                    analyze_result: Some(record),
                })
                .collect(),
        }
    }

    #[test]
    #[serial_test::serial]
    fn enabled_by_default_with_empty_manifest() {
        std::env::remove_var("EDGEQUAKE_MM_CHUNKS");
        assert!(mm_chunks_enabled());
        let manifest = MultimodalManifest::default();
        assert!(
            build_mm_chunks_from_manifest(&manifest, &MultimodalProcessOptions::default())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    #[serial_test::serial]
    fn disabled_when_env_opt_out() {
        std::env::set_var("EDGEQUAKE_MM_CHUNKS", "0");
        let manifest = manifest_with(vec![(
            "im-1".into(),
            "drawing",
            MultimodalItemRecord::success_image(
                "im-1",
                "chart".into(),
                "Chart".into(),
                "desc".into(),
            ),
        )]);
        let opts = MultimodalProcessOptions {
            images: true,
            ..Default::default()
        };
        assert!(build_mm_chunks_from_manifest(&manifest, &opts)
            .unwrap()
            .is_empty());
        std::env::remove_var("EDGEQUAKE_MM_CHUNKS");
    }

    #[test]
    fn builds_lightrag_image_chunk_labels() {
        let record = MultimodalItemRecord::success_image(
            "im-1",
            "revenue_chart".into(),
            "Chart".into(),
            "Revenue grew in Q4.".into(),
        );
        let text = render_mm_chunk(&record, "drawing", &[]);
        assert!(text.starts_with("[Image Name]revenue_chart"));
        assert!(text.contains("[Image Type]Chart"));
        assert!(text.contains("Revenue grew"));
    }

    #[test]
    fn nested_sidecar_schema_on_success_item() {
        let mut record = MultimodalItemRecord::success_image(
            "d1",
            "系统架构图".to_string(),
            "Chart".to_string(),
            "模块交互关系".to_string(),
        );
        record.llm_cache_list = vec!["default:analysis:abc123".into()];
        let manifest = manifest_with(vec![("d1".into(), "drawing", record)]);
        let opts = MultimodalProcessOptions {
            images: true,
            ..Default::default()
        };
        let chunks = collect_mm_chunks_from_manifest(&manifest, &opts).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].sidecar.id, "d1");
        assert_eq!(chunks[0].sidecar.sidecar_type, "drawing");
        assert_eq!(chunks[0].llm_cache_list, vec!["default:analysis:abc123"]);
        assert_eq!(
            chunks[0].heading.as_ref().map(|h| h.heading.as_str()),
            Some("章节A")
        );
    }

    #[test]
    fn process_options_filter_excludes_disabled_modalities() {
        let manifest = manifest_with(vec![
            (
                "im-1".into(),
                "drawing",
                MultimodalItemRecord::success_image(
                    "im-1",
                    "chart".into(),
                    "Chart".into(),
                    "Image body.".into(),
                ),
            ),
            (
                "tb-1".into(),
                "table",
                MultimodalItemRecord::success_modality(
                    "tb-1",
                    "table",
                    "sales_table".into(),
                    "Table".into(),
                    "Table body.".into(),
                ),
            ),
        ]);
        let opts = MultimodalProcessOptions {
            images: true,
            tables: false,
            equations: false,
        };
        let chunks = collect_mm_chunks_from_manifest(&manifest, &opts).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].modality, "drawing");
    }

    #[test]
    fn rejects_failed_item_for_enabled_modality() {
        let manifest = manifest_with(vec![(
            "im-1".into(),
            "drawing",
            MultimodalItemRecord::failed("im-1", "drawing", "json invalid"),
        )]);
        let opts = MultimodalProcessOptions {
            images: true,
            ..Default::default()
        };
        let err = collect_mm_chunks_from_manifest(&manifest, &opts).unwrap_err();
        assert_eq!(err.item_id, "im-1");
    }

    #[test]
    fn equation_chunk_includes_body_and_name_label() {
        let record = MultimodalItemRecord::success_equation(
            "eq-1",
            "mass_energy".into(),
            "E=mc^2".into(),
            "Mass-energy equivalence.".into(),
        );
        let text = render_mm_chunk(&record, "equation", &[]);
        assert!(text.starts_with("E=mc^2"));
        assert!(text.contains("[Equation Name]mass_energy"));
    }

    #[test]
    fn sanitize_strips_control_chars_from_chunk_text() {
        let record = MultimodalItemRecord::success_image(
            "im-1",
            "chart".into(),
            "Chart".into(),
            "Value\x00here.".into(),
        );
        let text = render_mm_chunk(&record, "drawing", &[]);
        assert!(!text.contains('\x00'));
        assert!(text.contains("Valuehere"));
    }

    #[test]
    #[serial_test::serial]
    fn builds_chunk_when_flag_enabled_and_item_success() {
        std::env::set_var("EDGEQUAKE_MM_CHUNKS", "1");
        let record = MultimodalItemRecord::success_image(
            "im-1",
            "chart".into(),
            "Chart".into(),
            "Revenue grew.".into(),
        );
        let manifest = manifest_with(vec![("im-1".into(), "drawing", record)]);
        let opts = MultimodalProcessOptions {
            images: true,
            ..Default::default()
        };
        let chunks = build_mm_chunks_from_manifest(&manifest, &opts).unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("[Image Name]chart"));
        std::env::remove_var("EDGEQUAKE_MM_CHUNKS");
    }
}
