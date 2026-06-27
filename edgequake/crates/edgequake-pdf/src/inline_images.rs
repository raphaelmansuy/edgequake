//! Inline image / drawing placeholder scanning (SPEC-026 Phase 4b).
//!
//! Mirrors LightRAG sidecar item loop at markdown level: detect
//! `<drawing …/>` tags and data-URI markdown images for VLM enrichment.

use once_cell::sync::Lazy;
use regex::Regex;

/// Result of VLM analysis for one inline image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineImageAnalysis {
    pub item_id: String,
    pub name: String,
    pub description: String,
}

/// A discoverable inline image reference inside converted markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineImageRef {
    pub item_id: String,
    /// Full matched span (tag or markdown image).
    pub matched: String,
    pub mime_type: String,
    pub bytes: Vec<u8>,
    /// Relative asset path from `<drawing path="…"/>`, if present.
    pub asset_path: Option<String>,
    /// Caption from `<drawing caption="…"/>` when present.
    pub caption: Option<String>,
    /// Footnote from tag attribute when present.
    pub footnote: Option<String>,
    /// Character offset in source markdown (for replacement).
    pub start: usize,
    pub end: usize,
}

/// LightRAG native drawing placeholder: `<drawing id="im-…" … />`.
static DRAWING_TAG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?i)<drawing\b[^>]*/>"#).unwrap());

static DRAWING_ID_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?i)\bid="([^"]+)""#).unwrap());

static DRAWING_PATH_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?i)\bpath="([^"]+)""#).unwrap());

static DRAWING_FORMAT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?i)\bformat="([^"]+)""#).unwrap());

static DRAWING_CAPTION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?i)\bcaption="([^"]+)""#).unwrap());

static DRAWING_FOOTNOTE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?i)\bfootnote="([^"]+)""#).unwrap());

/// Markdown data-URI image: `![alt](data:image/png;base64,...)`.
static DATA_URI_IMAGE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"!\[[^\]]*\]\(data:image/([a-zA-Z0-9.+-]+);base64,([A-Za-z0-9+/=\s]+)\)"#).unwrap()
});

/// Scan markdown for inline image references eligible for VLM describe.
pub fn scan_inline_image_refs(markdown: &str) -> Vec<InlineImageRef> {
    let mut refs = Vec::new();

    for cap in DRAWING_TAG_RE.captures_iter(markdown) {
        let matched = cap.get(0).unwrap();
        let item_id = DRAWING_ID_RE
            .captures(matched.as_str())
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| format!("drawing_{}", refs.len()));
        let asset_path = DRAWING_PATH_RE
            .captures(matched.as_str())
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());
        let format_hint = DRAWING_FORMAT_RE
            .captures(matched.as_str())
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_lowercase());
        let mime_type = format_hint
            .map(|f| format!("image/{f}"))
            .unwrap_or_default();
        let caption = DRAWING_CAPTION_RE
            .captures(matched.as_str())
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());
        let footnote = DRAWING_FOOTNOTE_RE
            .captures(matched.as_str())
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());
        refs.push(InlineImageRef {
            item_id,
            matched: matched.as_str().to_string(),
            mime_type,
            bytes: Vec::new(),
            asset_path,
            caption,
            footnote,
            start: matched.start(),
            end: matched.end(),
        });
    }

    for cap in DATA_URI_IMAGE_RE.captures_iter(markdown) {
        let matched = cap.get(0).unwrap();
        let mime = cap.get(1).map(|m| m.as_str()).unwrap_or("png");
        let b64 = cap
            .get(2)
            .map(|m| m.as_str().replace(['\n', ' ', '\t'], ""))
            .unwrap_or_default();
        let bytes = base64_decode(&b64).unwrap_or_default();
        if bytes.is_empty() {
            continue;
        }
        refs.push(InlineImageRef {
            item_id: format!("data_uri_{}", refs.len()),
            matched: matched.as_str().to_string(),
            mime_type: format!("image/{mime}"),
            bytes,
            asset_path: None,
            caption: None,
            footnote: None,
            start: matched.start(),
            end: matched.end(),
        });
    }

    refs.sort_by_key(|r| r.start);
    refs
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.decode(input).ok()
}

/// Analyzes inline images discovered during PDF/markdown conversion.
#[async_trait::async_trait]
pub trait InlineImageAnalyzer: Send + Sync {
    async fn analyze_items(&self, markdown: &str) -> Result<Vec<InlineImageAnalysis>, String>;
}

/// Default no-op analyzer (Phase 4b placeholder when VLM disabled).
#[derive(Debug, Default)]
pub struct NoopInlineImageAnalyzer;

#[async_trait::async_trait]
impl InlineImageAnalyzer for NoopInlineImageAnalyzer {
    async fn analyze_items(&self, _markdown: &str) -> Result<Vec<InlineImageAnalysis>, String> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_finds_lightrag_drawing_tag() {
        let md = r#"Intro text
<drawing id="im-cafe-0001" format="png" caption="Chart" path="assets/chart.png" />
Trailing"#;
        let refs = scan_inline_image_refs(md);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].item_id, "im-cafe-0001");
        assert!(refs[0].matched.starts_with("<drawing"));
        assert_eq!(refs[0].asset_path.as_deref(), Some("assets/chart.png"));
    }

    #[test]
    fn scan_finds_data_uri_markdown_image() {
        let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
        let md = format!("See ![tiny](data:image/png;base64,{b64}) here.");
        let refs = scan_inline_image_refs(&md);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].mime_type, "image/png");
        assert!(!refs[0].bytes.is_empty());
    }
}
