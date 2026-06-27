//! Multimodal analyze orchestrator (LightRAG `analyze_multimodal` image + table + equation).

use std::path::Path;
use std::sync::Arc;

use edgequake_llm::traits::LLMProvider;
use edgequake_pdf::inline_images::scan_inline_image_refs;
use edgequake_storage::traits::KVStorage;
use tracing::{debug, warn};

use serde::Deserialize;

use super::super::vision_content::{
    image_analysis_to_markdown, normalize_image_type, ImageAnalysisResult, MultimodalProcessOptions,
};
use super::super::vlm_limits::{probe_image_dimensions, validate_image_for_vlm};
use super::assets::resolve_image_asset;
use super::blocks::{enrich_items_with_block_ids, prepare_analyze_blocks};
use super::cache::{chat_json_with_analysis_cache, maybe_attach_cache_key};
use super::context::{max_extract_input_tokens, trim_content_to_budget, SurroundingContext};
use super::gates::{should_run_image_analysis, vlm_process_enabled, MultimodalFailMode};
use super::item_record::{MultimodalItemRecord, MultimodalItemStatus, MultimodalSummary};
use super::json_recovery::parse_json_object;
use super::manifest::{ManifestItem, MultimodalManifest};
use super::prompt_context::PromptContext;
use super::prompts::{
    equation_analysis_messages, image_analysis_messages, json_repair_user_message,
    table_analysis_messages,
};
use super::providers::MultimodalProviders;
use super::scan::scan_manifest_items;
use super::surrounding::SurroundingKind;

/// Outcome of the analyze stage.
#[derive(Debug, Clone)]
pub struct AnalyzeOutcome {
    pub markdown: String,
    pub manifest: MultimodalManifest,
    pub summary: MultimodalSummary,
    pub hard_error: Option<String>,
}

/// Analyze inline multimodal items in markdown.
pub async fn analyze_multimodal_images(
    markdown: &str,
    process_options: Option<&str>,
    _filename: &str,
    providers: MultimodalProviders<'_>,
    asset_base_dir: Option<&Path>,
    kv_storage: Option<Arc<dyn KVStorage>>,
) -> AnalyzeOutcome {
    let opts = process_options
        .map(MultimodalProcessOptions::from_option_str)
        .unwrap_or_default();

    let mut manifest = MultimodalManifest {
        version: MultimodalManifest::CURRENT_VERSION,
        items: scan_manifest_items(markdown),
    };

    let (blocks_map, sections) = prepare_analyze_blocks(markdown);
    enrich_items_with_block_ids(&mut manifest.items, &sections);

    if !opts.any_enabled() {
        debug!("multimodal analyze skipped — no i/t/e flags");
        return AnalyzeOutcome {
            markdown: markdown.to_string(),
            manifest,
            summary: MultimodalSummary::default(),
            hard_error: None,
        };
    }

    if !vlm_process_enabled() && opts.images {
        let msg = "VLM_PROCESS_ENABLE=false but process_options includes 'i'";
        if MultimodalFailMode::from_env() == MultimodalFailMode::Strict {
            return AnalyzeOutcome {
                markdown: markdown.to_string(),
                manifest,
                summary: MultimodalSummary::default(),
                hard_error: Some(msg.into()),
            };
        }
        warn!(%msg, "multimodal analyze degraded");
    }

    if manifest.items.is_empty() {
        return AnalyzeOutcome {
            markdown: markdown.to_string(),
            manifest,
            summary: MultimodalSummary::default(),
            hard_error: None,
        };
    }

    let fail_mode = MultimodalFailMode::from_env();
    let mut output = markdown.to_string();
    let mut records = Vec::new();

    if should_run_image_analysis(&opts) {
        let refs = scan_inline_image_refs(markdown);
        for image_ref in refs.into_iter().rev() {
            let surrounding = manifest
                .items
                .iter()
                .find(|i| i.modality == "drawing" && i.item_id == image_ref.item_id)
                .map(|item| SurroundingContext::from_item_with_blocks(markdown, item, &blocks_map))
                .unwrap_or_else(|| {
                    SurroundingContext::from_span(
                        markdown,
                        (image_ref.start, image_ref.end),
                        SurroundingKind::Drawings,
                    )
                });
            match analyze_one_image(
                &image_ref,
                providers.vlm,
                asset_base_dir,
                &surrounding,
                kv_storage.clone(),
            )
            .await
            {
                Ok((record, replacement)) => {
                    if image_ref.start <= output.len() && image_ref.end <= output.len() {
                        output.replace_range(image_ref.start..image_ref.end, &replacement);
                    }
                    attach_record(&mut manifest, &record);
                    records.push(record);
                }
                Err(record) => {
                    if let Some(fatal) = handle_item_failure(
                        record.clone(),
                        fail_mode,
                        &mut records,
                        &mut manifest,
                        output.clone(),
                    ) {
                        return fatal;
                    }
                    attach_record(&mut manifest, &record);
                    records.push(record);
                }
            }
        }
    }

    if opts.tables {
        let table_items: Vec<ManifestItem> = manifest
            .items
            .iter()
            .filter(|i| i.modality == "table")
            .cloned()
            .collect();
        for item in table_items.into_iter().rev() {
            let surrounding =
                SurroundingContext::from_item_with_blocks(markdown, &item, &blocks_map);
            match analyze_one_table(&item, providers.extract, &surrounding, kv_storage.clone())
                .await
            {
                Ok((record, replacement)) => {
                    if item.start <= output.len() && item.end <= output.len() {
                        output.replace_range(item.start..item.end, &replacement);
                    }
                    attach_record(&mut manifest, &record);
                    records.push(record);
                }
                Err(record) => {
                    if let Some(fatal) = handle_item_failure(
                        record.clone(),
                        fail_mode,
                        &mut records,
                        &mut manifest,
                        output.clone(),
                    ) {
                        return fatal;
                    }
                    attach_record(&mut manifest, &record);
                    records.push(record);
                }
            }
        }
    }

    if opts.equations {
        let equation_items: Vec<ManifestItem> = manifest
            .items
            .iter()
            .filter(|i| i.modality == "equation")
            .cloned()
            .collect();
        for item in equation_items.into_iter().rev() {
            let surrounding =
                SurroundingContext::from_item_with_blocks(markdown, &item, &blocks_map);
            match analyze_one_equation(&item, providers.extract, &surrounding, kv_storage.clone())
                .await
            {
                Ok((record, replacement)) => {
                    if item.start <= output.len() && item.end <= output.len() {
                        output.replace_range(item.start..item.end, &replacement);
                    }
                    attach_record(&mut manifest, &record);
                    records.push(record);
                }
                Err(record) => {
                    if let Some(fatal) = handle_item_failure(
                        record.clone(),
                        fail_mode,
                        &mut records,
                        &mut manifest,
                        output.clone(),
                    ) {
                        return fatal;
                    }
                    attach_record(&mut manifest, &record);
                    records.push(record);
                }
            }
        }
    }

    AnalyzeOutcome {
        markdown: output,
        summary: MultimodalSummary::from_records(&records),
        manifest,
        hard_error: None,
    }
}

fn handle_item_failure(
    record: MultimodalItemRecord,
    fail_mode: MultimodalFailMode,
    records: &mut Vec<MultimodalItemRecord>,
    manifest: &mut MultimodalManifest,
    output: String,
) -> Option<AnalyzeOutcome> {
    if record.status == MultimodalItemStatus::Failed && fail_mode == MultimodalFailMode::Strict {
        records.push(record.clone());
        attach_record(manifest, &record);
        let summary = MultimodalSummary::from_records(records);
        return Some(AnalyzeOutcome {
            markdown: output,
            manifest: manifest.clone(),
            summary,
            hard_error: record.message.clone(),
        });
    }
    warn!(
        item_id = %record.item_id,
        message = ?record.message,
        "Multimodal analysis failed; keeping placeholder"
    );
    records.push(record);
    None
}

fn attach_record(manifest: &mut MultimodalManifest, record: &MultimodalItemRecord) {
    if let Some(item) = manifest
        .items
        .iter_mut()
        .find(|i| i.item_id == record.item_id)
    {
        item.analyze_result = Some(record.clone());
    }
}

/// Shared image byte analysis (standalone upload + inline PDF path).
pub async fn analyze_image_bytes(
    item_id: &str,
    bytes: &[u8],
    mime_type: &str,
    llm: &dyn LLMProvider,
    ctx: &PromptContext,
    kv: Option<Arc<dyn KVStorage>>,
) -> Result<(MultimodalItemRecord, String), MultimodalItemRecord> {
    let (width, height) = match probe_image_dimensions(bytes, mime_type) {
        Some(d) => d,
        None => {
            return Err(MultimodalItemRecord::skipped(
                item_id,
                "drawing",
                "cannot determine image dimensions (fail-closed)",
            ));
        }
    };

    if let Err(e) = validate_image_for_vlm(bytes, width, height) {
        return Err(MultimodalItemRecord::skipped(item_id, "drawing", e));
    }

    let messages = image_analysis_messages(bytes, mime_type, ctx);
    let (analysis, cache_id): (ImageAnalysisResult, _) = chat_json_with_analysis_cache(
        llm,
        kv,
        item_id,
        "drawing",
        messages,
        parse_image_analysis,
        json_repair_user_message,
    )
    .await
    .map_err(|e| MultimodalItemRecord::failed(item_id, "drawing", e))?;

    let mut record = MultimodalItemRecord::success_image(
        item_id,
        analysis.name.clone(),
        analysis.image_type.clone(),
        analysis.description.clone(),
    );
    maybe_attach_cache_key(&mut record, cache_id.as_deref());
    let replacement = format!("\n\n{}\n\n", image_analysis_to_markdown(&analysis));
    Ok((record, replacement))
}

async fn analyze_one_image(
    image_ref: &edgequake_pdf::inline_images::InlineImageRef,
    llm: &dyn LLMProvider,
    asset_base_dir: Option<&Path>,
    surrounding: &SurroundingContext,
    kv: Option<Arc<dyn KVStorage>>,
) -> Result<(MultimodalItemRecord, String), MultimodalItemRecord> {
    let asset = resolve_image_asset(image_ref, asset_base_dir)
        .map_err(|e| MultimodalItemRecord::skipped(&image_ref.item_id, "drawing", e))?;
    let ctx = PromptContext::from_parts(
        image_ref.caption.as_deref(),
        image_ref.footnote.as_deref(),
        surrounding,
    );
    analyze_image_bytes(
        &image_ref.item_id,
        &asset.bytes,
        &asset.mime_type,
        llm,
        &ctx,
        kv,
    )
    .await
}

async fn analyze_one_table(
    item: &ManifestItem,
    extract: &dyn LLMProvider,
    surrounding: &SurroundingContext,
    kv: Option<Arc<dyn KVStorage>>,
) -> Result<(MultimodalItemRecord, String), MultimodalItemRecord> {
    let body = item.body.as_deref().unwrap_or("").trim();
    if body.is_empty() {
        return Err(MultimodalItemRecord::skipped(
            &item.item_id,
            "table",
            "empty table body",
        ));
    }
    let format = item.mime_type.as_deref().unwrap_or("html");
    let (trimmed, _) =
        trim_content_to_budget(body, max_extract_input_tokens(), SurroundingKind::Tables);
    let ctx = PromptContext::from_item_and_surrounding(item, surrounding);
    let messages = match table_analysis_messages(&trimmed, format, &ctx) {
        Ok(m) => m,
        Err(e) => return Err(MultimodalItemRecord::failed(&item.item_id, "table", e)),
    };
    analyze_text_modality(&item.item_id, "table", "Table", messages, extract, kv).await
}

async fn analyze_one_equation(
    item: &ManifestItem,
    extract: &dyn LLMProvider,
    surrounding: &SurroundingContext,
    kv: Option<Arc<dyn KVStorage>>,
) -> Result<(MultimodalItemRecord, String), MultimodalItemRecord> {
    let body = item.body.as_deref().unwrap_or("").trim();
    if body.is_empty() {
        return Err(MultimodalItemRecord::skipped(
            &item.item_id,
            "equation",
            "empty equation body",
        ));
    }
    let (trimmed, _) =
        trim_content_to_budget(body, max_extract_input_tokens(), SurroundingKind::Equations);
    let ctx = PromptContext::from_item_and_surrounding(item, surrounding);
    analyze_equation_modality(&item.item_id, &trimmed, extract, &ctx, kv).await
}

#[derive(Debug, Deserialize)]
struct EquationAnalysisResult {
    name: String,
    equation: String,
    description: String,
}

async fn analyze_equation_modality(
    item_id: &str,
    body: &str,
    extract: &dyn LLMProvider,
    ctx: &PromptContext,
    kv: Option<Arc<dyn KVStorage>>,
) -> Result<(MultimodalItemRecord, String), MultimodalItemRecord> {
    let messages = equation_analysis_messages(body, ctx);
    let (analysis, cache_id): (EquationAnalysisResult, _) = chat_json_with_analysis_cache(
        extract,
        kv,
        item_id,
        "equation",
        messages,
        parse_equation_analysis,
        json_repair_user_message,
    )
    .await
    .map_err(|e| MultimodalItemRecord::failed(item_id, "equation", e))?;

    let equation_body = if analysis.equation.trim().is_empty() {
        body.to_string()
    } else {
        analysis.equation.clone()
    };
    let name = if analysis.name.trim().is_empty() {
        "equation_content".to_string()
    } else {
        analysis.name.clone()
    };

    let mut record = MultimodalItemRecord::success_equation(
        item_id,
        name.clone(),
        equation_body.clone(),
        analysis.description.clone(),
    );
    maybe_attach_cache_key(&mut record, cache_id.as_deref());
    let replacement = format!(
        "\n\n{}\n\n",
        super::chunks::render_mm_chunk(&record, "equation", &[])
    );
    Ok((record, replacement))
}

async fn analyze_text_modality(
    item_id: &str,
    modality: &str,
    default_type: &str,
    messages: Vec<edgequake_llm::traits::ChatMessage>,
    extract: &dyn LLMProvider,
    kv: Option<Arc<dyn KVStorage>>,
) -> Result<(MultimodalItemRecord, String), MultimodalItemRecord> {
    let (analysis, cache_id): (ImageAnalysisResult, _) = chat_json_with_analysis_cache(
        extract,
        kv,
        item_id,
        modality,
        messages,
        |text| parse_text_analysis(text, default_type),
        json_repair_user_message,
    )
    .await
    .map_err(|e| MultimodalItemRecord::failed(item_id, modality, e))?;

    let item_type = if analysis.image_type.trim().is_empty() {
        default_type.to_string()
    } else {
        analysis.image_type.clone()
    };

    let mut record = MultimodalItemRecord::success_modality(
        item_id,
        modality,
        analysis.name.clone(),
        item_type.clone(),
        analysis.description.clone(),
    );
    maybe_attach_cache_key(&mut record, cache_id.as_deref());
    let replacement = format!(
        "\n\n{}\n\n",
        super::chunks::render_mm_chunk(&record, modality, &[])
    );
    Ok((record, replacement))
}

fn parse_equation_analysis(text: &str) -> Result<EquationAnalysisResult, String> {
    let mut parsed: EquationAnalysisResult = parse_json_object(text)?;
    if parsed.name.trim().is_empty() {
        parsed.name = "equation_content".to_string();
    }
    if parsed.equation.trim().is_empty() {
        return Err("equation field missing or empty".into());
    }
    Ok(parsed)
}

fn parse_text_analysis(text: &str, default_type: &str) -> Result<ImageAnalysisResult, String> {
    let mut parsed: ImageAnalysisResult = parse_json_object(text)?;
    if parsed.image_type.trim().is_empty() {
        parsed.image_type = default_type.to_string();
    }
    if parsed.name.trim().is_empty() {
        parsed.name = format!("{}_content", default_type.to_ascii_lowercase());
    }
    Ok(parsed)
}

fn parse_image_analysis(text: &str) -> Result<ImageAnalysisResult, String> {
    let mut parsed: ImageAnalysisResult = parse_json_object(text)?;
    parsed.image_type = normalize_image_type(&parsed.image_type);
    if parsed.name.trim().is_empty() {
        parsed.name = "image_content".to_string();
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::MockProvider;

    #[tokio::test]
    async fn skips_without_i_flag() {
        let mock = MockProvider::new();
        let out = analyze_multimodal_images(
            "plain",
            None,
            "doc.pdf",
            MultimodalProviders::single(&mock),
            None,
            None,
        )
        .await;
        assert_eq!(out.markdown, "plain");
        assert!(out.summary.success == 0);
    }

    #[tokio::test]
    async fn table_analyze_success_with_mock_extract() {
        let md = r#"Intro <table id="tb-1" format="html"><tr><td>Revenue</td></tr></table> end"#;
        let mock = MockProvider::new();
        mock.add_response(
            r#"{"name":"revenue_table","type":"Table","description":"Single row with Revenue."}"#,
        )
        .await;
        let out = analyze_multimodal_images(
            md,
            Some("t"),
            "doc.pdf",
            MultimodalProviders::single(&mock),
            None,
            None,
        )
        .await;
        assert!(out.markdown.contains("[Table Name]revenue_table"));
        assert_eq!(out.summary.success, 1);
    }
}
