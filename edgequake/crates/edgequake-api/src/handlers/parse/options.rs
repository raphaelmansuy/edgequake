//! Validate and map [`ParseOptions`] onto converter configs.

use edgequake_pdf::{
    parse_page_selection, PageDrawingAssetsConfig, PdfConversionConfig, PdfParserBackend,
    VisionConversionConfig,
};
use edgequake_pdf2md::PageSelection;

use super::errors::ParseErrorCode;
use super::types::ParseOptions;
use crate::error::ApiResult;

/// Resolved, validated options ready for conversion.
#[derive(Debug, Clone)]
pub struct ResolvedParseOptions {
    pub backend: PdfParserBackend,
    pub provider: String,
    pub model: Option<String>,
    pub dpi: u32,
    pub concurrency: usize,
    pub pages: Option<PageSelection>,
    pub table_method: Option<String>,
    pub emit_assets: bool,
    pub allow_fallback: bool,
    pub include_page_timings: bool,
    pub force_async: bool,
    pub filename: String,
}

pub fn resolve_options(
    options: &ParseOptions,
    filename: &str,
    server_default_backend: PdfParserBackend,
    server_default_provider: &str,
    server_default_concurrency: usize,
) -> ApiResult<ResolvedParseOptions> {
    let backend = match options.backend.as_deref() {
        None => server_default_backend,
        Some(raw) => PdfParserBackend::from_env_str(raw).ok_or_else(|| {
            ParseErrorCode::InvalidRequest.into_api_error(format!(
                "Unknown backend '{raw}'; expected 'vision' or 'edgeparse'"
            ))
        })?,
    };

    if let Some(dpi) = options.dpi {
        if !(72..=400).contains(&dpi) {
            return Err(ParseErrorCode::InvalidRequest
                .into_api_error(format!("dpi must be 72–400 (got {dpi})")));
        }
    }
    if let Some(c) = options.concurrency {
        if !(1..=16).contains(&c) {
            return Err(ParseErrorCode::InvalidRequest
                .into_api_error(format!("concurrency must be 1–16 (got {c})")));
        }
    }

    let pages = match options.pages.as_deref() {
        None => None,
        Some(raw) => Some(
            parse_page_selection(raw)
                .map_err(|e| ParseErrorCode::InvalidRequest.into_api_error(e.to_string()))?,
        ),
    };

    let provider = options
        .provider
        .clone()
        .unwrap_or_else(|| server_default_provider.to_string());

    let concurrency = options
        .concurrency
        .unwrap_or(server_default_concurrency)
        .clamp(1, 16);

    Ok(ResolvedParseOptions {
        backend,
        provider,
        model: options.model.clone(),
        dpi: options.dpi_or_default(),
        concurrency,
        pages,
        table_method: options.table_method.clone(),
        emit_assets: options.emit_assets(),
        allow_fallback: options.allow_fallback(),
        include_page_timings: options.include_page_timings(),
        force_async: options.force_async.unwrap_or(false),
        filename: filename.to_string(),
    })
}

impl ResolvedParseOptions {
    /// Build conversion config. `assets_root` only when emit_assets is true.
    pub fn to_conversion_config(
        &self,
        page_count_hint: Option<usize>,
        progress_callback: Option<std::sync::Arc<dyn edgequake_pdf2md::ConversionProgressCallback>>,
        assets_root: Option<std::path::PathBuf>,
    ) -> PdfConversionConfig {
        let vision = if self.backend == PdfParserBackend::Vision {
            let model = self
                .model
                .clone()
                .unwrap_or_else(|| default_vision_model_for_provider(&self.provider).to_string());
            Some(VisionConversionConfig {
                provider_name: Some(self.provider.clone()),
                model: Some(model),
                concurrency: Some(self.concurrency),
                dpi: Some(self.dpi),
                max_rendered_pixels: None,
                checkpoint_dir: None,
                no_resume: true,
                progress_callback,
                status_hook: None,
                pages: self.pages.clone(),
                reasoning_effort: None,
                api_timeout_secs: None,
            })
        } else {
            None
        };

        let page_drawing_assets = assets_root.map(|root| {
            let mut cfg = PageDrawingAssetsConfig::with_defaults(root, Some("parse".into()));
            cfg.emit_analyze_tags = false;
            cfg
        });

        PdfConversionConfig {
            page_count_hint,
            table_method: self.table_method.clone(),
            filename: Some(self.filename.clone()),
            vision,
            page_drawing_assets,
            pages: self.pages.clone(),
        }
    }
}

fn default_vision_model_for_provider(provider: &str) -> &'static str {
    match provider.to_ascii_lowercase().as_str() {
        "openai" => "gpt-4.1-nano",
        "ollama" | "lmstudio" => "gemma3:latest",
        "mock" => "mock-vision",
        _ => "gemma3:latest",
    }
}
