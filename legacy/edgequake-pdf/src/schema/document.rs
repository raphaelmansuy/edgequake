//! Document and page structure definitions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::block::{Block, BlockId};
use super::block_types::BlockType;
use super::geometry::BoundingBox;

/// Extraction method used for the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ExtractionMethod {
    /// Native PDF text extraction
    #[default]
    Native,
    /// OCR-based extraction
    Ocr,
    /// LLM-enhanced extraction
    LlmEnhanced,
    /// Vision model extraction (page rendering)
    Vision,
    /// Hybrid (combination of methods)
    Hybrid,
}

impl ExtractionMethod {
    /// Returns a human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            ExtractionMethod::Native => "Native Text",
            ExtractionMethod::Ocr => "OCR",
            ExtractionMethod::LlmEnhanced => "LLM Enhanced",
            ExtractionMethod::Vision => "Vision Model",
            ExtractionMethod::Hybrid => "Hybrid",
        }
    }
}

/// Statistics about a page extraction.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageStats {
    /// Number of text blocks
    pub text_blocks: usize,
    /// Number of table blocks
    pub tables: usize,
    /// Number of figures/images
    pub figures: usize,
    /// Number of headers
    pub headers: usize,
    /// Number of code blocks
    pub code_blocks: usize,
    /// Number of equations
    pub equations: usize,
    /// Total character count
    pub char_count: usize,
    /// Estimated word count
    pub word_count: usize,
    /// Extraction confidence (average across blocks)
    pub avg_confidence: f32,
    /// Whether OCR was used for this page
    pub ocr_used: bool,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

impl PageStats {
    /// Create stats from a list of blocks.
    pub fn from_blocks(blocks: &[Block]) -> Self {
        let mut stats = Self::default();

        for block in blocks {
            stats.char_count += block.text.len();
            stats.word_count += block.text.split_whitespace().count();

            match block.block_type {
                BlockType::Text | BlockType::Paragraph | BlockType::TextInlineMath => {
                    stats.text_blocks += 1;
                }
                BlockType::Table => stats.tables += 1,
                BlockType::Figure | BlockType::Picture => stats.figures += 1,
                BlockType::SectionHeader => stats.headers += 1,
                BlockType::Code => stats.code_blocks += 1,
                BlockType::Equation => stats.equations += 1,
                _ => {}
            }

            stats.avg_confidence += block.confidence;
        }

        if !blocks.is_empty() {
            stats.avg_confidence /= blocks.len() as f32;
        }

        stats
    }

    /// Combine with another PageStats.
    pub fn combine(&mut self, other: &PageStats) {
        self.text_blocks += other.text_blocks;
        self.tables += other.tables;
        self.figures += other.figures;
        self.headers += other.headers;
        self.code_blocks += other.code_blocks;
        self.equations += other.equations;
        self.char_count += other.char_count;
        self.word_count += other.word_count;
        // Note: avg_confidence would need reweighting for accuracy
        self.processing_time_ms += other.processing_time_ms;
    }
}

/// A single page in the document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// Page number (1-indexed for display)
    pub number: usize,
    /// Page width in points
    pub width: f32,
    /// Page height in points
    pub height: f32,
    /// Blocks on this page (in reading order)
    pub blocks: Vec<Block>,
    /// Extraction method used
    pub method: ExtractionMethod,
    /// Page statistics
    pub stats: PageStats,
    /// Detected columns (if any)
    pub columns: Vec<BoundingBox>,
    /// Detected page margins
    pub margins: Option<PageMargins>,
    /// Custom metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Page {
    /// Create a new page.
    pub fn new(number: usize, width: f32, height: f32) -> Self {
        Self {
            number,
            width,
            height,
            blocks: Vec::new(),
            method: ExtractionMethod::Native,
            stats: PageStats::default(),
            columns: Vec::new(),
            margins: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a block to the page.
    pub fn add_block(&mut self, block: Block) {
        self.blocks.push(block);
    }

    /// Get page bounding box.
    pub fn bbox(&self) -> BoundingBox {
        BoundingBox::new(0.0, 0.0, self.width, self.height)
    }

    /// Recalculate statistics from blocks.
    pub fn update_stats(&mut self) {
        self.stats = PageStats::from_blocks(&self.blocks);
    }

    /// Sort blocks by reading order.
    pub fn sort_blocks_by_reading_order(&mut self) {
        // Simple top-to-bottom, left-to-right sorting
        // More sophisticated reading order will be in layout module
        self.blocks.sort_by(|a, b| {
            let y_cmp = a.bbox.y1.partial_cmp(&b.bbox.y1).unwrap();
            if y_cmp != std::cmp::Ordering::Equal {
                return y_cmp;
            }
            a.bbox.x1.partial_cmp(&b.bbox.x1).unwrap()
        });

        // Update positions
        for (i, block) in self.blocks.iter_mut().enumerate() {
            block.position = i;
        }
    }

    /// Get all text content from this page.
    pub fn get_text(&self) -> String {
        self.blocks
            .iter()
            .filter(|b| b.block_type.has_text())
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get blocks of a specific type.
    pub fn blocks_of_type(&self, block_type: BlockType) -> Vec<&Block> {
        self.blocks
            .iter()
            .filter(|b| b.block_type == block_type)
            .collect()
    }

    /// Check if page is likely a scan (needs OCR).
    ///
    /// Uses statistical analysis of content density instead of
    /// fixed character count heuristics. This is a first-principles
    /// approach that adapts to different document types.
    pub fn needs_ocr(&self) -> bool {
        if self.blocks.is_empty() {
            return true; // No content at all - likely needs OCR
        }

        // Calculate content density metrics
        let text_len: usize = self.blocks.iter().map(|b| b.text.len()).sum();
        let block_count = self.blocks.len();

        // Calculate average block size (currently reserved for future use)
        let _avg_block_size = if block_count > 0 {
            text_len as f32 / block_count as f32
        } else {
            0.0
        };

        // Calculate page area
        let page_area = self.width * self.height;

        // Calculate text density (characters per square point)
        // NOTE: Currently unused but may be useful for future adaptive thresholds
        let _text_density = if page_area > 0.0 {
            text_len as f32 / page_area
        } else {
            0.0
        };

        // Use adaptive thresholds based on page dimensions
        // Larger pages can have more text naturally
        let min_text_threshold = (page_area / 1000.0) as usize; // ~1 char per 1000 sq pts
        let min_block_threshold = (page_area / 50000.0) as usize; // ~1 block per 50000 sq pts

        // Check if content is sparse (likely needs OCR)
        // This is a first-principles approach based on density
        text_len < min_text_threshold.max(20) && block_count < min_block_threshold.max(3)
    }
}

impl Default for Page {
    fn default() -> Self {
        Self::new(1, 612.0, 792.0) // US Letter size in points
    }
}

/// Page margins.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct PageMargins {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl PageMargins {
    /// Create uniform margins.
    pub fn uniform(margin: f32) -> Self {
        Self {
            top: margin,
            right: margin,
            bottom: margin,
            left: margin,
        }
    }

    /// Get content area within margins.
    pub fn content_area(&self, page_width: f32, page_height: f32) -> BoundingBox {
        BoundingBox::new(
            self.left,
            self.top,
            page_width - self.right,
            page_height - self.bottom,
        )
    }
}

/// Table of contents entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    /// Entry title
    pub title: String,
    /// Heading level (1-6)
    pub level: u8,
    /// Page number (1-indexed)
    pub page: usize,
    /// Block ID of the heading
    pub block_id: Option<BlockId>,
    /// Nested entries
    pub children: Vec<TocEntry>,
}

impl TocEntry {
    /// Create a new TOC entry.
    pub fn new(title: impl Into<String>, level: u8, page: usize) -> Self {
        Self {
            title: title.into(),
            level,
            page,
            block_id: None,
            children: Vec::new(),
        }
    }

    /// Add a child entry.
    pub fn with_child(mut self, child: TocEntry) -> Self {
        self.children.push(child);
        self
    }

    /// Flatten to a list of entries.
    pub fn flatten(&self) -> Vec<&TocEntry> {
        let mut result = vec![self];
        for child in &self.children {
            result.extend(child.flatten());
        }
        result
    }
}

/// Document metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Document title
    pub title: Option<String>,
    /// Document author(s)
    pub author: Option<String>,
    /// Subject/description
    pub subject: Option<String>,
    /// Keywords
    pub keywords: Vec<String>,
    /// Creator application
    pub creator: Option<String>,
    /// Producer application
    pub producer: Option<String>,
    /// Creation date
    pub creation_date: Option<String>,
    /// Modification date
    pub modification_date: Option<String>,
    /// PDF version
    pub pdf_version: Option<String>,
    /// Is encrypted
    pub encrypted: bool,
    /// Is linearized (web-optimized)
    pub linearized: bool,
    /// Language
    pub language: Option<String>,
}

/// A complete extracted document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Source file path
    pub source: Option<String>,
    /// Document metadata
    pub metadata: DocumentMetadata,
    /// Pages in the document
    pub pages: Vec<Page>,
    /// Extracted table of contents
    pub toc: Vec<TocEntry>,
    /// Overall extraction method
    pub method: ExtractionMethod,
    /// Aggregated statistics
    pub stats: PageStats,
    /// Custom metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom_metadata: HashMap<String, serde_json::Value>,
}

impl Document {
    /// Create a new empty document.
    pub fn new() -> Self {
        Self {
            source: None,
            metadata: DocumentMetadata::default(),
            pages: Vec::new(),
            toc: Vec::new(),
            method: ExtractionMethod::Native,
            stats: PageStats::default(),
            custom_metadata: HashMap::new(),
        }
    }

    /// Create a document with source path.
    pub fn from_path(path: impl Into<String>) -> Self {
        let mut doc = Self::new();
        doc.source = Some(path.into());
        doc
    }

    /// Add a page to the document.
    pub fn add_page(&mut self, page: Page) {
        self.pages.push(page);
    }

    /// Get total page count.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Get total block count across all pages.
    pub fn total_blocks(&self) -> usize {
        self.pages.iter().map(|p| p.blocks.len()).sum()
    }

    /// Recalculate aggregated statistics.
    pub fn update_stats(&mut self) {
        let mut stats = PageStats::default();
        for page in &self.pages {
            stats.combine(&page.stats);
        }
        // Recalculate average confidence
        let total_blocks = self.total_blocks();
        if total_blocks > 0 {
            stats.avg_confidence = self
                .pages
                .iter()
                .flat_map(|p| p.blocks.iter())
                .map(|b| b.confidence)
                .sum::<f32>()
                / total_blocks as f32;
        }
        self.stats = stats;
    }

    /// Generate table of contents from section headers.
    pub fn generate_toc(&mut self) {
        self.toc.clear();

        for page in &self.pages {
            for block in page.blocks_of_type(BlockType::SectionHeader) {
                let level = block.level.unwrap_or(2);
                let entry = TocEntry {
                    title: block.text.clone(),
                    level,
                    page: page.number,
                    block_id: Some(block.id.clone()),
                    children: Vec::new(),
                };

                // Simple flat list for now; hierarchical TOC requires more logic
                self.toc.push(entry);
            }
        }
    }

    /// Get all text content from the document.
    pub fn get_text(&self) -> String {
        self.pages
            .iter()
            .map(|p| p.get_text())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Find blocks matching a predicate.
    pub fn find_blocks<F>(&self, predicate: F) -> Vec<&Block>
    where
        F: Fn(&Block) -> bool,
    {
        self.pages
            .iter()
            .flat_map(|p| p.blocks.iter())
            .filter(|b| predicate(b))
            .collect()
    }

    /// Get blocks of a specific type across all pages.
    pub fn blocks_of_type(&self, block_type: BlockType) -> Vec<&Block> {
        self.find_blocks(|b| b.block_type == block_type)
    }

    /// Get blocks from a specific page.
    pub fn blocks_on_page(&self, page_num: usize) -> Option<&[Block]> {
        self.pages.get(page_num).map(|p| p.blocks.as_slice())
    }

    /// Iterate over all blocks in the document.
    pub fn iter_blocks(&self) -> impl Iterator<Item = &Block> {
        self.pages.iter().flat_map(|p| p.blocks.iter())
    }

    /// Get mutable reference to page.
    pub fn page_mut(&mut self, page_num: usize) -> Option<&mut Page> {
        self.pages.get_mut(page_num)
    }

    /// Check if any page needs OCR.
    pub fn needs_ocr(&self) -> bool {
        self.pages.iter().any(|p| p.needs_ocr())
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_creation() {
        let mut page = Page::new(1, 612.0, 792.0);
        let bbox = BoundingBox::new(72.0, 72.0, 540.0, 100.0);
        page.add_block(Block::text("Hello world", bbox));

        assert_eq!(page.number, 1);
        assert_eq!(page.blocks.len(), 1);
    }

    #[test]
    fn test_page_stats() {
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        let blocks = vec![
            Block::text("First paragraph", bbox),
            Block::text("Second paragraph", bbox),
            Block::header("Chapter 1", 1, bbox),
        ];

        let stats = PageStats::from_blocks(&blocks);
        assert_eq!(stats.text_blocks, 2);
        assert_eq!(stats.headers, 1);
    }

    #[test]
    fn test_document_creation() {
        let mut doc = Document::from_path("/test/doc.pdf");
        let page = Page::new(1, 612.0, 792.0);
        doc.add_page(page);

        assert_eq!(doc.page_count(), 1);
        assert_eq!(doc.source, Some("/test/doc.pdf".to_string()));
    }

    #[test]
    fn test_document_find_blocks() {
        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 50.0);

        page.add_block(Block::text("Paragraph 1", bbox));
        page.add_block(Block::header("Header", 1, bbox));
        page.add_block(Block::text("Paragraph 2", bbox));

        doc.add_page(page);

        let headers = doc.blocks_of_type(BlockType::SectionHeader);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].text, "Header");

        let text_blocks = doc.find_blocks(|b| b.block_type == BlockType::Text);
        assert_eq!(text_blocks.len(), 2);
    }

    #[test]
    fn test_toc_generation() {
        let mut doc = Document::new();
        let mut page1 = Page::new(1, 612.0, 792.0);
        let mut page2 = Page::new(2, 612.0, 792.0);
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 30.0);

        page1.add_block(Block::header("Introduction", 1, bbox));
        page2.add_block(Block::header("Methods", 1, bbox));

        doc.add_page(page1);
        doc.add_page(page2);
        doc.generate_toc();

        assert_eq!(doc.toc.len(), 2);
        assert_eq!(doc.toc[0].title, "Introduction");
        assert_eq!(doc.toc[0].page, 1);
        assert_eq!(doc.toc[1].title, "Methods");
        assert_eq!(doc.toc[1].page, 2);
    }

    #[test]
    fn test_toc_entry_flatten() {
        let entry = TocEntry::new("Chapter 1", 1, 1)
            .with_child(TocEntry::new("Section 1.1", 2, 2))
            .with_child(TocEntry::new("Section 1.2", 2, 3));

        let flattened = entry.flatten();
        assert_eq!(flattened.len(), 3);
    }

    #[test]
    fn test_page_margins() {
        let margins = PageMargins {
            top: 72.0,
            right: 72.0,
            bottom: 72.0,
            left: 72.0,
        };

        let content = margins.content_area(612.0, 792.0);
        assert_eq!(content.x1, 72.0);
        assert_eq!(content.y1, 72.0);
        assert_eq!(content.x2, 540.0);
        assert_eq!(content.y2, 720.0);
    }

    #[test]
    fn test_extraction_method() {
        assert_eq!(ExtractionMethod::default(), ExtractionMethod::Native);
        assert_eq!(ExtractionMethod::Vision.label(), "Vision Model");
    }

    #[test]
    fn test_document_serialization() {
        let mut doc = Document::from_path("/test.pdf");
        doc.metadata.title = Some("Test Document".to_string());

        let mut page = Page::new(1, 612.0, 792.0);
        let bbox = BoundingBox::new(72.0, 72.0, 540.0, 100.0);
        page.add_block(Block::text("Test content", bbox));
        doc.add_page(page);

        let json = serde_json::to_string(&doc).unwrap();
        let parsed: Document = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.metadata.title, Some("Test Document".to_string()));
        assert_eq!(parsed.page_count(), 1);
    }
}
