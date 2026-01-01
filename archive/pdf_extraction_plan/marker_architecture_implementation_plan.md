# EdgeQuake-PDF: Marker-Style Architecture Implementation Plan

**Date:** 2025-12-31  
**Version:** 1.0  
**Status:** ✅ COMPLETE  
**Target:** EdgeQuake-PDF v0.3.0

---

## ✅ IMPLEMENTATION COMPLETE

**All 8 phases have been implemented with 116 tests passing:**

- ✅ Phase 1: Block-based Schema Module (34 tests)
- ✅ Phase 2: Layout Detection Module (17 tests)
- ✅ Phase 3: Processor Traits/Implementations (12 tests)
- ✅ Phase 4: Renderer Traits/Implementations (8 tests)
- ✅ Phase 5: LLM Enhancement Processor (5 tests)
- ✅ Phase 6: Vision Mode Support (11 tests)
- ✅ Phase 7: Config & CLI Updates (6 tests)
- ✅ Phase 8: Integration & Testing (10 tests + 1 doctest)

See [implementation_summary.md](./implementation_summary.md) for details.

---

## Executive Summary

This document outlines a comprehensive plan to transform EdgeQuake-PDF from a simple text extraction library into a **Marker-equivalent Rust-native PDF-to-markdown converter** with:

1. **Block-based extraction** with semantic document structure
2. **Layout detection** using heuristics and optional ML models
3. **Optional LLM enhancement** (`--use_llm` flag)
4. **Vision mode** for complex documents

**Target Metrics:**

- Accuracy: 90%+ on standard benchmarks (vs Marker's 95.67%)
- Speed: 10+ pages/second on modern CPU
- Memory: < 500MB for typical documents

---

## Current State Analysis

### Existing Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         PdfExtractor                            │
├─────────────────────────────────────────────────────────────────┤
│  pdf_bytes → PdfDocument (pdf_oxide) → Page Loop                │
│                                           ↓                     │
│                                   doc.to_markdown(page)         │
│                                           ↓                     │
│                                   post_process_markdown()       │
│                                           ↓                     │
│                            [optional] refine_with_ai()          │
│                                           ↓                     │
│                                   Combined Markdown             │
└─────────────────────────────────────────────────────────────────┘
```

### Current Limitations

| Limitation                   | Impact         | Solution                  |
| ---------------------------- | -------------- | ------------------------- |
| No block detection           | Poor structure | Block-based schema        |
| No layout analysis           | Merged columns | Layout detection pipeline |
| Page-level processing only   | No context     | Document-level blocks     |
| No reading order             | Scrambled text | Reading order algorithm   |
| No table structure detection | Broken tables  | Table cell detection      |
| No vision fallback           | Fails on scans | Vision mode               |

### Current Strengths (Preserve)

- ✅ Rust-native, async-first design
- ✅ LLM provider abstraction (`LLMProvider` trait)
- ✅ Post-processing pipeline (spacing, headers, captions)
- ✅ Image extraction and AI description
- ✅ Clean configuration pattern (`PdfConfig`)

---

## Target Architecture (Marker-Style)

```
┌─────────────────────────────────────────────────────────────────────┐
│                       DocumentConverter                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────┐    ┌──────────────┐    ┌────────────────────────┐ │
│  │   Provider   │ →  │   Builder    │ →  │      Processors        │ │
│  │ (PDF Source) │    │ (Blocks)     │    │ (Table, Layout, etc.)  │ │
│  └──────────────┘    └──────────────┘    └────────────────────────┘ │
│                                                    ↓                 │
│                                          ┌────────────────────────┐ │
│                                          │       Renderer         │ │
│                                          │ (Markdown/JSON/HTML)   │ │
│                                          └────────────────────────┘ │
│                                                    ↓                 │
│                                          ┌────────────────────────┐ │
│                                          │    LLM Service         │ │
│                                          │ (Optional Enhancement) │ │
│                                          └────────────────────────┘ │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Plan

### Phase 1: Block-Based Schema (Week 1-2)

#### 1.1 Define Block Types

Create a comprehensive block schema in `src/schema/mod.rs`:

```rust
// src/schema/mod.rs

/// Block types in a document (similar to Marker's schema)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum BlockType {
    // Container blocks
    Page,
    Document,

    // Content blocks
    Text,
    TextInlineMath,
    Paragraph,
    SectionHeader,
    ListItem,

    // Special content
    Table,
    TableCell,
    TableRow,
    Figure,
    Picture,
    Caption,
    Code,
    Equation,
    Form,
    Footnote,

    // Document structure
    PageHeader,
    PageFooter,
    TableOfContents,

    // Handwritten content
    Handwriting,
}

/// A block in the document structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Unique identifier for this block
    pub id: BlockId,

    /// Type of block
    pub block_type: BlockType,

    /// Bounding box (x1, y1, x2, y2) in page coordinates
    pub bbox: BoundingBox,

    /// Polygon for non-rectangular regions
    pub polygon: Option<Vec<Point>>,

    /// Raw text content
    pub text: Option<String>,

    /// HTML representation (for complex blocks like tables)
    pub html: Option<String>,

    /// Child blocks (for nested structures)
    pub children: Vec<Block>,

    /// Reading order position
    pub position: Option<usize>,

    /// Section hierarchy (e.g., {1: "h1", 2: "h2"})
    pub section_hierarchy: HashMap<u8, String>,

    /// Confidence score from detection (0.0-1.0)
    pub confidence: f32,

    /// Page number (0-indexed)
    pub page: usize,
}

/// Bounding box representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl BoundingBox {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    pub fn width(&self) -> f32 {
        self.x2 - self.x1
    }

    pub fn height(&self) -> f32 {
        self.y2 - self.y1
    }

    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    pub fn center(&self) -> Point {
        Point {
            x: (self.x1 + self.x2) / 2.0,
            y: (self.y1 + self.y2) / 2.0,
        }
    }

    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.x1 < other.x2 && self.x2 > other.x1 &&
        self.y1 < other.y2 && self.y2 > other.y1
    }

    pub fn intersection_area(&self, other: &BoundingBox) -> f32 {
        let x_overlap = (self.x2.min(other.x2) - self.x1.max(other.x1)).max(0.0);
        let y_overlap = (self.y2.min(other.y2) - self.y1.max(other.y1)).max(0.0);
        x_overlap * y_overlap
    }

    pub fn iou(&self, other: &BoundingBox) -> f32 {
        let intersection = self.intersection_area(other);
        let union = self.area() + other.area() - intersection;
        if union > 0.0 { intersection / union } else { 0.0 }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// Unique block identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(String);

impl BlockId {
    pub fn new(page: usize, block_type: BlockType, index: usize) -> Self {
        Self(format!("/page/{}/{:?}/{}", page, block_type, index))
    }
}
```

#### 1.2 Define Document Structure

```rust
// src/schema/document.rs

/// Complete document representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Document pages
    pub pages: Vec<Page>,

    /// Document metadata
    pub metadata: DocumentMetadata,

    /// Table of contents (computed from headers)
    pub toc: Vec<TocEntry>,

    /// Page statistics
    pub page_stats: Vec<PageStats>,
}

/// A page in the document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// Page index (0-based)
    pub page_id: usize,

    /// Page dimensions (width, height)
    pub dimensions: (f32, f32),

    /// Blocks on this page (in reading order)
    pub blocks: Vec<Block>,

    /// Page-level image bbox
    pub image_bbox: BoundingBox,
}

/// Table of contents entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    pub title: String,
    pub heading_level: u8,
    pub page_id: usize,
    pub polygon: Option<Vec<Point>>,
}

/// Statistics for a page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageStats {
    pub page_id: usize,
    pub text_extraction_method: ExtractionMethod,
    pub block_counts: HashMap<BlockType, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractionMethod {
    PdfText,
    Ocr,
    Vision,
}
```

#### 1.3 File Structure for Phase 1

```
src/
├── lib.rs              # Updated exports
├── schema/
│   ├── mod.rs          # Block types, BoundingBox, Point
│   ├── block.rs        # Block struct and methods
│   ├── document.rs     # Document, Page structures
│   └── block_types.rs  # BlockType enum with helpers
├── config.rs           # Extended configuration
├── error.rs            # Extended error types
└── extractor.rs        # Existing (to be refactored)
```

---

### Phase 2: Provider/Builder/Processor/Renderer Pattern (Week 2-3)

#### 2.1 Provider Trait (PDF Source)

```rust
// src/providers/mod.rs

/// Provider trait for extracting raw content from documents
#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the document type
    fn document_type(&self) -> DocumentType;

    /// Get total page count
    fn page_count(&self) -> Result<usize>;

    /// Get page dimensions
    fn page_dimensions(&self, page: usize) -> Result<(f32, f32)>;

    /// Extract raw text from a page
    fn extract_text(&self, page: usize) -> Result<String>;

    /// Extract text with positions (for layout detection)
    fn extract_text_with_positions(&self, page: usize) -> Result<Vec<TextSpan>>;

    /// Render page to image (for vision mode)
    fn render_page(&self, page: usize, dpi: u32) -> Result<PageImage>;

    /// Extract images from page
    fn extract_images(&self, page: usize) -> Result<Vec<ExtractedImage>>;

    /// Get document metadata
    fn metadata(&self) -> Result<DocumentMetadata>;
}

/// Text span with position information
#[derive(Debug, Clone)]
pub struct TextSpan {
    pub text: String,
    pub bbox: BoundingBox,
    pub font_size: Option<f32>,
    pub font_name: Option<String>,
    pub is_bold: bool,
    pub is_italic: bool,
}

/// Rendered page image
#[derive(Debug)]
pub struct PageImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub format: ImageFormat,
}

#[derive(Debug, Clone, Copy)]
pub enum ImageFormat {
    Png,
    Jpeg,
    WebP,
}

#[derive(Debug, Clone, Copy)]
pub enum DocumentType {
    Pdf,
    Image,
    Docx,
    Html,
}
```

#### 2.2 PdfProvider Implementation

```rust
// src/providers/pdf.rs

use pdf_oxide::PdfDocument;

/// PDF provider using pdf_oxide
pub struct PdfProvider {
    doc: PdfDocument,
    temp_file: tempfile::NamedTempFile,
}

impl PdfProvider {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut temp_file = tempfile::Builder::new()
            .suffix(".pdf")
            .tempfile()?;
        temp_file.write_all(bytes)?;

        let doc = PdfDocument::open(temp_file.path())?;

        Ok(Self { doc, temp_file })
    }
}

#[async_trait]
impl Provider for PdfProvider {
    fn document_type(&self) -> DocumentType {
        DocumentType::Pdf
    }

    fn page_count(&self) -> Result<usize> {
        self.doc.page_count().map_err(Into::into)
    }

    fn extract_text(&self, page: usize) -> Result<String> {
        self.doc.extract_text(page).map_err(Into::into)
    }

    fn extract_text_with_positions(&self, page: usize) -> Result<Vec<TextSpan>> {
        // Use pdf_oxide's text extraction with positions
        // This requires extending pdf_oxide or using raw access
        todo!("Implement position-aware text extraction")
    }

    fn render_page(&self, page: usize, dpi: u32) -> Result<PageImage> {
        // Use pdfium-render or similar for page rendering
        todo!("Implement page rendering")
    }

    // ... other methods
}
```

#### 2.3 Builder Trait (Block Construction)

```rust
// src/builders/mod.rs

/// Builder trait for constructing document blocks
#[async_trait]
pub trait Builder: Send + Sync {
    /// Build blocks from provider for a specific page
    async fn build_page(&self, provider: &dyn Provider, page: usize) -> Result<Vec<Block>>;

    /// Build complete document
    async fn build_document(&self, provider: &dyn Provider) -> Result<Document>;
}

/// Default builder using heuristic layout detection
pub struct DefaultBuilder {
    config: BuilderConfig,
}

#[derive(Debug, Clone)]
pub struct BuilderConfig {
    /// Minimum font size to consider as header
    pub header_min_size: f32,

    /// Gap threshold for paragraph detection
    pub paragraph_gap: f32,

    /// Enable multi-column detection
    pub detect_columns: bool,

    /// Table detection sensitivity
    pub table_sensitivity: f32,
}

impl Default for BuilderConfig {
    fn default() -> Self {
        Self {
            header_min_size: 14.0,
            paragraph_gap: 1.5, // line height multiplier
            detect_columns: true,
            table_sensitivity: 0.7,
        }
    }
}
```

#### 2.4 Processor Trait (Block Processing)

```rust
// src/processors/mod.rs

/// Processor trait for transforming blocks
#[async_trait]
pub trait Processor: Send + Sync {
    /// Get processor name
    fn name(&self) -> &str;

    /// Get block types this processor handles
    fn handles(&self) -> &[BlockType];

    /// Process a block (may modify or return new block)
    async fn process(&self, block: Block, context: &ProcessorContext) -> Result<Block>;
}

/// Context passed to processors
pub struct ProcessorContext<'a> {
    /// Current page
    pub page: &'a Page,

    /// Full document (for cross-page context)
    pub document: Option<&'a Document>,

    /// LLM provider (if LLM mode enabled)
    pub llm: Option<&'a dyn LLMProvider>,

    /// Configuration
    pub config: &'a ProcessorConfig,
}

/// Configuration for processors
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    pub use_llm: bool,
    pub llm_model: String,
    pub ai_temperature: f32,
}
```

#### 2.5 Built-in Processors

```rust
// src/processors/table.rs
pub struct TableProcessor { /* ... */ }

// src/processors/header.rs
pub struct HeaderProcessor { /* ... */ }

// src/processors/reading_order.rs
pub struct ReadingOrderProcessor { /* ... */ }

// src/processors/math.rs
pub struct MathProcessor { /* ... */ }

// src/processors/llm_enhance.rs
pub struct LlmEnhanceProcessor { /* ... */ }
```

#### 2.6 Renderer Trait (Output Generation)

```rust
// src/renderers/mod.rs

/// Output format for rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Markdown,
    Html,
    Json,
    Chunks, // RAG-optimized
}

/// Renderer trait for generating output
pub trait Renderer: Send + Sync {
    /// Get output format
    fn format(&self) -> OutputFormat;

    /// Render document to string
    fn render(&self, document: &Document) -> Result<String>;

    /// Render single block
    fn render_block(&self, block: &Block) -> Result<String>;
}

// Implementations
pub struct MarkdownRenderer { /* ... */ }
pub struct HtmlRenderer { /* ... */ }
pub struct JsonRenderer { /* ... */ }
pub struct ChunksRenderer { chunk_size: usize, /* ... */ }
```

---

### Phase 3: Layout Detection Pipeline (Week 3-4)

#### 3.1 Layout Detection Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Layout Detection Pipeline                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  TextSpans → Column Detection → Block Grouping → Type Classification
│                                                                  │
│       ↓              ↓                ↓               ↓         │
│   Raw text    [Col1, Col2]    [Paragraph,     [Header, Text,    │
│   positions                    Table, ...]     Figure, ...]     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

#### 3.2 Column Detection (XY-Cut Algorithm)

```rust
// src/layout/column_detector.rs

/// Detect columns using recursive XY-cut algorithm
pub struct ColumnDetector {
    /// Minimum gap to consider as column separator
    pub min_gap: f32,

    /// Minimum column width
    pub min_width: f32,
}

impl ColumnDetector {
    /// Detect columns from text spans
    pub fn detect(&self, spans: &[TextSpan], page_width: f32) -> Vec<ColumnRegion> {
        self.xy_cut(spans, BoundingBox::new(0.0, 0.0, page_width, f32::MAX))
    }

    /// Recursive XY-cut
    fn xy_cut(&self, spans: &[TextSpan], region: BoundingBox) -> Vec<ColumnRegion> {
        if spans.is_empty() {
            return vec![];
        }

        // Try vertical cut first (column separation)
        if let Some(cut) = self.find_vertical_cut(spans, &region) {
            let (left, right) = self.split_by_x(spans, cut);
            let mut result = self.xy_cut(&left, region.left_half(cut));
            result.extend(self.xy_cut(&right, region.right_half(cut)));
            return result;
        }

        // Try horizontal cut (paragraph separation)
        if let Some(cut) = self.find_horizontal_cut(spans, &region) {
            let (top, bottom) = self.split_by_y(spans, cut);
            let mut result = self.xy_cut(&top, region.top_half(cut));
            result.extend(self.xy_cut(&bottom, region.bottom_half(cut)));
            return result;
        }

        // Base case: single region
        vec![ColumnRegion {
            bbox: region,
            spans: spans.to_vec(),
        }]
    }

    fn find_vertical_cut(&self, spans: &[TextSpan], region: &BoundingBox) -> Option<f32> {
        // Find largest vertical gap
        let mut x_coords: Vec<f32> = spans.iter()
            .flat_map(|s| vec![s.bbox.x1, s.bbox.x2])
            .collect();
        x_coords.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut max_gap = 0.0;
        let mut cut_point = None;

        for pair in x_coords.windows(2) {
            let gap = pair[1] - pair[0];
            if gap > max_gap && gap >= self.min_gap {
                max_gap = gap;
                cut_point = Some((pair[0] + pair[1]) / 2.0);
            }
        }

        cut_point
    }

    // ... similar for horizontal cut
}

#[derive(Debug, Clone)]
pub struct ColumnRegion {
    pub bbox: BoundingBox,
    pub spans: Vec<TextSpan>,
}
```

#### 3.3 Block Type Classification (Heuristics)

```rust
// src/layout/classifier.rs

/// Classify blocks based on heuristics
pub struct BlockClassifier {
    config: ClassifierConfig,
}

#[derive(Debug, Clone)]
pub struct ClassifierConfig {
    /// Font size threshold for headers
    pub header_size_threshold: f32,

    /// Font size difference for header detection
    pub header_size_ratio: f32,

    /// Keywords that indicate section headers
    pub section_keywords: Vec<String>,

    /// Table detection: minimum grid lines
    pub table_min_lines: usize,
}

impl BlockClassifier {
    /// Classify a region into a block type
    pub fn classify(&self, region: &ColumnRegion, page_context: &PageContext) -> BlockType {
        // 1. Check for table patterns (grid lines, aligned columns)
        if self.is_table(region) {
            return BlockType::Table;
        }

        // 2. Check for code blocks (monospace, indentation)
        if self.is_code(region) {
            return BlockType::Code;
        }

        // 3. Check for headers (large font, short text, section keywords)
        if self.is_header(region, page_context) {
            return BlockType::SectionHeader;
        }

        // 4. Check for equations (math symbols, centered)
        if self.is_equation(region) {
            return BlockType::Equation;
        }

        // 5. Check for list items (bullets, numbers)
        if self.is_list_item(region) {
            return BlockType::ListItem;
        }

        // 6. Check for captions (near figures, italic, "Figure X:")
        if self.is_caption(region, page_context) {
            return BlockType::Caption;
        }

        // 7. Check for footnotes (bottom of page, small font)
        if self.is_footnote(region, page_context) {
            return BlockType::Footnote;
        }

        // 8. Check for page header/footer
        if self.is_page_header(region, page_context) {
            return BlockType::PageHeader;
        }
        if self.is_page_footer(region, page_context) {
            return BlockType::PageFooter;
        }

        // Default: regular text
        BlockType::Text
    }

    fn is_table(&self, region: &ColumnRegion) -> bool {
        // Check for:
        // 1. Multiple aligned columns
        // 2. Consistent vertical spacing
        // 3. Presence of delimiters (|, -)
        // 4. Grid-like structure

        let spans = &region.spans;
        if spans.len() < 4 {
            return false;
        }

        // Check for column alignment
        let x_positions: std::collections::HashSet<i32> = spans.iter()
            .map(|s| (s.bbox.x1 * 10.0) as i32)
            .collect();

        // If many spans align to same X positions, likely a table
        let alignment_ratio = x_positions.len() as f32 / spans.len() as f32;
        alignment_ratio < 0.3 // Most spans align to few X positions
    }

    fn is_header(&self, region: &ColumnRegion, ctx: &PageContext) -> bool {
        let spans = &region.spans;
        if spans.is_empty() {
            return false;
        }

        // Short text
        let total_chars: usize = spans.iter().map(|s| s.text.len()).sum();
        if total_chars > 200 {
            return false;
        }

        // Check font size
        let avg_font_size = spans.iter()
            .filter_map(|s| s.font_size)
            .sum::<f32>() / spans.len() as f32;

        let is_larger = avg_font_size > ctx.avg_font_size * self.config.header_size_ratio;

        // Check for section keywords
        let text = spans.iter().map(|s| &s.text).collect::<Vec<_>>().join(" ");
        let has_section_keyword = self.config.section_keywords.iter()
            .any(|kw| text.to_lowercase().contains(&kw.to_lowercase()));

        // Check for bold
        let is_bold = spans.iter().all(|s| s.is_bold);

        (is_larger || is_bold) && (has_section_keyword || total_chars < 100)
    }

    // ... other classification methods
}
```

#### 3.4 Reading Order Detection

```rust
// src/layout/reading_order.rs

/// Determine reading order for blocks
pub struct ReadingOrderDetector {
    /// Column-first or top-first ordering
    pub strategy: OrderingStrategy,
}

#[derive(Debug, Clone, Copy)]
pub enum OrderingStrategy {
    /// Process top-to-bottom, left-to-right
    TopToBottom,

    /// Process by columns (for multi-column layouts)
    ByColumn,

    /// Auto-detect based on layout
    Auto,
}

impl ReadingOrderDetector {
    /// Order blocks in reading sequence
    pub fn order(&self, blocks: &mut Vec<Block>, columns: &[ColumnRegion]) {
        match self.strategy {
            OrderingStrategy::TopToBottom => {
                self.order_top_to_bottom(blocks);
            }
            OrderingStrategy::ByColumn => {
                self.order_by_column(blocks, columns);
            }
            OrderingStrategy::Auto => {
                if columns.len() > 1 {
                    self.order_by_column(blocks, columns);
                } else {
                    self.order_top_to_bottom(blocks);
                }
            }
        }
    }

    fn order_by_column(&self, blocks: &mut Vec<Block>, columns: &[ColumnRegion]) {
        // Sort columns left-to-right
        let mut column_order: Vec<usize> = (0..columns.len()).collect();
        column_order.sort_by(|&a, &b| {
            columns[a].bbox.x1.partial_cmp(&columns[b].bbox.x1).unwrap()
        });

        // Assign blocks to columns
        let mut block_columns: Vec<(usize, usize)> = blocks.iter().enumerate()
            .map(|(idx, block)| {
                let col = self.find_column(block, columns);
                (idx, col)
            })
            .collect();

        // Sort by column, then by y position
        block_columns.sort_by(|&(a_idx, a_col), &(b_idx, b_col)| {
            let a_col_order = column_order.iter().position(|&c| c == a_col).unwrap_or(0);
            let b_col_order = column_order.iter().position(|&c| c == b_col).unwrap_or(0);

            match a_col_order.cmp(&b_col_order) {
                std::cmp::Ordering::Equal => {
                    blocks[a_idx].bbox.y1.partial_cmp(&blocks[b_idx].bbox.y1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }
                other => other,
            }
        });

        // Apply order
        for (position, (block_idx, _)) in block_columns.iter().enumerate() {
            blocks[*block_idx].position = Some(position);
        }
    }

    fn find_column(&self, block: &Block, columns: &[ColumnRegion]) -> usize {
        columns.iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                let a_overlap = block.bbox.intersection_area(&a.bbox);
                let b_overlap = block.bbox.intersection_area(&b.bbox);
                a_overlap.partial_cmp(&b_overlap).unwrap()
            })
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    fn order_top_to_bottom(&self, blocks: &mut Vec<Block>) {
        blocks.sort_by(|a, b| {
            a.bbox.y1.partial_cmp(&b.bbox.y1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for (position, block) in blocks.iter_mut().enumerate() {
            block.position = Some(position);
        }
    }
}
```

---

### Phase 4: Optional LLM Enhancement (Week 4-5)

#### 4.1 LLM Service Trait

```rust
// src/llm/mod.rs

/// LLM service for document enhancement
#[async_trait]
pub trait LlmService: Send + Sync {
    /// Get service name
    fn name(&self) -> &str;

    /// Enhance a block with LLM
    async fn enhance_block(&self, block: &Block, context: &str) -> Result<Block>;

    /// Format table using LLM
    async fn format_table(&self, table: &Block) -> Result<String>;

    /// Convert inline math to LaTeX
    async fn convert_math(&self, text: &str) -> Result<String>;

    /// Merge content across pages
    async fn merge_pages(&self, page1: &str, page2: &str) -> Result<String>;

    /// Correct block with custom prompt
    async fn correct_block(&self, block: &Block, prompt: &str) -> Result<Block>;

    /// Check if vision is supported
    fn supports_vision(&self) -> bool;

    /// Process page image (vision mode)
    async fn process_image(&self, image: &PageImage, prompt: &str) -> Result<String>;
}

/// LLM configuration
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Model to use (e.g., "gpt-4o", "claude-3-sonnet")
    pub model: String,

    /// Temperature for generation
    pub temperature: f32,

    /// Maximum tokens
    pub max_tokens: usize,

    /// System prompt for enhancement
    pub system_prompt: Option<String>,

    /// Block correction prompt
    pub correction_prompt: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o-mini".to_string(),
            temperature: 0.1,
            max_tokens: 4096,
            system_prompt: None,
            correction_prompt: None,
        }
    }
}
```

#### 4.2 OpenAI LLM Service Implementation

```rust
// src/llm/openai.rs

use edgequake_llm::providers::openai::OpenAIProvider;

pub struct OpenAILlmService {
    provider: Arc<OpenAIProvider>,
    config: LlmConfig,
}

impl OpenAILlmService {
    pub fn new(provider: Arc<OpenAIProvider>, config: LlmConfig) -> Self {
        Self { provider, config }
    }
}

#[async_trait]
impl LlmService for OpenAILlmService {
    fn name(&self) -> &str {
        "openai"
    }

    fn supports_vision(&self) -> bool {
        // Models that support vision
        matches!(self.config.model.as_str(),
            "gpt-4o" | "gpt-4o-mini" | "gpt-4-vision-preview")
    }

    async fn format_table(&self, table: &Block) -> Result<String> {
        let prompt = format!(
            r#"Convert this table content to a properly formatted Markdown table.
Preserve all data and structure. Use proper alignment.

Input:
{}

Output the Markdown table only, no explanation:"#,
            table.text.as_deref().unwrap_or("")
        );

        let response = self.provider.complete(&prompt).await?;
        Ok(response.text)
    }

    async fn convert_math(&self, text: &str) -> Result<String> {
        let prompt = format!(
            r#"Convert any mathematical expressions in this text to LaTeX format.
Use $...$ for inline math and $$...$$ for display math.
Preserve all other text exactly as is.

Input: {}

Output:"#,
            text
        );

        let response = self.provider.complete(&prompt).await?;
        Ok(response.text)
    }

    async fn process_image(&self, image: &PageImage, prompt: &str) -> Result<String> {
        // Encode image as base64
        let base64_image = base64::encode(&image.data);

        let messages = vec![
            ChatMessage::system(
                "You are a document parser. Convert the document image to clean Markdown format. \
                 Preserve structure, tables, equations, and formatting."
            ),
            ChatMessage::user(format!(
                "[Image: data:image/{};base64,{}]\n\n{}",
                match image.format {
                    ImageFormat::Png => "png",
                    ImageFormat::Jpeg => "jpeg",
                    ImageFormat::WebP => "webp",
                },
                base64_image,
                prompt
            )),
        ];

        let response = self.provider.chat(&messages, None).await?;
        Ok(response.text)
    }

    // ... other implementations
}
```

#### 4.3 LLM Enhancement Processor

```rust
// src/processors/llm_enhance.rs

/// Processor that uses LLM to enhance blocks
pub struct LlmEnhanceProcessor {
    llm_service: Arc<dyn LlmService>,
    config: LlmEnhanceConfig,
}

#[derive(Debug, Clone)]
pub struct LlmEnhanceConfig {
    /// Enhance tables
    pub enhance_tables: bool,

    /// Convert inline math
    pub convert_math: bool,

    /// Merge cross-page content
    pub merge_pages: bool,

    /// Use custom correction prompt
    pub correction_prompt: Option<String>,
}

impl LlmEnhanceProcessor {
    pub fn new(llm_service: Arc<dyn LlmService>, config: LlmEnhanceConfig) -> Self {
        Self { llm_service, config }
    }
}

#[async_trait]
impl Processor for LlmEnhanceProcessor {
    fn name(&self) -> &str {
        "llm_enhance"
    }

    fn handles(&self) -> &[BlockType] {
        &[
            BlockType::Table,
            BlockType::TextInlineMath,
            BlockType::Equation,
            BlockType::Form,
        ]
    }

    async fn process(&self, mut block: Block, ctx: &ProcessorContext) -> Result<Block> {
        match block.block_type {
            BlockType::Table if self.config.enhance_tables => {
                if let Some(ref text) = block.text {
                    let formatted = self.llm_service.format_table(&block).await?;
                    block.html = Some(formatted);
                }
            }

            BlockType::TextInlineMath | BlockType::Equation if self.config.convert_math => {
                if let Some(ref text) = block.text {
                    let converted = self.llm_service.convert_math(text).await?;
                    block.text = Some(converted);
                }
            }

            _ => {}
        }

        // Apply custom correction if configured
        if let Some(ref prompt) = self.config.correction_prompt {
            block = self.llm_service.correct_block(&block, prompt).await?;
        }

        Ok(block)
    }
}
```

---

### Phase 5: Vision Mode (Week 5-6)

#### 5.1 Vision Mode Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      Vision Mode Pipeline                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  PDF Page → Render (DPI) → Encode Base64 → Vision LLM → Markdown │
│                                                                  │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────────────┐│
│  │ pdfium  │ →  │  PNG    │ →  │ base64  │ →  │ GPT-4o/Claude   ││
│  │ render  │    │ image   │    │ string  │    │ Vision Model    ││
│  └─────────┘    └─────────┘    └─────────┘    └─────────────────┘│
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

#### 5.2 Vision Provider

```rust
// src/providers/vision.rs

use crate::llm::LlmService;

/// Vision-based PDF provider (uses LLM to read page images)
pub struct VisionProvider {
    /// Underlying PDF provider for rendering
    pdf_provider: PdfProvider,

    /// LLM service for vision processing
    llm_service: Arc<dyn LlmService>,

    /// Rendering DPI
    dpi: u32,
}

impl VisionProvider {
    pub fn new(
        pdf_bytes: &[u8],
        llm_service: Arc<dyn LlmService>,
        dpi: u32,
    ) -> Result<Self> {
        let pdf_provider = PdfProvider::from_bytes(pdf_bytes)?;

        if !llm_service.supports_vision() {
            return Err(PdfError::Config(
                "LLM service does not support vision mode".to_string()
            ));
        }

        Ok(Self {
            pdf_provider,
            llm_service,
            dpi,
        })
    }

    /// Extract markdown from page using vision
    pub async fn extract_page_markdown(&self, page: usize) -> Result<String> {
        // Render page to image
        let image = self.pdf_provider.render_page(page, self.dpi)?;

        let prompt = r#"Convert this document page to clean Markdown format.

Instructions:
1. Preserve the document structure (headers, paragraphs, lists)
2. Format tables using Markdown table syntax
3. Convert equations to LaTeX (inline: $...$, display: $$...$$)
4. Describe images with [Image: description]
5. Preserve reading order (handle multi-column layouts correctly)
6. Remove headers, footers, and page numbers
7. Keep all text content accurate

Output the Markdown content only:"#;

        self.llm_service.process_image(&image, prompt).await
    }
}
```

#### 5.3 Page Rendering with pdfium-render

Add to `Cargo.toml`:

```toml
[dependencies]
pdfium-render = { version = "0.8", optional = true }

[features]
default = []
vision = ["pdfium-render"]
```

```rust
// src/providers/pdf_renderer.rs

#[cfg(feature = "vision")]
use pdfium_render::prelude::*;

pub struct PdfRenderer {
    pdfium: Pdfium,
}

impl PdfRenderer {
    #[cfg(feature = "vision")]
    pub fn new() -> Result<Self> {
        let pdfium = Pdfium::default();
        Ok(Self { pdfium })
    }

    #[cfg(feature = "vision")]
    pub fn render_page(&self, pdf_bytes: &[u8], page: usize, dpi: u32) -> Result<PageImage> {
        let document = self.pdfium.load_pdf_from_byte_slice(pdf_bytes, None)?;
        let page = document.pages().get(page)?;

        let render_config = PdfRenderConfig::new()
            .set_target_width((page.width().value * dpi as f32 / 72.0) as i32)
            .set_target_height((page.height().value * dpi as f32 / 72.0) as i32);

        let bitmap = page.render_with_config(&render_config)?;

        // Convert to PNG
        let image = bitmap.as_image();
        let mut png_data = Vec::new();
        image.write_to(&mut std::io::Cursor::new(&mut png_data), image::ImageFormat::Png)?;

        Ok(PageImage {
            width: image.width(),
            height: image.height(),
            data: png_data,
            format: ImageFormat::Png,
        })
    }

    #[cfg(not(feature = "vision"))]
    pub fn new() -> Result<Self> {
        Err(PdfError::Config("Vision feature not enabled. Add vision feature to Cargo.toml".to_string()))
    }
}
```

#### 5.4 Hybrid Extraction Mode

```rust
// src/extractors/hybrid.rs

/// Hybrid extractor that combines text and vision modes
pub struct HybridExtractor {
    /// Text-based extractor
    text_extractor: DocumentConverter,

    /// Vision provider (optional)
    vision_provider: Option<VisionProvider>,

    /// Threshold for switching to vision mode
    quality_threshold: f32,
}

impl HybridExtractor {
    /// Extract with automatic mode selection
    pub async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document> {
        // First, try text extraction
        let text_result = self.text_extractor.convert(pdf_bytes).await?;

        // Check quality of extraction
        let quality = self.assess_quality(&text_result);

        if quality >= self.quality_threshold {
            return Ok(text_result);
        }

        // Quality too low, use vision mode
        if let Some(ref vision) = self.vision_provider {
            info!("Text extraction quality low ({:.2}), switching to vision mode", quality);
            return self.extract_with_vision(vision, pdf_bytes).await;
        }

        // No vision available, return text result with warning
        warn!("Low quality extraction ({:.2}) but vision mode not available", quality);
        Ok(text_result)
    }

    /// Assess extraction quality (0.0 - 1.0)
    fn assess_quality(&self, doc: &Document) -> f32 {
        let mut score = 1.0;

        for page in &doc.pages {
            // Check for common quality issues
            for block in &page.blocks {
                if let Some(ref text) = block.text {
                    // Penalize for garbled text
                    let garbled_ratio = self.count_garbled_chars(text) as f32 / text.len() as f32;
                    score -= garbled_ratio * 0.5;

                    // Penalize for very short blocks (extraction failure)
                    if text.len() < 10 && block.block_type == BlockType::Text {
                        score -= 0.1;
                    }

                    // Penalize for merged words
                    let merged_ratio = self.count_merged_words(text) as f32 / text.split_whitespace().count() as f32;
                    score -= merged_ratio * 0.3;
                }
            }
        }

        score.max(0.0)
    }

    fn count_garbled_chars(&self, text: &str) -> usize {
        text.chars()
            .filter(|c| !c.is_ascii() && !c.is_alphanumeric() && !c.is_whitespace())
            .count()
    }

    fn count_merged_words(&self, text: &str) -> usize {
        // Count camelCase patterns that aren't normal
        let re = regex::Regex::new(r"[a-z][A-Z]").unwrap();
        re.find_iter(text).count()
    }
}
```

---

### Phase 6: Configuration & CLI (Week 6-7)

#### 6.1 Extended Configuration

```rust
// src/config.rs

/// Complete extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    // Extraction mode
    pub mode: ExtractionMode,

    // Output format
    pub output_format: OutputFormat,

    // Page range
    pub page_range: Option<PageRange>,

    // LLM enhancement
    pub use_llm: bool,
    pub llm_service: LlmServiceConfig,

    // Layout detection
    pub layout: LayoutConfig,

    // Processors to run
    pub processors: Vec<ProcessorConfig>,

    // Rendering options
    pub render: RenderConfig,

    // Performance
    pub parallel_pages: usize,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExtractionMode {
    /// Fast text-based extraction
    Text,

    /// Vision-based extraction (requires LLM)
    Vision,

    /// Auto-detect and switch modes
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// Enable column detection
    pub detect_columns: bool,

    /// Enable table detection
    pub detect_tables: bool,

    /// Enable equation detection
    pub detect_equations: bool,

    /// Reading order strategy
    pub reading_order: OrderingStrategy,

    /// Minimum gap for column separation
    pub column_gap_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmServiceConfig {
    /// Service type (openai, anthropic, gemini, ollama)
    pub service: String,

    /// Model name
    pub model: String,

    /// API key (from env if not set)
    pub api_key: Option<String>,

    /// Base URL (for custom endpoints)
    pub base_url: Option<String>,

    /// Temperature
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderConfig {
    /// Include page numbers
    pub include_page_numbers: bool,

    /// Paginate output with separators
    pub paginate: bool,

    /// Extract and include images
    pub extract_images: bool,

    /// Describe images with AI
    pub describe_images: bool,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            mode: ExtractionMode::Text,
            output_format: OutputFormat::Markdown,
            page_range: None,
            use_llm: false,
            llm_service: LlmServiceConfig::default(),
            layout: LayoutConfig::default(),
            processors: vec![],
            render: RenderConfig::default(),
            parallel_pages: 4,
            batch_size: 10,
        }
    }
}
```

#### 6.2 CLI Interface

```rust
// src/bin/edgequake-pdf.rs

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "edgequake-pdf")]
#[command(about = "Convert PDF documents to Markdown with AI enhancement")]
struct Cli {
    /// Input file or directory
    #[arg(required = true)]
    input: PathBuf,

    /// Output directory
    #[arg(short, long, default_value = ".")]
    output_dir: PathBuf,

    /// Output format
    #[arg(short, long, value_enum, default_value = "markdown")]
    format: OutputFormatArg,

    /// Use LLM for enhanced extraction
    #[arg(long)]
    use_llm: bool,

    /// LLM service (openai, anthropic, gemini, ollama)
    #[arg(long, default_value = "openai")]
    llm_service: String,

    /// LLM model
    #[arg(long)]
    model: Option<String>,

    /// Use vision mode for extraction
    #[arg(long)]
    vision: bool,

    /// Hybrid mode: auto-detect when to use vision
    #[arg(long)]
    hybrid: bool,

    /// Force OCR on all pages
    #[arg(long)]
    force_ocr: bool,

    /// Page range (e.g., "0,5-10,20")
    #[arg(long)]
    page_range: Option<String>,

    /// Parallel processing workers
    #[arg(long, default_value = "4")]
    workers: usize,

    /// Configuration file path
    #[arg(long)]
    config: Option<PathBuf>,

    /// Enable debug output
    #[arg(long)]
    debug: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(ValueEnum, Clone)]
enum OutputFormatArg {
    Markdown,
    Json,
    Html,
    Chunks,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let log_level = if cli.debug {
        tracing::Level::DEBUG
    } else if cli.verbose {
        tracing::Level::INFO
    } else {
        tracing::Level::WARN
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();

    // Build configuration
    let config = build_config(&cli)?;

    // Create converter
    let converter = DocumentConverter::with_config(config)?;

    // Process input
    if cli.input.is_dir() {
        process_directory(&converter, &cli.input, &cli.output_dir).await?;
    } else {
        process_file(&converter, &cli.input, &cli.output_dir).await?;
    }

    Ok(())
}
```

---

## File Structure (Final)

```
edgequake-pdf/
├── Cargo.toml
├── src/
│   ├── lib.rs                    # Main exports
│   │
│   ├── schema/                   # Document structure types
│   │   ├── mod.rs
│   │   ├── block.rs              # Block struct
│   │   ├── block_types.rs        # BlockType enum
│   │   ├── document.rs           # Document, Page
│   │   └── geometry.rs           # BoundingBox, Point
│   │
│   ├── providers/                # Document source providers
│   │   ├── mod.rs                # Provider trait
│   │   ├── pdf.rs                # PdfProvider (pdf_oxide)
│   │   ├── image.rs              # ImageProvider
│   │   └── vision.rs             # VisionProvider
│   │
│   ├── builders/                 # Block construction
│   │   ├── mod.rs                # Builder trait
│   │   └── default.rs            # DefaultBuilder
│   │
│   ├── layout/                   # Layout detection
│   │   ├── mod.rs
│   │   ├── column_detector.rs    # XY-cut algorithm
│   │   ├── classifier.rs         # Block type classification
│   │   ├── reading_order.rs      # Reading order detection
│   │   └── table_detector.rs     # Table structure detection
│   │
│   ├── processors/               # Block processors
│   │   ├── mod.rs                # Processor trait
│   │   ├── header.rs             # Header consolidation
│   │   ├── table.rs              # Table formatting
│   │   ├── math.rs               # Math/equation handling
│   │   ├── reading_order.rs      # Reading order processor
│   │   └── llm_enhance.rs        # LLM enhancement processor
│   │
│   ├── renderers/                # Output generation
│   │   ├── mod.rs                # Renderer trait
│   │   ├── markdown.rs           # Markdown output
│   │   ├── html.rs               # HTML output
│   │   ├── json.rs               # JSON output
│   │   └── chunks.rs             # RAG chunks output
│   │
│   ├── llm/                      # LLM services
│   │   ├── mod.rs                # LlmService trait
│   │   ├── openai.rs             # OpenAI implementation
│   │   ├── anthropic.rs          # Claude implementation
│   │   ├── gemini.rs             # Gemini implementation
│   │   └── ollama.rs             # Ollama implementation
│   │
│   ├── converter.rs              # Main DocumentConverter
│   ├── config.rs                 # Configuration types
│   ├── error.rs                  # Error types
│   └── extractor.rs              # Legacy (deprecated)
│
├── bin/
│   └── edgequake-pdf.rs          # CLI binary
│
├── examples/
│   ├── basic_conversion.rs
│   ├── with_llm.rs
│   ├── vision_mode.rs
│   └── custom_pipeline.rs
│
└── tests/
    ├── integration/
    │   ├── text_extraction.rs
    │   ├── table_detection.rs
    │   ├── layout_detection.rs
    │   └── llm_enhancement.rs
    └── fixtures/
        ├── simple.pdf
        ├── two_column.pdf
        ├── tables.pdf
        └── scanned.pdf
```

---

## Implementation Timeline

| Phase | Description                         | Duration | Dependencies |
| ----- | ----------------------------------- | -------- | ------------ |
| 1     | Block-based schema                  | 2 weeks  | None         |
| 2     | Provider/Builder/Processor/Renderer | 2 weeks  | Phase 1      |
| 3     | Layout detection pipeline           | 2 weeks  | Phase 2      |
| 4     | LLM enhancement                     | 1 week   | Phase 2      |
| 5     | Vision mode                         | 2 weeks  | Phase 4      |
| 6     | CLI and configuration               | 1 week   | Phase 1-5    |

**Total: 10 weeks**

---

## Success Criteria

### Functional Requirements

- [ ] Extract structured blocks from PDFs
- [ ] Detect multi-column layouts correctly
- [ ] Preserve reading order in complex documents
- [ ] Format tables properly (with/without LLM)
- [ ] Handle scanned PDFs via vision mode
- [ ] Support multiple output formats

### Performance Requirements

- [ ] Process 10+ pages/second on CPU (text mode)
- [ ] Process 2+ pages/second with LLM enhancement
- [ ] Memory usage < 500MB for typical documents
- [ ] Parallel processing for large documents

### Quality Requirements

- [ ] 85%+ accuracy on standard benchmarks (text mode)
- [ ] 90%+ accuracy with LLM enhancement
- [ ] 95%+ accuracy in vision mode
- [ ] Pass all regression tests

---

## Risks and Mitigations

| Risk                          | Impact | Mitigation                        |
| ----------------------------- | ------ | --------------------------------- |
| pdf_oxide lacks position data | High   | Contribute upstream or use pdfium |
| Vision mode cost too high     | Medium | Implement smart fallback, caching |
| Layout detection accuracy     | Medium | ML models via candle/ONNX         |
| Cross-platform pdfium issues  | Medium | Provide static linking option     |

---

## References

- [Marker Architecture](https://github.com/VikParuchuri/marker)
- [Surya OCR](https://github.com/VikParuchuri/surya)
- [XY-Cut Algorithm](https://en.wikipedia.org/wiki/Recursive_XY-cut)
- [pdf_oxide Documentation](https://docs.rs/pdf_oxide)
- [pdfium-render Documentation](https://docs.rs/pdfium-render)
