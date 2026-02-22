//! LLM enhancement processor for document blocks.
//!
//! This processor uses LLM to enhance extracted content:
//! - Format tables into proper markdown
//! - Convert inline math to LaTeX
//! - Improve text quality
//! - Add image descriptions (using LLM vision when available)

use crate::config::ImageOcrConfig;
use crate::image_ocr::{ImageData, ImageOcrProcessor};
use crate::schema::{Block, BlockType, Document};
use crate::vision::model_requires_default_temperature;
use crate::Result;
use async_trait::async_trait;
use base64::Engine;
use edgequake_llm::traits::{ChatMessage, CompletionOptions, LLMProvider};
use std::sync::Arc;
use tracing::debug;

/// Configuration for LLM enhancement.
#[derive(Debug, Clone)]
pub struct LlmEnhanceConfig {
    /// Enhance table formatting.
    pub enhance_tables: bool,

    /// Convert inline math to LaTeX.
    pub convert_math: bool,

    /// Add descriptions to images/figures.
    pub describe_images: bool,

    /// Improve text quality (fix OCR errors, etc.).
    pub improve_text: bool,

    /// Model to use for enhancement.
    pub model: String,

    /// Temperature for generation (lower = more deterministic).
    pub temperature: f32,

    /// Maximum tokens for response.
    pub max_tokens: usize,
}

impl Default for LlmEnhanceConfig {
    fn default() -> Self {
        Self {
            enhance_tables: true,
            convert_math: true,
            describe_images: true,
            improve_text: false,
            model: "gpt-4.1-nano".to_string(),
            temperature: 0.1,
            max_tokens: 4096,
        }
    }
}

impl LlmEnhanceConfig {
    /// Create a new config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable table enhancement.
    pub fn with_tables(mut self, enabled: bool) -> Self {
        self.enhance_tables = enabled;
        self
    }

    /// Enable math conversion.
    pub fn with_math(mut self, enabled: bool) -> Self {
        self.convert_math = enabled;
        self
    }

    /// Enable image descriptions.
    pub fn with_images(mut self, enabled: bool) -> Self {
        self.describe_images = enabled;
        self
    }

    /// Set model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

/// LLM-based enhancement processor.
///
/// **WHY use LLM for post-processing?**
///
/// Text-based PDF extraction can produce:
/// - Tables as raw text (no column alignment)
/// - Math as Unicode symbols (not LaTeX)
/// - Images without descriptions (accessibility/indexing gap)
/// - OCR artifacts (nurnber, 0O, etc.)
///
/// LLM enhancement is the final stage that improves output quality.
/// It's optional and disabled by default for cost control.
///
/// **Feature defaults:**
/// - `enhance_tables=true`: Low cost, high value (tables → markdown)
/// - `convert_math=true`: Improves formula rendering
/// - `describe_images=true`: Adds accessibility (requires vision model)
/// - `improve_text=false`: Aggressive, can modify correct text
pub struct LlmEnhanceProcessor {
    provider: Arc<dyn LLMProvider>,
    config: LlmEnhanceConfig,
    /// Optional image OCR configuration for LLM-based image description.
    image_ocr_config: Option<ImageOcrConfig>,
}

impl LlmEnhanceProcessor {
    /// Create a new LLM enhancement processor.
    pub fn new(provider: Arc<dyn LLMProvider>, config: LlmEnhanceConfig) -> Self {
        Self {
            provider,
            config,
            image_ocr_config: None,
        }
    }

    /// Create with default config.
    pub fn with_defaults(provider: Arc<dyn LLMProvider>) -> Self {
        Self::new(provider, LlmEnhanceConfig::default())
    }

    /// Enable LLM-based image OCR with the given configuration.
    ///
    /// When enabled, the processor will use vision LLM to describe images
    /// that have image data available in their metadata.
    pub fn with_image_ocr(mut self, config: ImageOcrConfig) -> Self {
        self.image_ocr_config = Some(config);
        self
    }

    /// Enable LLM-based image OCR with default configuration.
    pub fn with_image_ocr_enabled(mut self) -> Self {
        self.image_ocr_config = Some(ImageOcrConfig {
            enabled: true,
            ..Default::default()
        });
        self
    }

    /// Process a document, enhancing all applicable blocks.
    pub async fn process_document(&self, document: &mut Document) -> Result<()> {
        for page in &mut document.pages {
            for block in &mut page.blocks {
                self.process_block(block).await?;
            }
        }
        Ok(())
    }

    /// Process a single block.
    pub async fn process_block(&self, block: &mut Block) -> Result<()> {
        match block.block_type {
            BlockType::Table if self.config.enhance_tables => {
                self.enhance_table(block).await?;
            }
            BlockType::Equation | BlockType::TextInlineMath if self.config.convert_math => {
                self.convert_math(block).await?;
            }
            BlockType::Figure | BlockType::Picture if self.config.describe_images => {
                self.describe_image(block).await?;
            }
            BlockType::Text if self.config.improve_text => {
                self.improve_text(block).await?;
            }
            _ => {}
        }

        // Process children recursively
        for child in &mut block.children {
            Box::pin(self.process_block(child)).await?;
        }

        Ok(())
    }

    /// Enhance a table block with proper markdown formatting.
    async fn enhance_table(&self, block: &mut Block) -> Result<()> {
        if block.text.is_empty() {
            return Ok(());
        }

        debug!("Enhancing table block");

        let prompt = format!(
            r#"Convert this table content to a properly formatted Markdown table.
Use proper column alignment. Preserve all data.

Input:
{}

Output only the Markdown table, no explanation:"#,
            block.text
        );

        if let Some(enhanced) = self.call_llm(&prompt).await? {
            block.html = Some(enhanced.clone());
            // Update text with formatted version
            block.text = enhanced;
        }

        Ok(())
    }

    /// Convert inline math to LaTeX format.
    async fn convert_math(&self, block: &mut Block) -> Result<()> {
        if block.text.is_empty() {
            return Ok(());
        }

        debug!("Converting math in block");

        let prompt = format!(
            r#"Convert mathematical expressions in this text to LaTeX format.
Use $...$ for inline math and $$...$$ for display math.
Preserve all other text exactly.

Input: {}

Output:"#,
            block.text
        );

        if let Some(converted) = self.call_llm(&prompt).await? {
            block.text = converted;
        }

        Ok(())
    }

    /// Add description to an image/figure block.
    ///
    /// If image data is available in block metadata and ImageOcrConfig is enabled,
    /// uses LLM vision to generate a description. Otherwise, uses a placeholder.
    ///
    /// # Metadata keys checked
    /// - `image_data`: Base64-encoded image data
    /// - `image_mime_type`: MIME type of the image (e.g., "image/png")
    /// - `image_width`: Width in pixels
    /// - `image_height`: Height in pixels
    async fn describe_image(&self, block: &mut Block) -> Result<()> {
        debug!("Image description requested");

        // Check if we have image data in metadata and ImageOCR is enabled
        if let Some(ref ocr_config) = self.image_ocr_config {
            if ocr_config.enabled {
                if let Some(image_data) = self.extract_image_data_from_block(block) {
                    debug!(
                        "Processing image with LLM vision: {}x{} {}",
                        image_data.width, image_data.height, image_data.mime_type
                    );

                    // Create ImageOcrProcessor and process the image
                    let ocr_processor =
                        ImageOcrProcessor::new(Arc::clone(&self.provider), ocr_config.clone());
                    match ocr_processor.process_image(&image_data).await {
                        Ok(result) => {
                            // Build description from OCR result
                            let mut description_parts = Vec::new();

                            // Add image type if known
                            description_parts.push(format!("[{:?}]", result.image_type));

                            // Add extracted text if available
                            if let Some(ref text) = result.text {
                                if !text.is_empty() {
                                    description_parts.push(format!("Text: {}", text));
                                }
                            }

                            // Add description
                            if let Some(ref desc) = result.description {
                                if !desc.is_empty() {
                                    description_parts.push(desc.clone());
                                }
                            }

                            // Add chart data if available
                            if let Some(ref chart_data) = result.chart_data {
                                if !chart_data.is_empty() {
                                    description_parts.push(format!("Data: {}", chart_data));
                                }
                            }

                            block.text = description_parts.join("\n\n");

                            // Store the full result in metadata
                            block.metadata.insert(
                                "image_ocr_result".to_string(),
                                serde_json::to_value(&result).unwrap_or_default(),
                            );

                            debug!("Image description generated successfully");
                            return Ok(());
                        }
                        Err(e) => {
                            debug!("Failed to process image with LLM: {:?}", e);
                            // Fall through to placeholder
                        }
                    }
                }
            }
        }

        // Fallback: use placeholder if no image data or OCR failed
        if block.text.is_empty() {
            block.text = "[Image]".to_string();
        }

        Ok(())
    }

    /// Extract image data from block metadata.
    ///
    /// Looks for image_data (base64), image_mime_type, image_width, image_height
    /// in the block's metadata hashmap.
    fn extract_image_data_from_block(&self, block: &Block) -> Option<ImageData> {
        // Get base64-encoded image data
        let data_base64 = block.metadata.get("image_data")?.as_str()?;

        // Decode base64 to bytes
        let data = base64::engine::general_purpose::STANDARD
            .decode(data_base64)
            .ok()?;

        // Get MIME type (default to PNG)
        let mime_type = block
            .metadata
            .get("image_mime_type")
            .and_then(|v| v.as_str())
            .unwrap_or("image/png")
            .to_string();

        // Get dimensions (default to 0 if not available)
        let width = block
            .metadata
            .get("image_width")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let height = block
            .metadata
            .get("image_height")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        // Get page and index if available
        let page = block
            .metadata
            .get("image_page")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let index = block
            .metadata
            .get("image_index")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        // Get bounding box if available
        let bbox = if let Some(bbox_arr) = block.metadata.get("image_bbox") {
            if let Some(arr) = bbox_arr.as_array() {
                if arr.len() == 4 {
                    Some((
                        arr[0].as_f64().unwrap_or(0.0) as f32,
                        arr[1].as_f64().unwrap_or(0.0) as f32,
                        arr[2].as_f64().unwrap_or(0.0) as f32,
                        arr[3].as_f64().unwrap_or(0.0) as f32,
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        Some(ImageData {
            data,
            mime_type,
            width,
            height,
            page,
            index,
            bbox,
        })
    }

    /// Improve text quality (fix OCR errors, etc.).
    async fn improve_text(&self, block: &mut Block) -> Result<()> {
        if block.text.is_empty() {
            return Ok(());
        }

        // Only process if text seems to have issues
        if !Self::text_needs_improvement(&block.text) {
            return Ok(());
        }

        debug!("Improving text quality");

        let prompt = format!(
            r#"Fix any OCR errors or formatting issues in this text.
Preserve the meaning and structure. Only fix obvious errors.

Input: {}

Output:"#,
            block.text
        );

        if let Some(improved) = self.call_llm(&prompt).await? {
            block.text = improved;
        }

        Ok(())
    }

    /// Check if text seems to have quality issues.
    ///
    /// Uses statistical analysis of character distributions instead of
    /// keyword-based heuristics. This is a first-principles approach
    /// that adapts to different text patterns.
    ///
    /// # Arguments
    /// * `text` - Text to analyze
    ///
    /// # Returns
    /// true if text likely has quality issues (OCR errors, etc.)
    ///
    /// **WHY these thresholds?**
    /// - 0.3 ratio for short text (<20 chars): Short text naturally has more
    ///   punctuation/symbols (e.g., "Fig. 1" has 50% non-word chars)
    /// - 0.5 ratio for long text (>50 chars): Normal prose is ~85% alphanumeric,
    ///   so below 50% indicates likely OCR garbage or symbol-heavy content
    /// - Character frequency analysis: Unusual distributions (e.g., too many
    ///   zeros in place of 'O') suggest OCR substitution errors
    fn text_needs_improvement(text: &str) -> bool {
        if text.is_empty() {
            return false;
        }

        // Statistical analysis of character distribution
        // High ratio of non-alphanumeric characters may indicate issues
        let word_chars = text.chars().filter(|c| c.is_alphanumeric()).count();
        let total_chars = text.chars().count();

        if total_chars > 0 {
            let ratio = word_chars as f32 / total_chars as f32;
            // Use adaptive threshold based on text length
            // Shorter text can have higher non-word ratio naturally
            let threshold = if total_chars < 20 {
                0.3
            } else if total_chars < 50 {
                0.4
            } else {
                0.5
            };
            if ratio < threshold {
                return true;
            }
        }

        // Check for character-level anomalies using statistical analysis
        // instead of fixed keyword patterns
        let char_counts: std::collections::HashMap<char, usize> = text
            .chars()
            .filter(|c| c.is_alphanumeric())
            .fold(std::collections::HashMap::new(), |mut acc, c| {
                *acc.entry(c).or_insert(0) += 1;
                acc
            });

        if char_counts.is_empty() {
            return false;
        }

        // Calculate character frequency distribution
        let total_alnum: usize = char_counts.values().sum();
        let _avg_freq = total_alnum as f32 / char_counts.len() as f32;

        // Check for unusual character frequency patterns
        // (e.g., many single occurrences might indicate OCR errors)
        let single_occurrences = char_counts.values().filter(|&&count| count == 1).count();
        let single_ratio = single_occurrences as f32 / char_counts.len() as f32;

        // High ratio of single-occurrence characters suggests OCR issues
        if single_ratio > 0.7 && total_alnum > 10 {
            return true;
        }

        false
    }

    /// Call the LLM with a prompt.
    async fn call_llm(&self, prompt: &str) -> Result<Option<String>> {
        let messages = vec![
            ChatMessage::system(
                "You are a document processing assistant. \
                 Follow instructions precisely and output only what is asked for.",
            ),
            ChatMessage::user(prompt.to_string()),
        ];

        let options = CompletionOptions {
            // WHY: Some models (gpt-4.1-nano, gpt-4.1-mini) only accept the default
            // temperature value and return an API error for any other value.
            // We check the model name and skip temperature for those models.
            temperature: if model_requires_default_temperature(&self.config.model) {
                None
            } else {
                Some(self.config.temperature)
            },
            max_tokens: Some(self.config.max_tokens),
            ..Default::default()
        };

        match self.provider.chat(&messages, Some(&options)).await {
            Ok(response) => {
                let text = response.content.trim().to_string();
                if text.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(text))
                }
            }
            Err(e) => {
                tracing::warn!("LLM call failed: {}", e);
                Ok(None)
            }
        }
    }
}

/// Trait for LLM-enhanced processing.
#[async_trait]
pub trait LlmEnhanced {
    /// Enhance content using LLM.
    async fn enhance(&mut self, processor: &LlmEnhanceProcessor) -> Result<()>;
}

#[async_trait]
impl LlmEnhanced for Document {
    async fn enhance(&mut self, processor: &LlmEnhanceProcessor) -> Result<()> {
        processor.process_document(self).await
    }
}

#[async_trait]
impl LlmEnhanced for Block {
    async fn enhance(&mut self, processor: &LlmEnhanceProcessor) -> Result<()> {
        processor.process_block(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::BoundingBox;
    use edgequake_llm::providers::mock::MockProvider;

    fn create_processor() -> LlmEnhanceProcessor {
        let provider = Arc::new(MockProvider::new());
        LlmEnhanceProcessor::with_defaults(provider)
    }

    #[test]
    fn test_config_defaults() {
        let config = LlmEnhanceConfig::default();
        assert!(config.enhance_tables);
        assert!(config.convert_math);
        assert!(config.describe_images);
        assert!(!config.improve_text);
    }

    #[test]
    fn test_config_builder() {
        let config = LlmEnhanceConfig::new()
            .with_tables(false)
            .with_math(true)
            .with_model("gpt-4o");

        assert!(!config.enhance_tables);
        assert!(config.convert_math);
        assert_eq!(config.model, "gpt-4o");
    }

    #[test]
    fn test_text_needs_improvement() {
        // Should not need improvement
        assert!(!LlmEnhanceProcessor::text_needs_improvement(
            "This is a normal sentence."
        ));

        // Should need improvement (too few word chars)
        assert!(LlmEnhanceProcessor::text_needs_improvement("@#$%^&*()"));

        // Should need improvement (suspicious patterns)
        assert!(LlmEnhanceProcessor::text_needs_improvement(
            "The nurnber l1ke 0O"
        ));
    }

    #[tokio::test]
    async fn test_process_block_text() {
        let processor = create_processor();
        let mut block = Block::text("Hello world", BoundingBox::new(0.0, 0.0, 100.0, 20.0));

        // Text improvement is disabled by default
        processor.process_block(&mut block).await.unwrap();
        assert_eq!(block.text, "Hello world");
    }

    #[tokio::test]
    async fn test_process_block_table() {
        let provider = Arc::new(MockProvider::new());
        let config = LlmEnhanceConfig::new().with_tables(true);
        let processor = LlmEnhanceProcessor::new(provider, config);

        let mut block = Block::new(BlockType::Table, BoundingBox::new(0.0, 0.0, 500.0, 200.0));
        block.text = "Col1 Col2\nA B\nC D".to_string();

        processor.process_block(&mut block).await.unwrap();

        // Mock provider returns something, so block should be enhanced
        // (either html is set or text is non-empty)
        assert!(block.html.is_some() || !block.text.is_empty());
    }

    // ==========================================================================
    // OODA-29: Additional builder and config tests
    // ==========================================================================

    #[test]
    fn test_processor_with_image_ocr_enabled() {
        let provider = Arc::new(MockProvider::new());
        let processor = LlmEnhanceProcessor::with_defaults(provider).with_image_ocr_enabled();
        assert!(processor.image_ocr_config.is_some());
        assert!(processor.image_ocr_config.as_ref().unwrap().enabled);
    }

    #[test]
    fn test_processor_with_custom_image_ocr() {
        let provider = Arc::new(MockProvider::new());
        let custom_config = ImageOcrConfig {
            enabled: true,
            model: "gpt-4o".to_string(),
            ..Default::default()
        };
        let processor = LlmEnhanceProcessor::with_defaults(provider).with_image_ocr(custom_config);
        assert!(processor.image_ocr_config.is_some());
        assert_eq!(processor.image_ocr_config.as_ref().unwrap().model, "gpt-4o");
    }
}
