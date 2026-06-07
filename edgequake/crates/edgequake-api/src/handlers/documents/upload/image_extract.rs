//! Vision-based text extraction from uploaded images.
//!
//! ## Implements
//!
//! - [`FEAT0203`]: Image document upload — extract text via vision LLM
//!
//! ## Why
//!
//! Users want to upload PNG/JPG/GIF/WEBP files as knowledge-graph documents.
//! Since the pipeline expects text, we call the workspace's vision-capable LLM
//! once with a structured extraction prompt and use the response as the document
//! text content.  This mirrors what the PDF vision pipeline already does per
//! page, but for a single image.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use edgequake_llm::traits::{ChatMessage, ImageData, LLMProvider};
use tracing::debug;

use crate::error::{ApiError, ApiResult};

/// System prompt for image-to-text extraction.
///
/// WHY: Explicit instruction to preserve all text verbatim ensures the entity
/// extractor downstream gets clean, structured content rather than narrative
/// summaries that might lose entity names or relationships.
const EXTRACTION_SYSTEM_PROMPT: &str = "\
You are a precise document extraction assistant. \
Extract ALL text, data, tables, and meaningful structured content from the provided image. \
Preserve headings, bullet points, numbers, names, and technical terms exactly as they appear. \
Format the output as clean Markdown. \
Do NOT add commentary or explanations — only output the extracted content.";

/// Extract text from a binary image using the provided vision-capable LLM.
///
/// # Arguments
/// * `image_bytes` – Raw bytes of the image file.
/// * `mime_type`   – MIME type string (e.g., `"image/png"`).
/// * `filename`    – Original filename for diagnostic context.
/// * `llm`         – A vision-capable `LLMProvider` (e.g., OpenAI GPT-4V, Ollama gemma3).
///
/// # Returns
/// Extracted text as a Markdown string, or an `ApiError` if the LLM call fails
/// or the response is empty.
pub async fn extract_text_from_image(
    image_bytes: &[u8],
    mime_type: &str,
    filename: &str,
    llm: &dyn LLMProvider,
) -> ApiResult<String> {
    debug!(
        filename = %filename,
        mime_type = %mime_type,
        bytes = image_bytes.len(),
        "Extracting text from image via vision LLM"
    );

    let base64_data = B64.encode(image_bytes);
    let image_data = ImageData::new(&base64_data, mime_type);

    let user_message = ChatMessage::user_with_images(
        "Please extract all text and structured content from this image.",
        vec![image_data],
    );

    let system_message = ChatMessage::system(EXTRACTION_SYSTEM_PROMPT);

    let messages = vec![system_message, user_message];

    let response = llm.chat(&messages, None).await.map_err(|e| {
        ApiError::Internal(format!("Vision LLM call failed for '{}': {}", filename, e))
    })?;

    let text = response.content.trim().to_string();

    if text.is_empty() {
        return Err(ApiError::BadRequest(format!(
            "Vision LLM returned no content for '{}'. \
             Ensure your vision model (e.g., gpt-4o, gemma3:12b) is configured \
             in the workspace settings.",
            filename
        )));
    }

    debug!(
        filename = %filename,
        extracted_len = text.len(),
        "Image text extraction complete"
    );

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that base64-encoding a small payload does not panic and produces
    /// non-empty output (sanity check — does not call a real LLM).
    #[test]
    fn test_base64_encoding_non_empty() {
        let bytes = b"\x89PNG\r\n\x1a\n"; // PNG magic bytes
        let encoded = B64.encode(bytes);
        assert!(!encoded.is_empty());
    }
}
