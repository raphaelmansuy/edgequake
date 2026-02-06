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
use crate::layout::pymupdf_structs::{
    Block as TextBlock, BlockType as LayoutBlockType, Span as LayoutSpan,
};
use crate::progress::ProgressCallback;
use crate::schema::{
    Block, BlockId, BlockType, BoundingBox, Document, ExtractionMethod, FontStyle, Page, TextSpan,
};
use crate::Result;

/// PDF extraction backend using PDFium library.
///
/// ## Implements
///
/// - [`FEAT0720`]: Pdfium-based backend with accurate font style detection
/// - [`OODA-43`]: Bridge pdfium extraction to PdfBackend trait
///
/// ## WHY PDFium?
///
/// PDFium (Chromium's PDF engine) provides accurate font flags from
/// the font descriptor (bold/italic/monospace), matching how PyMuPDF4LLM
/// achieves high-quality markdown conversion.
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

        // Step 1: Extract raw characters with accurate font flags AND page dimensions
        // WHY (OODA-IT21): We need actual page heights to normalize Y coordinates
        // from PDF coordinate system (Y=0 at bottom) to document coordinate system
        // (Y=0 at top). Without this, the LayoutProcessor's reading order detection
        // reverses block order because it expects Y=0 at top.
        let (chars, page_sizes) = extractor.extract_chars_and_page_sizes_from_bytes(pdf_bytes)?;
        debug!(
            "PdfiumBackend: extracted {} raw characters, {} pages",
            chars.len(),
            page_sizes.len()
        );

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
            let page_chars = chars_by_page
                .get(&page_num)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            // Get actual page dimensions from PDFium (default to US Letter if missing)
            let (page_width, page_height) =
                page_sizes.get(page_num).copied().unwrap_or((612.0, 792.0));

            // Group into text blocks
            let text_blocks = grouper.group(page_chars);
            debug!(
                "PdfiumBackend: page {} has {} text blocks (page_height={:.1})",
                page_num,
                text_blocks.len(),
                page_height
            );

            // Step 3: Classify blocks by font size (detect headers)
            let body_size = detect_body_font_size(&text_blocks);
            let classified_blocks = classify_blocks(&text_blocks, body_size);

            // Step 4: Convert to schema::Block WITH Y-coordinate normalization
            // WHY (OODA-IT21): PDF coordinates have Y=0 at BOTTOM, increasing upward.
            // All downstream processors (LayoutProcessor, ReadingOrderDetector) expect
            // document coordinates with Y=0 at TOP, increasing downward.
            let schema_blocks: Vec<Block> = classified_blocks
                .iter()
                .enumerate()
                .map(|(idx, tb)| convert_text_block_to_schema_block(tb, page_num, idx, page_height))
                .collect();

            // WHY (OODA-IT28): Merge horizontally adjacent blocks on the same line.
            // PDF extraction creates separate blocks for text fragments on the same
            // visual line (e.g., "AI Services" + "—" + "Elitizon" = 3 blocks).
            // For correct reading, these must be merged into a single block:
            // "AI Services — Elitizon".
            let schema_blocks = merge_same_line_blocks(schema_blocks);

            // Create page with actual dimensions from PDFium
            let mut page = Page::new(page_num + 1, page_width, page_height);
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

        // Extract raw characters with accurate font flags AND page dimensions
        // WHY (OODA-IT21): Same Y normalization as extract()
        let (chars, page_sizes) = extractor.extract_chars_and_page_sizes_from_bytes(pdf_bytes)?;

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

            let page_chars = chars_by_page
                .get(&page_num)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            // Get actual page dimensions from PDFium
            let (page_width, page_height) =
                page_sizes.get(page_num).copied().unwrap_or((612.0, 792.0));

            // Group into text blocks
            let text_blocks = grouper.group(page_chars);

            // Classify and convert blocks
            let body_size = detect_body_font_size(&text_blocks);
            let classified_blocks = classify_blocks(&text_blocks, body_size);

            // Convert with Y normalization (OODA-IT21)
            let schema_blocks: Vec<Block> = classified_blocks
                .iter()
                .enumerate()
                .map(|(idx, tb)| convert_text_block_to_schema_block(tb, page_num, idx, page_height))
                .collect();

            // WHY (OODA-IT32): Merge horizontally adjacent blocks on the same line.
            // This was missing from extract_with_progress (bug from IT28/IT31).
            // Without this, text fragments like "AI Services" + "—" + "Elitizon"
            // remain as 3 separate blocks instead of one "AI Services — Elitizon".
            let schema_blocks = merge_same_line_blocks(schema_blocks);

            // Create page with actual dimensions
            let mut page = Page::new(page_num + 1, page_width, page_height);
            // Capture block count before moving blocks into page
            // WHY: on_page_complete expects markdown_len but we don't have
            // rendered markdown yet. Pass block count as a reasonable proxy.
            let block_count = schema_blocks.len();
            page.blocks = schema_blocks;
            page.method = ExtractionMethod::Native;
            page.update_stats();

            document.add_page(page);

            // BUG FIX: Previously passed page_count as markdown_len, which
            // is semantically wrong. Now passes block_count as a proxy.
            callback.on_page_complete(page_num, block_count);
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
            // KNOWN LIMITATION: Image presence detection not implemented
            // WHY: Would require scanning PDF page objects for XObject/Image types
            // PDFium API can enumerate page objects but adds complexity
            // WORKAROUND: Assume images present if Vision mode is requested
            // FUTURE: Could use pdfium_render's page_objects() iterator
            has_images: false,
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
/// - Font size > 1.4x body size (heading text is larger)
/// - Block has <= 3 lines (headers are typically short)
/// - Block doesn't start with bullet/number (not a list item)
///
/// OODA-39: Raised threshold from 1.2x to 1.4x and removed levels 3-4.
/// WHY: The old 1.2x threshold caused false header classification for
/// emphasized body text, figure captions, and column-split artifacts.
/// Levels 3-4 conflicted with the downstream processor chain (StyleDetection,
/// HeaderDetection, SectionPattern) which only assigns levels 1-2 and has
/// multi-signal classification (font + weight + content + patterns).
/// The downstream processors skip blocks with levels already set, so the
/// primitive font-only backend classification was overriding the
/// sophisticated multi-signal classification.
fn classify_blocks(blocks: &[TextBlock], body_size: f32) -> Vec<TextBlock> {
    let header_threshold = body_size * 1.4;

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
            // ──────────────────────────────────────────────────────────────
            // WHY: Exclude bullet and list items from header classification (OODA-30)
            //
            // Bullets (•, -, *) are never headers. Digit-starting text is
            // tricky: "0) AI Strategy" IS a section header, but "1. First
            // item" is a list item. The key distinction is FONT SIZE: if the
            // text is larger than body, it's a header regardless of starting
            // character. The not_list guard only needs to exclude bullets.
            //
            // Previously excluded ALL digit-starting text, which prevented
            // numbered section headers (15pt, ratio 1.25) from being
            // classified correctly.
            // ──────────────────────────────────────────────────────────────
            let not_list =
                !text.starts_with('•') && !text.starts_with('-') && !text.starts_with('*');

            if is_larger && is_short && not_list {
                // OODA-39: Only assign levels 1-2. Level 3+ caused false `###`
                // headers in output. The downstream processor chain handles
                // nuanced header detection with multi-signal classification.
                let size_ratio = block_font_size / body_size;
                let level = if size_ratio >= 1.8 {
                    1 // h1: very large (document titles)
                } else {
                    2 // h2: all other large text (major sections)
                };
                classified.block_type = LayoutBlockType::Header(level);
            } else {
                // Check for code (monospace font)
                let is_code = block
                    .lines
                    .iter()
                    .any(|line| line.spans.iter().any(|span| span.is_monospace()));

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

/// Convert a layout::Span to a schema::TextSpan with style preservation.
///
/// ## OODA-IT05: Why This Function?
///
/// Style information (bold, italic, monospace) is extracted by PDFium and
/// stored in `layout::Span`. Without this conversion, the markdown renderer
/// would only see plain text and could not apply inline styling like `**bold**`.
///
/// ## Conversion Rules
///
/// ```text
/// layout::Span.font_is_bold → FontStyle.weight = 700
/// layout::Span.font_is_italic → FontStyle.italic = true
/// layout::Span.font_is_monospace → FontStyle (for code detection)
/// ```
///
/// ## OODA-IT21: Y-Coordinate Normalization
///
/// Span Y coordinates are also normalized from PDF coords (Y=0 at bottom)
/// to document coords (Y=0 at top) for consistency with block-level coords.
fn convert_span_to_text_span(span: &LayoutSpan, page_height: f32) -> TextSpan {
    let mut style = FontStyle::default();

    // WHY 700: Font-weight 700 is the CSS standard for "bold"
    // PDFium extracts this from font descriptor flags
    if span.font_is_bold.unwrap_or(false) {
        style.weight = Some(700);
    }

    style.italic = span.font_is_italic.unwrap_or(false);
    style.size = Some(span.font_size);
    style.family = span.font_name.clone();

    // Create bounding box for the span with Y normalization (OODA-IT21)
    let norm_y1 = page_height - span.y1; // old top → new top (small y)
    let norm_y2 = page_height - span.y0; // old bottom → new bottom (large y)
    let bbox = BoundingBox::new(span.x0, norm_y1, span.x1, norm_y2);
    let mut text_span = TextSpan::styled(span.text.clone(), style);
    text_span.bbox = Some(bbox);

    text_span
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
/// - **OODA-IT05: Styled spans for bold/italic/code rendering**
///
/// OODA-IT28: Merge horizontally adjacent blocks that share the same visual line.
///
/// WHY: PDF extraction creates separate blocks for text fragments on the same
/// line (e.g., "AI Services" + "—" + "Elitizon" → 3 blocks). These must be
/// merged into a single block for correct reading and header detection.
///
/// ```text
/// ┌──────────────┐  ┌───┐  ┌──────────┐
/// │ AI Services  │  │ — │  │ Elitizon │   ← 3 separate blocks, same y-range
/// └──────────────┘  └───┘  └──────────┘
///         ↓ merge_same_line_blocks()
/// ┌────────────────────────────────────┐
/// │ AI Services — Elitizon             │   ← 1 merged block
/// └────────────────────────────────────┘
/// ```
///
/// Merge criteria:
/// 1. Blocks overlap in y-range (≥50% of smaller block's height)
/// 2. Horizontal gap between blocks is small (< 3x typical char width)
/// 3. Both blocks are text-like (not tables, code, etc.)
fn merge_same_line_blocks(mut blocks: Vec<Block>) -> Vec<Block> {
    if blocks.len() <= 1 {
        return blocks;
    }

    // Sort by y-position (top to bottom), then x-position (left to right)
    blocks.sort_by(|a, b| {
        let y_cmp = a
            .bbox
            .y1
            .partial_cmp(&b.bbox.y1)
            .unwrap_or(std::cmp::Ordering::Equal);
        if y_cmp == std::cmp::Ordering::Equal {
            a.bbox
                .x1
                .partial_cmp(&b.bbox.x1)
                .unwrap_or(std::cmp::Ordering::Equal)
        } else {
            y_cmp
        }
    });

    let mut merged: Vec<Block> = Vec::new();

    for block in blocks {
        let should_merge = if let Some(last) = merged.last() {
            // Check if blocks are on the same visual line
            let a_height = (last.bbox.y2 - last.bbox.y1).abs().max(1.0);
            let b_height = (block.bbox.y2 - block.bbox.y1).abs().max(1.0);
            let min_height = a_height.min(b_height);

            // Y-overlap check: blocks share >50% of the smaller block's height
            let y_overlap_start = last.bbox.y1.max(block.bbox.y1);
            let y_overlap_end = last.bbox.y2.min(block.bbox.y2);
            let y_overlap = (y_overlap_end - y_overlap_start).max(0.0);
            let y_overlap_ratio = y_overlap / min_height;

            // Horizontal gap: distance between right edge of last and left edge of current
            let h_gap = (block.bbox.x1 - last.bbox.x2).max(0.0);

            // WHY: Block has no font_size field, so we use block height as a
            // proxy for font size (block height ≈ line height ≈ 1.2 × font size).
            let avg_block_height = (a_height + b_height) / 2.0;
            let max_gap = avg_block_height * 2.0; // Allow up to 2x block height gap

            // Both must be text-like (Text or SectionHeader, not Table/Code/etc.)
            let same_type = matches!(
                (&last.block_type, &block.block_type),
                (BlockType::Text, BlockType::Text)
                    | (BlockType::SectionHeader, BlockType::SectionHeader)
                    | (BlockType::Text, BlockType::SectionHeader)
                    | (BlockType::SectionHeader, BlockType::Text)
            );

            y_overlap_ratio > 0.5 && h_gap < max_gap && same_type
        } else {
            false
        };

        if should_merge {
            let last = merged.last_mut().unwrap();
            // Merge text with space separator
            if !last.text.is_empty() && !block.text.is_empty() {
                last.text.push(' ');
            }
            last.text.push_str(&block.text);

            // Expand bounding box to cover both blocks
            last.bbox.x1 = last.bbox.x1.min(block.bbox.x1);
            last.bbox.y1 = last.bbox.y1.min(block.bbox.y1);
            last.bbox.x2 = last.bbox.x2.max(block.bbox.x2);
            last.bbox.y2 = last.bbox.y2.max(block.bbox.y2);

            // Merge spans: append block's spans to last's spans
            if !block.spans.is_empty() {
                // Add space span between the two block's spans
                if !last.spans.is_empty() {
                    last.spans.push(TextSpan::plain(" "));
                }
                last.spans.extend(block.spans);
            }

            // Keep the higher header level (lower number = higher level)
            if let (Some(a_level), Some(b_level)) = (last.level, block.level) {
                last.level = Some(a_level.min(b_level));
            } else if block.level.is_some() {
                last.level = block.level;
            }

            // Promote to SectionHeader if either was a header
            if block.block_type == BlockType::SectionHeader {
                last.block_type = BlockType::SectionHeader;
            }

            tracing::debug!(
                "SAME-LINE-MERGE: merged '{}' into block, result='{}'",
                block.text.chars().take(30).collect::<String>(),
                last.text.chars().take(60).collect::<String>()
            );
        } else {
            merged.push(block);
        }
    }

    merged
}

/// ## OODA-IT21: Y-Coordinate Normalization
///
/// PDF coordinates have Y=0 at BOTTOM, Y increases UPWARD.
/// Document coordinates have Y=0 at TOP, Y increases DOWNWARD.
///
/// This function normalizes: `new_y = page_height - old_y`
/// and swaps y0/y1 to maintain the y1 < y2 invariant.
///
/// Without this normalization, the LayoutProcessor's reading order
/// detection (which expects document coords) reverses block order.
fn convert_text_block_to_schema_block(
    text_block: &TextBlock,
    page_num: usize,
    position: usize,
    page_height: f32,
) -> Block {
    // Map layout block type to schema block type
    let block_type = match text_block.block_type {
        LayoutBlockType::Paragraph => BlockType::Paragraph,
        LayoutBlockType::Header(_) => BlockType::SectionHeader,
        LayoutBlockType::Code => BlockType::Code,
        LayoutBlockType::ListItem => BlockType::ListItem,
        LayoutBlockType::Table => BlockType::Table,
        LayoutBlockType::Footnote => BlockType::Paragraph, // OODA-08: Footnotes map to paragraph in schema
    };

    // Create bounding box WITH Y normalization (OODA-IT21)
    // PDF coords: y0 = bottom of glyph (smaller), y1 = top of glyph (larger)
    // Document coords: y1 = top of block (smaller), y2 = bottom of block (larger)
    // Normalization: new_y = page_height - old_y
    // After flipping, old y1 (top, larger) becomes smaller → new y1
    //                  old y0 (bottom, smaller) becomes larger → new y2
    let norm_y1 = page_height - text_block.y1; // old top → new top (small y)
    let norm_y2 = page_height - text_block.y0; // old bottom → new bottom (large y)
    let bbox = BoundingBox::new(text_block.x0, norm_y1, text_block.x1, norm_y2);

    // Create block with appropriate type
    let mut block = Block::new(block_type, bbox);
    block.id = BlockId::with_indices(page_num, position);
    block.page = page_num;
    block.position = position;
    block.text = text_block.text();
    block.confidence = 1.0;

    // OODA-IT05 + OODA-IT22: Populate spans with styled TextSpan objects
    // WHY: Without this, the markdown renderer cannot apply inline styling
    // Each span carries its own font style (bold/italic) from PDFium
    //
    // OODA-IT22 FIX: Insert space TextSpan between consecutive word spans.
    // WHY: The TextGrouper's chars_to_spans() strips space characters and creates
    // separate spans for each word. Line::text() reconstructs spaces by checking
    // inter-span gaps. But when we convert to schema::TextSpan, we must also
    // insert space TextSpans, otherwise render_spans_styled() concatenates
    // "AI" + "Services" → "AIServices" instead of "AI Services".
    //
    // Algorithm:
    // 1. Iterate lines top-to-bottom
    // 2. For each line, iterate spans left-to-right
    // 3. Between consecutive spans, check horizontal gap vs space threshold
    // 4. If gap > threshold → insert TextSpan::plain(" ")
    // 5. Convert each layout::Span to schema::TextSpan with style
    // 6. Add space TextSpan between lines for proper word separation
    for (line_idx, line) in text_block.lines.iter().enumerate() {
        for (span_idx, span) in line.spans.iter().enumerate() {
            // OODA-IT22: Insert space between consecutive spans if there's a word gap
            if span_idx > 0 {
                let prev = &line.spans[span_idx - 1];
                let gap = span.x0 - prev.x1;
                let avg_size = (prev.font_size + span.font_size) / 2.0;
                // WHY 0.15: Same threshold as Line::text() in pymupdf_structs.rs
                // 15% of font size = typical minimum word gap in proportional fonts
                let space_threshold = avg_size * 0.15;
                // WHY hyphen check: Same logic as Line::text() — don't break hyphenated words
                // OODA-IT28: Em dashes (—) are EXCLUDED from hyphen check — they are
                // sentence-level separators that need spaces (e.g., "AI Services — Elitizon")
                let starts_with_hyphen =
                    span.text.starts_with('-') || span.text.starts_with('\u{2013}'); // en-dash only
                let ends_with_hyphen = prev.text.ends_with('-') || prev.text.ends_with('\u{2013}');
                if gap > space_threshold && !starts_with_hyphen && !ends_with_hyphen {
                    block.spans.push(TextSpan::plain(" "));
                }
            }
            let text_span = convert_span_to_text_span(span, page_height);
            block.spans.push(text_span);
        }
        // OODA-IT22 FIX: Insert SPACE (not newline) between lines.
        // WHY: render_spans_styled() trims each span content, so "\n" appended
        // to the previous styled span (via consolidate_spans) gets trimmed away,
        // causing "build\n" + "vs-buy" → "buildvs-buy". Using " " instead
        // preserves the word boundary because trailing_space = content.ends_with(' ')
        // is true for ' ' but false for '\n'. This matches PDF semantics: line
        // breaks within a paragraph block are soft wraps, not semantic newlines.
        if line_idx < text_block.lines.len() - 1 && !line.spans.is_empty() {
            block.spans.push(TextSpan::plain(" "));
        }
    }

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

    #[test]
    fn test_convert_span_to_text_span_bold() {
        // OODA-IT05: Test that bold style is preserved in conversion
        let mut span = LayoutSpan::new(0);
        span.text = "Bold text".to_string();
        span.font_is_bold = Some(true);
        span.font_is_italic = Some(false);
        span.font_size = 12.0;
        span.x0 = 0.0;
        span.x1 = 50.0;
        span.y0 = 0.0;
        span.y1 = 14.0;

        let text_span = convert_span_to_text_span(&span, 792.0);

        assert_eq!(text_span.text, "Bold text");
        assert_eq!(
            text_span.style.weight,
            Some(700),
            "Bold should have weight 700"
        );
        assert!(!text_span.style.italic, "Should not be italic");
        assert!(text_span.bbox.is_some(), "Should have bounding box");
    }

    #[test]
    fn test_convert_span_to_text_span_italic() {
        // OODA-IT05: Test that italic style is preserved in conversion
        let mut span = LayoutSpan::new(0);
        span.text = "Italic text".to_string();
        span.font_is_bold = Some(false);
        span.font_is_italic = Some(true);
        span.font_size = 12.0;
        span.x0 = 0.0;
        span.x1 = 60.0;
        span.y0 = 0.0;
        span.y1 = 14.0;

        let text_span = convert_span_to_text_span(&span, 792.0);

        assert_eq!(text_span.text, "Italic text");
        assert!(text_span.style.italic, "Should be italic");
        assert!(
            text_span.style.weight.is_none() || text_span.style.weight == Some(400),
            "Non-bold should not have weight 700"
        );
    }

    #[test]
    fn test_convert_block_preserves_spans() {
        // OODA-IT05: Test that block conversion populates spans vector
        use crate::layout::pymupdf_structs::{Line, Span};

        // Create a line with bold and normal spans
        let mut bold_span = Span::new(0);
        bold_span.text = "Bold".to_string();
        bold_span.font_is_bold = Some(true);
        bold_span.font_size = 12.0;
        bold_span.x0 = 0.0;
        bold_span.x1 = 30.0;
        bold_span.y0 = 0.0;
        bold_span.y1 = 14.0;

        let mut normal_span = Span::new(0);
        normal_span.text = "normal".to_string();
        normal_span.font_is_bold = Some(false);
        normal_span.font_size = 12.0;
        normal_span.x0 = 35.0;
        normal_span.x1 = 80.0;
        normal_span.y0 = 0.0;
        normal_span.y1 = 14.0;

        let mut line = Line::from_span(bold_span);
        line.add_span(normal_span);

        let text_block = TextBlock::from_line(line);

        // Convert to schema block (page_height=792.0 for US Letter)
        let schema_block = convert_text_block_to_schema_block(&text_block, 0, 0, 792.0);

        // Verify spans are populated
        // OODA-IT22: Now includes space span between "Bold" and "normal" (gap=5.0 > threshold=1.8)
        assert_eq!(
            schema_block.spans.len(),
            3,
            "Should have 3 spans (Bold + space + normal)"
        );
        assert_eq!(schema_block.spans[0].text, "Bold");
        assert_eq!(
            schema_block.spans[0].style.weight,
            Some(700),
            "First span should be bold"
        );
        assert_eq!(
            schema_block.spans[1].text, " ",
            "Second span should be word space"
        );
        assert_eq!(schema_block.spans[2].text, "normal");
    }
}
