//! Document builder - constructs document representation from PDF.
//!
//! The builder extracts raw content from PDF and creates the block-based
//! document representation.

use crate::config::PdfConfig;
use crate::error::PdfError;
use crate::layout::LayoutAnalyzer;
use crate::schema::{Block, BlockId, BlockType, BoundingBox, Document, ExtractionMethod, Page};
use crate::Result;

#[cfg(feature = "pdf_oxide")]
use pdf_oxide::converters::ConversionOptions;
#[cfg(feature = "pdf_oxide")]
use pdf_oxide::PdfDocument;

use std::io::Write;
use tempfile::NamedTempFile;

/// Builder for constructing documents from PDF.
pub struct DocumentBuilder {
    config: PdfConfig,
    layout_analyzer: LayoutAnalyzer,
}

impl DocumentBuilder {
    /// Create a new document builder.
    pub fn new(config: PdfConfig) -> Self {
        Self {
            config,
            layout_analyzer: LayoutAnalyzer::new(),
        }
    }

    /// Create with default config.
    pub fn with_defaults() -> Self {
        Self::new(PdfConfig::default())
    }

    /// Build a document from PDF bytes.
    pub fn build(&self, bytes: &[u8], source: Option<String>) -> Result<Document> {
        // Write to temp file (pdf_oxide requires file path)
        let mut temp_file = NamedTempFile::new()
            .map_err(|e| PdfError::PdfParse(format!("Failed to create temp file: {}", e)))?;
        temp_file
            .write_all(bytes)
            .map_err(|e| PdfError::PdfParse(format!("Failed to write temp file: {}", e)))?;
        let temp_path = temp_file.path();

        let mut pdf_doc = PdfDocument::open(temp_path)
            .map_err(|e| PdfError::PdfParse(format!("Failed to open PDF: {}", e)))?;

        let page_count = pdf_doc
            .page_count()
            .map_err(|e| PdfError::PdfParse(format!("Failed to get page count: {}", e)))?;

        let mut document = Document::new();
        document.source = source;
        document.method = ExtractionMethod::Native;

        // Extract PDF version as metadata
        let (major, minor) = pdf_doc.version();
        document.metadata.creator = Some(format!("PDF {}.{}", major, minor));

        // Process each page
        let options = ConversionOptions::default();
        let max_pages = self.config.max_pages.unwrap_or(page_count);
        let pages_to_process = page_count.min(max_pages);

        for page_num in 0..pages_to_process {
            // Extract markdown from this page
            let markdown = match pdf_doc.to_markdown(page_num, &options) {
                Ok(md) => md,
                Err(e) => {
                    tracing::warn!("Failed to convert page {} to markdown: {}", page_num, e);
                    String::new()
                }
            };

            // Parse into blocks and create page
            let page = self.build_page_from_markdown(&markdown, page_num);
            document.add_page(page);
        }

        // Update statistics
        document.update_stats();

        // Generate table of contents
        document.generate_toc();

        Ok(document)
    }

    /// Build a single page from markdown content.
    fn build_page_from_markdown(&self, markdown: &str, page_num: usize) -> Page {
        let page_width = 612.0; // US Letter
        let page_height = 792.0;

        let blocks = self.parse_markdown_to_blocks(markdown, page_num);

        let mut page = Page::new(page_num + 1, page_width, page_height);
        page.method = ExtractionMethod::Native;

        // Run layout analysis
        let layout = self
            .layout_analyzer
            .analyze(&blocks, page_width, page_height);
        page.columns = layout.columns.clone();

        for block in blocks {
            page.add_block(block);
        }

        page.update_stats();
        page
    }

    /// Parse markdown string into blocks.
    fn parse_markdown_to_blocks(&self, markdown: &str, page_num: usize) -> Vec<Block> {
        let mut blocks = Vec::new();
        let mut position = 0;
        let mut y_pos = 72.0; // Start below top margin
        let line_height = 14.0;
        let margin = 72.0;
        let page_width = 612.0;

        for line in markdown.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                y_pos += line_height;
                continue;
            }

            let bbox = BoundingBox::new(margin, y_pos, page_width - margin, y_pos + line_height);
            let (block_type, level, text) = self.classify_line(trimmed);

            let mut block = Block::new(block_type, bbox);
            block.id = BlockId::with_indices(page_num, position);
            block.text = text;
            block.page = page_num;
            block.position = position;
            block.level = level;
            block.confidence = 1.0;

            blocks.push(block);
            position += 1;
            y_pos += line_height * 1.5; // Add spacing between blocks
        }

        blocks
    }

    /// Classify a markdown line into block type.
    fn classify_line(&self, line: &str) -> (BlockType, Option<u8>, String) {
        // Check for headers (most specific first)
        if let Some(stripped) = line.strip_prefix("###### ") {
            return (BlockType::SectionHeader, Some(6), stripped.to_string());
        }
        if let Some(stripped) = line.strip_prefix("##### ") {
            return (BlockType::SectionHeader, Some(5), stripped.to_string());
        }
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

        // Check for list items
        if line.starts_with("- ") || line.starts_with("* ") {
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

        // Check for code blocks (already fenced in markdown)
        if line.starts_with("```") {
            return (BlockType::Code, None, line.to_string());
        }

        // Default to text
        (BlockType::Text, None, line.to_string())
    }
}

impl Default for DocumentBuilder {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Builder for individual pages.
pub struct PageBuilder {
    page_num: usize,
    width: f32,
    height: f32,
    blocks: Vec<Block>,
}

impl PageBuilder {
    /// Create a new page builder.
    pub fn new(page_num: usize, width: f32, height: f32) -> Self {
        Self {
            page_num,
            width,
            height,
            blocks: Vec::new(),
        }
    }

    /// Add a text block.
    pub fn add_text(&mut self, text: impl Into<String>, bbox: BoundingBox) -> &mut Self {
        let block = Block::text(text, bbox).on_page(self.page_num);
        self.blocks.push(block);
        self
    }

    /// Add a header block.
    pub fn add_header(
        &mut self,
        text: impl Into<String>,
        level: u8,
        bbox: BoundingBox,
    ) -> &mut Self {
        let block = Block::header(text, level, bbox).on_page(self.page_num);
        self.blocks.push(block);
        self
    }

    /// Build the page.
    pub fn build(mut self) -> Page {
        let mut page = Page::new(self.page_num, self.width, self.height);

        // Assign positions
        for (i, block) in self.blocks.iter_mut().enumerate() {
            block.position = i;
        }

        for block in self.blocks {
            page.add_block(block);
        }

        page.update_stats();
        page
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_builder() {
        let mut builder = PageBuilder::new(1, 612.0, 792.0);

        builder.add_header("Title", 1, BoundingBox::new(72.0, 72.0, 540.0, 100.0));
        builder.add_text(
            "Some content here.",
            BoundingBox::new(72.0, 120.0, 540.0, 150.0),
        );

        let page = builder.build();

        assert_eq!(page.number, 1);
        assert_eq!(page.blocks.len(), 2);
        assert_eq!(page.blocks[0].block_type, BlockType::SectionHeader);
        assert_eq!(page.blocks[1].block_type, BlockType::Text);
    }

    #[test]
    fn test_document_builder_creation() {
        let builder = DocumentBuilder::with_defaults();
        assert_eq!(builder.config.max_pages, None);
    }

    #[test]
    fn test_classify_line_headers() {
        let builder = DocumentBuilder::with_defaults();

        let (block_type, level, text) = builder.classify_line("# Title");
        assert_eq!(block_type, BlockType::SectionHeader);
        assert_eq!(level, Some(1));
        assert_eq!(text, "Title");

        let (block_type, level, _) = builder.classify_line("## Subtitle");
        assert_eq!(block_type, BlockType::SectionHeader);
        assert_eq!(level, Some(2));

        let (block_type, level, _) = builder.classify_line("### Section");
        assert_eq!(block_type, BlockType::SectionHeader);
        assert_eq!(level, Some(3));
    }

    #[test]
    fn test_classify_line_lists() {
        let builder = DocumentBuilder::with_defaults();

        let (block_type, _, _) = builder.classify_line("- Item");
        assert_eq!(block_type, BlockType::ListItem);

        let (block_type, _, _) = builder.classify_line("* Item");
        assert_eq!(block_type, BlockType::ListItem);

        let (block_type, _, _) = builder.classify_line("1. First");
        assert_eq!(block_type, BlockType::ListItem);
    }

    #[test]
    fn test_classify_line_code() {
        let builder = DocumentBuilder::with_defaults();

        let (block_type, _, _) = builder.classify_line("```rust");
        assert_eq!(block_type, BlockType::Code);
    }

    #[test]
    fn test_classify_line_text() {
        let builder = DocumentBuilder::with_defaults();

        let (block_type, level, text) = builder.classify_line("Regular paragraph text.");
        assert_eq!(block_type, BlockType::Text);
        assert_eq!(level, None);
        assert_eq!(text, "Regular paragraph text.");
    }

    #[test]
    fn test_parse_markdown_to_blocks() {
        let builder = DocumentBuilder::with_defaults();
        let markdown = "# Title\n\nSome text.\n\n- Item 1\n- Item 2";
        let blocks = builder.parse_markdown_to_blocks(markdown, 0);

        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[0].block_type, BlockType::SectionHeader);
        assert_eq!(blocks[1].block_type, BlockType::Text);
        assert_eq!(blocks[2].block_type, BlockType::ListItem);
        assert_eq!(blocks[3].block_type, BlockType::ListItem);
    }
}
