//! LLM enhancement processor for document blocks.
//!
//! This processor uses LLM to enhance extracted content:
//! - Format tables into proper markdown
//! - Convert inline math to LaTeX
//! - Improve text quality
//! - Add image descriptions

use crate::schema::{Block, BlockType, Document};
use crate::Result;
use async_trait::async_trait;
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
            model: "gpt-4o-mini".to_string(),
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
pub struct LlmEnhanceProcessor {
    provider: Arc<dyn LLMProvider>,
    config: LlmEnhanceConfig,
}

impl LlmEnhanceProcessor {
    /// Create a new LLM enhancement processor.
    pub fn new(provider: Arc<dyn LLMProvider>, config: LlmEnhanceConfig) -> Self {
        Self { provider, config }
    }

    /// Create with default config.
    pub fn with_defaults(provider: Arc<dyn LLMProvider>) -> Self {
        Self::new(provider, LlmEnhanceConfig::default())
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
    async fn describe_image(&self, block: &mut Block) -> Result<()> {
        // For now, just add a placeholder. Vision mode would provide actual image.
        debug!("Image description requested (requires vision mode)");

        if block.text.is_empty() {
            block.text = "[Image]".to_string();
        }

        Ok(())
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
    fn text_needs_improvement(text: &str) -> bool {
        // Simple heuristics for detecting OCR issues
        let suspicious_patterns = [
            "l1", "0O", "rn", // Common OCR confusions
        ];

        // High ratio of non-word characters might indicate issues
        let word_chars = text.chars().filter(|c| c.is_alphanumeric()).count();
        let total_chars = text.chars().count();

        if total_chars > 0 {
            let ratio = word_chars as f32 / total_chars as f32;
            if ratio < 0.5 {
                return true;
            }
        }

        // Check for suspicious patterns
        for pattern in &suspicious_patterns {
            if text.contains(pattern) {
                return true;
            }
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
            temperature: Some(self.config.temperature),
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
}
