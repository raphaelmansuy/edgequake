//! Shared convert + fallback + metrics path for sync and async parse.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use edgequake_pdf::{
    build_edgeparse_fallback_message, create_pdf_converter, resolve_pdf_page_count,
    should_fallback_to_edgeparse, PdfConversionError, PdfParserBackend, VisionFailureKind,
};
use tempfile::TempDir;
use tracing::{info, warn};
use uuid::Uuid;

use super::errors::ParseErrorCode;
use super::metrics_hook::ParseMetricsHook;
use super::options::ResolvedParseOptions;
use super::types::{ParseMetrics, ParseResponse};
use crate::error::{ApiError, ApiResult};

/// Hard ceilings for sync vs async admission (LightOn parity + SPEC-094).
#[derive(Debug, Clone, Copy)]
pub struct ParseLimits {
    pub sync_max_pages: u32,
    pub sync_max_bytes: u64,
    pub async_max_pages: u32,
    pub async_max_bytes: u64,
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            sync_max_pages: 15,
            sync_max_bytes: 20 * 1024 * 1024,
            async_max_pages: 1000,
            async_max_bytes: edgequake_core::MAX_UPLOAD_BYTES as u64,
        }
    }
}

impl ParseLimits {
    pub fn from_env() -> Self {
        let mut limits = Self::default();
        if let Ok(v) = std::env::var("EDGEQUAKE_PARSE_SYNC_MAX_PAGES") {
            if let Ok(n) = v.parse() {
                limits.sync_max_pages = n;
            }
        }
        if let Ok(v) = std::env::var("EDGEQUAKE_PARSE_SYNC_MAX_BYTES") {
            if let Ok(n) = v.parse() {
                limits.sync_max_bytes = n;
            }
        }
        if let Ok(v) = std::env::var("EDGEQUAKE_PARSE_ASYNC_MAX_PAGES") {
            if let Ok(n) = v.parse() {
                limits.async_max_pages = n;
            }
        }
        limits
    }

    pub fn exceeds_async(&self, pages: u32, bytes: u64) -> bool {
        pages > self.async_max_pages || bytes > self.async_max_bytes
    }

    pub fn exceeds_sync(&self, pages: u32, bytes: u64) -> bool {
        pages > self.sync_max_pages || bytes > self.sync_max_bytes
    }
}

/// Run a stateless parse and return the SPEC-094 response body.
pub async fn run_parse(
    pdf_bytes: &[u8],
    resolved: &ResolvedParseOptions,
    request_id: Option<String>,
) -> ApiResult<ParseResponse> {
    let request_id = request_id.unwrap_or_else(|| format!("pr_{}", Uuid::new_v4().simple()));
    let span = tracing::info_span!(
        "parse_request",
        backend = resolved.backend.as_str(),
        provider = %resolved.provider,
        model = ?resolved.model,
        dpi = resolved.dpi,
        request_id = %request_id,
    );
    let _guard = span.enter();

    let page_count = resolve_pdf_page_count(pdf_bytes, None).await.unwrap_or(0) as u32;

    let metrics_hook = ParseMetricsHook::new();
    let progress: Arc<dyn edgequake_pdf2md::ConversionProgressCallback> =
        Arc::clone(&metrics_hook) as _;

    let assets_temp: Option<TempDir> = if resolved.emit_assets {
        Some(TempDir::new().map_err(|e| {
            ApiError::Internal(format!("Failed to create parse assets temp dir: {e}"))
        })?)
    } else {
        None
    };
    let assets_root: Option<PathBuf> = assets_temp.as_ref().map(|t| t.path().to_path_buf());

    let mut config = resolved.to_conversion_config(
        Some(page_count as usize).filter(|&n| n > 0),
        Some(progress),
        assets_root,
    );
    if let Some(ref mut assets) = config.page_drawing_assets {
        let model = resolved
            .model
            .clone()
            .unwrap_or_else(|| "gemma3:latest".into());
        let built = if resolved.provider.eq_ignore_ascii_case("databricks") {
            crate::safety_limits::create_safe_vision_provider(&resolved.provider, &model)
                .map_err(|error| error.to_string())
        } else {
            edgequake_llm::ProviderFactory::create_llm_provider(&resolved.provider, &model)
                .map_err(|error| error.to_string())
        };
        match built {
            Ok(provider) => {
                if let Some(vision) = config.vision.as_mut() {
                    vision.llm = Some(std::sync::Arc::clone(&provider));
                }
                assets.attach_figure_filter_if_enabled(Some(provider));
            }
            Err(e) => warn!(error = %e, "SPEC-128 parse: figure filter provider unavailable"),
        }
    }

    let mut warnings = Vec::new();
    let mut fallback_applied = false;
    let mut backend_effective = resolved.backend.as_str().to_string();

    let timeout = parse_timeout(page_count, &resolved.provider);
    let convert_result = tokio::time::timeout(
        timeout,
        create_pdf_converter(resolved.backend).convert(pdf_bytes, &config),
    )
    .await;

    let markdown = match convert_result {
        Ok(Ok(md)) => md,
        Ok(Err(err)) => {
            let kind = classify_vision_failure(&err);
            let backend_explicit = !resolved.allow_fallback;
            if resolved.backend == PdfParserBackend::Vision
                && resolved.allow_fallback
                && should_fallback_to_edgeparse(resolved.backend, kind, backend_explicit)
            {
                warn!(error = %err, "Vision parse failed; falling back to EdgeParse");
                let edge_config = edgequake_pdf::PdfConversionConfig {
                    vision: None,
                    page_drawing_assets: None,
                    ..config.clone()
                };
                let md = create_pdf_converter(PdfParserBackend::EdgeParse)
                    .convert(pdf_bytes, &edge_config)
                    .await
                    .map_err(|e| map_conversion_error(&e))?;
                fallback_applied = true;
                backend_effective = PdfParserBackend::EdgeParse.as_str().to_string();
                warnings.push(build_edgeparse_fallback_message(
                    &resolved.provider,
                    kind.as_detail_str(),
                ));
                md
            } else {
                return Err(map_conversion_error_with_kind(&err, kind));
            }
        }
        Err(_elapsed) => {
            return Err(ParseErrorCode::Timeout
                .into_api_error(format!("Parse exceeded deadline of {}s", timeout.as_secs())));
        }
    };

    // Ensure temp assets are dropped (cleanup) before returning.
    drop(assets_temp);

    let total_ms = metrics_hook.elapsed_ms();
    let pages_per_second = if total_ms > 0 && page_count > 0 {
        Some((page_count as f64) / (total_ms as f64 / 1000.0))
    } else {
        None
    };

    let page_timings = if resolved.include_page_timings {
        Some(metrics_hook.page_timings())
    } else {
        None
    };

    info!(
        backend = %backend_effective,
        fallback_applied,
        page_count,
        total_ms,
        "Parse completed"
    );

    Ok(ParseResponse {
        markdown,
        backend: resolved.backend.as_str().to_string(),
        backend_effective,
        fallback_applied,
        page_count,
        metrics: ParseMetrics {
            total_ms,
            render_ms: Some(metrics_hook.render_ms()).filter(|&v| v > 0),
            ocr_ms: Some(metrics_hook.ocr_ms()).filter(|&v| v > 0),
            assemble_ms: metrics_hook.assemble_ms_hint(),
            pages_per_second,
            prompt_tokens: None,
            completion_tokens: None,
            estimated_cost_usd: None,
        },
        page_timings,
        warnings,
        request_id,
    })
}

fn parse_timeout(page_count: u32, provider: &str) -> Duration {
    let per_page = if provider.eq_ignore_ascii_case("ollama")
        || provider.eq_ignore_ascii_case("lmstudio")
        || provider.eq_ignore_ascii_case("mock")
    {
        120
    } else {
        30
    };
    let secs = (page_count as u64).saturating_mul(per_page).clamp(60, 3600);
    Duration::from_secs(secs)
}

fn classify_vision_failure(err: &PdfConversionError) -> VisionFailureKind {
    let msg = err.to_string().to_ascii_lowercase();
    if msg.contains("timeout") || msg.contains("timed out") {
        VisionFailureKind::Timeout
    } else if msg.contains("connect")
        || msg.contains("unreachable")
        || msg.contains("refused")
        || msg.contains("provider")
        || msg.contains("not found")
        || msg.contains("unavailable")
    {
        VisionFailureKind::ProviderUnavailable
    } else if msg.contains("not configured") || msg.contains("feature") {
        VisionFailureKind::FeatureUnavailable
    } else {
        VisionFailureKind::ConversionFailed
    }
}

fn map_conversion_error(err: &PdfConversionError) -> ApiError {
    map_conversion_error_with_kind(err, classify_vision_failure(err))
}

fn map_conversion_error_with_kind(err: &PdfConversionError, kind: VisionFailureKind) -> ApiError {
    let msg = err.to_string();
    match kind {
        VisionFailureKind::Timeout => ParseErrorCode::Timeout.into_api_error(msg),
        VisionFailureKind::ProviderUnavailable | VisionFailureKind::FeatureUnavailable => {
            ParseErrorCode::BackendUnavailable.into_api_error(msg)
        }
        VisionFailureKind::ConversionFailed => {
            if msg.to_ascii_lowercase().contains("encrypt")
                || msg.to_ascii_lowercase().contains("malform")
                || msg.to_ascii_lowercase().contains("corrupt")
            {
                ParseErrorCode::DocumentUnreadable.into_api_error(msg)
            } else {
                ParseErrorCode::BackendUnavailable.into_api_error(msg)
            }
        }
    }
}
