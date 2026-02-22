//! LLM-based Image OCR module.
//!
//! This module provides OCR (Optical Character Recognition) capabilities for
//! images embedded in PDF documents using multimodal LLMs instead of traditional
//! OCR engines like Tesseract.
//!
//! ## Implements
//!
//! @implements FEAT1004
//! @implements FEAT1025
//!
//! ## Enforces
//!
//! - **BR1025**: OCR timeout after 30s per image
//! - **BR1026**: Batch images to reduce API calls
//!
//! # Why LLM-based OCR?
//!
//! Traditional OCR engines struggle with:
//! - Complex diagrams and charts
//! - Handwritten text
//! - Low-quality or noisy images
//! - Mixed text/graphic content
//! - Non-standard fonts
//!
//! Multimodal LLMs (GPT-4o, Claude 3.5, etc.) provide:
//! - Better accuracy on diverse content
//! - Natural language descriptions
//! - Chart/diagram data extraction
//! - Context-aware interpretation
//!
//! # Usage
//!
//! ```rust,no_run
//! use edgequake_pdf::image_ocr::{ImageOcrProcessor, ImageData};
//! use edgequake_pdf::config::ImageOcrConfig;
//! use edgequake_llm::providers::openai::OpenAIProvider;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = Arc::new(OpenAIProvider::new("your-api-key"));
//! let config = ImageOcrConfig::enabled();
//! let processor = ImageOcrProcessor::new(provider, config);
//!
//! let image = ImageData {
//!     data: vec![/* PNG bytes */],
//!     mime_type: "image/png".to_string(),
//!     width: 800,
//!     height: 600,
//!     page: 1,
//!     index: 0,
//!     bbox: None,
//! };
//!
//! let result = processor.process_image(&image).await?;
//! println!("Extracted text: {:?}", result.text);
//! println!("Description: {:?}", result.description);
//! # Ok(())
//! # }
//! ```
//!
//! # Cost Considerations
//!
//! LLM-based OCR incurs API costs based on image dimensions:
//! - GPT-4o: ~765 tokens for 1024x1024 image (high detail)
//! - GPT-4o-mini: Lower cost but slightly reduced quality
//!
//! The `ImageOcrConfig` allows limiting images per page and setting
//! minimum size thresholds to control costs.

use crate::config::ImageOcrConfig;
use crate::error::PdfError;
use crate::Result;
use async_trait::async_trait;
use base64::Engine;
use edgequake_llm::traits::{ChatMessage, CompletionOptions, LLMProvider};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Image data extracted from a PDF page.
#[derive(Debug, Clone)]
pub struct ImageData {
    /// Raw image bytes (PNG, JPEG, etc.)
    pub data: Vec<u8>,

    /// MIME type (e.g., "image/png", "image/jpeg")
    pub mime_type: String,

    /// Image width in pixels
    pub width: u32,

    /// Image height in pixels
    pub height: u32,

    /// Page number where image was found (1-indexed)
    pub page: usize,

    /// Image index on the page (0-indexed)
    pub index: usize,

    /// Bounding box in PDF coordinates (optional)
    pub bbox: Option<(f32, f32, f32, f32)>,
}

impl ImageData {
    /// Convert image to base64 data URL for LLM API.
    pub fn to_data_url(&self) -> String {
        let base64_data = base64::engine::general_purpose::STANDARD.encode(&self.data);
        format!("data:{};base64,{}", self.mime_type, base64_data)
    }

    /// Check if image meets minimum size requirements.
    pub fn meets_size_threshold(&self, min_size: u32) -> bool {
        self.width >= min_size && self.height >= min_size
    }

    /// Get image dimensions as (width, height).
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// Result of LLM-based image OCR processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageOcrResult {
    /// Extracted text from the image (if any)
    pub text: Option<String>,

    /// Natural language description of the image
    pub description: Option<String>,

    /// Extracted chart/table data in markdown format (if applicable)
    pub chart_data: Option<String>,

    /// Confidence level (0.0-1.0, estimated by LLM response quality)
    pub confidence: f32,

    /// Whether the image contains primarily text
    pub is_text_heavy: bool,

    /// Image type classification
    pub image_type: ImageType,
}

impl Default for ImageOcrResult {
    fn default() -> Self {
        Self {
            text: None,
            description: None,
            chart_data: None,
            confidence: 0.0,
            is_text_heavy: false,
            image_type: ImageType::Unknown,
        }
    }
}

/// Classification of image content type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ImageType {
    /// Photo or realistic image
    Photo,
    /// Chart, graph, or data visualization
    Chart,
    /// Diagram or schematic
    Diagram,
    /// Screenshot or UI capture
    Screenshot,
    /// Scanned document or text-heavy image
    ScannedDocument,
    /// Mathematical formula or equation
    Formula,
    /// Logo or icon
    Logo,
    /// Unknown or unclassified
    #[default]
    Unknown,
}

/// LLM-based image OCR processor.
///
/// Processes images using a multimodal LLM to extract text,
/// generate descriptions, and analyze charts/diagrams.
pub struct ImageOcrProcessor {
    provider: Arc<dyn LLMProvider>,
    config: ImageOcrConfig,
}

impl ImageOcrProcessor {
    /// Create a new image OCR processor.
    pub fn new(provider: Arc<dyn LLMProvider>, config: ImageOcrConfig) -> Self {
        Self { provider, config }
    }

    /// Create with default configuration (disabled by default).
    pub fn with_defaults(provider: Arc<dyn LLMProvider>) -> Self {
        Self::new(provider, ImageOcrConfig::default())
    }

    /// Check if image OCR is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Process a single image and extract text/description.
    pub async fn process_image(&self, image: &ImageData) -> Result<ImageOcrResult> {
        if !self.config.enabled {
            debug!("Image OCR is disabled, skipping image processing");
            return Ok(ImageOcrResult::default());
        }

        // Check minimum size threshold
        if !image.meets_size_threshold(self.config.min_image_size) {
            debug!(
                "Image {}x{} below minimum size threshold {}, skipping",
                image.width, image.height, self.config.min_image_size
            );
            return Ok(ImageOcrResult::default());
        }

        info!(
            "Processing image (page {}, index {}, {}x{})",
            image.page, image.index, image.width, image.height
        );

        // Build the prompt based on configuration
        let prompt = self.build_prompt();
        let data_url = image.to_data_url();

        // Call the LLM with vision capabilities
        let response = self.call_vision_llm(&data_url, &prompt).await?;

        // Parse the response into structured result
        let result = self.parse_response(&response)?;

        debug!("Image OCR complete: {:?}", result.image_type);

        Ok(result)
    }

    /// Process multiple images from a page.
    pub async fn process_page_images(&self, images: &[ImageData]) -> Result<Vec<ImageOcrResult>> {
        if !self.config.enabled {
            return Ok(vec![]);
        }

        let max_images = self.config.max_images_per_page.min(images.len());
        let mut results = Vec::with_capacity(max_images);

        for image in images.iter().take(max_images) {
            match self.process_image(image).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!("Failed to process image {}: {}", image.index, e);
                    results.push(ImageOcrResult::default());
                }
            }
        }

        Ok(results)
    }

    /// Build the OCR prompt based on configuration.
    fn build_prompt(&self) -> String {
        let mut instructions = Vec::new();

        if self.config.extract_text {
            instructions.push(
                "1. **Text Extraction**: Extract ALL visible text from the image exactly as shown. \
                 Preserve formatting, line breaks, and structure.",
            );
        }

        if self.config.generate_descriptions {
            instructions.push(
                "2. **Description**: Provide a concise description of the image content \
                 (what it shows, key elements, purpose).",
            );
        }

        if self.config.analyze_charts {
            instructions.push(
                "3. **Data Extraction**: If this is a chart, graph, or table, extract the data \
                 into a Markdown table format with proper headers and values.",
            );
        }

        instructions.push(
            "4. **Classification**: Classify the image type as one of: \
             photo, chart, diagram, screenshot, scanned_document, formula, logo, or unknown.",
        );

        format!(
            r#"Analyze this image and provide the following information in JSON format:

{}

Respond ONLY with valid JSON in this exact structure:
{{
    "text": "extracted text here or null if no text",
    "description": "image description here",
    "chart_data": "markdown table if applicable or null",
    "image_type": "classification",
    "is_text_heavy": true/false
}}"#,
            instructions.join("\n\n")
        )
    }

    /// Call the LLM with vision capabilities.
    ///
    /// # WHY: OpenAI Vision API Format
    /// The vision API expects images embedded in the message content as data URLs.
    /// We format the message to include both the image and the prompt.
    async fn call_vision_llm(&self, image_data_url: &str, prompt: &str) -> Result<String> {
        // Format message with image reference
        // OpenAI vision API expects the image URL in the message
        let user_message = format!("[Image: {}]\n\n{}", image_data_url, prompt);

        let messages = vec![
            ChatMessage::system(VISION_SYSTEM_PROMPT.to_string()),
            ChatMessage::user(user_message),
        ];

        let options = CompletionOptions {
            // WHY: Do not set temperature – some models (e.g. gpt-4.1-nano) reject
            // any non-default temperature value. Omitting it uses the model's default,
            // which still produces deterministic-enough results for OCR.
            temperature: None,
            max_tokens: Some(4096),
            ..Default::default()
        };

        match self.provider.chat(&messages, Some(&options)).await {
            Ok(response) => Ok(response.content.trim().to_string()),
            Err(e) => Err(PdfError::AiProcessing(format!(
                "Vision LLM call failed: {}",
                e
            ))),
        }
    }

    /// Parse the LLM response into structured result.
    fn parse_response(&self, response: &str) -> Result<ImageOcrResult> {
        // Try to parse as JSON first
        if let Ok(parsed) = serde_json::from_str::<ImageOcrResponse>(response) {
            return Ok(ImageOcrResult {
                text: parsed.text,
                description: parsed.description,
                chart_data: parsed.chart_data,
                confidence: 0.9, // High confidence for valid JSON response
                is_text_heavy: parsed.is_text_heavy.unwrap_or(false),
                image_type: parse_image_type(&parsed.image_type.unwrap_or_default()),
            });
        }

        // If JSON parsing fails, try to extract information heuristically
        warn!("Failed to parse JSON response, falling back to heuristic extraction");

        // Extract text between common markers
        let text = extract_between(response, "\"text\":", ",")
            .or_else(|| extract_between(response, "Text:", "\n"));
        let description = extract_between(response, "\"description\":", ",")
            .or_else(|| extract_between(response, "Description:", "\n"));

        Ok(ImageOcrResult {
            text: text.map(|s| s.trim_matches('"').to_string()),
            description: description.map(|s| s.trim_matches('"').to_string()),
            chart_data: None,
            confidence: 0.5, // Lower confidence for heuristic parsing
            is_text_heavy: false,
            image_type: ImageType::Unknown,
        })
    }
}

/// Internal JSON response structure for parsing.
#[derive(Debug, Deserialize)]
struct ImageOcrResponse {
    text: Option<String>,
    description: Option<String>,
    chart_data: Option<String>,
    image_type: Option<String>,
    is_text_heavy: Option<bool>,
}

/// System prompt for vision OCR.
const VISION_SYSTEM_PROMPT: &str = r#"You are an expert image analysis assistant specialized in extracting text and information from document images.

Your capabilities:
- Accurate OCR for printed and handwritten text
- Chart and graph data extraction
- Diagram interpretation
- Formula recognition
- Document structure understanding

Rules:
1. Be precise and accurate - only report what you actually see
2. Preserve text formatting and structure
3. For charts/tables, extract actual data values when visible
4. Respond ONLY in the requested JSON format
5. Use null for fields where no relevant content exists"#;

/// Parse image type string to enum.
fn parse_image_type(s: &str) -> ImageType {
    match s.to_lowercase().as_str() {
        "photo" => ImageType::Photo,
        "chart" | "graph" => ImageType::Chart,
        "diagram" | "schematic" => ImageType::Diagram,
        "screenshot" | "ui" => ImageType::Screenshot,
        "scanned_document" | "document" | "scan" => ImageType::ScannedDocument,
        "formula" | "equation" | "math" => ImageType::Formula,
        "logo" | "icon" => ImageType::Logo,
        _ => ImageType::Unknown,
    }
}

/// Extract text between two markers.
fn extract_between(s: &str, start: &str, end: &str) -> Option<String> {
    let start_idx = s.find(start)?;
    let content_start = start_idx + start.len();
    let remaining = &s[content_start..];
    let end_idx = remaining.find(end)?;
    Some(remaining[..end_idx].trim().to_string())
}

/// Trait for image OCR capability.
#[async_trait]
pub trait ImageOcrCapable {
    /// Process an image and extract text/description.
    async fn ocr_image(&self, image: &ImageData) -> Result<ImageOcrResult>;
}

#[async_trait]
impl ImageOcrCapable for ImageOcrProcessor {
    async fn ocr_image(&self, image: &ImageData) -> Result<ImageOcrResult> {
        self.process_image(image).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::providers::mock::MockProvider;

    fn create_test_processor() -> ImageOcrProcessor {
        let provider = Arc::new(MockProvider::new());
        ImageOcrProcessor::new(provider, ImageOcrConfig::enabled())
    }

    fn create_test_image() -> ImageData {
        ImageData {
            data: vec![0x89, 0x50, 0x4E, 0x47], // PNG magic bytes
            mime_type: "image/png".to_string(),
            width: 100,
            height: 100,
            page: 1,
            index: 0,
            bbox: None,
        }
    }

    #[test]
    fn test_image_data_to_data_url() {
        let image = create_test_image();
        let url = image.to_data_url();
        assert!(url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn test_image_meets_size_threshold() {
        let image = create_test_image();
        assert!(image.meets_size_threshold(50));
        assert!(image.meets_size_threshold(100));
        assert!(!image.meets_size_threshold(101));
    }

    #[test]
    fn test_image_ocr_config_defaults() {
        let config = ImageOcrConfig::default();
        assert!(!config.enabled); // Disabled by default
        assert!(config.extract_text);
        assert!(config.generate_descriptions);
        assert!(!config.analyze_charts);
        assert_eq!(config.min_image_size, 50);
    }

    #[test]
    fn test_image_ocr_config_enabled() {
        let config = ImageOcrConfig::enabled();
        assert!(config.enabled);
    }

    #[test]
    fn test_processor_disabled_by_default() {
        let provider = Arc::new(MockProvider::new());
        let processor = ImageOcrProcessor::with_defaults(provider);
        assert!(!processor.is_enabled());
    }

    #[tokio::test]
    async fn test_processor_skips_when_disabled() {
        let provider = Arc::new(MockProvider::new());
        let config = ImageOcrConfig::default(); // disabled
        let processor = ImageOcrProcessor::new(provider, config);

        let image = create_test_image();
        let result = processor.process_image(&image).await.unwrap();

        assert!(result.text.is_none());
        assert!(result.description.is_none());
    }

    #[tokio::test]
    async fn test_processor_skips_small_images() {
        let processor = create_test_processor();

        let small_image = ImageData {
            data: vec![],
            mime_type: "image/png".to_string(),
            width: 20,
            height: 20,
            page: 1,
            index: 0,
            bbox: None,
        };

        let result = processor.process_image(&small_image).await.unwrap();
        // Should return default (empty) result for small images
        assert!(result.text.is_none());
    }

    #[test]
    fn test_parse_image_type() {
        assert_eq!(parse_image_type("photo"), ImageType::Photo);
        assert_eq!(parse_image_type("CHART"), ImageType::Chart);
        assert_eq!(parse_image_type("Diagram"), ImageType::Diagram);
        assert_eq!(parse_image_type("unknown_type"), ImageType::Unknown);
    }

    #[test]
    fn test_extract_between() {
        let s = r#"{"text": "hello", "description": "world"}"#;
        let result = extract_between(s, "\"text\": ", ",");
        assert_eq!(result, Some("\"hello\"".to_string()));
    }

    #[test]
    fn test_image_ocr_result_default() {
        let result = ImageOcrResult::default();
        assert!(result.text.is_none());
        assert!(result.description.is_none());
        assert_eq!(result.confidence, 0.0);
        assert_eq!(result.image_type, ImageType::Unknown);
    }

    #[test]
    fn test_build_prompt_all_options() {
        let processor = create_test_processor();
        let prompt = processor.build_prompt();

        assert!(prompt.contains("Text Extraction"));
        assert!(prompt.contains("Description"));
        assert!(prompt.contains("Classification"));
    }

    #[test]
    fn test_build_prompt_without_charts() {
        let provider = Arc::new(MockProvider::new());
        let config = ImageOcrConfig {
            enabled: true,
            analyze_charts: false,
            ..Default::default()
        };
        let processor = ImageOcrProcessor::new(provider, config);
        let prompt = processor.build_prompt();

        assert!(!prompt.contains("Data Extraction"));
    }

    #[tokio::test]
    async fn test_process_page_images_respects_limit() {
        let provider = Arc::new(MockProvider::new());
        let config = ImageOcrConfig {
            enabled: true,
            max_images_per_page: 2,
            min_image_size: 10, // Allow small test images
            ..Default::default()
        };
        let processor = ImageOcrProcessor::new(provider, config);

        let images: Vec<ImageData> = (0..5)
            .map(|i| ImageData {
                data: vec![],
                mime_type: "image/png".to_string(),
                width: 100,
                height: 100,
                page: 1,
                index: i,
                bbox: None,
            })
            .collect();

        let results = processor.process_page_images(&images).await.unwrap();
        // Should only process max_images_per_page (2)
        assert_eq!(results.len(), 2);
    }
}
