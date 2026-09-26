mod edgeparse;
mod vision;

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::PdfConversionError;

pub use edgeparse::EdgeParsePdfConverter;
pub use vision::VisionPdfConverter;

/// PDF parser backend / config choice.
///
/// `Vision` and `EdgeParse` are runtime converters. `Auto` is a **config-only**
/// choice (SPEC-123): start as Vision intent and allow SPEC-038 EdgeParse
/// fast-path when text density is sufficient.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PdfParserBackend {
    #[default]
    Vision,
    EdgeParse,
    /// Explicit opt-in for SPEC-038 auto-routing (never inferred from unset).
    Auto,
}

/// Provenance of the winning PDF parser choice (LAW-123-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PdfParserResolutionSource {
    Upload,
    Workspace,
    Tenant,
    Env,
    Default,
}

/// Result of Upload → Workspace → Tenant → Env → Vision resolution (SPEC-123).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedPdfParser {
    /// Winning config choice (may be [`PdfParserBackend::Auto`]).
    pub choice: PdfParserBackend,
    /// Runtime converter: Vision or EdgeParse (`Auto` starts as Vision).
    pub runtime_backend: PdfParserBackend,
    pub source: PdfParserResolutionSource,
    /// When true, SPEC-038 may try EdgeParse before Vision.
    pub allows_auto_route: bool,
}

impl ResolvedPdfParser {
    /// Task payload flag: Vision/EdgeParse are inviolable (`true`); Auto is not.
    pub fn backend_explicit(self) -> bool {
        !self.allows_auto_route
    }
}

/// LAW-123-2 / LAW-123-4: resolve PDF parser with inviolable Vision/EdgeParse
/// and explicit Auto only for silent EdgeParse routing.
pub fn resolve_pdf_parser_choice(
    upload: Option<PdfParserBackend>,
    workspace: Option<PdfParserBackend>,
    tenant: Option<PdfParserBackend>,
    env: Option<PdfParserBackend>,
) -> ResolvedPdfParser {
    let (choice, source) = if let Some(choice) = upload {
        (choice, PdfParserResolutionSource::Upload)
    } else if let Some(choice) = workspace {
        (choice, PdfParserResolutionSource::Workspace)
    } else if let Some(choice) = tenant {
        (choice, PdfParserResolutionSource::Tenant)
    } else if let Some(choice) = env {
        (choice, PdfParserResolutionSource::Env)
    } else {
        (PdfParserBackend::Vision, PdfParserResolutionSource::Default)
    };

    match choice {
        PdfParserBackend::Auto => ResolvedPdfParser {
            choice: PdfParserBackend::Auto,
            runtime_backend: PdfParserBackend::Vision,
            source,
            allows_auto_route: true,
        },
        other => ResolvedPdfParser {
            choice: other,
            runtime_backend: other,
            source,
            allows_auto_route: false,
        },
    }
}

impl PdfParserBackend {
    pub fn from_env_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "vision" | "llm" => Some(Self::Vision),
            "edgeparse" | "edge-parse" | "edge_parse" => Some(Self::EdgeParse),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }

    pub fn from_env() -> Option<Self> {
        std::env::var("EDGEQUAKE_PDF_PARSER_BACKEND")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .and_then(|value| Self::from_env_str(&value))
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vision => "vision",
            Self::EdgeParse => "edgeparse",
            Self::Auto => "auto",
        }
    }

    /// Runtime converter backend (`Auto` → Vision start).
    pub fn runtime_backend(self) -> Self {
        match self {
            Self::Auto => Self::Vision,
            other => other,
        }
    }

    pub fn is_auto(self) -> bool {
        matches!(self, Self::Auto)
    }
}

/// Optional status reporter for post-OCR work inside vision convert (PNG render, etc.).
///
/// Arguments: `(stage_message, stage_progress_0_to_1)`.
pub type VisionStatusHook = Arc<dyn Fn(&str, f64) + Send + Sync>;

/// Per-task vision conversion options preserved from the existing processor.
#[derive(Clone, Default)]
pub struct VisionConversionConfig {
    /// LLM provider id (e.g. `"ollama"`, `"openai"`). Passed to pdf2md factory.
    pub provider_name: Option<String>,
    pub model: Option<String>,
    pub concurrency: Option<usize>,
    pub dpi: Option<u32>,
    /// SPEC-134: max_rendered_pixels forwarded to pdf2md Pass-A (long-edge;
    /// pdf2md 0.9.11 ignores dpi at raster time). None → pdf2md default 2000.
    /// Viewer page PNGs also read this field.
    pub max_rendered_pixels: Option<u32>,
    pub checkpoint_dir: Option<String>,
    pub no_resume: bool,
    pub progress_callback: Option<Arc<dyn edgequake_pdf2md::ConversionProgressCallback>>,
    /// Fired between OCR complete and markdown return (viewer PNG / chart crops).
    pub status_hook: Option<VisionStatusHook>,
    /// Optional page selection forwarded to pdf2md (SPEC-094).
    pub pages: Option<edgequake_pdf2md::PageSelection>,
    /// SPEC-109: desired vision reasoning effort (clamped at provider wrap).
    pub reasoning_effort: Option<String>,
    /// Per-page VLM API timeout forwarded to pdf2md `api_timeout_secs`.
    pub api_timeout_secs: Option<u64>,
}

impl std::fmt::Debug for VisionConversionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisionConversionConfig")
            .field("provider_name", &self.provider_name)
            .field("model", &self.model)
            .field("concurrency", &self.concurrency)
            .field("dpi", &self.dpi)
            .field("max_rendered_pixels", &self.max_rendered_pixels)
            .field("checkpoint_dir", &self.checkpoint_dir)
            .field("no_resume", &self.no_resume)
            .field(
                "progress_callback",
                &self.progress_callback.as_ref().map(|_| "<callback>"),
            )
            .field(
                "status_hook",
                &self.status_hook.as_ref().map(|_| "<status_hook>"),
            )
            .field("pages", &self.pages)
            .field("reasoning_effort", &self.reasoning_effort)
            .field("api_timeout_secs", &self.api_timeout_secs)
            .finish()
    }
}

/// When set, vision conversion writes page PNG assets under `assets_root` and
/// injects viewer `![…](assets/…)` links (SPEC-047 Phase C MV-21 / MV-28).
///
/// First principle: page PNGs serve the markdown viewer. Multimodal VLM
/// analyze (`<drawing/>` tags) is optional via [`Self::emit_analyze_tags`].
///
/// SPEC-015V: `extract_*` gates writers; absent/default all true (compat).
#[derive(Clone)]
pub struct PageDrawingAssetsConfig {
    /// Root passed to multimodal `resolve_image_asset` as `base_dir`.
    pub assets_root: PathBuf,
    /// Optional stable prefix for drawing ids (typically document id).
    pub id_prefix: Option<String>,
    /// When true, also emit `<drawing/>` tags for multimodal analyze scan (`i`).
    pub emit_analyze_tags: bool,
    /// SPEC-015V: write full-page viewer PNGs (`write_page_png_assets`).
    pub extract_images: bool,
    /// SPEC-015V: write chart ink crops (+ fig-as-chart promotion).
    pub extract_charts: bool,
    /// SPEC-015V: write embedded + caption figure crops.
    pub extract_figures: bool,
    /// SPEC-015V: Pass A page OCR system prompt override (None → SSOT).
    pub page_system_prompt: Option<String>,
    /// SPEC-134 P0: resolved page modality. Manuscript-class pages suppress
    /// scan-tiling fragment links/analyze tags (LAW-134-1 page-as-unit).
    /// `None` preserves legacy (print) behavior.
    pub page_modality: Option<crate::PageModality>,
    /// When set, run the SPEC-049 two-pass VLM figure filter after all crops
    /// are written.  The provider should be the same vision LLM used for page
    /// OCR.  Results are written to `figure_filter_manifest.json` under
    /// `assets_root`.
    pub figure_filter_provider: Option<Arc<dyn edgequake_llm::LLMProvider>>,
}

impl PageDrawingAssetsConfig {
    /// Defaults for extract flags (all ON) and no prompt override.
    pub fn with_defaults(assets_root: PathBuf, id_prefix: Option<String>) -> Self {
        Self {
            assets_root,
            id_prefix,
            emit_analyze_tags: false,
            extract_images: true,
            extract_charts: true,
            extract_figures: true,
            page_system_prompt: None,
            page_modality: None,
            figure_filter_provider: None,
        }
    }

    /// Apply SPEC-015V resolved extract policy onto this config.
    pub fn apply_vision_extract(&mut self, extract: &crate::VisionExtractConfig) {
        self.extract_images = extract.extract_images;
        self.extract_charts = extract.extract_charts;
        self.extract_figures = extract.extract_figures;
        self.page_system_prompt = extract.page_system_prompt.clone();
    }

    /// SPEC-015V writer plan (SSOT with [`crate::VisionAssetWritePlan`]).
    pub fn write_plan(&self) -> crate::VisionAssetWritePlan {
        crate::VisionAssetWritePlan::from_flags(
            self.extract_images,
            self.extract_charts,
            self.extract_figures,
        )
    }

    /// Attach the SPEC-049/128 VLM figure filter when figures or charts are
    /// extracted and `EDGEQUAKE_FIGURE_FILTER` is not `0`/`false`/`off`/`no`.
    /// Default is **on** whenever a vision LLM `provider` is supplied (LAW-128-14).
    pub fn attach_figure_filter_if_enabled(
        &mut self,
        provider: Option<Arc<dyn edgequake_llm::LLMProvider>>,
    ) {
        if !self.extract_figures && !self.extract_charts {
            return;
        }
        if !crate::figure_filter::figure_filter_env_enabled() {
            tracing::info!("SPEC-128 figure filter disabled via EDGEQUAKE_FIGURE_FILTER");
            return;
        }
        match provider {
            Some(p) => {
                tracing::info!(
                    provider = p.name(),
                    "SPEC-128 figure filter attached (default on)"
                );
                self.figure_filter_provider = Some(p);
            }
            None => tracing::warn!(
                "SPEC-128 figure filter skipped: no vision LLM provider (artefacts will not be pruned)"
            ),
        }
    }
}

impl std::fmt::Debug for PageDrawingAssetsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PageDrawingAssetsConfig")
            .field("assets_root", &self.assets_root)
            .field("id_prefix", &self.id_prefix)
            .field("emit_analyze_tags", &self.emit_analyze_tags)
            .field("extract_images", &self.extract_images)
            .field("extract_charts", &self.extract_charts)
            .field("extract_figures", &self.extract_figures)
            .field(
                "page_system_prompt",
                &self.page_system_prompt.as_ref().map(|s| s.len()),
            )
            .field(
                "figure_filter_provider",
                &self.figure_filter_provider.as_ref().map(|p| p.name()),
            )
            .finish()
    }
}

/// Configuration shared by PDF conversion backends.
#[derive(Clone, Debug, Default)]
pub struct PdfConversionConfig {
    pub page_count_hint: Option<usize>,
    pub table_method: Option<String>,
    pub filename: Option<String>,
    pub vision: Option<VisionConversionConfig>,
    pub page_drawing_assets: Option<PageDrawingAssetsConfig>,
    /// Optional page selection (vision backends; EdgeParse ignores for now).
    pub pages: Option<edgequake_pdf2md::PageSelection>,
}

#[async_trait]
pub trait PdfConverter: Send + Sync {
    async fn convert(
        &self,
        pdf_bytes: &[u8],
        config: &PdfConversionConfig,
    ) -> Result<String, PdfConversionError>;

    fn backend_name(&self) -> &'static str;
}

pub fn create_pdf_converter(backend: PdfParserBackend) -> Arc<dyn PdfConverter> {
    match backend.runtime_backend() {
        PdfParserBackend::EdgeParse => Arc::new(EdgeParsePdfConverter),
        // Vision and Auto (config-only) both start on the Vision converter.
        PdfParserBackend::Vision | PdfParserBackend::Auto => Arc::new(VisionPdfConverter::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        resolve_pdf_parser_choice, PdfParserBackend, PdfParserResolutionSource, ResolvedPdfParser,
    };

    #[test]
    fn backend_env_aliases_roundtrip() {
        assert_eq!(
            PdfParserBackend::from_env_str("vision"),
            Some(PdfParserBackend::Vision)
        );
        assert_eq!(
            PdfParserBackend::from_env_str("edge-parse"),
            Some(PdfParserBackend::EdgeParse)
        );
        assert_eq!(
            PdfParserBackend::from_env_str("auto"),
            Some(PdfParserBackend::Auto)
        );
        assert_eq!(PdfParserBackend::Vision.as_str(), "vision");
        assert_eq!(PdfParserBackend::EdgeParse.as_str(), "edgeparse");
        assert_eq!(PdfParserBackend::Auto.as_str(), "auto");
    }

    #[test]
    fn resolve_priority_upload_wins() {
        let resolved = resolve_pdf_parser_choice(
            Some(PdfParserBackend::EdgeParse),
            Some(PdfParserBackend::Vision),
            Some(PdfParserBackend::Auto),
            Some(PdfParserBackend::Vision),
        );
        assert_eq!(
            resolved,
            ResolvedPdfParser {
                choice: PdfParserBackend::EdgeParse,
                runtime_backend: PdfParserBackend::EdgeParse,
                source: PdfParserResolutionSource::Upload,
                allows_auto_route: false,
            }
        );
        assert!(resolved.backend_explicit());
    }

    #[test]
    fn resolve_unset_defaults_to_inviolable_vision() {
        let resolved = resolve_pdf_parser_choice(None, None, None, None);
        assert_eq!(resolved.choice, PdfParserBackend::Vision);
        assert_eq!(resolved.runtime_backend, PdfParserBackend::Vision);
        assert_eq!(resolved.source, PdfParserResolutionSource::Default);
        assert!(!resolved.allows_auto_route);
        assert!(resolved.backend_explicit());
    }

    #[test]
    fn resolve_auto_allows_route() {
        let resolved = resolve_pdf_parser_choice(None, Some(PdfParserBackend::Auto), None, None);
        assert_eq!(resolved.choice, PdfParserBackend::Auto);
        assert_eq!(resolved.runtime_backend, PdfParserBackend::Vision);
        assert_eq!(resolved.source, PdfParserResolutionSource::Workspace);
        assert!(resolved.allows_auto_route);
        assert!(!resolved.backend_explicit());
    }

    #[test]
    fn resolve_tenant_before_env() {
        let resolved = resolve_pdf_parser_choice(
            None,
            None,
            Some(PdfParserBackend::EdgeParse),
            Some(PdfParserBackend::Vision),
        );
        assert_eq!(resolved.source, PdfParserResolutionSource::Tenant);
        assert_eq!(resolved.runtime_backend, PdfParserBackend::EdgeParse);
    }
}
