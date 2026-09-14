use std::sync::Arc;

use async_trait::async_trait;
use edgequake_llm::ProviderFactory;
use edgequake_pdf2md::{convert_from_bytes, ConversionConfig, FileCheckpointStore, PageSelection};
use tracing::{info, warn};

use super::{PdfConversionConfig, PdfConverter};
use crate::chart_crop::{
    chart_residual_alongside_fig_pages, chart_residual_candidate_pages,
    filter_chart_pages_by_page_png_ink, write_chart_crop_assets, CropCoverageReport,
    CHART_CROP_RENDER,
};
use crate::error::PdfConversionError;
use crate::figure_extract::{
    extract_embedded_figures, inventory_figure_pages, prune_artifact_figures,
    skip_all_region_crops, write_caption_region_assets_for_keep_pages, EmbeddedFigureExtract,
};
use crate::page_assets::{write_page_png_assets, PageAssetRenderConfig};
use crate::reasoning_effort_inject::ReasoningEffortInjectProvider;
use crate::region_assets::tables_by_page;
use crate::vision_markdown::{normalize_vision_pages, VisionPageSlice};

/// Vision-based PDF converter backed by `edgequake-pdf2md`.
///
/// Uses `provider_name` + `model` factory resolution inside pdf2md instead of
/// injecting `Arc<dyn LLMProvider>` — avoids dual edgequake-llm versions until
/// pdf2md@0.9.3 aligns on 0.10.0 (SPEC-043 P0).
pub struct VisionPdfConverter;

impl std::fmt::Debug for VisionPdfConverter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisionPdfConverter").finish()
    }
}

impl Default for VisionPdfConverter {
    fn default() -> Self {
        Self
    }
}

impl VisionPdfConverter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PdfConverter for VisionPdfConverter {
    async fn convert(
        &self,
        pdf_bytes: &[u8],
        config: &PdfConversionConfig,
    ) -> Result<String, PdfConversionError> {
        let vision = config
            .vision
            .as_ref()
            .ok_or(PdfConversionError::BackendNotConfigured("vision config"))?;
        let provider_name = vision
            .provider_name
            .clone()
            .ok_or(PdfConversionError::BackendNotConfigured("vision provider"))?;
        let model = vision
            .model
            .clone()
            .ok_or(PdfConversionError::BackendNotConfigured("vision model"))?;

        let page_prompt = config
            .page_drawing_assets
            .as_ref()
            .and_then(|c| c.page_system_prompt.as_deref())
            .unwrap_or(crate::vision_prompts::RAG_PAGE_VISION_SYSTEM_PROMPT);
        let mut builder = ConversionConfig::builder()
            // SPEC-047 / SPEC-015V: chart/figure number dump for RAG indexing
            .system_prompt(page_prompt);

        // SPEC-109: when effort is set, inject a clamped provider so pdf2md page
        // OCR forwards reasoning_effort (ConversionConfig has no effort field yet).
        // SPEC-134 WP-1: always build the provider explicitly (identical to
        // pdf2md's internal `provider_name` path — same factory call) so the
        // image size guard wraps every Pass-A page OCR call. Providers reject
        // oversized page renders silently; the guard re-encodes them first.
        {
            let base = ProviderFactory::create_llm_provider(&provider_name, &model)
                .map_err(|error| PdfConversionError::Backend(error.to_string()))?;
            let modality = config
                .page_drawing_assets
                .as_ref()
                .and_then(|c| c.page_modality)
                .unwrap_or(crate::PageModality::Print);
            let guarded = crate::image_guard::ImageGuardProvider::wrap_for_modality(base, modality);
            let wrapped = ReasoningEffortInjectProvider::wrap(
                guarded,
                &provider_name,
                vision.reasoning_effort.as_deref(),
            );
            builder = builder.provider(wrapped).model(model.clone());
        }

        if let Some(concurrency) = vision.concurrency {
            builder = builder.concurrency(concurrency);
        }
        if let Some(dpi) = vision.dpi {
            builder = builder.dpi(dpi);
        }
        if let Some(max_px) = vision.max_rendered_pixels {
            builder = builder.max_rendered_pixels(max_px);
        }
        if let Some(progress_callback) = vision.progress_callback.clone() {
            builder = builder.progress_callback(progress_callback);
        }
        if let Some(checkpoint_dir) = vision.checkpoint_dir.clone() {
            builder = builder.checkpoint_store(Arc::new(FileCheckpointStore::new(&checkpoint_dir)));
        }
        if vision.no_resume {
            builder = builder.no_resume(true);
        }
        // Prefer vision.pages; fall back to top-level PdfConversionConfig.pages.
        let pages = vision
            .pages
            .clone()
            .or_else(|| config.pages.clone())
            .unwrap_or(PageSelection::All);
        if !matches!(pages, PageSelection::All) {
            builder = builder.pages(pages);
        }

        let conversion_config = builder
            .build()
            .map_err(|error| PdfConversionError::Backend(error.to_string()))?;
        let output = convert_from_bytes(pdf_bytes, &conversion_config)
            .await
            .map_err(|error| PdfConversionError::Backend(error.to_string()))?;

        if output.markdown.trim().is_empty() && output.stats.processed_pages == 0 {
            return Err(PdfConversionError::EmptyOutput(
                "vision returned no markdown",
            ));
        }

        let emit_viewer_images = config
            .page_drawing_assets
            .as_ref()
            .is_some_and(|c| c.write_plan().any_writer());
        let emit_analyze_tags = config
            .page_drawing_assets
            .as_ref()
            .is_some_and(|c| c.emit_analyze_tags);
        let status_hook = vision.status_hook.as_ref();
        let mut chart_crop_paths = std::collections::HashMap::new();
        let mut figure_map = std::collections::HashMap::new();
        let mut table_map = std::collections::HashMap::new();
        let mut crop_coverage_comment: Option<String> = None;
        let mut vision_filter_results: Vec<crate::figure_filter::FigureFilterResult> = Vec::new();
        if emit_viewer_images {
            if let Some(page_assets) = &config.page_drawing_assets {
                let plan = page_assets.write_plan();
                let page_as_unit = page_assets
                    .page_modality
                    .is_some_and(|m| m.is_manuscript_like());
                let total_pages = output.stats.total_pages.max(output.pages.len()).max(1);
                let page_numbers: Vec<usize> = (1..=total_pages).collect();
                let mut extract =
                    EmbeddedFigureExtract::from_plan(Vec::new(), page_numbers.clone());
                let render = PageAssetRenderConfig {
                    dpi: vision.dpi.unwrap_or(150),
                    max_rendered_pixels: vision.max_rendered_pixels.unwrap_or(2000),
                };

                // 1) Figure page plan + optional ImageXObject extract.
                // Always inventory when figures or charts are enabled so artifact
                // pages never become chart residuals.
                if plan.write_figures || plan.write_charts {
                    if let Some(hook) = status_hook {
                        hook("Planning figure pages from PDF…", 0.92);
                    }
                    let planned = if plan.write_figures {
                        extract_embedded_figures(
                            pdf_bytes,
                            &page_assets.assets_root,
                            Some(&page_numbers),
                        )
                        .await
                    } else {
                        inventory_figure_pages(pdf_bytes, Some(&page_numbers)).await
                    };
                    match planned {
                        Ok(e) => {
                            extract = e;
                            figure_map = extract.figure_map();
                            if extract.keep_pages.is_empty() {
                                if let Some(hook) = status_hook {
                                    hook(
                                        "Skipping encoding-artifact ImageXObjects — analyzing page images…",
                                        0.92,
                                    );
                                }
                            } else if plan.write_figures {
                                info!(
                                    figures = extract.written.len(),
                                    keep_pages = extract.keep_pages.len(),
                                    artifact_pages = extract.artifact_pages.len(),
                                    assets_root = %page_assets.assets_root.display(),
                                    "Embedded figure assets written for VLM analyze"
                                );
                                if let Some(hook) = status_hook {
                                    hook(
                                        &format!(
                                            "Extracted {} embedded figure(s) — rendering page images…",
                                            extract.written.len()
                                        ),
                                        0.93,
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            warn!(
                                error = %e,
                                "Figure page plan failed; fail-closed (no fig/caption/chart crops)"
                            );
                            extract = EmbeddedFigureExtract::fail_closed();
                        }
                    }

                    // 1b) Caption-anchored regions on keep pages only.
                    if plan.write_figures && !extract.skip_all_region_crops(page_as_unit) {
                        match write_caption_region_assets_for_keep_pages(
                            pdf_bytes,
                            &page_assets.assets_root,
                            &figure_map,
                            &extract.keep_pages,
                        )
                        .await
                        {
                            Ok((region_figs, region_tables)) => {
                                if !region_figs.is_empty() {
                                    info!(
                                        figures = region_figs.len(),
                                        "Caption-anchored figure regions written"
                                    );
                                    for fig in region_figs {
                                        figure_map.entry(fig.page_num).or_default().push(fig);
                                    }
                                }
                                if !region_tables.is_empty() {
                                    info!(
                                        tables = region_tables.len(),
                                        "Caption-anchored table regions written"
                                    );
                                    table_map = tables_by_page(&region_tables);
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, "Caption region extract failed");
                            }
                        }
                    }
                    if plan.write_figures {
                        prune_artifact_figures(&mut figure_map);
                    }
                }

                // 2) Full-page PNGs for markdown viewer only (SPEC-015V: extract_images).
                if plan.write_page_pngs {
                    if let Some(hook) = status_hook {
                        hook(
                            &format!(
                                "Rendering page images for the viewer (0/{total_pages} pages)…"
                            ),
                            0.94,
                        );
                    }
                    match write_page_png_assets(
                        pdf_bytes,
                        &page_assets.assets_root,
                        &page_numbers,
                        render,
                    )
                    .await
                    {
                        Ok(written) => {
                            info!(
                                pages = written.len(),
                                assets_root = %page_assets.assets_root.display(),
                                "Vision page PNG assets written for markdown viewer"
                            );
                            if let Some(hook) = status_hook {
                                hook(
                                    &format!(
                                        "Rendered page images ({}/{} pages) — assembling markdown…",
                                        written.len(),
                                        total_pages
                                    ),
                                    0.95,
                                );
                            }
                        }
                        Err(e) => {
                            warn!(
                                error = %e,
                                "Failed to write vision page PNG assets; figure images may not resolve"
                            );
                        }
                    }
                }

                // 3) Chart ink-residual — SPEC-015V: gated by extract_charts.
                let page_nums: Vec<usize> = output.pages.iter().map(|p| p.page_num).collect();
                let mut coverage =
                    CropCoverageReport::from_pages(&page_nums, &figure_map, &table_map);
                if plan.write_charts && !skip_all_region_crops(page_as_unit, &extract) {
                    let candidates: Vec<usize> =
                        chart_residual_candidate_pages(&page_nums, &figure_map, &table_map)
                            .into_iter()
                            .filter(|p| !extract.omit_page(*p, page_as_unit))
                            .collect();
                    // EC-015V-4: without page PNGs, skip ink prefilter and crop candidates directly.
                    let chart_pages = if plan.write_page_pngs {
                        filter_chart_pages_by_page_png_ink(&page_assets.assets_root, &candidates)
                    } else {
                        candidates
                    };
                    coverage = coverage.with_ink_filter_count(chart_pages.len());
                    if !chart_pages.is_empty() {
                        if let Some(hook) = status_hook {
                            hook(
                                &format!(
                                    "Rendering chart ink-crops ({} pages; alongside_fig={}, table_skip={})…",
                                    chart_pages.len(),
                                    coverage.residual_alongside_fig,
                                    coverage.residual_skipped_due_to_fig_or_table,
                                ),
                                0.96,
                            );
                        }
                        match write_chart_crop_assets(
                            pdf_bytes,
                            &page_assets.assets_root,
                            &chart_pages,
                            CHART_CROP_RENDER,
                        )
                        .await
                        {
                            Ok(paths) => {
                                chart_crop_paths = paths;
                            }
                            Err(e) => {
                                warn!(
                                    error = %e,
                                    "MV-24 chart crop render failed; no analyze fallback for those pages"
                                );
                            }
                        }
                    }
                    // W1-fig-as-chart: only when both charts+figures enabled (EC-015V-2).
                    if plan.promote_fig_as_chart {
                        let alongside: Vec<usize> =
                            chart_residual_alongside_fig_pages(&page_nums, &figure_map, &table_map)
                                .into_iter()
                                .filter(|p| !extract.omit_page(*p, page_as_unit))
                                .collect();
                        let promoted = crate::chart_crop::promote_fig_as_chart_when_ink_empty(
                            &page_assets.assets_root,
                            &alongside,
                            &chart_crop_paths,
                        );
                        if !promoted.is_empty() {
                            info!(
                                promoted = promoted.len(),
                                "W1-fig-as-chart: promoted fig assets to chart crops (ink residual empty)"
                            );
                            chart_crop_paths.extend(promoted);
                        }
                    }
                }
                coverage = coverage.with_crops_written(chart_crop_paths.len());
                info!(
                    total_pages = coverage.total_pages,
                    pages_with_fig = coverage.pages_with_fig,
                    pages_with_table = coverage.pages_with_table,
                    residual_candidates = coverage.residual_candidates,
                    residual_alongside_fig = coverage.residual_alongside_fig,
                    residual_skipped = coverage.residual_skipped_due_to_fig_or_table,
                    residual_after_ink = coverage.residual_after_ink_filter,
                    residual_crops_written = coverage.residual_crops_written,
                    "SPEC-047 crop coverage telemetry"
                );
                crop_coverage_comment = Some(coverage.to_html_comment());

                // 1c) SPEC-049/128 two-pass VLM filter — after figures AND chart crops.
                // Default-on when figure_filter_provider is attached (LAW-128-14).
                if let Some(ref provider) = page_assets.figure_filter_provider {
                    let candidates = crate::figure_filter::collect_filter_candidates(
                        &page_assets.assets_root,
                        &figure_map,
                        &chart_crop_paths,
                    );
                    if !candidates.is_empty() {
                        if let Some(hook) = status_hook {
                            hook(
                                &format!(
                                    "Running two-pass figure filter on {} crops…",
                                    candidates.len()
                                ),
                                0.965,
                            );
                        }
                        let filter = crate::figure_filter::FigureFilter::new(Arc::clone(provider));
                        let run = filter.run(&candidates).await;
                        if let Ok(ref results) = run {
                            let kept = results.iter().filter(|r| r.is_figure).count();
                            info!(
                                total = results.len(),
                                kept,
                                discarded = results.len() - kept,
                                "SPEC-049 two-pass figure filter complete"
                            );
                            if let Err(e) = crate::figure_filter::write_manifest(
                                &page_assets.assets_root,
                                results,
                            ) {
                                warn!(error = %e, "Failed to write figure filter manifest");
                            }
                            crate::page_layout::write_sidecar_from_assets(
                                &page_assets.assets_root,
                                pdf_bytes,
                                &figure_map,
                                &table_map,
                                Some(results),
                            );
                            vision_filter_results = results.clone();
                        }
                        figure_map = crate::figure_filter::apply_filter_result_or_keep(
                            figure_map,
                            run,
                            &page_assets.assets_root,
                            true,
                        );
                        chart_crop_paths = crate::figure_filter::prune_chart_crop_paths(
                            chart_crop_paths,
                            &vision_filter_results,
                        );
                    }
                }
                if !crate::page_layout::sidecar_exists(&page_assets.assets_root) {
                    crate::page_layout::write_sidecar_from_assets(
                        &page_assets.assets_root,
                        pdf_bytes,
                        &figure_map,
                        &table_map,
                        None,
                    );
                }
            }
        }

        let page_slices: Vec<VisionPageSlice> = output
            .pages
            .iter()
            .map(|p| VisionPageSlice {
                page_num: p.page_num,
                markdown: p.markdown.clone(),
            })
            .collect();

        let total_pages = output.stats.total_pages.max(page_slices.len()).max(1);
        let normalized = normalize_vision_pages(&page_slices, total_pages, output.markdown.trim());
        let id_prefix = config
            .page_drawing_assets
            .as_ref()
            .and_then(|c| c.id_prefix.as_deref());
        let overrides = if chart_crop_paths.is_empty() {
            None
        } else {
            Some(&chart_crop_paths)
        };
        let figures = if figure_map.is_empty() {
            None
        } else {
            Some(&figure_map)
        };
        let tables = if table_map.is_empty() {
            None
        } else {
            Some(&table_map)
        };
        let page_as_unit = config
            .page_drawing_assets
            .as_ref()
            .and_then(|c| c.page_modality)
            .is_some_and(|m| m.is_manuscript_like());
        let mut markdown = crate::vision_markdown::assemble_vision_markdown_with_policy(
            &normalized,
            emit_viewer_images,
            emit_analyze_tags,
            id_prefix,
            overrides,
            figures,
            tables,
            page_as_unit,
        );
        if !vision_filter_results.is_empty() {
            markdown =
                crate::figure_filter::inject_kept_descriptions(&markdown, &vision_filter_results);
        }
        if let Some(comment) = crop_coverage_comment {
            if !markdown.contains("edgequake-crop-coverage:") {
                markdown.push('\n');
                markdown.push_str(&comment);
                markdown.push('\n');
            }
        }

        info!(
            pages = total_pages,
            processed_pages = output.stats.processed_pages,
            markdown_len = markdown.len(),
            viewer_images = emit_viewer_images,
            analyze_tags = emit_analyze_tags,
            embedded_figures = figure_map.values().map(|v| v.len()).sum::<usize>(),
            table_regions = table_map.values().map(|v| v.len()).sum::<usize>(),
            chart_crops = chart_crop_paths.len(),
            "Vision conversion completed"
        );

        Ok(markdown)
    }

    fn backend_name(&self) -> &'static str {
        "vision"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedded_images::WrittenFigureAsset;
    use crate::inline_images::scan_inline_image_refs;
    use crate::vision_markdown::assemble_vision_markdown_with_figures;
    use std::collections::HashMap;

    #[test]
    fn assemble_emits_figure_bounded_drawing_tags() {
        let pages = vec![
            VisionPageSlice {
                page_num: 1,
                markdown: "Chart title".into(),
            },
            VisionPageSlice {
                page_num: 2,
                markdown: String::new(),
            },
        ];
        let mut figs = HashMap::new();
        figs.insert(
            1,
            vec![WrittenFigureAsset {
                page_num: 1,
                index: 1,
                rel_path: "assets/page-0001-fig-01.png".into(),
                width: 40,
                height: 30,
                bbox: None,
            }],
        );
        figs.insert(
            2,
            vec![WrittenFigureAsset {
                page_num: 2,
                index: 1,
                rel_path: "assets/page-0002-fig-01.png".into(),
                width: 40,
                height: 30,
                bbox: None,
            }],
        );
        let md = assemble_vision_markdown_with_figures(
            &pages,
            true,
            true,
            Some("doc-x"),
            None,
            Some(&figs),
            None,
        );
        assert!(md.contains("<!-- edgequake-page:1 -->"));
        assert!(md.contains("<!-- edgequake-page:2 -->"));
        assert!(md.contains(crate::drawing_tags::EMPTY_VISION_PAGE_PLACEHOLDER));
        let refs = scan_inline_image_refs(&md);
        assert_eq!(refs.len(), 1, "empty Pass-A must not grow fig hrefs: {md}");
        assert_eq!(
            refs[0].asset_path.as_deref(),
            Some("assets/page-0001-fig-01.png")
        );
        assert!(md.contains(crate::drawing_tags::EMPTY_VISION_PAGE_PLACEHOLDER));
        assert!(
            !md.contains("page-0002-fig-01.png"),
            "empty page 2 must not inject scan tiles"
        );
        assert!(
            refs.iter().all(|r| {
                !r.asset_path
                    .as_deref()
                    .is_some_and(|p| p.ends_with("page-0001.png") || p.ends_with("page-0002.png"))
            }),
            "must not use full-page paths for analyze"
        );
    }

    #[test]
    fn assemble_without_figures_does_not_emit_full_page_drawings() {
        let pages = vec![VisionPageSlice {
            page_num: 1,
            markdown: "Plain text page".into(),
        }];
        let md = assemble_vision_markdown_with_figures(
            &pages,
            true,
            true,
            Some("doc"),
            None,
            None,
            None,
        );
        let refs = scan_inline_image_refs(&md);
        assert!(
            refs.is_empty(),
            "no ImageXObject and no chart crop → no VLM drawing on full page"
        );
    }
}
