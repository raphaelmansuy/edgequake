//! Vision-based text extraction — re-exports SSOT in `services::vision_content`.
//!
//! Kept for backward compatibility with existing imports.

pub use crate::services::vision_content::{
    describe_image, describe_image_as_markdown, image_analysis_to_markdown,
    parse_image_analysis_json, ImageAnalysisResult,
};

/// Legacy alias for [`describe_image_as_markdown`].
pub async fn extract_text_from_image(
    image_bytes: &[u8],
    mime_type: &str,
    filename: &str,
    llm: &dyn edgequake_llm::traits::LLMProvider,
) -> crate::error::ApiResult<String> {
    describe_image_as_markdown(image_bytes, mime_type, filename, llm).await
}

#[cfg(test)]
mod tests {
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

    #[test]
    fn test_base64_encoding_non_empty() {
        let bytes = b"\x89PNG\r\n\x1a\n";
        let encoded = B64.encode(bytes);
        assert!(!encoded.is_empty());
    }
}
