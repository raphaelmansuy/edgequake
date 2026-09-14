//! Geometry-first keep/drop for Vision figure analysis (no VLM, no EdgeParse).
//!
//! First principles (LAW-049 / LAW-128 / LAW-134-1 generalized to Print):
//!
//! * A PDF has no native `Figure` type. Image XObjects are paint. Most of them
//!   are **encoding artifacts** (HTML table hairlines, 1×1 spacers, scan tiles).
//! * A visual is worth a VLM call **iff** it can carry meaning the page raster
//!   does not already give. Hairline XObjects fail that test geometrically.
//! * Decide **before decode / write / Pass-B**. Asking a VLM “is this 5×1 px
//!   image a figure?” is crop theater.
//!
//! ```text
//!  Image XObject
//!    ├─ native Width×Height   (dict, no pixel decode)
//!    └─ displayed bbox        (CTM unit square)
//!           │
//!           ▼
//!  classify_embedded_figure → Keep | Drop*
//!           │
//!           ▼
//!  page_artifact_kind (N≥50 Dos  OR  ≥12 small-median tiles)
//!             → skip pdfium extract; Pass-A raster is SSOT
//! ```

/// Match pdf2md `MIN_FIGURE_EDGE_PX` — bullets / 1 px spacers / table rules.
pub const MIN_FIGURE_EDGE_PX: u32 = 24;
/// LightRAG / Pass-B `VLM_MIN_IMAGE_PIXEL` default (width×height).
pub const MIN_FIGURE_PIXELS: u64 = 64;
/// SPEC-128 image placement floor (logos still pass; hairlines do not).
pub const MIN_IMAGE_AREA_FRAC: f32 = 0.002;
/// SPEC-049 / pdf2md — reject near-full-page dumps as “figures”.
pub const MAX_IMAGE_AREA_FRAC: f32 = 0.55;
/// SPEC-128 `max_figure_aspect` — needle-thin rules / stretched cell borders.
pub const MAX_FIGURE_ASPECT: f32 = 8.0;
/// HTML-to-PDF table pages and OCR-searchable scans emit dozens–thousands of
/// Image XObjects. Below this count we still extract; at/above, skip pdfium
/// decode — that many paints are an encoder, not a figure gallery (SPEC-128
/// `max_figure_vlm_per_page` is 12).
pub const DECODE_STORM_MIN_COUNT: usize = 50;
/// SPEC-134: minimum placements before scan/OCR tiling is considered.
pub const TILE_STORM_MIN_COUNT: usize = 12;
/// Median displayed-area ceiling (pt²) for tiling fragments.
/// Observed OCR-chip / scan-tile medians are tens–hundreds of pt²; a real
/// pasted figure is at least ~1 in² (5,184 pt²).
pub const TILE_STORM_MAX_MEDIAN_AREA_PT2: f64 = 2_000.0;
/// Fallback page size for per-object classify when MediaBox is unknown (A4).
pub const DEFAULT_PAGE_WIDTH_PT: f32 = 595.0;
/// Fallback page height (A4).
pub const DEFAULT_PAGE_HEIGHT_PT: f32 = 842.0;

/// Why an Image XObject must not be a VLM analyze target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FigureDropReason {
    Keep,
    /// Native min edge below [`MIN_FIGURE_EDGE_PX`] (5×1 table rules, 1×1 spacers).
    HairlineNative,
    /// Native pixel count below [`MIN_FIGURE_PIXELS`].
    TinyPixels,
    /// Displayed bbox is a needle (aspect > [`MAX_FIGURE_ASPECT`]).
    ThinAspect,
    /// Displayed area is an ornament, not a figure.
    TinyArea,
    /// Placement covers most of the page — that is Pass-A's unit, not a crop.
    FullPage,
}

impl FigureDropReason {
    pub fn is_keep(self) -> bool {
        matches!(self, Self::Keep)
    }
}

/// Native-pixel gate (no bbox required).
pub fn keep_native_pixels(width: u32, height: u32) -> bool {
    width >= MIN_FIGURE_EDGE_PX
        && height >= MIN_FIGURE_EDGE_PX
        && u64::from(width).saturating_mul(u64::from(height)) >= MIN_FIGURE_PIXELS
}

/// Classify one embedded image. `bbox` is PDF-space `(left, bottom, right, top)`.
pub fn classify_embedded_figure(
    native_width: u32,
    native_height: u32,
    bbox: Option<(f32, f32, f32, f32)>,
    page_width_pt: f32,
    page_height_pt: f32,
) -> FigureDropReason {
    if native_width > 0 && native_height > 0 && !keep_native_pixels(native_width, native_height) {
        return if native_width < MIN_FIGURE_EDGE_PX || native_height < MIN_FIGURE_EDGE_PX {
            FigureDropReason::HairlineNative
        } else {
            FigureDropReason::TinyPixels
        };
    }

    let Some((x0, y0, x1, y1)) = bbox else {
        return FigureDropReason::Keep;
    };

    let w = (x1 - x0).abs();
    let h = (y1 - y0).abs();
    let page_area = page_width_pt.max(1.0) * page_height_pt.max(1.0);
    let frac = (w * h) / page_area;
    let aspect = w.max(h).max(1.0) / w.min(h).max(1.0);

    if aspect > MAX_FIGURE_ASPECT {
        return FigureDropReason::ThinAspect;
    }
    if frac < MIN_IMAGE_AREA_FRAC {
        return FigureDropReason::TinyArea;
    }
    if frac > MAX_IMAGE_AREA_FRAC {
        return FigureDropReason::FullPage;
    }
    FigureDropReason::Keep
}

/// Why a page's Image XObjects must not be extracted for Pass-B.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageArtifactKind {
    /// ≥[`DECODE_STORM_MIN_COUNT`] Image `Do`s — an encoder, not a figure list.
    DecodeStorm,
    /// ≥[`TILE_STORM_MIN_COUNT`] small-median placements (OCR chips / scan tiles).
    PlacementTiling,
}

/// Count gate: ≥[`DECODE_STORM_MIN_COUNT`] paints cannot be a figure list.
pub fn is_decode_storm(native_sizes: &[(u32, u32)]) -> bool {
    native_sizes.len() >= DECODE_STORM_MIN_COUNT
}

/// Placement area in pt² from an axis-aligned PDF bbox.
pub fn placement_area_pt2(bbox: (f64, f64, f64, f64)) -> f64 {
    let (x0, y0, x1, y1) = bbox;
    (x1 - x0).abs() * (y1 - y0).abs()
}

/// True when placements are a sliced scan / OCR-chip dump (median area).
pub fn is_fragment_area_tiling(areas_pt2: &[f64]) -> bool {
    if areas_pt2.len() < TILE_STORM_MIN_COUNT {
        return false;
    }
    let mut areas = areas_pt2.to_vec();
    areas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    areas[areas.len() / 2] <= TILE_STORM_MAX_MEDIAN_AREA_PT2
}

/// Placement-bbox form of [`is_fragment_area_tiling`].
pub fn is_placement_tiling(bboxes: &[(f64, f64, f64, f64)]) -> bool {
    let areas: Vec<f64> = bboxes.iter().copied().map(placement_area_pt2).collect();
    is_fragment_area_tiling(&areas)
}

/// Width/height of a crop PNG (Pass-B). Needle-thin in either direction.
pub fn crop_aspect_is_needle(width_over_height: f32) -> bool {
    let ar = width_over_height.abs().max(1e-6);
    !((1.0 / MAX_FIGURE_ASPECT)..=MAX_FIGURE_ASPECT).contains(&ar)
}

/// Single page-level keep/drop: storm **or** tiling (`areas_pt2` may use
/// displayed bbox or, after decode, pixel-area fallback).
pub fn page_artifact_kind(
    native_sizes: &[(u32, u32)],
    areas_pt2: &[f64],
) -> Option<PageArtifactKind> {
    if is_decode_storm(native_sizes) {
        Some(PageArtifactKind::DecodeStorm)
    } else if is_fragment_area_tiling(areas_pt2) {
        Some(PageArtifactKind::PlacementTiling)
    } else {
        None
    }
}

/// Skip pdfium ImageXObject extract: decode storm or scan/OCR tiling.
pub fn is_encoding_artifact_page(
    native_sizes: &[(u32, u32)],
    bboxes: &[(f64, f64, f64, f64)],
) -> bool {
    let areas: Vec<f64> = bboxes.iter().copied().map(placement_area_pt2).collect();
    page_artifact_kind(native_sizes, &areas).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const A4_W: f32 = DEFAULT_PAGE_WIDTH_PT;
    const A4_H: f32 = DEFAULT_PAGE_HEIGHT_PT;

    #[test]
    fn nt_drm_table_rule_5x1_is_hairline() {
        assert_eq!(
            classify_embedded_figure(5, 1, Some((40.0, 400.0, 80.0, 400.5)), A4_W, A4_H),
            FigureDropReason::HairlineNative
        );
        assert!(!keep_native_pixels(5, 1));
        assert!(!keep_native_pixels(1, 1));
    }

    #[test]
    fn logo_289x99_is_keep() {
        assert!(keep_native_pixels(289, 99));
        assert_eq!(
            classify_embedded_figure(289, 99, Some((40.0, 720.0, 180.0, 780.0)), A4_W, A4_H),
            FigureDropReason::Keep
        );
    }

    #[test]
    fn stretched_rule_fails_aspect_even_if_native_is_large() {
        assert_eq!(
            classify_embedded_figure(40, 24, Some((50.0, 400.0, 250.0, 404.0)), A4_W, A4_H),
            FigureDropReason::ThinAspect
        );
    }

    #[test]
    fn annex_48x48_icon_is_tiny_area() {
        assert_eq!(
            classify_embedded_figure(48, 48, Some((100.0, 500.0, 118.0, 518.0)), A4_W, A4_H),
            FigureDropReason::TinyArea
        );
    }

    #[test]
    fn full_page_scan_tile_is_not_a_crop() {
        assert_eq!(
            classify_embedded_figure(1240, 1754, Some((0.0, 0.0, 595.0, 842.0)), A4_W, A4_H),
            FigureDropReason::FullPage
        );
    }

    #[test]
    fn decode_storm_nt_drm_page6_shape() {
        let mut sizes: Vec<(u32, u32)> = (0..3079).map(|_| (5, 1)).collect();
        sizes.extend((0..259).map(|_| (1, 1)));
        sizes.push((590, 136));
        assert!(is_decode_storm(&sizes));
        assert_eq!(
            page_artifact_kind(&sizes, &[]),
            Some(PageArtifactKind::DecodeStorm)
        );
        assert!(is_decode_storm(&[(800, 600); 50]));
        assert!(!is_decode_storm(&[(800, 600); 8]));
        assert!(!is_decode_storm(&[(5, 1); 20]));
    }

    #[test]
    fn ocr_chip_24x12_is_hairline() {
        assert!(!keep_native_pixels(24, 12));
        assert_eq!(
            classify_embedded_figure(24, 12, Some((40.0, 400.0, 52.0, 406.0)), A4_W, A4_H),
            FigureDropReason::HairlineNative
        );
    }

    #[test]
    fn ocr_searchable_scan_is_placement_tiling() {
        let mut bboxes: Vec<(f64, f64, f64, f64)> = (0..24)
            .map(|i| (10.0, 20.0 + i as f64, 30.0, 28.0 + i as f64))
            .collect();
        bboxes.push((0.0, 0.0, 595.0, 842.0));
        bboxes.push((10.0, 600.0, 530.0, 820.0));
        bboxes.push((20.0, 100.0, 500.0, 230.0));
        assert!(is_placement_tiling(&bboxes));
        assert!(is_encoding_artifact_page(&[(24, 12); 27], &bboxes));
        assert_eq!(
            page_artifact_kind(
                &[(24, 12); 27],
                &bboxes
                    .iter()
                    .copied()
                    .map(placement_area_pt2)
                    .collect::<Vec<_>>()
            ),
            Some(PageArtifactKind::PlacementTiling)
        );
        assert!(!is_placement_tiling(&[(0.0, 0.0, 200.0, 150.0); 8]));
    }

    #[test]
    fn classify_drops_hairlines_keeps_banner() {
        assert!(
            !classify_embedded_figure(5, 1, Some((10.0, 400.0, 40.0, 400.4)), A4_W, A4_H).is_keep()
        );
        assert!(
            classify_embedded_figure(590, 136, Some((20.0, 700.0, 400.0, 800.0)), A4_W, A4_H)
                .is_keep()
        );
    }

    #[test]
    fn crop_aspect_needle_matches_max_figure_aspect() {
        assert!(crop_aspect_is_needle(40.0));
        assert!(crop_aspect_is_needle(0.05));
        assert!(!crop_aspect_is_needle(1.8));
        assert!(!crop_aspect_is_needle(MAX_FIGURE_ASPECT));
        assert!(crop_aspect_is_needle(MAX_FIGURE_ASPECT + 0.01));
    }
}
