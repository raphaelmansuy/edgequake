//! Multimodal analyze orchestrator (LightRAG `analyze_multimodal` image + table + equation).

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use edgequake_llm::traits::LLMProvider;
use edgequake_pdf::inline_images::scan_inline_image_refs;
use edgequake_pdf::{
    crop_descriptor_from_asset, should_suppress_crop_for_analyze, CropGeometryCache, PageModality,
};
use edgequake_storage::traits::KVStorage;
use futures::stream::{self, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use serde::Deserialize;

use super::super::vision_content::{
    image_analysis_to_markdown_with_asset, normalize_image_type, ImageAnalysisResult,
    MultimodalProcessOptions,
};
use super::super::vlm_limits::{probe_image_dimensions, validate_image_for_vlm};
use super::assets::resolve_image_asset;
use super::blocks::{enrich_items_with_block_ids, prepare_analyze_blocks};
use super::cache::{chat_json_with_analysis_cache, maybe_attach_cache_key};
use super::context::{max_extract_input_tokens, trim_content_to_budget, SurroundingContext};
use super::gates::{should_run_image_analysis, vlm_process_enabled, MultimodalFailMode};
use super::image_specialize::specialize_image_analysis;
use super::item_record::{MultimodalItemRecord, MultimodalItemStatus, MultimodalSummary};
use super::json_recovery::parse_json_object;
use super::local_profile::LocalMmProfile;
use super::manifest::{ManifestItem, MultimodalManifest};
use super::prompt_context::PromptContext;
use super::prompts::{
    equation_analysis_messages, image_analysis_messages, json_repair_user_message,
    table_analysis_messages,
};
use super::providers::MultimodalProviders;
use super::scan::scan_manifest_items;
use super::surrounding::SurroundingKind;
use crate::services::converting_subprogress::{
    report_vision_figure_analyze_ex, ConvertingSubstepReporter, VisionFigureProgressOpts,
};

tokio::task_local! {
    /// SPEC-015V: Pass B prompt / specialize gates for the current analyze stage.
    pub static VISION_EXTRACT_CTX: edgequake_pdf::VisionExtractConfig;
    /// SPEC-134: page modality resolved by the ingestion pipeline (env override
    /// or heuristic classifier). Scoped around the analyze stage so Pass-B
    /// suppression sees the same modality as the conversion profile.
    pub static PAGE_MODALITY_CTX: PageModality;
}

fn current_vision_extract() -> edgequake_pdf::VisionExtractConfig {
    VISION_EXTRACT_CTX
        .try_with(|c| c.clone())
        .unwrap_or_default()
}

/// Run `fut` with the SPEC-134 page modality visible to Pass-B suppression.
pub async fn with_page_modality_ctx<R>(
    modality: PageModality,
    fut: impl std::future::Future<Output = R>,
) -> R {
    PAGE_MODALITY_CTX.scope(modality, fut).await
}

/// Resolution order: pipeline-scoped modality → env override → Print (fail-open).
fn current_page_modality() -> PageModality {
    PAGE_MODALITY_CTX
        .try_with(|m| *m)
        .ok()
        .or_else(PageModality::from_env)
        .unwrap_or(PageModality::Print)
}

/// Remove `<drawing …/>` placeholders that Pass B did not replace (viewer hygiene).
fn strip_drawing_tags(markdown: &str) -> String {
    let refs = scan_inline_image_refs(markdown);
    if refs.is_empty() {
        return markdown.to_string();
    }
    let mut out = markdown.to_string();
    for image_ref in refs.into_iter().rev() {
        if image_ref.start <= out.len() && image_ref.end <= out.len() {
            out.replace_range(image_ref.start..image_ref.end, "");
        }
    }
    out
}

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
    filename: &str,
    providers: MultimodalProviders<'_>,
    asset_base_dir: Option<&Path>,
    kv_storage: Option<Arc<dyn KVStorage>>,
) -> AnalyzeOutcome {
    analyze_multimodal_images_with_substep(
        markdown,
        process_options,
        filename,
        providers,
        asset_base_dir,
        kv_storage,
        None,
        None,
    )
    .await
}

/// Same as [`analyze_multimodal_images`] with optional converting sub-step reporter (PDF Pass B).
#[allow(clippy::too_many_arguments)]
pub async fn analyze_multimodal_images_with_substep(
    markdown: &str,
    process_options: Option<&str>,
    _filename: &str,
    providers: MultimodalProviders<'_>,
    asset_base_dir: Option<&Path>,
    kv_storage: Option<Arc<dyn KVStorage>>,
    converting_substep: Option<ConvertingSubstepReporter>,
    cancel_token: Option<CancellationToken>,
) -> AnalyzeOutcome {
    let opts = process_options
        .map(MultimodalProcessOptions::from_option_str)
        .unwrap_or_default();

    let mm_profile = LocalMmProfile::resolve_from_env();
    let fail_mode = MultimodalFailMode::resolve(mm_profile.is_local);

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
        if fail_mode == MultimodalFailMode::Strict {
            return AnalyzeOutcome {
                markdown: markdown.to_string(),
                manifest,
                summary: MultimodalSummary::default(),
                hard_error: Some(msg.into()),
            };
        }
        // First principle: never leave unscanned `<drawing/>` placeholders in
        // viewer markdown when Pass B cannot run — they leak as raw HTML text.
        warn!(%msg, "multimodal analyze degraded — stripping unanalyzed drawing tags");
        return AnalyzeOutcome {
            markdown: strip_drawing_tags(markdown),
            manifest,
            summary: MultimodalSummary::default(),
            hard_error: None,
        };
    }

    if manifest.items.is_empty() {
        return AnalyzeOutcome {
            markdown: markdown.to_string(),
            manifest,
            summary: MultimodalSummary::default(),
            hard_error: None,
        };
    }

    let mut output = markdown.to_string();
    let mut records = Vec::new();

    if should_run_image_analysis(&opts) {
        if let Some(fatal) = analyze_images_pass_b(
            markdown,
            &mut output,
            &mut manifest,
            &mut records,
            &blocks_map,
            providers.vlm,
            asset_base_dir,
            kv_storage.clone(),
            converting_substep.as_ref(),
            cancel_token.as_ref(),
            mm_profile,
            fail_mode,
        )
        .await
        {
            return fatal;
        }
    }

    if opts.tables {
        let table_items: Vec<ManifestItem> = manifest
            .items
            .iter()
            .filter(|i| i.modality == "table")
            .cloned()
            .collect();
        let total = table_items.len();
        let concurrency = mm_item_concurrency();
        if total > 0 {
            info!(
                total_tables = total,
                concurrency, "multimodal table analyze starting (parallel VLM)"
            );
        }
        let jobs: Vec<_> = table_items
            .into_iter()
            .map(|item| {
                let surrounding =
                    SurroundingContext::from_item_with_blocks(markdown, &item, &blocks_map);
                (item, surrounding)
            })
            .collect();
        let completed = AtomicUsize::new(0);
        type ManifestAnalyzeOutcome = (
            ManifestItem,
            Result<(MultimodalItemRecord, String), MultimodalItemRecord>,
        );
        let mut results: Vec<ManifestAnalyzeOutcome> = stream::iter(jobs)
            .map(|(item, surrounding)| {
                let kv = kv_storage.clone();
                async move {
                    let r = analyze_one_table(&item, providers.extract, &surrounding, kv).await;
                    (item, r)
                }
            })
            .buffer_unordered(concurrency)
            .inspect(|_| {
                let n = completed.fetch_add(1, Ordering::Relaxed) + 1;
                if n == 1 || n == total || n.is_multiple_of(5) {
                    info!(completed = n, total, "multimodal table analyze progress");
                }
            })
            .collect()
            .await;
        // Apply replacements in reverse document order so byte spans stay valid.
        results.sort_by_key(|b| std::cmp::Reverse(b.0.start));
        for (item, result) in results {
            match result {
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
        let total = equation_items.len();
        let concurrency = mm_item_concurrency();
        if total > 0 {
            info!(
                total_equations = total,
                concurrency, "multimodal equation analyze starting (parallel VLM)"
            );
        }
        let jobs: Vec<_> = equation_items
            .into_iter()
            .map(|item| {
                let surrounding =
                    SurroundingContext::from_item_with_blocks(markdown, &item, &blocks_map);
                (item, surrounding)
            })
            .collect();
        let completed = AtomicUsize::new(0);
        type ManifestAnalyzeOutcome = (
            ManifestItem,
            Result<(MultimodalItemRecord, String), MultimodalItemRecord>,
        );
        let mut results: Vec<ManifestAnalyzeOutcome> = stream::iter(jobs)
            .map(|(item, surrounding)| {
                let kv = kv_storage.clone();
                async move {
                    let r = analyze_one_equation(&item, providers.extract, &surrounding, kv).await;
                    (item, r)
                }
            })
            .buffer_unordered(concurrency)
            .inspect(|_| {
                let n = completed.fetch_add(1, Ordering::Relaxed) + 1;
                if n == 1 || n == total || n.is_multiple_of(5) {
                    info!(completed = n, total, "multimodal equation analyze progress");
                }
            })
            .collect()
            .await;
        results.sort_by_key(|b| std::cmp::Reverse(b.0.start));
        for (item, result) in results {
            match result {
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

/// Pass B figure analyze with local budgets: figure cap, wall clock, cancel, progress.
#[allow(clippy::too_many_arguments)]
async fn analyze_images_pass_b(
    markdown: &str,
    output: &mut String,
    manifest: &mut MultimodalManifest,
    records: &mut Vec<MultimodalItemRecord>,
    blocks_map: &std::collections::HashMap<String, String>,
    vlm: &dyn LLMProvider,
    asset_base_dir: Option<&Path>,
    kv_storage: Option<Arc<dyn KVStorage>>,
    converting_substep: Option<&ConvertingSubstepReporter>,
    cancel_token: Option<&CancellationToken>,
    mm_profile: LocalMmProfile,
    fail_mode: MultimodalFailMode,
) -> Option<AnalyzeOutcome> {
    let all_refs = scan_inline_image_refs(markdown);
    let discovered = all_refs.len();
    if discovered == 0 {
        return None;
    }

    let analyze_cap = mm_profile.figures_to_analyze(discovered);
    let skipped = discovered.saturating_sub(analyze_cap);
    let refs: Vec<_> = all_refs.into_iter().take(analyze_cap).collect();

    // Graphic-as-unit Pass-B: drop encoding-artifact crops (HTML table
    // hairlines, tick strips, chart fragments) before VLM analysis. Print
    // uses area/aspect; manuscript keeps the extra ink/fragment gates.
    let page_modality = current_page_modality();
    let refs: Vec<_> = {
        let before = refs.len();
        let mut geometry = CropGeometryCache::default();
        let filtered: Vec<_> = refs
            .into_iter()
            .filter(|image_ref| {
                let crop = crop_descriptor_from_asset(
                    &mut geometry,
                    asset_base_dir,
                    image_ref.asset_path.as_deref(),
                    &image_ref.bytes,
                );
                !should_suppress_crop_for_analyze(page_modality, &crop)
            })
            .collect();
        let suppressed = before - filtered.len();
        if suppressed > 0 {
            info!(
                suppressed,
                before,
                after = filtered.len(),
                modality = page_modality.as_str(),
                "Pass-B suppression: filtered encoding-artifact / fragment crops"
            );
        }
        filtered
    };

    let total = refs.len();
    let concurrency = mm_image_concurrency();
    let progress_opts = VisionFigureProgressOpts {
        every_figure: mm_profile.emit_every_figure || total <= 50,
        local_classify_only: mm_profile.is_local && mm_profile.classify_only,
        discovered_total: discovered,
        analyzed_cap: total,
    };

    info!(
        discovered,
        analyzing = total,
        skipped,
        concurrency,
        classify_only = mm_profile.classify_only,
        is_local = mm_profile.is_local,
        pass_b_timeout_secs = mm_profile.pass_b_timeout.map(|d| d.as_secs()),
        "multimodal image analyze starting (Pass B)"
    );
    report_vision_figure_analyze_ex(converting_substep, 0, total, progress_opts);

    let jobs: Vec<_> = refs
        .into_iter()
        .map(|image_ref| {
            let surrounding = manifest
                .items
                .iter()
                .find(|i| i.modality == "drawing" && i.item_id == image_ref.item_id)
                .map(|item| SurroundingContext::from_item_with_blocks(markdown, item, blocks_map))
                .unwrap_or_else(|| {
                    SurroundingContext::from_span(
                        markdown,
                        (image_ref.start, image_ref.end),
                        SurroundingKind::Drawings,
                    )
                });
            (image_ref, surrounding)
        })
        .collect();

    // Local / wall-budget path: sequential so we can cancel between figures,
    // honor wall budget, and apply replacements progressively.
    // Cloud keeps parallel specialize when concurrency > 1 and no wall budget.
    let use_sequential =
        mm_profile.is_local || mm_profile.pass_b_timeout.is_some() || concurrency <= 1;

    if use_sequential {
        let pass_b_started = Instant::now();
        // Reverse document order so each replace keeps later spans valid.
        let mut ordered = jobs;
        ordered.sort_by_key(|b| std::cmp::Reverse(b.0.start));
        let mut completed = 0usize;
        for (image_ref, surrounding) in ordered {
            if cancel_token.is_some_and(|t| t.is_cancelled()) {
                warn!(
                    completed,
                    total,
                    discovered,
                    "Pass B cancelled between figures — keeping analyzed markdown"
                );
                break;
            }
            if let Some(budget) = mm_profile.pass_b_timeout {
                if pass_b_started.elapsed() >= budget {
                    warn!(
                        completed,
                        total,
                        discovered,
                        budget_secs = budget.as_secs(),
                        "pass_b_budget_exhausted — stopping remaining figures (degraded)"
                    );
                    break;
                }
            }

            let result = analyze_one_image(
                &image_ref,
                vlm,
                asset_base_dir,
                &surrounding,
                kv_storage.clone(),
                mm_profile.classify_only,
            )
            .await;
            completed += 1;
            report_vision_figure_analyze_ex(converting_substep, completed, total, progress_opts);
            if completed == 1 || completed == total || completed.is_multiple_of(5) {
                info!(
                    completed,
                    total,
                    discovered,
                    elapsed_ms = pass_b_started.elapsed().as_millis() as u64,
                    "multimodal image analyze progress"
                );
            }

            match result {
                Ok((record, replacement)) => {
                    if image_ref.start <= output.len() && image_ref.end <= output.len() {
                        output.replace_range(image_ref.start..image_ref.end, &replacement);
                    }
                    attach_record(manifest, &record);
                    records.push(record);
                }
                Err(record) => {
                    if let Some(fatal) = handle_item_failure(
                        record.clone(),
                        fail_mode,
                        records,
                        manifest,
                        output.clone(),
                    ) {
                        return Some(fatal);
                    }
                    attach_record(manifest, &record);
                    records.push(record);
                }
            }
        }
    } else {
        let completed = AtomicUsize::new(0);
        type ImageAnalyzeOutcome = (
            edgequake_pdf::inline_images::InlineImageRef,
            Result<(MultimodalItemRecord, String), MultimodalItemRecord>,
        );
        let classify_only = mm_profile.classify_only;
        let cancel = cancel_token.cloned();
        let mut results: Vec<ImageAnalyzeOutcome> = stream::iter(jobs)
            .map(|(image_ref, surrounding)| {
                let kv = kv_storage.clone();
                let asset_owned = asset_base_dir.map(|p| p.to_path_buf());
                let cancel = cancel.clone();
                async move {
                    if cancel.as_ref().is_some_and(|t| t.is_cancelled()) {
                        let item_id = image_ref.item_id.clone();
                        return (
                            image_ref,
                            Err(MultimodalItemRecord::skipped(
                                &item_id,
                                "drawing",
                                "cancelled before figure analyze",
                            )),
                        );
                    }
                    let r = analyze_one_image(
                        &image_ref,
                        vlm,
                        asset_owned.as_deref(),
                        &surrounding,
                        kv,
                        classify_only,
                    )
                    .await;
                    (image_ref, r)
                }
            })
            .buffer_unordered(concurrency)
            .inspect(|_| {
                let n = completed.fetch_add(1, Ordering::Relaxed) + 1;
                report_vision_figure_analyze_ex(converting_substep, n, total, progress_opts);
                if n == 1 || n == total || n.is_multiple_of(5) {
                    info!(completed = n, total, "multimodal image analyze progress");
                }
            })
            .collect()
            .await;

        results.sort_by_key(|b| std::cmp::Reverse(b.0.start));
        for (image_ref, result) in results {
            match result {
                Ok((record, replacement)) => {
                    if image_ref.start <= output.len() && image_ref.end <= output.len() {
                        output.replace_range(image_ref.start..image_ref.end, &replacement);
                    }
                    attach_record(manifest, &record);
                    records.push(record);
                }
                Err(record) => {
                    if let Some(fatal) = handle_item_failure(
                        record.clone(),
                        fail_mode,
                        records,
                        manifest,
                        output.clone(),
                    ) {
                        return Some(fatal);
                    }
                    attach_record(manifest, &record);
                    records.push(record);
                }
            }
        }
    }

    if skipped > 0 {
        let notice = format!("\n\n<!-- mm: skipped {skipped} figures (local budget) -->\n");
        output.push_str(&notice);
        info!(
            skipped,
            discovered,
            analyzed = total,
            "Pass B figure cap applied"
        );
    }
    None
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

/// Parallel VLM item analyze concurrency (I/O-bound). Default 4; clamp 1..=16.
/// Override with `EDGEQUAKE_MM_IMAGE_CONCURRENCY` (shared for images/tables/equations).
///
/// Local vision (Ollama / LM Studio) defaults to **1** unless
/// `EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY` is set — gemma4 cannot absorb fan-out.
fn mm_item_concurrency() -> usize {
    let requested = std::env::var("EDGEQUAKE_MM_IMAGE_CONCURRENCY")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(4)
        .clamp(1, 16);
    let vision_provider = std::env::var("EDGEQUAKE_VISION_PROVIDER")
        .or_else(|_| std::env::var("EDGEQUAKE_LLM_PROVIDER"))
        .unwrap_or_default();
    if edgequake_pipeline::is_local_extraction_provider(&vision_provider)
        && !edgequake_pipeline::allow_local_high_concurrency()
    {
        requested.min(1)
    } else {
        requested
    }
}

/// Backward-compatible alias used by the image path.
fn mm_image_concurrency() -> usize {
    mm_item_concurrency()
}

/// Shared image byte analysis (standalone upload + inline PDF path).
///
/// `asset_path` / `asset_alt`: when present, replacement keeps a viewer-visible
/// `![alt](path)` on the page (MV-28) so the markdown viewer can render the PNG.
pub async fn analyze_image_bytes(
    item_id: &str,
    bytes: &[u8],
    mime_type: &str,
    llm: &dyn LLMProvider,
    ctx: &PromptContext,
    kv: Option<Arc<dyn KVStorage>>,
) -> Result<(MultimodalItemRecord, String), MultimodalItemRecord> {
    analyze_image_bytes_with_asset(item_id, bytes, mime_type, llm, ctx, kv, None, None).await
}

/// Like [`analyze_image_bytes`] with optional asset path for inline markdown image.
#[allow(clippy::too_many_arguments)]
pub async fn analyze_image_bytes_with_asset(
    item_id: &str,
    bytes: &[u8],
    mime_type: &str,
    llm: &dyn LLMProvider,
    ctx: &PromptContext,
    kv: Option<Arc<dyn KVStorage>>,
    asset_path: Option<&str>,
    asset_alt: Option<&str>,
) -> Result<(MultimodalItemRecord, String), MultimodalItemRecord> {
    analyze_image_bytes_with_asset_ex(
        item_id,
        bytes,
        mime_type,
        llm,
        ctx,
        kv,
        asset_path,
        asset_alt,
        LocalMmProfile::resolve_from_env().classify_only,
    )
    .await
}

/// Image analyze with explicit classify-only (local Pass B skips specialize).
#[allow(clippy::too_many_arguments)]
pub async fn analyze_image_bytes_with_asset_ex(
    item_id: &str,
    bytes: &[u8],
    mime_type: &str,
    llm: &dyn LLMProvider,
    ctx: &PromptContext,
    kv: Option<Arc<dyn KVStorage>>,
    asset_path: Option<&str>,
    asset_alt: Option<&str>,
    classify_only: bool,
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
    let (classified, cache_id): (ImageAnalysisResult, _) = chat_json_with_analysis_cache(
        llm,
        kv.clone(),
        item_id,
        "drawing",
        messages,
        parse_image_analysis,
        json_repair_user_message,
    )
    .await
    .map_err(|e| MultimodalItemRecord::failed(item_id, "drawing", e))?;

    // Phase B: classify → specialize Chart/Figure (MV-27 soft-fail to Pass A dump).
    // Local never-stuck profile: classify-only (skip specialize + dense retry).
    let analysis = if classify_only {
        classified
    } else {
        specialize_image_analysis(item_id, bytes, mime_type, llm, ctx, kv, classified).await
    };

    let mut record = MultimodalItemRecord::success_image(
        item_id,
        analysis.name.clone(),
        analysis.image_type.clone(),
        analysis.description.clone(),
    );
    maybe_attach_cache_key(&mut record, cache_id.as_deref());
    let replacement = format!(
        "\n\n{}\n\n",
        image_analysis_to_markdown_with_asset(&analysis, asset_path, asset_alt)
    );
    Ok((record, replacement))
}

async fn analyze_one_image(
    image_ref: &edgequake_pdf::inline_images::InlineImageRef,
    llm: &dyn LLMProvider,
    asset_base_dir: Option<&Path>,
    surrounding: &SurroundingContext,
    kv: Option<Arc<dyn KVStorage>>,
    classify_only: bool,
) -> Result<(MultimodalItemRecord, String), MultimodalItemRecord> {
    let asset = resolve_image_asset(image_ref, asset_base_dir)
        .map_err(|e| MultimodalItemRecord::skipped(&image_ref.item_id, "drawing", e))?;
    let mut ctx = PromptContext::from_parts(
        image_ref.caption.as_deref(),
        image_ref.footnote.as_deref(),
        surrounding,
    );
    ctx.apply_vision_extract(&current_vision_extract());
    // MV-28: `format_drawing_block` already emits `![alt](assets/…)` above the
    // `<drawing/>` tag. Replace only the tag with analysis body so the viewer
    // image stays on-page without duplication.
    analyze_image_bytes_with_asset_ex(
        &image_ref.item_id,
        &asset.bytes,
        &asset.mime_type,
        llm,
        &ctx,
        kv,
        None,
        None,
        classify_only,
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
    let mut ctx = PromptContext::from_item_and_surrounding(item, surrounding);
    ctx.apply_vision_extract(&current_vision_extract());
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
    let mut ctx = PromptContext::from_item_and_surrounding(item, surrounding);
    ctx.apply_vision_extract(&current_vision_extract());
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

    #[test]
    #[serial_test::serial]
    fn mm_concurrency_caps_to_one_for_local_vision() {
        std::env::remove_var("EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY");
        std::env::set_var("EDGEQUAKE_VISION_PROVIDER", "ollama");
        std::env::set_var("EDGEQUAKE_MM_IMAGE_CONCURRENCY", "4");
        assert_eq!(mm_item_concurrency(), 1);
        std::env::set_var("EDGEQUAKE_VISION_PROVIDER", "openai");
        assert_eq!(mm_item_concurrency(), 4);
        std::env::remove_var("EDGEQUAKE_VISION_PROVIDER");
        std::env::remove_var("EDGEQUAKE_MM_IMAGE_CONCURRENCY");
    }

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
    async fn page_modality_ctx_visible_inside_scope() {
        with_page_modality_ctx(PageModality::Manuscript, async {
            assert_eq!(current_page_modality(), PageModality::Manuscript);
        })
        .await;
    }

    #[tokio::test]
    async fn page_modality_ctx_nested_scope_overrides_then_restores() {
        with_page_modality_ctx(PageModality::Manuscript, async {
            with_page_modality_ctx(PageModality::Mixed, async {
                assert_eq!(current_page_modality(), PageModality::Mixed);
            })
            .await;
            assert_eq!(current_page_modality(), PageModality::Manuscript);
        })
        .await;
    }

    #[tokio::test]
    async fn page_modality_defaults_to_print_outside_scope() {
        // No scope and no env override (lib tests never set
        // EDGEQUAKE_PDF_PAGE_MODALITY) → fail-open default.
        assert_eq!(current_page_modality(), PageModality::Print);
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

    /// E2E (mock VLM): classify Chart → specialize → key_values land in description.
    #[tokio::test]
    #[serial_test::serial]
    async fn chart_classify_then_specialize_lands_key_values() {
        // Isolate from operator / `make dev` env (ollama + MM_LOCAL_CLASSIFY_ONLY=1
        // would skip specialize via LocalMmProfile::resolve_from_env).
        let prev_mm = std::env::var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY").ok();
        let prev_vision = std::env::var("EDGEQUAKE_VISION_PROVIDER").ok();
        let prev_llm = std::env::var("EDGEQUAKE_LLM_PROVIDER").ok();
        let prev_vlm_min = std::env::var("VLM_MIN_IMAGE_PIXEL").ok();
        // SAFETY: #[serial]; restore in cleanup below.
        unsafe {
            std::env::remove_var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY");
            std::env::remove_var("EDGEQUAKE_VISION_PROVIDER");
            std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
            std::env::set_var("VLM_MIN_IMAGE_PIXEL", "1");
        }
        let png: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89,
        ];
        let mock = MockProvider::new();
        mock.add_response(r#"{"name":"rev","type":"Chart","description":"generic chart"}"#)
            .await;
        mock.add_response(
            r#"{"name":"rev_q4","chart_kind":"bar","title":"Q4 Revenue","x_axis":"Quarter","y_axis":"USD M","key_values":[{"label":"Q4","value_raw":"42"}],"series":[],"description":"Revenue rose."}"#,
        )
        .await;
        let ctx = PromptContext {
            language: "English".into(),
            captions: "n/a".into(),
            footnotes: "n/a".into(),
            leading: "n/a".into(),
            trailing: "n/a".into(),

            image_system_prompt: None,
            chart_system_prompt: None,
            figure_system_prompt: None,
            extract_images: true,
            extract_charts: true,
            extract_figures: true,
        };
        let result = analyze_image_bytes("im-chart", png, "image/png", &mock, &ctx, None).await;
        unsafe {
            match prev_mm {
                Some(v) => std::env::set_var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY", v),
                None => std::env::remove_var("EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY"),
            }
            match prev_vision {
                Some(v) => std::env::set_var("EDGEQUAKE_VISION_PROVIDER", v),
                None => std::env::remove_var("EDGEQUAKE_VISION_PROVIDER"),
            }
            match prev_llm {
                Some(v) => std::env::set_var("EDGEQUAKE_LLM_PROVIDER", v),
                None => std::env::remove_var("EDGEQUAKE_LLM_PROVIDER"),
            }
            match prev_vlm_min {
                Some(v) => std::env::set_var("VLM_MIN_IMAGE_PIXEL", v),
                None => std::env::remove_var("VLM_MIN_IMAGE_PIXEL"),
            }
        }
        let (record, replacement) = result.expect("chart specialize path");
        let desc = record.description.as_deref().unwrap_or("");
        assert!(
            desc.contains("42"),
            "expected key value in description: {desc}"
        );
        assert!(desc.contains("bar"));
        assert_eq!(record.item_type.as_deref(), Some("Chart"));
        assert!(replacement.contains("42"));
    }
}
