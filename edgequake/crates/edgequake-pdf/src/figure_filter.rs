//! Two-pass VLM figure filter for SPEC-049 (SOLID / DRY).
//!
//! ## Design
//!
//! The geometry layer (L0/L1 pdfium) is a **conservative proposer**:
//! it intentionally keeps more crops than needed so it never misses a real
//! figure.  Geometry alone cannot distinguish a vector bar-chart from a
//! decorated text-box (both are rectangular path fills).
//!
//! This module is the **semantic oracle**: it runs two cheap VLM passes on
//! each proposed PNG to decide whether it is a real figure and, if so, what
//! specialised description to emit:
//!
//! ```text
//! Geometry proposes PNGs
//!        │
//!        ▼
//!  Pass 1 — FILTER  (is_figure: bool, kind: FigureKind)
//!        │           discard: logo, text_block, icon, decorative_rule
//!        ▼ (kept only)
//!  Pass 2 — SPECIALIZE  (kind-aware structured prompt → Markdown description)
//!        │               chart → data-table, diagram → component list, …
//!        ▼
//!  FigureFilterResult  (written to figure_filter_manifest.json)
//!        │
//!        ▼
//!  prune figure_map + chart crops; inject Pass-2 Markdown into the page body
//! ```
//!
//! ## SOLID
//!
//! - **S**: Filter only — does not write PNG files (that is `region_assets`).
//! - **O**: Default-on when a vision LLM is attached; `EDGEQUAKE_FIGURE_FILTER=0` forces off.
//! - **L**: Any `LLMProvider` implementation works identically.
//! - **I**: Depends on `LLMProvider` trait only, not on the full VisionConversionConfig.
//! - **D**: `FigureFilter` is injected with a provider; does not create one.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use base64::Engine as _;
use edgequake_llm::{
    resolve_effective_temperature, ChatMessage, CompletionOptions, ImageData, LLMProvider,
};
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::embedded_images::WrittenFigureAsset;

use crate::error::PdfConversionError;
use crate::vision_prompts::{
    figure_filter_pass1_prompt, figure_filter_pass2_prompt, FIGURE_FILTER_PASS1_SYSTEM,
    FIGURE_FILTER_PASS2_SYSTEM,
};

// ── Public types ──────────────────────────────────────────────────────────────

/// Semantic classification of one proposed figure crop (Pass 1 output).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FigureKind {
    BarChart,
    LineChart,
    ScatterPlot,
    Heatmap,
    Histogram,
    PieChart,
    RadarChart,
    ArchitectureDiagram,
    Flowchart,
    Diagram,
    Illustration,
    Photograph,
    SystemDemo,
    TableVisual,
    /// Not a figure — discard.
    Logo,
    IconLogo,
    TextBlock,
    DecorativeRule,
    Empty,
    /// SPEC-128 discard taxonomy.
    Stamp,
    Signature,
    ScanArtefact,
    Watermark,
    Other,
}

impl FigureKind {
    /// True when this kind carries unique visual signal worth keeping.
    pub fn is_figure(&self) -> bool {
        !matches!(
            self,
            Self::Logo
                | Self::IconLogo
                | Self::TextBlock
                | Self::DecorativeRule
                | Self::Empty
                | Self::Stamp
                | Self::Signature
                | Self::ScanArtefact
                | Self::Watermark
        )
    }

    /// Parse from a raw string returned by the VLM.
    fn from_str_fuzzy(s: &str) -> Self {
        match s.to_ascii_lowercase().replace([' ', '-'], "_").as_str() {
            "bar_chart" => Self::BarChart,
            "line_chart" => Self::LineChart,
            "scatter_plot" => Self::ScatterPlot,
            "heatmap" => Self::Heatmap,
            "histogram" => Self::Histogram,
            "pie_chart" => Self::PieChart,
            "radar_chart" => Self::RadarChart,
            "architecture_diagram" | "architecture" => Self::ArchitectureDiagram,
            "flowchart" | "flow_chart" => Self::Flowchart,
            "diagram" => Self::Diagram,
            "illustration" => Self::Illustration,
            "photograph" | "photo" => Self::Photograph,
            "system_demo" => Self::SystemDemo,
            "table_visual" | "table" => Self::TableVisual,
            "logo" => Self::Logo,
            "icon_logo" | "icon" => Self::IconLogo,
            "text_block" | "text" => Self::TextBlock,
            "decorative_rule" | "decorative" => Self::DecorativeRule,
            "empty" => Self::Empty,
            "stamp" => Self::Stamp,
            "signature" => Self::Signature,
            "scan_artefact" | "scan_artifact" => Self::ScanArtefact,
            "watermark" => Self::Watermark,
            _ => Self::Other,
        }
    }
}

/// Input reference to one candidate figure PNG.
#[derive(Debug, Clone)]
pub struct FigureCandidate {
    /// Relative path (e.g. `assets/page-0001-fig-01.png`).
    pub rel_path: String,
    /// Absolute path to the PNG on disk.
    pub full_path: std::path::PathBuf,
    /// 1-indexed page number.
    pub page_num: usize,
    /// Caption label from geometry (`Figure 1`, `Figure`, …).
    pub label: String,
    /// Pixel area for VLM budget (lowest-area dropped first). SPEC-128 WP-4.
    pub area_px: u64,
}

/// Result for one candidate after both passes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FigureFilterResult {
    pub rel_path: String,
    pub page_num: usize,
    pub label: String,
    /// Pass-1 classification.
    pub kind: FigureKind,
    /// True when kind carries visual signal and Pass 2 ran.
    pub is_figure: bool,
    /// Pass-2 description (empty when `is_figure` is false).
    pub description: String,
}

/// Written sidecar file under `{assets_root}/figure_filter_manifest.json`.
pub const FIGURE_FILTER_MANIFEST: &str = "figure_filter_manifest.json";

// ── FigureFilter ─────────────────────────────────────────────────────────────

/// Two-pass VLM figure filter.
///
/// Construct with [`FigureFilter::new`], then call [`FigureFilter::run`].
pub struct FigureFilter {
    provider: Arc<dyn LLMProvider>,
    concurrency: usize,
    max_per_page: usize,
}

/// Default Pass-1 in-flight cap (SPEC-128 WP-4).
pub const DEFAULT_FIGURE_FILTER_CONCURRENCY: usize = 4;
/// Soft cap of VLM calls per page; lowest-area crops dropped first.
pub const DEFAULT_MAX_FIGURE_VLM_PER_PAGE: usize = 12;

/// `EDGEQUAKE_FIGURE_FILTER=0|false|off|no` disables; default **on**.
pub fn figure_filter_env_enabled() -> bool {
    match std::env::var("EDGEQUAKE_FIGURE_FILTER") {
        Ok(v) => {
            let t = v.trim().to_ascii_lowercase();
            !matches!(t.as_str(), "0" | "false" | "off" | "no")
        }
        Err(_) => true,
    }
}

impl FigureFilter {
    /// Create a new filter backed by `provider` (default concurrency/budget).
    pub fn new(provider: Arc<dyn LLMProvider>) -> Self {
        Self::with_limits(
            provider,
            DEFAULT_FIGURE_FILTER_CONCURRENCY,
            DEFAULT_MAX_FIGURE_VLM_PER_PAGE,
        )
    }

    pub fn with_limits(
        provider: Arc<dyn LLMProvider>,
        concurrency: usize,
        max_per_page: usize,
    ) -> Self {
        Self {
            provider,
            concurrency: concurrency.max(1),
            max_per_page: max_per_page.max(1),
        }
    }

    /// Run both passes on every (budgeted) candidate.  Noise kinds skip Pass 2
    /// but still appear in the result list with `is_figure = false`.
    pub async fn run(
        &self,
        candidates: &[FigureCandidate],
    ) -> Result<Vec<FigureFilterResult>, PdfConversionError> {
        let selected = budget_candidates(candidates, self.max_per_page);
        let conc = self.concurrency;
        let provider = Arc::clone(&self.provider);
        let mut results: Vec<FigureFilterResult> = stream::iter(selected)
            .map(|c| {
                let provider = Arc::clone(&provider);
                async move { process_one(provider, c).await }
            })
            .buffer_unordered(conc)
            .collect()
            .await;
        results.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        Ok(results)
    }
}

async fn process_one(provider: Arc<dyn LLMProvider>, c: FigureCandidate) -> FigureFilterResult {
    let (kind, is_figure) = match pass1_classify(provider.as_ref(), &c.full_path).await {
        Ok(d) => (d.kind, d.is_figure),
        Err(e) => {
            warn!(rel_path = %c.rel_path, error = %e, "Pass-1 filter failed; keeping crop");
            (FigureKind::Other, true)
        }
    };
    debug!(rel_path = %c.rel_path, kind = ?kind, is_figure, "Pass-1 result");

    if !is_figure {
        return FigureFilterResult {
            rel_path: c.rel_path,
            page_num: c.page_num,
            label: c.label,
            kind,
            is_figure: false,
            description: String::new(),
        };
    }

    let description = match pass2_describe(provider.as_ref(), &c.full_path, &kind).await {
        Ok(d) => d,
        Err(e) => {
            warn!(rel_path = %c.rel_path, error = %e, "Pass-2 specialize failed");
            String::new()
        }
    };
    debug!(rel_path = %c.rel_path, desc_chars = description.len(), "Pass-2 result");

    FigureFilterResult {
        rel_path: c.rel_path,
        page_num: c.page_num,
        label: c.label,
        kind,
        is_figure: true,
        description,
    }
}

pub(crate) fn budget_candidates(
    candidates: &[FigureCandidate],
    max_per_page: usize,
) -> Vec<FigureCandidate> {
    let mut by_page: HashMap<usize, Vec<FigureCandidate>> = HashMap::new();
    for c in candidates {
        by_page.entry(c.page_num).or_default().push(c.clone());
    }
    let mut out = Vec::new();
    for mut list in by_page.into_values() {
        list.sort_by_key(|b| std::cmp::Reverse(b.area_px));
        list.truncate(max_per_page);
        out.extend(list);
    }
    out
}

// ── SPEC-134: Graphic-as-unit Pass-B suppression ──────────────────────────────

/// Crop descriptor for suppression decision (no file I/O).
#[derive(Debug, Clone, Copy)]
pub struct CropDescriptor {
    /// Fraction of page area covered by this crop (0.0–1.0).
    pub area_frac: f32,
    /// Fraction of dark pixels in crop (ink density proxy).
    pub ink_frac: f32,
    /// Aspect ratio width/height (e.g. 0.1 for tall thin strip).
    pub aspect_ratio: f32,
    /// Whether this crop is a child of a larger chart band (IoU > 0.5 with parent,
    /// area < 0.35 of parent).
    pub is_chart_fragment: bool,
}

/// Default noise area threshold (SPEC-134 WP-4).
pub const DEFAULT_SUPPRESS_AREA_FRAC: f32 = 0.008;

/// Default ink density threshold (SPEC-134 WP-4).
pub const DEFAULT_SUPPRESS_INK_FRAC: f32 = 0.01;

/// Shared Pass-B area/aspect (Print + manuscript). Constants from [`figure_keep`].
fn crop_fails_shared_geometry(crop: &CropDescriptor) -> bool {
    crop.area_frac < DEFAULT_SUPPRESS_AREA_FRAC
        || crate::figure_keep::crop_aspect_is_needle(crop.aspect_ratio)
}

/// Manuscript-only extras: empty ink, tighter tick-strip, chart children.
fn manuscript_extra_suppress(crop: &CropDescriptor) -> bool {
    crop.ink_frac < DEFAULT_SUPPRESS_INK_FRAC
        || crop.aspect_ratio < 0.15
        || crop.aspect_ratio > 6.0
        || crop.is_chart_fragment
}

/// Decide whether to suppress Pass-B specialize for a crop on manuscript pages.
///
/// First principles: a hand-drawn chart is a single semantic unit. Axis ticks,
/// single bars, and scribbles are fragments — analyzing them separately is
/// "crop theater" that wastes VLM calls and pollutes the markdown.
///
/// Print callers must use [`should_suppress_crop_for_analyze`] — this returns
/// `false` for Print so legacy SPEC-134 contracts stay stable.
pub fn should_suppress_crop_manuscript(
    modality: crate::page_modality::PageModality,
    crop: &CropDescriptor,
) -> bool {
    modality.is_manuscript_like() && should_suppress_crop_for_analyze(modality, crop)
}

/// Pass-B keep/drop for **any** page modality.
///
/// Print HTML-to-PDF table rules fail the same area/aspect gates as manuscript
/// tick strips. The extra manuscript ink/fragment rules stay MS-only.
pub fn should_suppress_crop_for_analyze(
    modality: crate::page_modality::PageModality,
    crop: &CropDescriptor,
) -> bool {
    if crop_fails_shared_geometry(crop) {
        return true;
    }
    modality.is_manuscript_like() && manuscript_extra_suppress(crop)
}

// ── SPEC-134: real crop geometry for the suppression gate ────────────────────

/// Per-document cache of page-level geometry shared across crops.
///
/// Page PNG headers are probed once per page (not once per crop) so a
/// 40-figure page costs one page-file read, not forty.
#[derive(Debug, Default)]
pub struct CropGeometryCache {
    page_dims: HashMap<usize, Option<(u32, u32)>>,
    chart_area_px: HashMap<usize, Option<u64>>,
}

/// Fail-open descriptor used whenever real geometry is unavailable — mirrors
/// the historical placeholder values so a missing asset never over-suppresses.
const FALLBACK_CROP: CropDescriptor = CropDescriptor {
    area_frac: 0.05,
    ink_frac: 0.05,
    aspect_ratio: 1.0,
    is_chart_fragment: false,
};

/// A fig crop covering less than this fraction of its page's chart-crop area is
/// treated as a fragment of that chart (SPEC-134 parent-area threshold).
const CHART_FRAGMENT_AREA_RATIO: f32 = 0.35;

/// Build a [`CropDescriptor`] from real asset geometry (SPEC-134 WP-4).
///
/// Resolution order (each signal fails open to `FALLBACK_CROP` values):
///
/// - **bytes**: inline data-URI bytes, else read `{asset_base_dir}/{asset_path}`.
/// - **aspect_ratio / crop area**: header-only dimension probe of the crop PNG.
/// - **ink_frac**: dark-pixel fraction of the (downscaled) crop.
/// - **area_frac**: crop pixel area ÷ full-page render (`page-NNNN.png`) area.
/// - **is_chart_fragment**: `-fig-` crop whose area < 35% of the same page's
///   `-chart.png` ink-crop area (the chart band is the graphic-as-unit parent).
pub fn crop_descriptor_from_asset(
    cache: &mut CropGeometryCache,
    asset_base_dir: Option<&Path>,
    asset_path: Option<&str>,
    inline_bytes: &[u8],
) -> CropDescriptor {
    let bytes_owned;
    let bytes: &[u8] = if !inline_bytes.is_empty() {
        inline_bytes
    } else {
        let Some(base) = asset_base_dir else {
            return FALLBACK_CROP;
        };
        let Some(rel) = asset_path else {
            return FALLBACK_CROP;
        };
        match std::fs::read(base.join(rel)) {
            Ok(b) => {
                bytes_owned = b;
                &bytes_owned
            }
            Err(_) => return FALLBACK_CROP,
        }
    };

    let Some((w, h)) = crate::chart_crop::image_dimensions_from_bytes(bytes) else {
        return FALLBACK_CROP;
    };
    if w == 0 || h == 0 {
        return FALLBACK_CROP;
    }

    let aspect_ratio = w as f32 / h as f32;
    let ink_frac =
        crate::chart_crop::ink_fraction_from_bytes(bytes).unwrap_or(FALLBACK_CROP.ink_frac);
    let crop_area = u64::from(w) * u64::from(h);

    let Some(page_num) = asset_path.and_then(crate::drawing_tags::page_num_from_asset_rel_path)
    else {
        // No page provenance (data-URI ref): real aspect/ink, fallback area.
        return CropDescriptor {
            aspect_ratio,
            ink_frac,
            ..FALLBACK_CROP
        };
    };
    let page_num = page_num as usize;

    let area_frac = match page_dims(cache, asset_base_dir, page_num) {
        Some((pw, ph)) if pw > 0 && ph > 0 => {
            let page_area = u64::from(pw) * u64::from(ph);
            (crop_area as f32 / page_area as f32).min(1.0)
        }
        _ => FALLBACK_CROP.area_frac,
    };

    let is_chart_fragment = is_fig_crop(asset_path)
        && match chart_area_px(cache, asset_base_dir, page_num) {
            Some(chart_area) => (crop_area as f32) < CHART_FRAGMENT_AREA_RATIO * chart_area as f32,
            None => false,
        };

    CropDescriptor {
        area_frac,
        ink_frac,
        aspect_ratio,
        is_chart_fragment,
    }
}

fn is_fig_crop(asset_path: Option<&str>) -> bool {
    asset_path
        .map(|p| p.rsplit('/').next().unwrap_or(p).contains("-fig-"))
        .unwrap_or(false)
}

fn page_dims(
    cache: &mut CropGeometryCache,
    asset_base_dir: Option<&Path>,
    page_num: usize,
) -> Option<(u32, u32)> {
    if let Some(cached) = cache.page_dims.get(&page_num) {
        return *cached;
    }
    let probed = probe_asset_dimensions(
        asset_base_dir,
        &crate::drawing_tags::page_asset_filename(page_num),
    );
    cache.page_dims.insert(page_num, probed);
    probed
}

fn chart_area_px(
    cache: &mut CropGeometryCache,
    asset_base_dir: Option<&Path>,
    page_num: usize,
) -> Option<u64> {
    if let Some(cached) = cache.chart_area_px.get(&page_num) {
        return *cached;
    }
    let area = probe_asset_dimensions(
        asset_base_dir,
        &crate::drawing_tags::page_chart_crop_filename(page_num),
    )
    .map(|(w, h)| u64::from(w) * u64::from(h));
    cache.chart_area_px.insert(page_num, area);
    area
}

fn probe_asset_dimensions(asset_base_dir: Option<&Path>, filename: &str) -> Option<(u32, u32)> {
    let path = asset_base_dir?
        .join(crate::drawing_tags::ASSETS_SUBDIR)
        .join(filename);
    let bytes = std::fs::read(path).ok()?;
    crate::chart_crop::image_dimensions_from_bytes(&bytes)
}

#[cfg(test)]
mod suppress_tests {
    use super::*;
    use crate::page_modality::PageModality;

    #[test]
    fn print_modality_never_suppresses_via_manuscript_gate() {
        let crop = CropDescriptor {
            area_frac: 0.001,
            ink_frac: 0.001,
            aspect_ratio: 0.05,
            is_chart_fragment: true,
        };
        assert!(!should_suppress_crop_manuscript(PageModality::Print, &crop));
    }

    #[test]
    fn print_table_hairline_suppressed_for_analyze() {
        let crop = CropDescriptor {
            area_frac: 0.001,
            ink_frac: 0.5,
            aspect_ratio: 40.0,
            is_chart_fragment: false,
        };
        assert!(should_suppress_crop_for_analyze(PageModality::Print, &crop));
    }

    #[test]
    fn tiny_area_suppressed() {
        let crop = CropDescriptor {
            area_frac: 0.005,
            ink_frac: 0.05,
            aspect_ratio: 1.0,
            is_chart_fragment: false,
        };
        assert!(should_suppress_crop_manuscript(
            PageModality::Manuscript,
            &crop
        ));
    }

    #[test]
    fn low_ink_suppressed() {
        let crop = CropDescriptor {
            area_frac: 0.05,
            ink_frac: 0.005,
            aspect_ratio: 1.0,
            is_chart_fragment: false,
        };
        assert!(should_suppress_crop_manuscript(
            PageModality::Manuscript,
            &crop
        ));
    }

    #[test]
    fn tick_strip_suppressed() {
        let crop = CropDescriptor {
            area_frac: 0.05,
            ink_frac: 0.05,
            aspect_ratio: 0.1, // tall thin
            is_chart_fragment: false,
        };
        assert!(should_suppress_crop_manuscript(
            PageModality::Manuscript,
            &crop
        ));
    }

    #[test]
    fn chart_fragment_suppressed() {
        let crop = CropDescriptor {
            area_frac: 0.05,
            ink_frac: 0.05,
            aspect_ratio: 1.0,
            is_chart_fragment: true,
        };
        assert!(should_suppress_crop_manuscript(
            PageModality::Manuscript,
            &crop
        ));
    }

    #[test]
    fn real_chart_not_suppressed() {
        let crop = CropDescriptor {
            area_frac: 0.15,
            ink_frac: 0.08,
            aspect_ratio: 1.5,
            is_chart_fragment: false,
        };
        assert!(!should_suppress_crop_manuscript(
            PageModality::Manuscript,
            &crop
        ));
    }

    #[test]
    fn mixed_modality_suppresses_like_manuscript() {
        let crop = CropDescriptor {
            area_frac: 0.005,
            ink_frac: 0.05,
            aspect_ratio: 1.0,
            is_chart_fragment: false,
        };
        assert!(should_suppress_crop_manuscript(PageModality::Mixed, &crop));
    }
}

#[cfg(test)]
mod geometry_tests {
    use super::*;
    use crate::page_modality::PageModality;
    use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};

    /// Solid-color PNG of `w`×`h`; `dark` pixels count as ink.
    fn solid_png(w: u32, h: u32, dark: bool) -> Vec<u8> {
        let px = if dark {
            Rgba([0, 0, 0, 255])
        } else {
            Rgba([255, 255, 255, 255])
        };
        let img: RgbaImage = ImageBuffer::from_pixel(w, h, px);
        crate::chart_crop::encode_png(&DynamicImage::ImageRgba8(img)).unwrap()
    }

    /// Assets root with a 1000×2000 full-page render for page 1.
    fn assets_root_with_page() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        std::fs::write(
            assets.join(crate::drawing_tags::page_asset_filename(1)),
            solid_png(1000, 2000, false),
        )
        .unwrap();
        dir
    }

    fn descriptor(
        root: &std::path::Path,
        cache: &mut CropGeometryCache,
        rel: &str,
        png: &[u8],
    ) -> CropDescriptor {
        std::fs::write(root.join(rel), png).unwrap();
        crop_descriptor_from_asset(cache, Some(root), Some(rel), &[])
    }

    #[test]
    fn tick_strip_real_geometry_suppressed() {
        let root = assets_root_with_page();
        let mut cache = CropGeometryCache::default();
        // 40×900 tall-thin strip: area 0.018 (above area gate) but aspect 0.044.
        let crop = descriptor(
            root.path(),
            &mut cache,
            "assets/page-0001-fig-01.png",
            &solid_png(40, 900, true),
        );
        assert!(crop.aspect_ratio < 0.15, "aspect={}", crop.aspect_ratio);
        assert!(
            (crop.area_frac - 0.018).abs() < 0.005,
            "area_frac={}",
            crop.area_frac
        );
        assert!(should_suppress_crop_manuscript(
            PageModality::Manuscript,
            &crop
        ));
    }

    #[test]
    fn tiny_crop_real_area_suppressed() {
        let root = assets_root_with_page();
        let mut cache = CropGeometryCache::default();
        // 30×30 on a 1000×2000 page → area 0.00045 < 0.008.
        let crop = descriptor(
            root.path(),
            &mut cache,
            "assets/page-0001-fig-01.png",
            &solid_png(30, 30, true),
        );
        assert!(crop.area_frac < DEFAULT_SUPPRESS_AREA_FRAC);
        assert!(should_suppress_crop_manuscript(
            PageModality::Manuscript,
            &crop
        ));
    }

    #[test]
    fn whole_graphic_real_geometry_kept() {
        let root = assets_root_with_page();
        let mut cache = CropGeometryCache::default();
        // 800×1000 inked graphic: area 0.4, aspect 0.8, ink 1.0 — all gates pass.
        let crop = descriptor(
            root.path(),
            &mut cache,
            "assets/page-0001-fig-01.png",
            &solid_png(800, 1000, true),
        );
        assert!(!should_suppress_crop_manuscript(
            PageModality::Manuscript,
            &crop
        ));
    }

    #[test]
    fn fig_fragment_of_chart_band_suppressed() {
        let root = assets_root_with_page();
        let mut cache = CropGeometryCache::default();
        // Chart band 800×800 (640k px); fig 200×200 (40k < 35% × 640k) with
        // otherwise-passing geometry — only the fragment gate can fire.
        std::fs::write(
            root.path()
                .join("assets")
                .join(crate::drawing_tags::page_chart_crop_filename(1)),
            solid_png(800, 800, true),
        )
        .unwrap();
        let crop = descriptor(
            root.path(),
            &mut cache,
            "assets/page-0001-fig-02.png",
            &solid_png(200, 200, true),
        );
        assert!(crop.is_chart_fragment, "crop={crop:?}");
        assert!(should_suppress_crop_manuscript(
            PageModality::Manuscript,
            &crop
        ));
    }

    #[test]
    fn fig_covering_most_of_chart_band_kept() {
        let root = assets_root_with_page();
        let mut cache = CropGeometryCache::default();
        // Fig 700×700 (490k ≥ 35% × 640k) is the graphic-as-unit, not a fragment.
        std::fs::write(
            root.path()
                .join("assets")
                .join(crate::drawing_tags::page_chart_crop_filename(1)),
            solid_png(800, 800, true),
        )
        .unwrap();
        let crop = descriptor(
            root.path(),
            &mut cache,
            "assets/page-0001-fig-01.png",
            &solid_png(700, 700, true),
        );
        assert!(!crop.is_chart_fragment, "crop={crop:?}");
        assert!(!should_suppress_crop_manuscript(
            PageModality::Manuscript,
            &crop
        ));
    }

    #[test]
    fn missing_assets_fail_open() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = CropGeometryCache::default();
        // Nothing on disk: fallback descriptor must not trigger any gate.
        let crop = crop_descriptor_from_asset(
            &mut cache,
            Some(dir.path()),
            Some("assets/page-0001-fig-01.png"),
            &[],
        );
        assert!(!should_suppress_crop_manuscript(
            PageModality::Manuscript,
            &crop
        ));
    }

    #[test]
    fn inline_bytes_without_path_use_real_aspect_and_ink() {
        let mut cache = CropGeometryCache::default();
        // Data-URI ref: no asset path → area falls back, aspect/ink are real.
        let crop = crop_descriptor_from_asset(&mut cache, None, None, &solid_png(20, 500, true));
        assert!(crop.aspect_ratio < 0.15);
        assert!((crop.area_frac - FALLBACK_CROP.area_frac).abs() < f32::EPSILON);
        assert!(should_suppress_crop_manuscript(
            PageModality::Manuscript,
            &crop
        ));
    }

    #[test]
    fn page_dims_probed_once_per_page() {
        let root = assets_root_with_page();
        let mut cache = CropGeometryCache::default();
        let a = descriptor(
            root.path(),
            &mut cache,
            "assets/page-0001-fig-01.png",
            &solid_png(100, 100, true),
        );
        let b = descriptor(
            root.path(),
            &mut cache,
            "assets/page-0001-fig-02.png",
            &solid_png(100, 100, true),
        );
        assert_eq!(cache.page_dims.len(), 1);
        assert!((a.area_frac - b.area_frac).abs() < f32::EPSILON);
    }
}

struct Pass1Decision {
    kind: FigureKind,
    is_figure: bool,
}

async fn pass1_classify(
    provider: &dyn LLMProvider,
    png_path: &Path,
) -> Result<Pass1Decision, PdfConversionError> {
    let image = load_image_data(png_path)?;
    let opts = CompletionOptions {
        max_tokens: Some(80),
        temperature: resolve_effective_temperature(provider.model(), 0.0),
        ..Default::default()
    }
    .with_role_cache("figure-filter", provider);
    let messages = vec![
        ChatMessage::system(FIGURE_FILTER_PASS1_SYSTEM),
        ChatMessage::user_with_images(figure_filter_pass1_prompt(), vec![image]),
    ];
    let response = provider
        .chat(&messages, Some(&opts))
        .await
        .map_err(|e| PdfConversionError::Backend(format!("Pass-1 LLM call failed: {e}")))?;
    parse_pass1_json(response.content.trim())
}

async fn pass2_describe(
    provider: &dyn LLMProvider,
    png_path: &Path,
    kind: &FigureKind,
) -> Result<String, PdfConversionError> {
    let image = load_image_data(png_path)?;
    let opts = CompletionOptions {
        max_tokens: Some(600),
        temperature: resolve_effective_temperature(provider.model(), 0.0),
        ..Default::default()
    }
    .with_role_cache("figure-filter", provider);
    let messages = vec![
        ChatMessage::system(FIGURE_FILTER_PASS2_SYSTEM),
        ChatMessage::user_with_images(figure_filter_pass2_prompt(kind), vec![image]),
    ];
    let response = provider
        .chat(&messages, Some(&opts))
        .await
        .map_err(|e| PdfConversionError::Backend(format!("Pass-2 LLM call failed: {e}")))?;
    Ok(response.content.trim().to_string())
}

/// Rebuild `figure_map` to kept paths only (G-prune). Paths not in `results` are
/// kept (fail-open for crops the filter never saw). Empty pages are dropped.
pub fn prune_figure_map(
    figure_map: HashMap<usize, Vec<WrittenFigureAsset>>,
    results: &[FigureFilterResult],
) -> HashMap<usize, Vec<WrittenFigureAsset>> {
    let seen: HashSet<&str> = results.iter().map(|r| r.rel_path.as_str()).collect();
    let kept: HashSet<&str> = results
        .iter()
        .filter(|r| r.is_figure)
        .map(|r| r.rel_path.as_str())
        .collect();
    let mut out: HashMap<usize, Vec<WrittenFigureAsset>> = HashMap::new();
    for (page, figs) in figure_map {
        let filtered: Vec<WrittenFigureAsset> = figs
            .into_iter()
            .filter(|f| {
                if !seen.contains(f.rel_path.as_str()) {
                    true
                } else {
                    kept.contains(f.rel_path.as_str())
                }
            })
            .collect();
        if !filtered.is_empty() {
            out.insert(page, filtered);
        }
    }
    out
}

/// Delete discarded PNG files under `assets_root`. Best-effort.
pub fn delete_discarded_pngs(assets_root: &Path, results: &[FigureFilterResult]) {
    for r in results.iter().filter(|r| !r.is_figure) {
        let path = assets_root.join(&r.rel_path);
        if path.is_file() {
            if let Err(e) = std::fs::remove_file(&path) {
                warn!(path = %path.display(), error = %e, "failed to delete discarded figure PNG");
            }
        }
    }
}

/// Apply prune + optional PNG delete. Logs kept/discarded by kind.
pub fn apply_filter_to_figure_map(
    figure_map: HashMap<usize, Vec<WrittenFigureAsset>>,
    results: &[FigureFilterResult],
    assets_root: &Path,
    delete_discarded: bool,
) -> HashMap<usize, Vec<WrittenFigureAsset>> {
    let kept_n = results.iter().filter(|r| r.is_figure).count();
    let mut discarded_by_kind: HashMap<String, usize> = HashMap::new();
    for r in results.iter().filter(|r| !r.is_figure) {
        *discarded_by_kind
            .entry(format!("{:?}", r.kind))
            .or_insert(0) += 1;
    }
    info!(
        figure_filter_kept = kept_n,
        discarded = results.len().saturating_sub(kept_n),
        discarded_by_kind = ?discarded_by_kind,
        total = results.len(),
        "SPEC-128 figure filter prune"
    );
    if delete_discarded {
        delete_discarded_pngs(assets_root, results);
    }
    prune_figure_map(figure_map, results)
}

/// Fail-open: on filter `Err`, return the original map unchanged (LAW-128-13).
pub fn apply_filter_result_or_keep(
    figure_map: HashMap<usize, Vec<WrittenFigureAsset>>,
    run: Result<Vec<FigureFilterResult>, PdfConversionError>,
    assets_root: &Path,
    delete_discarded: bool,
) -> HashMap<usize, Vec<WrittenFigureAsset>> {
    match run {
        Ok(results) => {
            apply_filter_to_figure_map(figure_map, &results, assets_root, delete_discarded)
        }
        Err(e) => {
            warn!(error = %e, "figure filter failed; keeping all crops (fail-open)");
            figure_map
        }
    }
}

/// Apply an on-disk `figure_filter_manifest.json` if present (include-from-pdf).
pub fn prune_figure_map_using_manifest(
    figure_map: HashMap<usize, Vec<WrittenFigureAsset>>,
    assets_root: &Path,
    delete_discarded: bool,
) -> HashMap<usize, Vec<WrittenFigureAsset>> {
    let manifest = load_manifest(assets_root);
    if manifest.is_empty() {
        figure_map
    } else {
        apply_filter_to_figure_map(figure_map, &manifest, assets_root, delete_discarded)
    }
}

/// Collect unique figure + chart crops for Pass-1/Pass-2 (LAW-128-13).
pub fn collect_filter_candidates(
    assets_root: &Path,
    figure_map: &HashMap<usize, Vec<WrittenFigureAsset>>,
    chart_paths: &HashMap<usize, String>,
) -> Vec<FigureCandidate> {
    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for fig in figure_map.values().flatten() {
        if !seen.insert(fig.rel_path.clone()) {
            continue;
        }
        out.push(FigureCandidate {
            rel_path: fig.rel_path.clone(),
            full_path: assets_root.join(&fig.rel_path),
            page_num: fig.page_num,
            label: String::new(),
            area_px: u64::from(fig.width)
                .saturating_mul(u64::from(fig.height))
                .max(1),
        });
    }
    for (page, rel) in chart_paths {
        if !seen.insert(rel.clone()) {
            continue;
        }
        let full_path = assets_root.join(rel);
        out.push(FigureCandidate {
            rel_path: rel.clone(),
            full_path: full_path.clone(),
            page_num: *page,
            label: "Chart".into(),
            area_px: png_area_px(&full_path),
        });
    }
    out
}

/// Drop discarded chart crops from the assemble override map (fail-open for unseen paths).
pub fn prune_chart_crop_paths(
    chart_paths: HashMap<usize, String>,
    results: &[FigureFilterResult],
) -> HashMap<usize, String> {
    if results.is_empty() {
        return chart_paths;
    }
    let seen: HashSet<&str> = results.iter().map(|r| r.rel_path.as_str()).collect();
    let kept: HashSet<&str> = results
        .iter()
        .filter(|r| r.is_figure)
        .map(|r| r.rel_path.as_str())
        .collect();
    chart_paths
        .into_iter()
        .filter(|(_, rel)| {
            if !seen.contains(rel.as_str()) {
                true
            } else {
                kept.contains(rel.as_str())
            }
        })
        .collect()
}

/// Rel-paths Pass-1 classified as artefacts.
pub fn discarded_rel_paths(results: &[FigureFilterResult]) -> HashSet<String> {
    results
        .iter()
        .filter(|r| !r.is_figure)
        .map(|r| r.rel_path.clone())
        .collect()
}

/// Rel-paths discarded by an on-disk manifest (empty when the filter never ran).
pub fn discarded_rel_paths_from_manifest(assets_root: &Path) -> HashSet<String> {
    discarded_rel_paths(&load_manifest(assets_root))
}

/// Inject Pass-2 specialised Markdown after each kept crop's image href.
///
/// Artefacts are never injected. Already-injected paths are skipped (idempotent).
pub fn inject_kept_descriptions(markdown: &str, results: &[FigureFilterResult]) -> String {
    let mut out = markdown.to_string();
    for r in results
        .iter()
        .filter(|r| r.is_figure && !r.description.trim().is_empty())
    {
        let marker = format!("<!-- edgequake-figure-vision:{} -->", r.rel_path);
        if out.contains(&marker) {
            continue;
        }
        let needle = format!("]({})", r.rel_path);
        let Some(idx) = out.find(&needle) else {
            continue;
        };
        let after = idx + needle.len();
        let line_end = out[after..]
            .find('\n')
            .map(|n| after + n)
            .unwrap_or(out.len());
        let block = format!("\n\n{}\n{}\n", marker, r.description.trim());
        out.insert_str(line_end, &block);
    }
    out
}

/// Drop markdown image lines that point at discarded artefact crops.
pub fn strip_discarded_asset_lines(markdown: &str, discarded: &HashSet<String>) -> String {
    if discarded.is_empty() {
        return markdown.to_string();
    }
    let mut out = String::with_capacity(markdown.len());
    for line in markdown.split_inclusive('\n') {
        let trimmed = line.trim();
        let drop = discarded.iter().any(|rel| {
            trimmed.contains(&format!("]({rel})")) || trimmed.contains(&format!("path=\"{rel}\""))
        });
        if !drop {
            out.push_str(line);
        }
    }
    out
}

// ── Manifest I/O ─────────────────────────────────────────────────────────────

/// Write `figure_filter_manifest.json` under `assets_root`.
pub fn write_manifest(
    assets_root: &Path,
    results: &[FigureFilterResult],
) -> Result<(), PdfConversionError> {
    let path = assets_root.join(FIGURE_FILTER_MANIFEST);
    let json = serde_json::to_string_pretty(results).map_err(|e| {
        PdfConversionError::Backend(format!("serialize figure filter manifest: {e}"))
    })?;
    std::fs::write(&path, json).map_err(|e| {
        PdfConversionError::Backend(format!("write figure filter manifest {path:?}: {e}"))
    })?;
    Ok(())
}

/// Load the manifest if it exists; return empty vec otherwise.
pub fn load_manifest(assets_root: &Path) -> Vec<FigureFilterResult> {
    let path = assets_root.join(FIGURE_FILTER_MANIFEST);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Encode a PNG file as `ImageData` for the LLM provider.
fn load_image_data(png_path: &Path) -> Result<ImageData, PdfConversionError> {
    let bytes = std::fs::read(png_path).map_err(|e| {
        PdfConversionError::Backend(format!("read PNG {}: {e}", png_path.display()))
    })?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(ImageData::new(b64, "image/png"))
}

/// Parse `{"kind": "...", "is_figure": bool}` from Pass-1 VLM response.
/// Tolerates fenced JSON and missing `is_figure` key (derives from kind).
/// Explicit `is_figure: false` discards even when kind is `other`.
fn parse_pass1_json(raw: &str) -> Result<Pass1Decision, PdfConversionError> {
    let cleaned = if let Some(start) = raw.find('{') {
        &raw[start..]
    } else {
        raw
    };
    let cleaned = if let Some(end) = cleaned.rfind('}') {
        &cleaned[..=end]
    } else {
        cleaned
    };

    let v: serde_json::Value = serde_json::from_str(cleaned)
        .map_err(|e| PdfConversionError::Backend(format!("Pass-1 JSON parse: {e} — raw={raw}")))?;

    let kind_str = v.get("kind").and_then(|k| k.as_str()).unwrap_or("other");
    let kind = FigureKind::from_str_fuzzy(kind_str);
    let json_flag = v.get("is_figure").and_then(|x| x.as_bool());
    let is_figure = kind.is_figure() && json_flag.unwrap_or(true);

    Ok(Pass1Decision { kind, is_figure })
}

fn png_area_px(path: &Path) -> u64 {
    let Ok(bytes) = std::fs::read(path) else {
        return 1;
    };
    if bytes.len() < 24 || &bytes[0..4] != b"\x89PNG" {
        return 1;
    }
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as u64;
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]) as u64;
    w.saturating_mul(h).max(1)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_classification() {
        assert!(FigureKind::BarChart.is_figure());
        assert!(FigureKind::ArchitectureDiagram.is_figure());
        assert!(FigureKind::SystemDemo.is_figure());
        assert!(!FigureKind::Logo.is_figure());
        assert!(!FigureKind::TextBlock.is_figure());
        assert!(!FigureKind::DecorativeRule.is_figure());
        assert!(!FigureKind::Stamp.is_figure());
        assert!(!FigureKind::Watermark.is_figure());
        assert!(!FigureKind::Signature.is_figure());
        // Other is kept (conservative)
        assert!(FigureKind::Other.is_figure());
    }

    #[test]
    fn prune_figure_map_drops_discarded_only() {
        let mut map = HashMap::new();
        let fig = |rel: &str, page: usize| WrittenFigureAsset {
            page_num: page,
            index: 1,
            rel_path: rel.into(),
            width: 10,
            height: 10,
            bbox: Some((0.0, 0.0, 10.0, 10.0)),
        };
        map.insert(
            1usize,
            vec![
                fig("assets/a.png", 1),
                fig("assets/b.png", 1),
                fig("assets/c.png", 1),
            ],
        );
        map.insert(2usize, vec![fig("assets/d.png", 2), fig("assets/e.png", 2)]);
        let results = vec![
            FigureFilterResult {
                rel_path: "assets/a.png".into(),
                page_num: 1,
                label: String::new(),
                kind: FigureKind::BarChart,
                is_figure: true,
                description: "ok".into(),
            },
            FigureFilterResult {
                rel_path: "assets/b.png".into(),
                page_num: 1,
                label: String::new(),
                kind: FigureKind::Logo,
                is_figure: false,
                description: String::new(),
            },
            FigureFilterResult {
                rel_path: "assets/c.png".into(),
                page_num: 1,
                label: String::new(),
                kind: FigureKind::Diagram,
                is_figure: true,
                description: "ok".into(),
            },
            FigureFilterResult {
                rel_path: "assets/d.png".into(),
                page_num: 2,
                label: String::new(),
                kind: FigureKind::Stamp,
                is_figure: false,
                description: String::new(),
            },
            FigureFilterResult {
                rel_path: "assets/e.png".into(),
                page_num: 2,
                label: String::new(),
                kind: FigureKind::Photograph,
                is_figure: true,
                description: "ok".into(),
            },
        ];
        let pruned = prune_figure_map(map, &results);
        let paths: Vec<String> = pruned
            .values()
            .flatten()
            .map(|f| f.rel_path.clone())
            .collect();
        assert_eq!(paths.len(), 3);
        assert!(paths.contains(&"assets/a.png".into()));
        assert!(paths.contains(&"assets/c.png".into()));
        assert!(paths.contains(&"assets/e.png".into()));
        assert!(!paths.contains(&"assets/b.png".into()));
        assert!(!paths.contains(&"assets/d.png".into()));
    }

    #[test]
    #[serial_test::serial]
    fn figure_filter_env_default_on() {
        std::env::remove_var("EDGEQUAKE_FIGURE_FILTER");
        assert!(figure_filter_env_enabled());
        std::env::set_var("EDGEQUAKE_FIGURE_FILTER", "0");
        assert!(!figure_filter_env_enabled());
        std::env::remove_var("EDGEQUAKE_FIGURE_FILTER");
    }

    #[test]
    fn parse_pass1_json_ok() {
        let raw = r#"{"kind": "bar_chart", "is_figure": true, "confidence": 0.99}"#;
        let d = parse_pass1_json(raw).unwrap();
        assert_eq!(d.kind, FigureKind::BarChart);
        assert!(d.is_figure);
    }

    #[test]
    fn parse_pass1_json_strips_fences() {
        let raw = "```json\n{\"kind\": \"text_block\", \"is_figure\": false}\n```";
        let d = parse_pass1_json(raw).unwrap();
        assert_eq!(d.kind, FigureKind::TextBlock);
        assert!(!d.is_figure);
    }

    #[test]
    fn parse_pass1_json_unknown_kind() {
        let raw = r#"{"kind": "unknown_thing"}"#;
        let d = parse_pass1_json(raw).unwrap();
        assert_eq!(d.kind, FigureKind::Other);
        assert!(d.is_figure, "missing is_figure on other → fail-open keep");
    }

    #[test]
    fn parse_pass1_other_explicit_false_discards() {
        let d = parse_pass1_json(r#"{"kind":"other","is_figure":false}"#).unwrap();
        assert_eq!(d.kind, FigureKind::Other);
        assert!(!d.is_figure);
    }

    #[test]
    fn parse_pass1_logo_true_still_discards() {
        let d = parse_pass1_json(r#"{"kind":"logo","is_figure":true}"#).unwrap();
        assert_eq!(d.kind, FigureKind::Logo);
        assert!(!d.is_figure);
    }

    #[test]
    fn kind_fuzzy_parsing() {
        assert_eq!(
            FigureKind::from_str_fuzzy("architecture"),
            FigureKind::ArchitectureDiagram
        );
        assert_eq!(
            FigureKind::from_str_fuzzy("flow chart"),
            FigureKind::Flowchart
        );
        assert_eq!(FigureKind::from_str_fuzzy("LOGO"), FigureKind::Logo);
    }

    #[test]
    fn filter_error_fail_open_keeps_all() {
        let mut map = HashMap::new();
        map.insert(
            1usize,
            vec![WrittenFigureAsset {
                page_num: 1,
                index: 1,
                rel_path: "assets/keep.png".into(),
                width: 10,
                height: 10,
                bbox: Some((0.0, 0.0, 10.0, 10.0)),
            }],
        );
        let dir = tempfile::tempdir().unwrap();
        let out = apply_filter_result_or_keep(
            map.clone(),
            Err(PdfConversionError::Backend("vlm down".into())),
            dir.path(),
            false,
        );
        assert_eq!(out.get(&1).map(|v| v.len()), Some(1));
    }

    #[test]
    fn manifest_on_disk_prunes_include_from_pdf() {
        let dir = tempfile::tempdir().unwrap();
        let results = vec![
            FigureFilterResult {
                rel_path: "assets/keep.png".into(),
                page_num: 1,
                label: String::new(),
                kind: FigureKind::Diagram,
                is_figure: true,
                description: "ok".into(),
            },
            FigureFilterResult {
                rel_path: "assets/drop.png".into(),
                page_num: 1,
                label: String::new(),
                kind: FigureKind::Logo,
                is_figure: false,
                description: String::new(),
            },
        ];
        write_manifest(dir.path(), &results).unwrap();
        let mut map = HashMap::new();
        let fig = |rel: &str| WrittenFigureAsset {
            page_num: 1,
            index: 1,
            rel_path: rel.into(),
            width: 10,
            height: 10,
            bbox: Some((0.0, 0.0, 10.0, 10.0)),
        };
        map.insert(1usize, vec![fig("assets/keep.png"), fig("assets/drop.png")]);
        let pruned = prune_figure_map_using_manifest(map, dir.path(), false);
        let paths: Vec<_> = pruned
            .values()
            .flatten()
            .map(|f| f.rel_path.as_str())
            .collect();
        assert_eq!(paths, vec!["assets/keep.png"]);
    }

    #[test]
    fn budget_drops_lowest_area_first() {
        let cands: Vec<FigureCandidate> = (0..3)
            .map(|i| FigureCandidate {
                rel_path: format!("assets/{i}.png"),
                full_path: std::path::PathBuf::from("x"),
                page_num: 1,
                label: String::new(),
                area_px: (i as u64 + 1) * 10,
            })
            .collect();
        let kept = budget_candidates(&cands, 2);
        assert_eq!(kept.len(), 2);
        let areas: Vec<u64> = kept.iter().map(|c| c.area_px).collect();
        assert!(areas.contains(&20));
        assert!(areas.contains(&30));
        assert!(!areas.contains(&10));
    }

    #[test]
    fn prune_chart_crop_paths_drops_artefacts() {
        let mut charts = HashMap::new();
        charts.insert(1usize, "assets/page-0001-chart.png".into());
        charts.insert(2usize, "assets/page-0002-chart.png".into());
        charts.insert(3usize, "assets/page-0003-chart.png".into());
        let results = vec![
            FigureFilterResult {
                rel_path: "assets/page-0001-chart.png".into(),
                page_num: 1,
                label: "Chart".into(),
                kind: FigureKind::BarChart,
                is_figure: true,
                description: "bars".into(),
            },
            FigureFilterResult {
                rel_path: "assets/page-0002-chart.png".into(),
                page_num: 2,
                label: "Chart".into(),
                kind: FigureKind::Logo,
                is_figure: false,
                description: String::new(),
            },
        ];
        let pruned = prune_chart_crop_paths(charts, &results);
        assert_eq!(
            pruned.get(&1).map(String::as_str),
            Some("assets/page-0001-chart.png")
        );
        assert!(!pruned.contains_key(&2));
        assert_eq!(
            pruned.get(&3).map(String::as_str),
            Some("assets/page-0003-chart.png"),
            "unseen chart paths fail-open"
        );
    }

    #[test]
    fn inject_kept_descriptions_promotes_charts_not_logos() {
        let md = "![keep](assets/chart.png)\n\n![logo](assets/logo.png)\n";
        let results = vec![
            FigureFilterResult {
                rel_path: "assets/chart.png".into(),
                page_num: 1,
                label: String::new(),
                kind: FigureKind::LineChart,
                is_figure: true,
                description: "Revenue rose from 10 to 20.".into(),
            },
            FigureFilterResult {
                rel_path: "assets/logo.png".into(),
                page_num: 1,
                label: String::new(),
                kind: FigureKind::Logo,
                is_figure: false,
                description: "should not appear".into(),
            },
        ];
        let out = inject_kept_descriptions(md, &results);
        assert!(out.contains("Revenue rose from 10 to 20."));
        assert!(out.contains("<!-- edgequake-figure-vision:assets/chart.png -->"));
        assert!(!out.contains("should not appear"));
        let again = inject_kept_descriptions(&out, &results);
        assert_eq!(
            again.matches("Revenue rose from 10 to 20.").count(),
            1,
            "inject is idempotent"
        );
    }

    #[test]
    fn strip_discarded_asset_lines_drops_logo_href() {
        let md = "![keep](assets/fig.png)\n![logo](assets/logo.png)\n";
        let mut discarded = HashSet::new();
        discarded.insert("assets/logo.png".into());
        let out = strip_discarded_asset_lines(md, &discarded);
        assert!(out.contains("assets/fig.png"));
        assert!(!out.contains("assets/logo.png"));
    }
}
