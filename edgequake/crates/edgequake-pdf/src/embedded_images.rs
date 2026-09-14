//! Persist embedded PDF ImageXObjects as figure-bounded PNG assets.
//!
//! First principle (SPEC-047): VLM figure/chart/illustration analysis must be
//! bounded to the **image object** in the PDF. Full-page renders are for the
//! markdown viewer only; analyze `<drawing/>` paths prefer these assets.
//!
//! DRY: decoding uses [`edgequake_pdf2md::extract_embedded_images_from_bytes`]
//! (shared Pdfium singleton). This module only names files and writes PNGs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use image::ImageFormat;
use tracing::{debug, warn};

use crate::drawing_tags::{page_figure_asset_filename, ASSETS_SUBDIR};
use crate::error::PdfConversionError;

/// Written figure asset under `{assets_root}/assets/page-NNNN-fig-MM.png`.
#[derive(Debug, Clone, PartialEq)]
pub struct WrittenFigureAsset {
    pub page_num: usize,
    pub index: usize,
    pub rel_path: String,
    pub width: u32,
    pub height: u32,
    /// PDF-space bbox when known (enables IoU dedup with region crops).
    pub bbox: Option<(f32, f32, f32, f32)>,
}

/// Decode ImageXObjects and write PNGs that pass [`crate::figure_keep::keep_native_pixels`].
///
/// `page_remap`: when decoding a subset PDF, `page_remap[subset_page - 1]` is the
/// original page number used for filenames. `None` = identity.
///
/// No inventory — callers that need skip-before-decode use
/// [`crate::figure_extract::extract_embedded_figures`].
pub(crate) fn write_decoded_figure_pngs(
    pdf_bytes: &[u8],
    assets_root: &Path,
    page_remap: Option<&[usize]>,
) -> Result<Vec<WrittenFigureAsset>, PdfConversionError> {
    let extracted = edgequake_pdf2md::extract_embedded_images_from_bytes(pdf_bytes, None)
        .map_err(|e| PdfConversionError::Backend(format!("embedded figure extract: {e}")))?;

    let assets_dir = assets_root.join(ASSETS_SUBDIR);
    std::fs::create_dir_all(&assets_dir).map_err(|e| {
        PdfConversionError::Backend(format!("create assets dir {assets_dir:?}: {e}"))
    })?;

    let mut written = Vec::new();
    for fig in extracted {
        let page_num = match page_remap {
            Some(r) => r
                .get(fig.page_num.saturating_sub(1))
                .copied()
                .unwrap_or(fig.page_num),
            None => fig.page_num,
        };
        if !crate::figure_keep::keep_native_pixels(fig.width, fig.height) {
            debug!(
                page_num,
                index = fig.index,
                width = fig.width,
                height = fig.height,
                "skipping encoding-artifact ImageXObject (not a VLM figure)"
            );
            continue;
        }
        let filename = page_figure_asset_filename(page_num, fig.index);
        let full_path: PathBuf = assets_dir.join(&filename);
        if let Err(e) = fig.image.save_with_format(&full_path, ImageFormat::Png) {
            warn!(
                page_num,
                index = fig.index,
                path = %full_path.display(),
                error = %e,
                "Failed to write embedded figure PNG"
            );
            continue;
        }
        let rel_path = format!("{ASSETS_SUBDIR}/{filename}");
        debug!(
            page_num,
            index = fig.index,
            width = fig.width,
            height = fig.height,
            path = %rel_path,
            "Wrote embedded figure asset"
        );
        written.push(WrittenFigureAsset {
            page_num,
            index: fig.index,
            rel_path,
            width: fig.width,
            height: fig.height,
            bbox: Some(fig.bbox),
        });
    }
    Ok(written)
}

/// Group written figures by 1-indexed page number.
pub fn figures_by_page(written: &[WrittenFigureAsset]) -> HashMap<usize, Vec<WrittenFigureAsset>> {
    let mut map: HashMap<usize, Vec<WrittenFigureAsset>> = HashMap::new();
    for fig in written {
        map.entry(fig.page_num).or_default().push(fig.clone());
    }
    for list in map.values_mut() {
        list.sort_by_key(|f| f.index);
    }
    map
}

/// Displayed area in pt² from the PDF-space bbox; falls back to pixel area
/// when the bbox is unknown or degenerate.
pub(crate) fn display_area_pt2(fig: &WrittenFigureAsset) -> f64 {
    fig.bbox
        .map(|b| ((b.2 - b.0).abs() as f64) * ((b.3 - b.1).abs() as f64))
        .filter(|a| *a > 0.0)
        .unwrap_or((fig.width as f64) * (fig.height as f64))
}

/// True when a page's embedded images are a **sliced scan** — many small
/// tiles whose union reconstructs the page — rather than standalone figures.
///
/// First principle (LAW-134-1): on a tiled page the page render is the
/// semantic unit; each tile is an encoding artifact with no standalone
/// meaning. Narrating or linking tiles is crop theater by construction.
///
/// The rule is purely geometric and modality-agnostic (count + median
/// fragment size); the modality gate is the caller's policy decision.
pub fn is_scan_tiling_page(figs: &[WrittenFigureAsset]) -> bool {
    let areas: Vec<f64> = figs.iter().map(display_area_pt2).collect();
    crate::figure_keep::is_fragment_area_tiling(&areas)
}

#[cfg(test)]
mod tiling_tests {
    use super::*;

    fn fig(index: usize, w: u32, h: u32, bbox: Option<(f32, f32, f32, f32)>) -> WrittenFigureAsset {
        WrittenFigureAsset {
            page_num: 1,
            index,
            rel_path: format!("assets/page-0001-fig-{index:02}.png"),
            width: w,
            height: h,
            bbox,
        }
    }

    /// Measured on the assessment manuscript (2026-08-20): 43 tiles on page 3,
    /// median displayed area 354 pt².
    #[test]
    fn tiled_scan_page_is_detected() {
        let figs: Vec<_> = (0..43)
            .map(|i| fig(i, 100, 60, Some((0.0, 0.0, 20.0, 18.0))))
            .collect();
        assert!(is_scan_tiling_page(&figs));
    }

    #[test]
    fn few_real_figures_are_not_tiling() {
        let figs = vec![
            fig(1, 800, 600, Some((36.0, 200.0, 300.0, 400.0))),
            fig(2, 640, 480, Some((36.0, 450.0, 280.0, 620.0))),
        ];
        assert!(!is_scan_tiling_page(&figs));
    }

    #[test]
    fn many_large_figures_are_not_tiling() {
        // A genuine image gallery: 15 substantial figures must survive.
        let figs: Vec<_> = (0..15)
            .map(|i| fig(i, 800, 600, Some((0.0, 0.0, 200.0, 150.0))))
            .collect();
        assert!(!is_scan_tiling_page(&figs));
    }

    #[test]
    fn pixel_area_fallback_when_bbox_missing() {
        let figs: Vec<_> = (0..20).map(|i| fig(i, 45, 40, None)).collect();
        assert!(is_scan_tiling_page(&figs));
    }

    #[test]
    fn empty_and_small_sets_are_not_tiling() {
        assert!(!is_scan_tiling_page(&[]));
        let below_min_count: Vec<_> = (0..11).map(|i| fig(i, 10, 10, None)).collect();
        assert!(!is_scan_tiling_page(&below_min_count));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn sample_pdf() -> Vec<u8> {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data/embedded_figure_sample.pdf");
        std::fs::read(&path).unwrap_or_else(|e| panic!("read sample pdf {path:?}: {e}"))
    }

    #[test]
    #[serial]
    fn writes_fig_asset_filename_ssot() {
        let dir = tempfile::tempdir().unwrap();
        let written = match crate::embedded_images::write_decoded_figure_pngs(
            &sample_pdf(),
            dir.path(),
            None,
        ) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("pdfium unavailable: {e}");
                return;
            }
        };
        assert!(!written.is_empty());
        assert!(
            written[0].rel_path.contains("-fig-"),
            "analyze assets must use fig path, got {}",
            written[0].rel_path
        );
        assert!(
            written[0].width <= 80 && written[0].height <= 80,
            "must be object-sized, got {}x{}",
            written[0].width,
            written[0].height
        );
        assert!(dir.path().join(&written[0].rel_path).is_file());
    }
}
