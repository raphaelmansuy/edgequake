//! Caption-anchored / object-cluster region PNG writers (SPEC-047 / SPEC-049).
//!
//! Complements [`crate::embedded_images`]: ImageXObjects cover raster figures;
//! Form XObject / vector diagrams and ruled tables use the SPEC-049 visual
//! cascade (object cluster + caption labels) via pdf2md.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use image::ImageFormat;
use tracing::{debug, warn};

use crate::drawing_tags::{page_figure_asset_filename, page_table_asset_filename, ASSETS_SUBDIR};
use crate::embedded_images::WrittenFigureAsset;
use crate::error::PdfConversionError;

/// Written table region under `{assets_root}/assets/page-NNNN-table-MM.png`.
#[derive(Debug, Clone, PartialEq)]
pub struct WrittenTableAsset {
    pub page_num: usize,
    pub index: usize,
    pub rel_path: String,
    pub width: u32,
    pub height: u32,
    pub label: String,
    /// PDF-space bbox when known (SPEC-128 overlay persist).
    pub bbox: Option<(f32, f32, f32, f32)>,
}

/// Extract caption-anchored regions; write figure gaps + table crops.
///
/// Figures: write Form/vector crops even when the page already has ImageXObject
/// embeds; skip only duplicates (IoU / pure-image clusters). See
/// [`should_write_region_figure`].
pub async fn write_caption_region_assets(
    pdf_bytes: &[u8],
    assets_root: &Path,
    existing_figures_by_page: &HashMap<usize, Vec<WrittenFigureAsset>>,
) -> Result<(Vec<WrittenFigureAsset>, Vec<WrittenTableAsset>), PdfConversionError> {
    let bytes = pdf_bytes.to_vec();
    let root = assets_root.to_path_buf();
    let existing = existing_figures_by_page.clone();
    tokio::task::spawn_blocking(move || {
        write_caption_region_assets_blocking(&bytes, &root, &existing, None)
    })
    .await
    .map_err(|e| PdfConversionError::Backend(format!("region write task panicked: {e}")))?
}

/// Decode caption regions from `pdf_bytes`, writing with optional page remap
/// (`page_remap[subset_page - 1]` → original page for filenames).
pub(crate) fn write_caption_region_assets_blocking(
    pdf_bytes: &[u8],
    assets_root: &Path,
    existing_figures_by_page: &HashMap<usize, Vec<WrittenFigureAsset>>,
    page_remap: Option<&[usize]>,
) -> Result<(Vec<WrittenFigureAsset>, Vec<WrittenTableAsset>), PdfConversionError> {
    let regions = edgequake_pdf2md::extract_caption_regions_from_bytes(pdf_bytes, None)
        .map_err(|e| PdfConversionError::Backend(format!("caption region extract: {e}")))?;

    let assets_dir = assets_root.join(ASSETS_SUBDIR);
    std::fs::create_dir_all(&assets_dir).map_err(|e| {
        PdfConversionError::Backend(format!("create assets dir {assets_dir:?}: {e}"))
    })?;

    let remap_page = |subset: usize| -> usize {
        page_remap
            .and_then(|r| r.get(subset.saturating_sub(1)).copied())
            .unwrap_or(subset)
    };

    // Index counters keyed by **original** page number.
    let mut figures = Vec::new();
    let mut tables = Vec::new();
    let mut fig_index_by_page: HashMap<usize, usize> = HashMap::new();
    for (page, list) in existing_figures_by_page {
        let orig = remap_page(*page);
        fig_index_by_page.insert(orig, list.len());
    }

    for region in regions {
        let page_num = remap_page(region.page_num);
        match region.kind {
            edgequake_pdf2md::RegionKind::Figure => {
                let existing = existing_figures_by_page
                    .get(&region.page_num)
                    .or_else(|| existing_figures_by_page.get(&page_num))
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);
                if !should_write_region_figure(
                    region.bbox,
                    region.has_image,
                    region.has_form,
                    existing,
                ) {
                    continue;
                }
                let next = fig_index_by_page.entry(page_num).or_insert(0);
                *next += 1;
                let index = *next;
                let filename = page_figure_asset_filename(page_num, index);
                let full_path: PathBuf = assets_dir.join(&filename);
                if let Err(e) = region.image.save_with_format(&full_path, ImageFormat::Png) {
                    warn!(
                        page_num,
                        label = %region.label,
                        error = %e,
                        "Failed to write caption figure region"
                    );
                    continue;
                }
                let rel_path = format!("{ASSETS_SUBDIR}/{filename}");
                debug!(
                    page_num,
                    label = %region.label,
                    path = %rel_path,
                    "Wrote caption-anchored figure region"
                );
                figures.push(WrittenFigureAsset {
                    page_num,
                    index,
                    rel_path,
                    width: region.width,
                    height: region.height,
                    bbox: Some(region.bbox),
                });
            }
            edgequake_pdf2md::RegionKind::Table => {
                let filename = page_table_asset_filename(page_num, region.index);
                let full_path: PathBuf = assets_dir.join(&filename);
                if let Err(e) = region.image.save_with_format(&full_path, ImageFormat::Png) {
                    warn!(
                        page_num,
                        label = %region.label,
                        error = %e,
                        "Failed to write caption table region"
                    );
                    continue;
                }
                let rel_path = format!("{ASSETS_SUBDIR}/{filename}");
                debug!(
                    page_num,
                    label = %region.label,
                    path = %rel_path,
                    "Wrote caption-anchored table region"
                );
                tables.push(WrittenTableAsset {
                    page_num,
                    index: region.index,
                    rel_path,
                    width: region.width,
                    height: region.height,
                    label: region.label,
                    bbox: Some(region.bbox),
                });
            }
        }
    }

    Ok((figures, tables))
}

/// P1a — write region figure unless it duplicates an existing embed.
///
/// - IoU ≥ [`edgequake_pdf2md::DEDUP_IOU`] with a known embed bbox → skip
/// - Pure ImageXObject cluster (`has_image && !has_form`) when any embed exists
///   on the page without bboxes → skip (same paint channel)
/// - Form / vector / path crops → write (gap-fill beside embeds)
pub fn should_write_region_figure(
    region_bbox: (f32, f32, f32, f32),
    has_image: bool,
    has_form: bool,
    existing_on_page: &[WrittenFigureAsset],
) -> bool {
    if existing_on_page.is_empty() {
        return true;
    }
    for e in existing_on_page {
        if let Some(eb) = e.bbox {
            if edgequake_pdf2md::iou(region_bbox, eb) >= edgequake_pdf2md::DEDUP_IOU {
                return false;
            }
        }
    }
    // Pure image cluster duplicates the embed writer when bboxes were unavailable.
    if has_image && !has_form {
        let any_bbox = existing_on_page.iter().any(|e| e.bbox.is_some());
        if !any_bbox {
            return false;
        }
    }
    true
}

/// Group table assets by 1-indexed page.
pub fn tables_by_page(written: &[WrittenTableAsset]) -> HashMap<usize, Vec<WrittenTableAsset>> {
    let mut map: HashMap<usize, Vec<WrittenTableAsset>> = HashMap::new();
    for t in written {
        map.entry(t.page_num).or_default().push(t.clone());
    }
    for list in map.values_mut() {
        list.sort_by_key(|t| t.index);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fig(bbox: Option<(f32, f32, f32, f32)>) -> WrittenFigureAsset {
        WrittenFigureAsset {
            page_num: 1,
            index: 1,
            rel_path: "assets/page-0001-fig-01.png".into(),
            width: 10,
            height: 10,
            bbox,
        }
    }

    #[test]
    fn writes_form_beside_embed() {
        let existing = [fig(Some((0.0, 0.0, 50.0, 50.0)))];
        assert!(should_write_region_figure(
            (200.0, 200.0, 300.0, 300.0),
            false,
            true,
            &existing
        ));
    }

    #[test]
    fn skips_iou_duplicate() {
        let existing = [fig(Some((0.0, 0.0, 100.0, 100.0)))];
        assert!(!should_write_region_figure(
            (5.0, 5.0, 95.0, 95.0),
            false,
            true,
            &existing
        ));
    }

    #[test]
    fn skips_pure_image_when_embed_lacks_bbox() {
        let existing = [fig(None)];
        assert!(!should_write_region_figure(
            (0.0, 0.0, 40.0, 40.0),
            true,
            false,
            &existing
        ));
    }

    #[test]
    fn writes_when_no_existing() {
        assert!(should_write_region_figure(
            (0.0, 0.0, 40.0, 40.0),
            false,
            true,
            &[]
        ));
    }
}
