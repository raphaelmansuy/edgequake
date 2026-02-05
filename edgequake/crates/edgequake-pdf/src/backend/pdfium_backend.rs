//! PdfiumBackend: High-quality PDF extraction using PDFium + pymupdf4llm-style grouping.
//!
//! This backend bridges the modern pdfium extraction pipeline with the existing
//! `PdfBackend` trait, enabling the API server and tests to use accurate font
//! style detection from PDFium.
//!
//! ## Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────┐
//! │                      PdfiumBackend (this module)                       │
//! │                                                                        │
//! │  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────────┐ │
//! │  │ PdfiumExtr.  │ →  │ TextGrouper  │ →  │ Convert layout::Block    │ │
//! │  │ (RawChar[])  │    │ (TextBlock[])│    │ to schema::Block        │ │
//! │  └──────────────┘    └──────────────┘    └──────────────────────────┘ │
//! │                                                    │                   │
//! │                                                    ▼                   │
//! │                           ┌────────────────────────────────────────┐  │
//! │                           │ Build schema::Document with Pages      │  │
//! │                           └────────────────────────────────────────┘  │
//! │                                                                        │
//! └────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## WHY This Backend?
//!
//! 1. **Accurate font styles**: PDFium extracts bold/italic from font descriptor flags,
//!    not font name pattern matching (which is unreliable)
//!
//! 2. **Unified API**: Implements `PdfBackend` trait so existing code works unchanged
//!
//! 3. **Quality parity**: The eval script uses pdfium → quality 0.786. This backend
//!    brings that quality to the API server and tests.
//!
//! ## Thread Safety
//!
//! PDFium's `Pdfium` struct is not `Send + Sync` (it wraps native library bindings).
//! To satisfy the `PdfBackend: Send + Sync` trait bound, we store only the config
//! and create a new `PdfiumExtractor` for each extraction call. This is safe because:
//! - PDFium initialization is fast (~1ms)
//! - Each extraction is independent
//! - No state needs to be shared between extractions

use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info};

use crate::backend::pdfium::PdfiumExtractor;
use crate::backend::PdfBackend;
use crate::config::PdfConfig;
use crate::extractor::PdfInfo;
use crate::layout::{GroupingParams, TextGrouper};
// Import TextBlock from pymupdf_structs module directly to avoid shadowing
use crate::layout::pymupdf_structs::{Block as TextBlock, BlockType as LayoutBlockType};
use crate::progress::ProgressCallback;
use crate::schema::{
    Block, BlockId, BlockType, BoundingBox, Document, ExtractionMethod, Page,
};
use crate::Result;

/// PDF extraction backend using PDFium library.
///
/// ## Implements
///
/// - [`FEAT0720`]: Pdfium-based backend with accurate font style detection
/// - [`OODA-43`]: Bridge pdfium extraction to PdfBackend trait
///
/// ## WHY PdfiumBackend instead of ExtractionEngine?
///
/// The ExtractionEngine uses lopdf which relies on font name pattern matching
/// for bold/italic detection. This is unreliable because:
/// - Many PDFs don't use "Bold" or "Italic" in font names
/// - Academic papers often use font weights (700) instead of name patterns
///
/// PDFium provides accurate font flags from the font descriptor, matching
/// how PyMuPDF4LLM achieves high-quality markdown conversion.
///
/// ## Thread Safety Design
///
/// This struct only holds configuration, not the PDFium instance itself.
/// A new `PdfiumExtractor` is created for each extraction call, which allows
/// this type to be `Send + Sync` while still using PDFium.
pub struct PdfiumBackend {
    /// Configuration options (stored for creating extractors on demand)
    #[allow(dead_code)] // WHY: Reserved for future config-based extractor customization
    config: PdfConfig,
}

// Manual Send + Sync impl is safe because we only hold config, not PdfiumExtractor
// The PdfiumExtractor is created fresh for each extraction call
unsafe impl Send for PdfiumBackend {}
unsafe impl Sync for PdfiumBackend {}

impl PdfiumBackend {
    /// Create a new PdfiumBackend with default configuration.
    ///
    /// # Errors
    ///
    /// Returns error if PDFium library cannot be initialized (missing bindings).
    pub fn new() -> Result<Self> {
        Self::with_config(PdfConfig::default())
    }

    /// Create a new PdfiumBackend with custom configuration.
    ///
    /// This validates that PDFium can be initialized, but doesn't hold
    /// the instance (to maintain thread safety).
    pub fn with_config(config: PdfConfig) -> Result<Self> {
        // Validate PDFium can be initialized
        let _extractor = PdfiumExtractor::new()?;
        Ok(Self { config })
    }

    /// Create a fresh PdfiumExtractor for extraction.
    ///
    /// WHY: Creates extractor on demand to maintain Send + Sync.
    fn create_extractor(&self) -> Result<PdfiumExtractor> {
        PdfiumExtractor::new()
    }
}

#[async_trait]
impl PdfBackend for PdfiumBackend {
    /// Extract document structure from PDF bytes.
    ///
    /// ## Algorithm
    ///
    /// 1. Extract raw characters with PDFium (accurate positions, font flags)
    /// 2. Group chars → spans → lines → blocks using TextGrouper
    /// 3. Classify blocks (headers, lists, code) by font size analysis
    /// 4. Convert layout::Block to schema::Block for ProcessorChain compatibility
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document> {
        info!("PdfiumBackend: extracting PDF ({} bytes)", pdf_bytes.len());

        // Create fresh extractor for this call (thread safety)
        let extractor = self.create_extractor()?;

        // Step 1: Extract raw characters with accurate font flags
        let chars = extractor.extract_chars_from_bytes(pdf_bytes)?;
        debug!("PdfiumBackend: extracted {} raw characters", chars.len());

        if chars.is_empty() {
            debug!("PdfiumBackend: no characters found, returning empty document");
            return Ok(Document::new());
        }

        // Group characters by page
        let mut chars_by_page: std::collections::HashMap<usize, Vec<_>> =
            std::collections::HashMap::new();
        for ch in chars {
            chars_by_page.entry(ch.page_num).or_default().push(ch);
        }

        // Step 2: Group chars → blocks for each page
        let grouper = TextGrouper::with_params(GroupingParams::default());
        let mut document = Document::new();
        document.method = ExtractionMethod::Native;

        for page_num in 0..chars_by_page.len() {
            let page_chars = chars_by_page.get(&page_num).map(|v| v.as_slice()).unwrap_or(&[]);

            // Group into text blocks
            let text_blocks = grouper.group(page_chars);
            debug!(
                "PdfiumBackend: page {} has {} text blocks",
                page_num,
                text_blocks.len()
            );

            // Step 3: Classify blocks by font size (detect headers)
            let body_size = detect_body_font_size(&text_blocks);
            let classified_blocks = classify_blocks(&text_blocks, body_size);

            // Step 4: Convert to schema::Block
            let schema_blocks: Vec<Block> = classified_blocks
                .iter()
                .enumerate()
                .map(|(idx, tb)| convert_text_block_to_schema_block(tb, page_num, idx))
                .collect();

            // Create page (default US Letter size, would need PDF metadata for actual size)
            let mut page = Page::new(page_num + 1, 612.0, 792.0);
            page.blocks = schema_blocks;
            page.method = ExtractionMethod::Native;
            page.update_stats();

            document.add_page(page);
        }

        document.update_stats();
        info!(
            "PdfiumBackend: extracted {} pages, {} total blocks",
            document.page_count(),
            document.total_blocks()
        );

        Ok(document)
    }

    /// Extract with progress callbacks.
    ///
    /// Reports progress per page during extraction.
    async fn extract_with_progress(
        &self,
        pdf_bytes: &[u8],
        callback: Arc<dyn ProgressCallback>,
    ) -> Result<Document> {
        info!(
            "PdfiumBackend: extracting with progress ({} bytes)",
            pdf_bytes.len()
        );

        // Create fresh extractor for this call
        let extractor = self.create_extractor()?;

        // Extract raw characters with accurate font flags
        let chars = extractor.extract_chars_from_bytes(pdf_bytes)?;

        if chars.is_empty() {
            callback.on_extraction_start(0);
            callback.on_extraction_complete(0, 0);
            return Ok(Document::new());
        }

        // Group characters by page
        let mut chars_by_page: std::collections::HashMap<usize, Vec<_>> =
            std::collections::HashMap::new();
        for ch in chars {
            chars_by_page.entry(ch.page_num).or_default().push(ch);
        }

        let page_count = chars_by_page.len();
        callback.on_extraction_start(page_count);

        // Process each page
        let grouper = TextGrouper::with_params(GroupingParams::default());
        let mut document = Document::new();
        document.method = ExtractionMethod::Native;
        let mut success_count = 0;

        for page_num in 0..page_count {
            callback.on_page_start(page_num, page_count);

            let page_chars = chars_by_page.get(&page_num).map(|v| v.as_slice()).unwrap_or(&[]);

            // Group into text blocks
            let text_blocks = grouper.group(page_chars);

            // Classify and convert blocks
            let body_size = detect_body_font_size(&text_blocks);
            let classified_blocks = classify_blocks(&text_blocks, body_size);

            let schema_blocks: Vec<Block> = classified_blocks
                .iter()
                .enumerate()
                .map(|(idx, tb)| convert_text_block_to_schema_block(tb, page_num, idx))
                .collect();

            // Create page
            let mut page = Page::new(page_num + 1, 612.0, 792.0);
            page.blocks = schema_blocks;
            page.method = ExtractionMethod::Native;
            page.update_stats();

            document.add_page(page);

            callback.on_page_complete(page_num, page_count);
            success_count += 1;
        }

        document.update_stats();
        callback.on_extraction_complete(page_count, success_count);

        Ok(document)
    }

    /// Get PDF metadata without full extraction.
    fn get_info(&self, pdf_bytes: &[u8]) -> Result<PdfInfo> {
        // Create fresh extractor
        let extractor = self.create_extractor()?;

        // Extract chars to count pages
        let chars = extractor.extract_chars_from_bytes(pdf_bytes)?;

        // Count pages from character metadata
        let page_count = if chars.is_empty() {
            0
        } else {
            chars.iter().map(|c| c.page_num).max().unwrap_or(0) + 1
        };

        Ok(PdfInfo {
            page_count,
            pdf_version: "Unknown".to_string(),
            has_images: false, // TODO: Could scan for images
            image_count: 0,
            file_size: pdf_bytes.len(),
        })
    }
}

/// Detect the body font size from text blocks.
///
/// Uses the most common font size, weighted by text length.
/// This helps distinguish headers (larger) from body text.
fn detect_body_font_size(blocks: &[TextBlock]) -> f32 {
    let mut size_weights: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();

    for block in blocks {
        for line in &block.lines {
            let size_key = (line.dominant_font_size() * 10.0) as i32; // Round to 0.1pt
            let weight = line.text().len();
            *size_weights.entry(size_key).or_insert(0) += weight;
        }
    }

    // Find the most common size
    size_weights
        .iter()
        .max_by_key(|(_, weight)| *weight)
        .map(|(size, _)| *size as f32 / 10.0)
        .unwrap_or(12.0) // Default to 12pt if no data
}

/// Classify blocks by analyzing font size relative to body size.
///
/// ## Header Detection
///
/// A block is classified as a header if:
/// - Font size > 1.2x body size (heading text is larger)
/// - Block has <= 3 lines (headers are typically short)
/// - Block doesn't start with bullet/number (not a list item)
fn classify_blocks(blocks: &[TextBlock], body_size: f32) -> Vec<TextBlock> {
    let header_threshold = body_size * 1.2;

    blocks
        .iter()
        .map(|block| {
            let mut classified = block.clone();

            // Get dominant font size for the block
            let block_font_size = if let Some(first_line) = block.lines.first() {
                first_line.dominant_font_size()
            } else {
                body_size
            };

            // Check for header characteristics
            let is_larger = block_font_size >= header_threshold;
            let is_short = block.lines.len() <= 3;
            let text = block.text();
            let not_list = !text.starts_with('•')
                && !text.starts_with('-')
                && !text.starts_with('*')
                && !text.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);

            if is_larger && is_short && not_list {
                // Calculate header level based on size ratio
                let size_ratio = block_font_size / body_size;
                let level = if size_ratio >= 1.8 {
                    1 // h1: very large
                } else if size_ratio >= 1.5 {
                    2 // h2: large
                } else if size_ratio >= 1.3 {
                    3 // h3: medium
                } else {
                    4 // h4: slightly larger
                };
                classified.block_type = LayoutBlockType::Header(level);
            } else {
                // Check for code (monospace font)
                let is_code = block.lines.iter().any(|line| {
                    line.spans.iter().any(|span| span.is_monospace())
                });

                if is_code {
                    classified.block_type = LayoutBlockType::Code;
                } else {
                    classified.block_type = LayoutBlockType::Paragraph;
                }
            }

            classified
        })
        .collect()
}

/// Convert a layout::Block (TextBlock) to a schema::Block.
///
/// ## WHY This Conversion?
///
/// The layout module uses its own `Block` struct optimized for text grouping
/// (with Span/Line hierarchy). The schema module uses a different `Block`
/// struct designed for document representation and serialization.
///
/// This function bridges the two representations, preserving:
/// - Text content with proper line breaks
/// - Block type (paragraph, header, code, list)
/// - Bounding box coordinates
/// - Page and position metadata
fn convert_text_block_to_schema_block(
    text_block: &TextBlock,
    page_num: usize,
    position: usize,
) -> Block {
    // Map layout block type to schema block type
    let block_type = match text_block.block_type {
        LayoutBlockType::Paragraph => BlockType::Paragraph,
        LayoutBlockType::Header(_) => BlockType::SectionHeader,
        LayoutBlockType::Code => BlockType::Code,
        LayoutBlockType::ListItem => BlockType::ListItem,
        LayoutBlockType::Table => BlockType::Table,
    };

    // Create bounding box
    let bbox = BoundingBox::new(text_block.x0, text_block.y0, text_block.x1, text_block.y1);

    // Create block with appropriate type
    let mut block = Block::new(block_type, bbox);
    block.id = BlockId::with_indices(page_num, position);
    block.page = page_num;
    block.position = position;
    block.text = text_block.text();
    block.confidence = 1.0;

    // Set header level if applicable
    if let LayoutBlockType::Header(level) = text_block.block_type {
        block.level = Some(level);
    }

    // Mark source for debugging
    block.source = Some("pdfium".to_string());

    block
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_body_font_size_empty() {
        let blocks: Vec<TextBlock> = vec![];
        let body_size = detect_body_font_size(&blocks);
        assert!((body_size - 12.0).abs() < 0.1, "Default should be 12pt");
    }

    #[test]
    fn test_classify_blocks_header_detection() {
        // A block with large font should be classified as header
        use crate::layout::pymupdf_structs::{Line, Span};

        let mut span = Span::new(0);
        span.font_size = 18.0;
        span.text = "Introduction".to_string();
        span.x0 = 0.0;
        span.x1 = 100.0;
        span.y0 = 0.0;
        span.y1 = 20.0;

        let line = Line::from_span(span);
        let block = TextBlock::from_line(line);

        let blocks = vec![block];
        let classified = classify_blocks(&blocks, 12.0); // 12pt body size

        assert_eq!(classified.len(), 1);
        assert!(
            matches!(classified[0].block_type, LayoutBlockType::Header(_)),
            "18pt text with 12pt body should be header"
        );
    }
}
