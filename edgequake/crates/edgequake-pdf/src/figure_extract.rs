//! Single entry for Vision figure assets (SRP: orchestrate plan + write + prune).
//!
//! Callers (Vision convert, include-pdf-assets) must not invent a second skip
//! policy. This module:
//!
//! 1. Inventories Image `Do`s on **every** page (cheap, no pdfium).
//! 2. Artifact pages → omitted from pdfium extract / caption / chart crops.
//! 3. Keep pages → subset PDF (lopdf delete) → decode → remap page numbers.
//! 4. Prune leftovers with MediaBox-aware classify.
//!
//! pdf2md has no page filter: it calls `get_processed_image` on every page.
//! Document-level skip is wrong when only one page is a storm (NT_DRM page 6).

use std::collections::HashMap;
use std::path::Path;

use tracing::{info, warn};

use crate::embedded_images::{figures_by_page, write_decoded_figure_pngs, WrittenFigureAsset};
use crate::error::PdfConversionError;
use crate::figure_keep::{
    classify_embedded_figure, page_artifact_kind, DEFAULT_PAGE_HEIGHT_PT, DEFAULT_PAGE_WIDTH_PT,
};
use crate::pdf_subset::subset_keeping_pages;
use crate::xobject_inventory::XObjectInventory;

/// Result of the gated ImageXObject extract pass (one skip type).
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddedFigureExtract {
    /// 1-indexed pages that must not be decoded / captioned / chart-cropped.
    pub artifact_pages: Vec<usize>,
    /// 1-indexed pages safe to hand to pdfium (may be empty → skip all crops).
    pub keep_pages: Vec<usize>,
    /// Surviving figure PNGs (original page numbers).
    pub written: Vec<WrittenFigureAsset>,
}

impl EmbeddedFigureExtract {
    /// Fail-closed: no keep pages → skip extract, captions, and charts.
    pub fn fail_closed() -> Self {
        Self {
            artifact_pages: Vec::new(),
            keep_pages: Vec::new(),
            written: Vec::new(),
        }
    }

    /// Inventory-only plan (no decode). Used when charts need omit flags
    /// without writing figures.
    pub fn from_plan(artifact_pages: Vec<usize>, keep_pages: Vec<usize>) -> Self {
        Self {
            artifact_pages,
            keep_pages,
            written: Vec::new(),
        }
    }

    pub fn figure_map(&self) -> HashMap<usize, Vec<WrittenFigureAsset>> {
        figures_by_page(&self.written)
    }

    /// Omit this page from fig/caption/chart crops.
    pub fn omit_page(&self, page: usize, page_as_unit: bool) -> bool {
        page_as_unit || self.artifact_pages.contains(&page)
    }

    /// Manuscript page-as-unit **or** no keep pages: skip all region crops.
    pub fn skip_all_region_crops(&self, page_as_unit: bool) -> bool {
        page_as_unit || self.keep_pages.is_empty()
    }
}

/// Manuscript page-as-unit **or** empty keep_pages: no fig/caption/chart crops.
pub fn skip_all_region_crops(page_as_unit: bool, extract: &EmbeddedFigureExtract) -> bool {
    extract.skip_all_region_crops(page_as_unit)
}

/// Inventory only (fail-closed on error). No pdfium decode.
pub async fn inventory_figure_pages(
    pdf_bytes: &[u8],
    page_filter: Option<&[usize]>,
) -> Result<EmbeddedFigureExtract, PdfConversionError> {
    let bytes = pdf_bytes.to_vec();
    let filter = page_filter.map(|p| p.to_vec());
    tokio::task::spawn_blocking(move || inventory_figure_pages_blocking(&bytes, filter.as_deref()))
        .await
        .map_err(|e| PdfConversionError::Backend(format!("figure inventory task panicked: {e}")))?
}

fn inventory_figure_pages_blocking(
    pdf_bytes: &[u8],
    page_filter: Option<&[usize]>,
) -> Result<EmbeddedFigureExtract, PdfConversionError> {
    let inv = match XObjectInventory::from_pdf_bytes(pdf_bytes) {
        Ok(inv) => inv,
        Err(e) => {
            warn!(
                error = %e,
                "xobject inventory failed — fail-closed (skip figure/caption/chart crops)"
            );
            return Ok(EmbeddedFigureExtract::fail_closed());
        }
    };
    Ok(plan_from_inventory(&inv, page_filter))
}

fn plan_from_inventory(
    inv: &XObjectInventory,
    page_filter: Option<&[usize]>,
) -> EmbeddedFigureExtract {
    let artifact_pages = inv.artifact_pages();
    let mut keep = inv.keep_pages();
    if let Some(filter) = page_filter {
        keep.retain(|p| filter.contains(p));
    }
    info!(
        artifact_pages = ?artifact_pages,
        keep_pages = ?keep,
        total_xobjects = inv.total_images(),
        keep_native = inv.keep_native_count(),
        "figure page plan from ImageXObject inventory"
    );
    EmbeddedFigureExtract::from_plan(artifact_pages, keep)
}

/// Async facade used by Vision convert and include-pdf-assets.
pub async fn extract_embedded_figures(
    pdf_bytes: &[u8],
    assets_root: &Path,
    page_filter: Option<&[usize]>,
) -> Result<EmbeddedFigureExtract, PdfConversionError> {
    let bytes = pdf_bytes.to_vec();
    let root = assets_root.to_path_buf();
    let filter = page_filter.map(|p| p.to_vec());
    tokio::task::spawn_blocking(move || {
        extract_embedded_figures_blocking(&bytes, &root, filter.as_deref())
    })
    .await
    .map_err(|e| PdfConversionError::Backend(format!("figure extract task panicked: {e}")))?
}

/// Backward-compatible writer: surviving figure PNGs only.
pub async fn write_embedded_figure_assets(
    pdf_bytes: &[u8],
    assets_root: &Path,
    page_filter: Option<&[usize]>,
) -> Result<Vec<WrittenFigureAsset>, PdfConversionError> {
    Ok(
        extract_embedded_figures(pdf_bytes, assets_root, page_filter)
            .await?
            .written,
    )
}

pub(crate) fn extract_embedded_figures_blocking(
    pdf_bytes: &[u8],
    assets_root: &Path,
    page_filter: Option<&[usize]>,
) -> Result<EmbeddedFigureExtract, PdfConversionError> {
    let mut plan = inventory_figure_pages_blocking(pdf_bytes, page_filter)?;
    if plan.keep_pages.is_empty() {
        info!(
            artifact_pages = ?plan.artifact_pages,
            "no keep pages — Pass-A page raster is the analysis unit"
        );
        return Ok(plan);
    }

    let (decode_bytes, remap) = if plan.artifact_pages.is_empty() {
        (pdf_bytes.to_vec(), plan.keep_pages.clone())
    } else {
        match subset_keeping_pages(pdf_bytes, &plan.keep_pages) {
            Ok(pair) => pair,
            Err(e) => {
                warn!(
                    error = %e,
                    "PDF subset failed — fail-closed (skip figure decode)"
                );
                return Ok(EmbeddedFigureExtract::fail_closed());
            }
        }
    };

    let written = write_decoded_figure_pngs(&decode_bytes, assets_root, Some(&remap))?;
    let media = page_media_dims(pdf_bytes);
    let mut map = figures_by_page(&written);
    prune_artifact_figures_with_media(&mut map, &media);
    let mut surviving: Vec<WrittenFigureAsset> = map.into_values().flatten().collect();
    surviving.sort_by_key(|f| (f.page_num, f.index));
    plan.written = surviving;
    Ok(plan)
}

fn page_media_dims(pdf_bytes: &[u8]) -> HashMap<usize, (f32, f32)> {
    edgequake_pdf2md::extract_page_media_boxes_from_bytes(pdf_bytes, None)
        .unwrap_or_default()
        .into_iter()
        .map(|b| (b.page_num, (b.width_pt, b.height_pt)))
        .collect()
}

/// Drop encoding-artifact pages and hairline leftovers from a figure map.
///
/// Page-level policy is [`page_artifact_kind`] (same SSOT as inventory).
/// Per-object leftover uses [`classify_embedded_figure`] with MediaBox when known.
pub fn prune_artifact_figures(figure_map: &mut HashMap<usize, Vec<WrittenFigureAsset>>) {
    prune_artifact_figures_with_media(figure_map, &HashMap::new());
}

/// Like [`prune_artifact_figures`] with per-page MediaBox `(width_pt, height_pt)`.
pub fn prune_artifact_figures_with_media(
    figure_map: &mut HashMap<usize, Vec<WrittenFigureAsset>>,
    media: &HashMap<usize, (f32, f32)>,
) {
    let mut suppressed = 0usize;
    for (page, figs) in figure_map.iter_mut() {
        let natives: Vec<(u32, u32)> = figs.iter().map(|f| (f.width, f.height)).collect();
        let areas: Vec<f64> = figs
            .iter()
            .map(crate::embedded_images::display_area_pt2)
            .collect();
        if page_artifact_kind(&natives, &areas).is_some() {
            suppressed += figs.len();
            figs.clear();
            info!(
                page_num = page,
                "encoding-artifact page — fragment links suppressed"
            );
            continue;
        }
        let (pw, ph) = media
            .get(page)
            .copied()
            .unwrap_or((DEFAULT_PAGE_WIDTH_PT, DEFAULT_PAGE_HEIGHT_PT));
        let before = figs.len();
        figs.retain(|fig| {
            classify_embedded_figure(fig.width, fig.height, fig.bbox, pw, ph).is_keep()
        });
        suppressed += before.saturating_sub(figs.len());
    }
    if suppressed > 0 {
        info!(
            suppressed,
            "encoding-artifact fragments excluded from markdown/analyze"
        );
    }
}

/// Caption regions on keep pages only (subset → remap). Empty keep → no work.
pub async fn write_caption_region_assets_for_keep_pages(
    pdf_bytes: &[u8],
    assets_root: &Path,
    existing_figures_by_page: &HashMap<usize, Vec<WrittenFigureAsset>>,
    keep_pages: &[usize],
) -> Result<
    (
        Vec<WrittenFigureAsset>,
        Vec<crate::region_assets::WrittenTableAsset>,
    ),
    PdfConversionError,
> {
    if keep_pages.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let bytes = pdf_bytes.to_vec();
    let root = assets_root.to_path_buf();
    let existing = existing_figures_by_page.clone();
    let keep = keep_pages.to_vec();
    tokio::task::spawn_blocking(move || {
        write_captions_keep_blocking(&bytes, &root, &existing, &keep)
    })
    .await
    .map_err(|e| PdfConversionError::Backend(format!("caption keep task panicked: {e}")))?
}

fn write_captions_keep_blocking(
    pdf_bytes: &[u8],
    assets_root: &Path,
    existing: &HashMap<usize, Vec<WrittenFigureAsset>>,
    keep_pages: &[usize],
) -> Result<
    (
        Vec<WrittenFigureAsset>,
        Vec<crate::region_assets::WrittenTableAsset>,
    ),
    PdfConversionError,
> {
    let (decode_bytes, remap) = subset_keeping_pages(pdf_bytes, keep_pages)?;
    // Remap existing figure keys to subset page numbers for IoU dedup.
    let mut existing_subset: HashMap<usize, Vec<WrittenFigureAsset>> = HashMap::new();
    for (i, orig_page) in remap.iter().enumerate() {
        let subset_page = i + 1;
        if let Some(figs) = existing.get(orig_page) {
            let mut list = figs.clone();
            for f in &mut list {
                f.page_num = subset_page;
            }
            existing_subset.insert(subset_page, list);
        }
    }
    crate::region_assets::write_caption_region_assets_blocking(
        &decode_bytes,
        assets_root,
        &existing_subset,
        Some(&remap),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fig(
        page: usize,
        index: usize,
        w: u32,
        h: u32,
        bbox: Option<(f32, f32, f32, f32)>,
    ) -> WrittenFigureAsset {
        WrittenFigureAsset {
            page_num: page,
            index,
            rel_path: format!("assets/page-{page:04}-fig-{index:02}.png"),
            width: w,
            height: h,
            bbox,
        }
    }

    #[test]
    fn prune_drops_table_rule_storm_keeps_real_logo() {
        let mut map = HashMap::new();
        map.insert(
            6,
            (0..60)
                .map(|i| fig(6, i + 1, 5, 1, Some((10.0, 400.0, 40.0, 400.4))))
                .collect(),
        );
        map.insert(
            1,
            vec![fig(1, 1, 289, 99, Some((40.0, 720.0, 180.0, 780.0)))],
        );
        prune_artifact_figures(&mut map);
        assert!(map.get(&6).is_some_and(|v| v.is_empty()));
        assert_eq!(map.get(&1).map(|v| v.len()), Some(1));
    }

    #[test]
    fn prune_drops_ocr_tiles_on_print() {
        let mut map = HashMap::new();
        map.insert(
            1,
            (0..24)
                .map(|i| {
                    fig(
                        1,
                        i + 1,
                        24,
                        12,
                        Some((10.0, 20.0 + i as f32, 22.0, 26.0 + i as f32)),
                    )
                })
                .collect(),
        );
        prune_artifact_figures(&mut map);
        assert!(
            map.get(&1).is_some_and(|v| v.is_empty()),
            "Print OCR chips must not become Pass-B figures"
        );
    }

    #[test]
    fn skip_all_region_crops_when_empty_keep_or_page_as_unit() {
        let empty = EmbeddedFigureExtract::fail_closed();
        assert!(skip_all_region_crops(false, &empty));
        assert!(!empty.omit_page(1, false)); // omit is per-artifact; empty keep uses skip_all
        let plan = EmbeddedFigureExtract::from_plan(vec![6], vec![1, 2, 3]);
        assert!(!skip_all_region_crops(false, &plan));
        assert!(plan.omit_page(6, false));
        assert!(!plan.omit_page(1, false));
        assert!(skip_all_region_crops(true, &plan));
        assert!(plan.omit_page(1, true));
    }
}
