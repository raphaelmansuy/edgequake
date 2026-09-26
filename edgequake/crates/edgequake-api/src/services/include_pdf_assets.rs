//! Include extracted PDF page/figure assets into document markdown (SPEC-047).
//!
//! Identity chain:
//! 1. Stored PDF bytes are the durable visual source.
//! 2. Embedded ImageXObjects → `assets/page-NNNN-fig-MM.png` (VLM Drawing + viewer).
//! 3. Caption-anchored Form XObject / vector figures + table crops → `-fig-` / `-table-`.
//! 4. Chart ink-crops → `assets/page-NNNN-chart.png` (VLM Drawing + viewer).
//! 5. Pdfium page renders → `assets/page-NNNN.png` (dual-pane PDF context only — never Drawing).
//! 6. Persist to `document_mm_assets` (+ FS cache).
//! 7. Inject only on-disk fig/table/chart assets into markdown (never invent paths).
//!
//! This does **not** re-run VLM OCR. Optional multimodal analyze (`i`) remains separate.

use serde::Serialize;
use tracing::info;
use uuid::Uuid;

use edgequake_pdf::page_numbers_from_markdown;
#[cfg(feature = "postgres")]
use edgequake_pdf::{
    enrich_markdown_with_viewer_assets, figures_by_page, inject_on_disk_region_assets,
    write_caption_region_assets, write_embedded_figure_assets, write_page_png_assets,
    PageAssetRenderConfig,
};
#[cfg(feature = "postgres")]
use edgequake_storage::UpdatePdfProcessingRequest;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
#[cfg(feature = "postgres")]
use crate::services::document_assets::document_mm_assets_root;
#[cfg(feature = "postgres")]
use crate::services::document_body_loader::load_document_body;
use crate::services::document_metadata_scan::metadata_key_for_document;
use crate::state::AppState;

/// True when a chart PNG still covers most of a companion full-page raster.
#[cfg_attr(not(any(test, feature = "postgres")), allow(dead_code))]
fn chart_png_is_near_full_page(chart_path: &std::path::Path) -> bool {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(chart_path) else {
        return false;
    };
    let mut hdr = [0u8; 24];
    if f.read_exact(&mut hdr).is_err() || &hdr[0..4] != b"\x89PNG" {
        return false;
    }
    let w = u32::from_be_bytes([hdr[16], hdr[17], hdr[18], hdr[19]]) as u64;
    let h = u32::from_be_bytes([hdr[20], hdr[21], hdr[22], hdr[23]]) as u64;
    let chart_area = w.saturating_mul(h);
    // Companion page-NNNN.png shares the same page digits prefix.
    let name = chart_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let page_stem = name.split("-chart").next().unwrap_or("");
    let page_path = chart_path.with_file_name(format!("{page_stem}.png"));
    let page_area = if let Ok(mut pf) = std::fs::File::open(&page_path) {
        let mut ph = [0u8; 24];
        if pf.read_exact(&mut ph).is_ok() && &ph[0..4] == b"\x89PNG" {
            let pw = u32::from_be_bytes([ph[16], ph[17], ph[18], ph[19]]) as u64;
            let phh = u32::from_be_bytes([ph[20], ph[21], ph[22], ph[23]]) as u64;
            pw.saturating_mul(phh).max(1)
        } else {
            1545u64 * 2000
        }
    } else {
        1545u64 * 2000
    };
    (chart_area as f64 / page_area as f64) > 0.55
}

/// Pages that need PNG assets: all markdown page markers (non-flaky).
/// Caption keywords must not gate which pages get object crops (SPEC-049/005).
#[cfg_attr(not(any(test, feature = "postgres")), allow(dead_code))]
fn pages_for_asset_include(markdown: &str) -> Vec<usize> {
    page_numbers_from_markdown(markdown)
}

#[derive(Debug, Clone, Serialize)]
pub struct IncludePdfAssetsResult {
    pub document_id: String,
    pub pages_rendered: usize,
    pub assets_persisted: usize,
    pub markdown_updated: bool,
}

/// Render page PNGs from the linked PDF, persist mm-assets, enrich markdown with figure images.
pub async fn include_extracted_pdf_assets(
    state: &AppState,
    tenant: &TenantContext,
    document_id: &str,
) -> ApiResult<IncludePdfAssetsResult> {
    let meta_key = metadata_key_for_document(document_id);
    let metadata = state
        .storage
        .kv_storage
        .get_by_id(&meta_key)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("Document not found: {document_id}")))?;

    let workspace_id = tenant
        .workspace_id_uuid()
        .or_else(|| {
            metadata
                .get("workspace_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
        })
        .ok_or_else(|| ApiError::BadRequest("workspace_id required".into()))?;

    let pdf_id = metadata
        .get("pdf_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ApiError::BadRequest("Document has no linked PDF (pdf_id missing)".into())
        })?;
    let pdf_uuid =
        Uuid::parse_str(pdf_id).map_err(|_| ApiError::BadRequest("Invalid pdf_id".into()))?;

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, tenant, document_id, metadata, workspace_id, pdf_uuid);
        return Err(ApiError::Internal(
            "include_extracted_pdf_assets requires the postgres feature".into(),
        ));
    }

    #[cfg(feature = "postgres")]
    {
        let pdf_storage = state
            .storage
            .pdf_storage
            .as_ref()
            .ok_or_else(|| ApiError::Internal("PDF storage not available".into()))?;
        let pdf = pdf_storage
            .get_pdf(&pdf_uuid)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to load PDF: {e}")))?
            .ok_or_else(|| ApiError::NotFound(format!("PDF not found: {pdf_id}")))?;

        if let Some(ws) = tenant.workspace_id_uuid() {
            if pdf.workspace_id != ws {
                return Err(ApiError::forbidden());
            }
        }

        let body = load_document_body(&state.storage, document_id, &metadata)
            .await
            .ok_or_else(|| ApiError::NotFound("Document markdown body not found".into()))?;
        let pages = pages_for_asset_include(&body.markdown);

        let assets_root = document_mm_assets_root(document_id);
        let figures = write_embedded_figure_assets(&pdf.pdf_data, &assets_root, Some(&pages))
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(%document_id, error = %e, "Embedded figure extract skipped");
                Vec::new()
            });
        let mut figure_map = figures_by_page(&figures);
        let (region_figs, region_tables) =
            write_caption_region_assets(&pdf.pdf_data, &assets_root, &figure_map, Some(&pages))
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!(%document_id, error = %e, "Caption region extract skipped");
                    (Vec::new(), Vec::new())
                });
        for fig in &region_figs {
            figure_map
                .entry(fig.page_num)
                .or_default()
                .push(fig.clone());
        }
        // LAW-128-13: prune after every writer so include cannot resurrect logos.
        figure_map = edgequake_pdf::prune_figure_map_using_manifest(figure_map, &assets_root, true);
        let written = write_page_png_assets(
            &pdf.pdf_data,
            &assets_root,
            &pages,
            PageAssetRenderConfig::default(),
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to render PDF page assets: {e}")))?;
        let pages_rendered = written
            .len()
            .max(figures.len() + region_figs.len() + region_tables.len());

        // Drop stale / near-full chart PNGs (first principles: chart ≠ full page).
        let mut pages_with_region = std::collections::HashSet::new();
        for t in &region_tables {
            pages_with_region.insert(t.page_num);
        }
        for f in figure_map.keys() {
            pages_with_region.insert(*f);
        }
        for f in &region_figs {
            pages_with_region.insert(f.page_num);
        }
        for page in &pages_with_region {
            let chart = assets_root.join(format!("assets/page-{page:04}-chart.png"));
            if chart.is_file() {
                let _ = std::fs::remove_file(&chart);
                tracing::info!(
                    %document_id,
                    page,
                    "Removed stale chart crop superseded by fig/table region"
                );
            }
        }
        // Also drop legacy chart files that are still near-full-page rasters.
        if let Ok(entries) = std::fs::read_dir(assets_root.join("assets")) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if !name.contains("-chart.png") {
                    continue;
                }
                if chart_png_is_near_full_page(&entry.path()) {
                    let _ = std::fs::remove_file(entry.path());
                    tracing::info!(
                        %document_id,
                        file = %name,
                        "Removed near-full-page chart crop (not a real region)"
                    );
                }
            }
        }

        let assets_persisted = crate::services::persist_uploaded_mm_assets(
            state,
            document_id,
            workspace_id,
            &assets_root,
        )
        .await?;
        crate::services::persist_page_layout_best_effort(
            state.storage.page_layout_storage.as_ref(),
            document_id,
            workspace_id,
            &assets_root,
        )
        .await;

        // Inject real on-disk fig/table assets into markdown (never invent missing paths).
        let mut enriched = inject_on_disk_region_assets(&body.markdown, &assets_root);
        enriched = enrich_markdown_with_viewer_assets(&enriched);
        let markdown_updated = enriched != body.markdown;
        if markdown_updated {
            let content_key = format!("{document_id}-content");
            let mut content_val = state
                .storage
                .kv_storage
                .get_by_id(&content_key)
                .await
                .map_err(ApiError::from)?
                .unwrap_or_else(|| serde_json::json!({}));
            if let Some(obj) = content_val.as_object_mut() {
                obj.insert(
                    "content".into(),
                    serde_json::Value::String(enriched.clone()),
                );
            } else {
                content_val = serde_json::json!({ "content": enriched });
            }
            state
                .storage
                .kv_storage
                .upsert(&[(content_key, content_val)])
                .await
                .map_err(ApiError::from)?;

            if let Some(ref pdf_store) = state.storage.pdf_storage {
                if let Err(e) = pdf_store
                    .update_pdf_processing(UpdatePdfProcessingRequest {
                        pdf_id: pdf_uuid,
                        processing_status: pdf.processing_status,
                        extraction_method: pdf.extraction_method,
                        markdown_content: Some(enriched),
                        extraction_errors: None,
                        document_id: pdf.document_id,
                        vision_model: pdf.vision_model.clone(),
                    })
                    .await
                {
                    tracing::warn!(
                        %document_id,
                        error = %e,
                        "Failed to sync enriched markdown to PDF storage"
                    );
                }
            }
        }

        info!(
            %document_id,
            pages = pages_rendered,
            figures = figures.len(),
            region_figs = region_figs.len(),
            region_tables = region_tables.len(),
            assets_persisted,
            markdown_updated,
            "Included extracted PDF page/figure assets into document"
        );

        Ok(IncludePdfAssetsResult {
            document_id: document_id.to_string(),
            pages_rendered,
            assets_persisted,
            markdown_updated,
        })
    } // cfg(feature = "postgres")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal 1×1 PNG IHDR (w=1,h=1) — same bytes as multimodal TINY_PNG.
    const TINY_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    fn png_with_dims(w: u32, h: u32) -> Vec<u8> {
        let mut bytes = TINY_PNG.to_vec();
        bytes[16..20].copy_from_slice(&w.to_be_bytes());
        bytes[20..24].copy_from_slice(&h.to_be_bytes());
        bytes
    }

    #[test]
    fn pages_for_include_all_when_no_figure_table() {
        let md = "<!-- edgequake-page:1 -->\nProse.\n\n<!-- edgequake-page:2 -->\nMore.\n";
        assert_eq!(pages_for_asset_include(md), vec![1, 2]);
    }

    #[test]
    fn pages_for_include_all_marked_pages() {
        let md = "<!-- edgequake-page:1 -->\nIntro.\n\n<!-- edgequake-page:4 -->\n## Figure 1: Loop\n\n<!-- edgequake-page:6 -->\n## Table 1: Rates\n";
        assert_eq!(pages_for_asset_include(md), vec![1, 4, 6]);
    }

    #[test]
    fn near_full_chart_detected_vs_companion_page() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        // Page 1000×1000, chart 800×800 → 0.64 > 0.55
        std::fs::write(assets.join("page-0002.png"), png_with_dims(1000, 1000)).unwrap();
        let chart = assets.join("page-0002-chart.png");
        std::fs::write(&chart, png_with_dims(800, 800)).unwrap();
        assert!(chart_png_is_near_full_page(&chart));
    }

    #[test]
    fn real_chart_crop_not_near_full() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        std::fs::write(assets.join("page-0002.png"), png_with_dims(1000, 1000)).unwrap();
        let chart = assets.join("page-0002-chart.png");
        // 300×300 / 1e6 = 0.09 < 0.55
        std::fs::write(&chart, png_with_dims(300, 300)).unwrap();
        assert!(!chart_png_is_near_full_page(&chart));
    }

    #[test]
    fn missing_or_invalid_chart_not_near_full() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("assets/page-0001-chart.png");
        assert!(!chart_png_is_near_full_page(&missing));
        std::fs::create_dir_all(missing.parent().unwrap()).unwrap();
        std::fs::write(&missing, b"not-a-png").unwrap();
        assert!(!chart_png_is_near_full_page(&missing));
    }
}
