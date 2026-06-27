//! Vision / VLM content extraction SSOT (SPEC-026 Phase 4 P-07).
//!
//! Mirrors LightRAG `prompt_multimodal.py` image_analysis template and
//! `llm/_vision_utils.py` image normalization — produces structured JSON
//! then emits markdown for the existing text ingestion pipeline.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use edgequake_llm::traits::{ChatMessage, ImageData, LLMProvider};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::{ApiError, ApiResult};

/// LightRAG-compatible image type enum (subset).
pub const IMAGE_TYPE_ENUM: &[&str] = &[
    "Photo",
    "Illustration",
    "Screenshot",
    "Icon",
    "Chart",
    "Table",
    "Infographic",
    "Flowchart",
    "Chat Log",
    "Wireframe",
    "Texture",
    "Other",
];

pub const IMAGE_TYPE_FALLBACK: &str = "Other";

/// Structured VLM output for a single image (LightRAG image_analysis schema).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageAnalysisResult {
    pub name: String,
    #[serde(rename = "type")]
    pub image_type: String,
    pub description: String,
}

/// Per-document multimodal process flags (LightRAG `process_options` i/t/e).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultimodalProcessOptions {
    pub images: bool,
    pub tables: bool,
    pub equations: bool,
}

impl MultimodalProcessOptions {
    /// Parse LightRAG-style option string: `"i"`, `"it"`, `"ite"`, etc.
    pub fn from_option_str(raw: &str) -> Self {
        let lower = raw.trim().to_ascii_lowercase();
        Self {
            images: lower.contains('i'),
            tables: lower.contains('t'),
            equations: lower.contains('e'),
        }
    }

    pub fn any_enabled(self) -> bool {
        self.images || self.tables || self.equations
    }
}

const IMAGE_ANALYSIS_SYSTEM_PROMPT: &str = "\
You are an expert image analyzer. Analyze the provided image and return a single JSON object with exactly these keys:
- \"name\": concise snake_case label (3-8 words)
- \"type\": one of Photo, Illustration, Screenshot, Icon, Chart, Table, Infographic, Flowchart, Chat Log, Wireframe, Texture, Other
- \"description\": detailed markdown description of visible content; quote short text verbatim

Return ONLY valid JSON. No markdown fences or commentary.";

/// Normalize image type to LightRAG enum; unknown values fold to `Other`.
pub fn normalize_image_type(raw: &str) -> String {
    let trimmed = raw.trim();
    if IMAGE_TYPE_ENUM.contains(&trimmed) {
        trimmed.to_string()
    } else {
        IMAGE_TYPE_FALLBACK.to_string()
    }
}

/// Parse tolerant JSON object from VLM response text.
pub fn parse_image_analysis_json(text: &str) -> ApiResult<ImageAnalysisResult> {
    let trimmed = text.trim();
    let json_str = if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            &trimmed[start..=end]
        } else {
            trimmed
        }
    } else {
        trimmed
    };

    let mut parsed: ImageAnalysisResult = serde_json::from_str(json_str).map_err(|e| {
        ApiError::BadRequest(format!("VLM returned invalid image analysis JSON: {e}"))
    })?;
    parsed.image_type = normalize_image_type(&parsed.image_type);
    if parsed.name.trim().is_empty() {
        parsed.name = "image_content".to_string();
    }
    Ok(parsed)
}

/// Convert structured analysis to markdown body for entity extraction.
pub fn image_analysis_to_markdown(analysis: &ImageAnalysisResult) -> String {
    format!(
        "# {}\n\n**Type:** {}\n\n{}",
        analysis.name.replace('_', " "),
        analysis.image_type,
        analysis.description.trim()
    )
}

/// Extract structured image content via vision-capable LLM.
pub async fn describe_image(
    image_bytes: &[u8],
    mime_type: &str,
    filename: &str,
    llm: &dyn LLMProvider,
) -> ApiResult<ImageAnalysisResult> {
    debug!(
        filename = %filename,
        mime_type = %mime_type,
        bytes = image_bytes.len(),
        "VLM image analysis"
    );

    let base64_data = B64.encode(image_bytes);
    let image_data = ImageData::new(&base64_data, mime_type);

    let messages = vec![
        ChatMessage::system(IMAGE_ANALYSIS_SYSTEM_PROMPT),
        ChatMessage::user_with_images(
            "Analyze this image and return the JSON object.",
            vec![image_data],
        ),
    ];

    let response = llm
        .chat(&messages, None)
        .await
        .map_err(|e| ApiError::Internal(format!("Vision LLM call failed for '{filename}': {e}")))?;

    let text = response.content.trim();
    if text.is_empty() {
        return Err(ApiError::BadRequest(format!(
            "Vision LLM returned no content for '{filename}'. \
             Configure a vision-capable model in workspace settings."
        )));
    }

    parse_image_analysis_json(text)
}

/// Describe image and return markdown suitable for document admission.
pub async fn describe_image_as_markdown(
    image_bytes: &[u8],
    mime_type: &str,
    filename: &str,
    llm: &dyn LLMProvider,
) -> ApiResult<String> {
    let analysis = describe_image(image_bytes, mime_type, filename, llm).await?;
    Ok(image_analysis_to_markdown(&analysis))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_analysis_json_parses_lightrag_schema() {
        let raw = r#"{"name":"sarah_chen_profile","type":"Photo","description":"Dr. Sarah Chen, research lead."}"#;
        let parsed = parse_image_analysis_json(raw).unwrap();
        assert_eq!(parsed.name, "sarah_chen_profile");
        assert_eq!(parsed.image_type, "Photo");
        assert!(parsed.description.contains("Sarah Chen"));
    }

    #[test]
    fn image_analysis_invalid_type_falls_back_to_other() {
        let raw = r#"{"name":"x","type":"AlienType","description":"body"}"#;
        let parsed = parse_image_analysis_json(raw).unwrap();
        assert_eq!(parsed.image_type, "Other");
    }

    #[test]
    fn vision_markdown_includes_name_heading() {
        let analysis = ImageAnalysisResult {
            name: "sarah_chen_profile".into(),
            image_type: "Photo".into(),
            description: "Research lead.".into(),
        };
        let md = image_analysis_to_markdown(&analysis);
        assert!(md.starts_with("# sarah chen profile"));
        assert!(md.contains("**Type:** Photo"));
    }

    #[test]
    fn multimodal_process_options_parse_ite() {
        let opts = MultimodalProcessOptions::from_option_str("ite");
        assert!(opts.images && opts.tables && opts.equations);
        let partial = MultimodalProcessOptions::from_option_str("i");
        assert!(partial.images);
        assert!(!partial.tables);
    }
}
