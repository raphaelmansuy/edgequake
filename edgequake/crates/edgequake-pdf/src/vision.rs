//! Vision mode support for PDF extraction.
//!
//! Vision mode uses a multimodal LLM to extract content from PDF page images.
//! This is useful for:
//! - Scanned documents (OCR)
//! - Complex layouts
//! - Documents with poor text extraction
//!
//! ## Implements
//!
//! - **FEAT1010**: Vision-based PDF extraction
//! - **FEAT1011**: Multi-page image extraction
//! - **FEAT1012**: LLM-powered layout understanding
//!
//! ## Enforces
//!
//! - **BR1010**: Fallback to text extraction if vision fails
//! - **BR1011**: Image resolution capped at 2048px

use crate::error::PdfError;
use crate::progress::ProgressCallback;
use crate::schema::{Block, BlockType, BoundingBox, Document, ExtractionMethod, Page};
use crate::Result;
use async_trait::async_trait;
use base64::Engine;
use edgequake_llm::traits::{ChatMessage, CompletionOptions, ImageData, LLMProvider};
use std::sync::Arc;
use tracing::{debug, info};

/// Image format for page rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// PNG format (lossless, best quality)
    Png,
    /// JPEG format (lossy, smaller size)
    Jpeg,
    /// WebP format (modern, good compression)
    WebP,
}

impl ImageFormat {
    /// Get the MIME type for this format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::WebP => "image/webp",
        }
    }

    /// Get the file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::WebP => "webp",
        }
    }
}

/// A rendered page image.
#[derive(Debug, Clone)]
pub struct PageImage {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Raw image data (encoded).
    pub data: Vec<u8>,
    /// Image format.
    pub format: ImageFormat,
    /// Page number (0-indexed).
    pub page: usize,
    /// DPI used for rendering.
    pub dpi: u32,
}

impl PageImage {
    /// Create a new page image.
    pub fn new(data: Vec<u8>, width: u32, height: u32, format: ImageFormat) -> Self {
        Self {
            width,
            height,
            data,
            format,
            page: 0,
            dpi: 72,
        }
    }

    /// Set the page number.
    pub fn with_page(mut self, page: usize) -> Self {
        self.page = page;
        self
    }

    /// Set the DPI.
    pub fn with_dpi(mut self, dpi: u32) -> Self {
        self.dpi = dpi;
        self
    }

    /// Encode the image as base64.
    pub fn to_base64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(&self.data)
    }

    /// Get a data URL for embedding in HTML/Markdown.
    pub fn to_data_url(&self) -> String {
        format!(
            "data:{};base64,{}",
            self.format.mime_type(),
            self.to_base64()
        )
    }
}

/// Configuration for vision mode extraction.
#[derive(Debug, Clone)]
/// @implements FEAT1024
pub struct VisionConfig {
    /// Model to use for vision (must support images).
    pub model: String,
    /// DPI for rendering pages.
    pub dpi: u32,
    /// Temperature for generation.
    pub temperature: f32,
    /// Maximum tokens for response.
    pub max_tokens: usize,
    /// Custom prompt for extraction.
    pub prompt: Option<String>,
    /// Whether to extract tables.
    pub extract_tables: bool,
    /// Whether to extract equations as LaTeX.
    pub extract_equations: bool,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            dpi: 150,
            temperature: 0.1,
            max_tokens: 8192,
            prompt: None,
            extract_tables: true,
            extract_equations: true,
        }
    }
}

impl VisionConfig {
    /// Create a new config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the DPI for rendering.
    pub fn with_dpi(mut self, dpi: u32) -> Self {
        self.dpi = dpi;
        self
    }

    /// Set the temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Returns the temperature to pass to the LLM, or None if the model
    /// requires only the default temperature (e.g., gpt-4.1-nano, gpt-4.1-mini).
    ///
    /// WHY: Some OpenAI models (gpt-4.1-nano, gpt-4.1-mini) reject any temperature
    /// value other than the default (1.0), returning API error
    /// "'temperature' does not support X with this model. Only the default (1) value is supported."
    pub fn effective_temperature(&self) -> Option<f32> {
        if model_requires_default_temperature(&self.model) {
            None
        } else {
            Some(self.temperature)
        }
    }
}

/// Returns true if the model only accepts the default temperature (1.0).
///
/// WHY: OpenAI's gpt-4.1-nano and gpt-4.1-mini series reject custom temperature
/// values. Sending temperature=0.1 causes an API error. We detect these models
/// by name and skip the temperature parameter entirely.
pub fn model_requires_default_temperature(model: &str) -> bool {
    let lower = model.to_lowercase();
    // gpt-4.1-nano and gpt-4.1-mini only accept default temperature
    lower.contains("gpt-4.1-nano")
        || lower.contains("gpt-4.1-mini")
        || lower.contains("o1-")
        || lower.starts_with("o1")
        || lower.contains("o4-")
        || lower.starts_with("o4")
}

/// Vision-based document extractor.
///
/// Uses a multimodal LLM to extract content from page images.
pub struct VisionExtractor {
    provider: Arc<dyn LLMProvider>,
    config: VisionConfig,
}

impl VisionExtractor {
    /// Create a new vision extractor.
    pub fn new(provider: Arc<dyn LLMProvider>, config: VisionConfig) -> Self {
        Self { provider, config }
    }

    /// Create with default config.
    pub fn with_defaults(provider: Arc<dyn LLMProvider>) -> Self {
        Self::new(provider, VisionConfig::default())
    }

    /// Extract document from PDF bytes using vision mode.
    ///
    /// This renders PDF pages to images and processes them with a vision LLM.
    /// Requires the `vision` feature to be enabled.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use edgequake_pdf::{VisionExtractor, VisionConfig};
    /// use edgequake_llm::providers::mock::MockProvider;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = Arc::new(MockProvider::new());
    /// let config = VisionConfig::default().with_model("gpt-4o-mini");
    /// let extractor = VisionExtractor::new(provider, config);
    ///
    /// let pdf_bytes = std::fs::read("document.pdf")?;
    /// let document = extractor.extract_from_pdf(&pdf_bytes).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "vision")]
    pub async fn extract_from_pdf(&self, pdf_bytes: &[u8]) -> Result<Document> {
        info!("Extracting document from PDF using vision mode");

        // 1. Render pages to images
        let renderer = crate::rendering::PageRenderer::new()?
            .with_dpi(self.config.dpi)
            .with_format(ImageFormat::Png);

        let images = renderer.render_pages(pdf_bytes)?;
        info!("Rendered {} pages to images", images.len());

        // 2. Extract from rendered images (existing method)
        self.extract_from_images(&images).await
    }

    /// Extract document from PDF bytes using vision mode (stub when feature disabled).
    #[cfg(not(feature = "vision"))]
    pub async fn extract_from_pdf(&self, _pdf_bytes: &[u8]) -> Result<Document> {
        Err(PdfError::Unsupported(
            "Vision mode requires the 'vision' feature flag. \
             Recompile edgequake-pdf with --features vision"
                .into(),
        ))
    }

    /// OODA-11: Extract document from PDF with progress callbacks.
    ///
    /// Like `extract_from_pdf` but emits page-by-page progress events via the
    /// provided callback. Use this when you need real-time visibility into
    /// vision extraction progress for UI display.
    ///
    /// # Arguments
    /// * `pdf_bytes` - Raw PDF bytes to extract
    /// * `progress` - Callback for progress notifications
    ///
    /// # Progress Events
    /// - `on_extraction_start(total_pages)` - Called after rendering pages
    /// - `on_page_start(page_num, total)` - Called before processing each page
    /// - `on_page_complete(page_num, content_len)` - Called after successful extraction
    /// - `on_page_error(page_num, error)` - Called if page extraction fails
    /// - `on_extraction_complete(total, success_count)` - Called at the end
    #[cfg(feature = "vision")]
    pub async fn extract_from_pdf_with_progress<P>(
        &self,
        pdf_bytes: &[u8],
        progress: Arc<P>,
    ) -> Result<Document>
    where
        P: ProgressCallback + ?Sized,
    {
        info!("Extracting document from PDF using vision mode with progress");

        // 1. Render pages to images
        let renderer = crate::rendering::PageRenderer::new()?
            .with_dpi(self.config.dpi)
            .with_format(ImageFormat::Png);

        let images = renderer.render_pages(pdf_bytes)?;
        info!("Rendered {} pages to images", images.len());

        // 2. Emit extraction start
        progress.on_extraction_start(images.len());

        // 3. Extract with progress callbacks
        self.extract_from_images_with_progress(&images, progress)
            .await
    }

    /// OODA-11: Stub for extract_from_pdf_with_progress when vision disabled.
    #[cfg(not(feature = "vision"))]
    pub async fn extract_from_pdf_with_progress<P>(
        &self,
        _pdf_bytes: &[u8],
        _progress: Arc<P>,
    ) -> Result<Document>
    where
        P: ProgressCallback + ?Sized,
    {
        Err(PdfError::Unsupported(
            "Vision mode requires the 'vision' feature flag. \
             Recompile edgequake-pdf with --features vision"
                .into(),
        ))
    }

    /// Extract a document from pre-rendered page images.
    pub async fn extract_from_images(&self, images: &[PageImage]) -> Result<Document> {
        info!("Extracting document from {} page images", images.len());

        let mut document = Document::new();
        document.method = ExtractionMethod::Vision;

        for image in images {
            let page = self.extract_page(image).await?;
            document.add_page(page);
        }

        document.update_stats();
        document.generate_toc();

        Ok(document)
    }

    /// OODA-11: Extract document from images with progress callbacks.
    ///
    /// This is the core extraction loop with progress reporting. Used by
    /// `extract_from_pdf_with_progress` and can be called directly if you
    /// have pre-rendered images.
    ///
    /// # Error Handling
    /// Unlike `extract_from_images`, this method continues on page errors
    /// (emitting `on_page_error`) and reports the success count at completion.
    pub async fn extract_from_images_with_progress<P>(
        &self,
        images: &[PageImage],
        progress: Arc<P>,
    ) -> Result<Document>
    where
        P: ProgressCallback + ?Sized,
    {
        info!(
            "Extracting document from {} page images with progress",
            images.len()
        );

        let mut document = Document::new();
        document.method = ExtractionMethod::Vision;
        let mut success_count = 0;
        let total = images.len();

        for (idx, image) in images.iter().enumerate() {
            let page_num = idx + 1;
            progress.on_page_start(page_num, total);

            match self.extract_page(image).await {
                Ok(page) => {
                    let content_len = page.get_text().len();
                    progress.on_page_complete(page_num, content_len);
                    document.add_page(page);
                    success_count += 1;
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    progress.on_page_error(page_num, &error_msg);
                    // WHY: Vision extraction continues on page errors to maximize
                    // content extraction. User is informed via progress callback.
                }
            }
        }

        progress.on_extraction_complete(total, success_count);

        document.update_stats();
        document.generate_toc();

        Ok(document)
    }

    /// Extract a single page from an image.
    pub async fn extract_page(&self, image: &PageImage) -> Result<Page> {
        debug!("Extracting page {} via vision mode", image.page);

        let markdown = self.extract_markdown(image).await?;
        let page = self.parse_markdown_to_page(&markdown, image);

        Ok(page)
    }

    /// Extract markdown from a page image.
    async fn extract_markdown(&self, image: &PageImage) -> Result<String> {
        let prompt = self
            .config
            .prompt
            .as_deref()
            .unwrap_or(DEFAULT_VISION_PROMPT);

        // Build the multimodal message with proper ImageData (OODA-51).
        // WHY: edgequake-llm 0.2.3 supports ChatMessage::user_with_images() which
        // sends images as structured multipart content blocks via the OpenAI image_url
        // API — required for vision-capable models (gpt-4o, gpt-4.1, etc.).
        // Embedding the data-URL as plain text was silently ignored by non-vision models.
        let image_data = ImageData::new(image.to_base64(), image.format.mime_type());

        let messages = vec![
            ChatMessage::system(VISION_SYSTEM_PROMPT.to_string()),
            ChatMessage::user_with_images(prompt, vec![image_data]),
        ];

        let options = CompletionOptions {
            // WHY: Some models (gpt-4.1-nano, gpt-4.1-mini) only accept the default
            // temperature (1.0) and will return an API error if a different value is sent.
            // effective_temperature() returns None for those models so no temperature
            // parameter is sent in the request.
            temperature: self.config.effective_temperature(),
            max_tokens: Some(self.config.max_tokens),
            ..Default::default()
        };

        match self.provider.chat(&messages, Some(&options)).await {
            Ok(response) => Ok(response.content.trim().to_string()),
            Err(e) => Err(PdfError::AiProcessing(format!(
                "Vision extraction failed: {}",
                e
            ))),
        }
    }

    /// Parse extracted markdown into a Page with blocks.
    fn parse_markdown_to_page(&self, markdown: &str, image: &PageImage) -> Page {
        // Calculate page dimensions from image and DPI
        let width = image.width as f32 * 72.0 / image.dpi as f32;
        let height = image.height as f32 * 72.0 / image.dpi as f32;

        let mut page = Page::new(image.page + 1, width, height);
        page.method = ExtractionMethod::Vision;

        // Parse markdown into blocks
        let mut y_pos = 72.0;
        let line_height = 14.0;
        let margin = 72.0;

        for (position, line) in markdown.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                y_pos += line_height;
                continue;
            }

            let bbox = BoundingBox::new(margin, y_pos, width - margin, y_pos + line_height);
            let (block_type, level, text) = Self::classify_markdown_line(trimmed);

            let mut block = Block::new(block_type, bbox);
            block.text = text;
            block.page = image.page;
            block.position = position;
            block.level = level;
            block.confidence = 0.9; // Vision extraction confidence

            page.add_block(block);
            y_pos += line_height * 1.5;
        }

        page.update_stats();
        page
    }

    /// Classify a markdown line into block type.
    fn classify_markdown_line(line: &str) -> (BlockType, Option<u8>, String) {
        // Headers
        if let Some(stripped) = line.strip_prefix("#### ") {
            return (BlockType::SectionHeader, Some(4), stripped.to_string());
        }
        if let Some(stripped) = line.strip_prefix("### ") {
            return (BlockType::SectionHeader, Some(3), stripped.to_string());
        }
        if let Some(stripped) = line.strip_prefix("## ") {
            return (BlockType::SectionHeader, Some(2), stripped.to_string());
        }
        if let Some(stripped) = line.strip_prefix("# ") {
            return (BlockType::SectionHeader, Some(1), stripped.to_string());
        }

        // List items
        if line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ") {
            return (BlockType::ListItem, None, line.to_string());
        }
        if line
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
            && (line.contains(". ") || line.contains(") "))
        {
            return (BlockType::ListItem, None, line.to_string());
        }

        // Code blocks
        if line.starts_with("```") {
            return (BlockType::Code, None, line.to_string());
        }
        if line.starts_with("    ") || line.starts_with("\t") {
            return (BlockType::Code, None, line.to_string());
        }

        // Equations (LaTeX)
        if line.starts_with("$$") || line.contains("\\begin{") {
            return (BlockType::Equation, None, line.to_string());
        }

        // Tables
        if line.starts_with("|") && line.ends_with("|") {
            return (BlockType::Table, None, line.to_string());
        }

        // Image descriptions
        if line.starts_with("![") || line.starts_with("[Image:") {
            return (BlockType::Figure, None, line.to_string());
        }

        // Default to text
        (BlockType::Text, None, line.to_string())
    }
}

/// Trait for providers that support vision mode.
#[async_trait]
pub trait VisionCapable {
    /// Check if the provider supports vision/image input.
    fn supports_vision(&self) -> bool;

    /// Get the list of vision-capable models.
    fn vision_models(&self) -> Vec<&str>;
}

const VISION_SYSTEM_PROMPT: &str = r#"You are a document parser that converts document images to clean Markdown format.
Be precise and accurate. Preserve all text content exactly as shown.
Handle multi-column layouts by reading left-to-right, top-to-bottom within each column.
"#;

const DEFAULT_VISION_PROMPT: &str = r#"Convert this document page to clean Markdown format.

Instructions:
1. Preserve the document structure (headers, paragraphs, lists)
2. Format tables using Markdown table syntax with proper alignment
3. Convert equations to LaTeX: inline math uses $...$ and display math uses $$...$$
4. Describe images with [Image: brief description]
5. Handle multi-column layouts by reading columns left-to-right
6. Remove headers, footers, and page numbers
7. Keep all text content accurate - do not paraphrase

Output the Markdown content only, no explanations:"#;

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::providers::mock::MockProvider;

    fn create_extractor() -> VisionExtractor {
        let provider = Arc::new(MockProvider::new());
        VisionExtractor::with_defaults(provider)
    }

    #[test]
    fn test_vision_config_defaults() {
        let config = VisionConfig::default();
        assert_eq!(config.dpi, 150);
        assert_eq!(config.model, "gpt-4o");
        assert!(config.extract_tables);
    }

    #[test]
    fn test_vision_config_builder() {
        let config = VisionConfig::new()
            .with_model("claude-3-opus")
            .with_dpi(300)
            .with_temperature(0.0);

        assert_eq!(config.model, "claude-3-opus");
        assert_eq!(config.dpi, 300);
        assert_eq!(config.temperature, 0.0);
    }

    #[test]
    fn test_image_format_mime_type() {
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::WebP.mime_type(), "image/webp");
    }

    #[test]
    fn test_page_image_to_base64() {
        let image = PageImage::new(vec![0x89, 0x50, 0x4E, 0x47], 100, 100, ImageFormat::Png);
        let base64 = image.to_base64();
        assert!(!base64.is_empty());
    }

    #[test]
    fn test_page_image_to_data_url() {
        let image = PageImage::new(vec![0xFF, 0xD8, 0xFF], 100, 100, ImageFormat::Jpeg);
        let url = image.to_data_url();
        assert!(url.starts_with("data:image/jpeg;base64,"));
    }

    #[test]
    fn test_classify_markdown_headers() {
        let (block_type, level, text) = VisionExtractor::classify_markdown_line("# Title");
        assert_eq!(block_type, BlockType::SectionHeader);
        assert_eq!(level, Some(1));
        assert_eq!(text, "Title");

        let (block_type, level, _) = VisionExtractor::classify_markdown_line("## Subtitle");
        assert_eq!(block_type, BlockType::SectionHeader);
        assert_eq!(level, Some(2));
    }

    #[test]
    fn test_classify_markdown_lists() {
        let (block_type, _, _) = VisionExtractor::classify_markdown_line("- Item");
        assert_eq!(block_type, BlockType::ListItem);

        let (block_type, _, _) = VisionExtractor::classify_markdown_line("1. First");
        assert_eq!(block_type, BlockType::ListItem);
    }

    #[test]
    fn test_classify_markdown_code() {
        let (block_type, _, _) = VisionExtractor::classify_markdown_line("```python");
        assert_eq!(block_type, BlockType::Code);

        let (block_type, _, _) = VisionExtractor::classify_markdown_line("    indented code");
        assert_eq!(block_type, BlockType::Code);
    }

    #[test]
    fn test_classify_markdown_equation() {
        let (block_type, _, _) = VisionExtractor::classify_markdown_line("$$E = mc^2$$");
        assert_eq!(block_type, BlockType::Equation);
    }

    #[test]
    fn test_classify_markdown_table() {
        let (block_type, _, _) = VisionExtractor::classify_markdown_line("| Col1 | Col2 |");
        assert_eq!(block_type, BlockType::Table);
    }

    #[tokio::test]
    async fn test_extract_from_images() {
        let extractor = create_extractor();
        let image = PageImage::new(vec![0, 0, 0], 100, 100, ImageFormat::Png).with_page(0);

        let result = extractor.extract_from_images(&[image]).await;
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.pages.len(), 1);
        assert_eq!(doc.method, ExtractionMethod::Vision);
    }
}
