# PDF Processing Deep Dive

**Status**: ✅ Production Ready  
**Crate**: `edgequake-pdf`  
**Source**: [`edgequake/crates/edgequake-pdf/`](../../../edgequake/crates/edgequake-pdf/)

---

## Table of Contents

1. [Introduction](#introduction)
2. [Architecture](#architecture)
3. [Basic Usage](#basic-usage)
4. [Table Detection](#table-detection)
5. [Layout Analysis](#layout-analysis)
6. [Processing Pipeline](#processing-pipeline)
7. [Advanced Topics](#advanced-topics)
8. [Troubleshooting](#troubleshooting)
9. [Comparison](#comparison)
10. [References](#references)

---

## Introduction

### What Problem Does EdgeQuake PDF Solve?

**The Challenge**: Most RAG systems require clean, structured text input. However, real-world knowledge often exists in PDF documents with:

- Complex table structures
- Multi-column layouts
- Mixed text encodings
- Embedded formulas and images
- Inconsistent reading order

**Why Existing Tools Fail**:

- **PyPDF2**: Simple text extraction, no structure preservation
- **pdfplumber**: Good for tables but slow, Python-only
- **Camelot**: Requires exact table borders, brittle
- **Marker**: Excellent but lacks customization, black-box LLM calls

**EdgeQuake's Approach**:

1. **Block-Based Representation**: Inspired by Marker's schema
2. **Spatial Analysis**: Y-coordinate clustering for rows, X-coordinate for columns
3. **Graceful Degradation**: Extract partial content when pages fail
4. **LLM Enhancement**: Optional AI-powered cleanup and formatting
5. **Pipeline Architecture**: Pluggable processors for customization

### When to Use PDF Extraction

**Use EdgeQuake PDF when**:

- ✅ You need structured Markdown from academic papers
- ✅ Your PDFs contain tables (financial reports, research data)
- ✅ Multi-column layouts must preserve reading order
- ✅ You want quality metrics to filter bad extractions
- ✅ Integration with EdgeQuake's RAG pipeline

**Alternatives**:

- ⚠️ **Pre-extracted text available**: Skip PDF processing entirely
- ⚠️ **Scanned documents (images)**: Use Vision models or OCR first
- ⚠️ **Highly complex layouts**: May require manual review

---

## Architecture

### Processing Pipeline

```
┌──────────────────────────────────────────────────────────────────┐
│                    PDF PROCESSING PIPELINE                       │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  INPUT                                                           │
│  ┌────────────┐                                                  │
│  │ PDF File   │                                                  │
│  │ (bytes)    │                                                  │
│  └──────┬─────┘                                                  │
│         │                                                        │
│         ▼                                                        │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │ STAGE 1: Backend Extraction                               │   │
│  │                                                           │   │
│  │ Backend: lopdf (default) or Mock (testing)                │   │
│  │ • Load PDF structure (pages, fonts, metadata)             │   │
│  │ • Extract raw text blocks with bounding boxes             │   │
│  │ • Parse content streams (Tj, TJ operators)                │   │
│  │ • Track fonts, styles, positions                          │   │
│  │                                                           │   │
│  │ Output: Document { pages: [Page] }                        │   │
│  └─────────────────────┬─────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │ STAGE 2: Layout Analysis                                  │   │
│  │                                                           │   │
│  │ ColumnDetector:                                           │   │
│  │   • Detect multi-column layouts (XY-Cut algorithm)        │   │
│  │   • Split text blocks by columns                          │   │
│  │                                                           │   │
│  │ ReadingOrderDetector:                                     │   │
│  │   • Establish top-to-bottom, left-to-right order          │   │
│  │   • Handle zig-zag patterns in multi-column docs          │   │
│  │                                                           │   │
│  │ Output: Page { blocks: [Block], columns: [BBox] }         │   │
│  └─────────────────────┬─────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │ STAGE 3: Structure Detection (Processor Chain)            │   │
│  │                                                           │   │
│  │ 1. MarginFilterProcessor                                  │   │
│  │    • Remove headers/footers (top/bottom 10% of page)      │   │
│  │                                                           │   │
│  │ 2. StyleDetectionProcessor                                │   │
│  │    • Detect bold, italic, font sizes                      │   │
│  │                                                           │   │
│  │ 3. HeaderDetectionProcessor                               │   │
│  │    • Identify section headers (font > avg + 2pt)          │   │
│  │                                                           │   │
│  │ 4. ListDetectionProcessor                                 │   │
│  │    • Detect bullets (•, -, *, numbers)                    │   │
│  │                                                           │   │
│  │ 5. TableDetectionProcessor                                │   │
│  │    • Group blocks by Y-coordinate (rows)                  │   │
│  │    • Detect columnar structure (X-alignment)              │   │
│  │    • Create Table blocks with TableCell children          │   │
│  │                                                           │   │
│  │ 6. CaptionDetectionProcessor                              │   │
│  │    • Detect "Table 1.", "Figure 2." patterns              │   │
│  │                                                           │   │
│  │ 7. CodeBlockDetectionProcessor                            │   │
│  │    • Identify monospace fonts                             │   │
│  │                                                           │   │
│  │ 8. BlockMergeProcessor                                    │   │
│  │    • Merge adjacent paragraphs                            │   │
│  │                                                           │   │
│  │ 9. GarbledTextFilterProcessor                             │   │
│  │    • Remove non-printable, control characters             │   │
│  │                                                           │   │
│  │ 10. HyphenContinuationProcessor                           │   │
│  │     • Join hyphenated words across lines                  │   │
│  │                                                           │   │
│  │ Output: Document with structured BlockType annotations    │   │
│  └─────────────────────┬─────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │ STAGE 4: LLM Enhancement (Optional)                       │   │
│  │                                                           │   │
│  │ LlmEnhanceProcessor:                                      │   │
│  │   • Clean garbled text                                    │   │
│  │   • Fix OCR errors                                        │   │
│  │   • Normalize formatting                                  │   │
│  │                                                           │   │
│  │ Requires: LLM provider (OpenAI, Ollama, etc.)             │   │
│  └─────────────────────┬─────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │ STAGE 5: Markdown Rendering                               │   │
│  │                                                           │   │
│  │ MarkdownRenderer:                                         │   │
│  │   • Convert blocks to Markdown                            │   │
│  │   • Format tables as | Col1 | Col2 |                      │   │
│  │   • Add headings (#, ##, ###)                             │   │
│  │   • Preserve lists and code blocks                        │   │
│  │                                                           │   │
│  │ Styles: Standard, GitHub, Custom                          │   │
│  └─────────────────────┬─────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  OUTPUT                                                          │
│  ┌────────────────────────────────────────────────────────┐      │
│  │ ExtractionResult {                                     │      │
│  │   markdown: String,                                    │      │
│  │   pages: Vec<PageContent>,                             │      │
│  │   images: Vec<ExtractedImage>,                         │      │
│  │   metadata: DocumentMetadata,                          │      │
│  │   page_errors: Vec<(usize, String)>                    │      │
│  │ }                                                      │      │
│  └────────────────────────────────────────────────────────┘      │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘

WHY THIS DESIGN:
1. **Staged pipeline**: Early failure detection, modular processing
2. **Block-based schema**: Semantic units (table, header, code) not raw text
3. **Spatial analysis**: Leverages PDF coordinates, not just text order
4. **Processor chain**: Each processor has single responsibility
5. **Graceful degradation**: Failed pages don't break entire document
6. **Optional LLM**: Quality improvement without mandatory cloud costs
```

### Key Components

#### 1. PdfBackend

**Purpose**: Abstract PDF parsing library (currently `lopdf`)

**Trait**:

```rust
pub trait PdfBackend {
    fn extract_document(&self, pdf_bytes: &[u8]) -> Result<Document>;
}
```

**Implementations**:

- `LopdfBackend`: Production backend using lopdf crate
- `MockBackend`: Testing backend with deterministic output

**Why abstraction?** Future-proofing for alternative backends (pdfium, poppler)

#### 2. Document Schema

**Block-Based Representation** (inspired by Marker):

```rust
pub struct Document {
    pub pages: Vec<Page>,
    pub metadata: DocumentMetadata,
    pub toc: Vec<TocEntry>, // Table of contents
}

pub struct Page {
    pub number: usize,
    pub width: f32,  // points
    pub height: f32, // points
    pub blocks: Vec<Block>,
    pub columns: Vec<BoundingBox>,
    pub stats: PageStats,
}

pub struct Block {
    pub id: BlockId,
    pub block_type: BlockType,
    pub text: String,
    pub bbox: BoundingBox, // x1, y1, x2, y2
    pub font_size: f32,
    pub font_name: String,
    pub confidence: f32,   // 0.0-1.0
    pub children: Vec<Block>, // For tables, lists
}

pub enum BlockType {
    Text,
    Paragraph,
    SectionHeader,
    Title,
    Table,
    TableCell,
    Figure,
    Caption,
    List,
    ListItem,
    Code,
    Equation,
    // ...
}
```

**Why blocks?**

- **Semantic meaning**: Not just "text at (x,y)" but "this is a table header"
- **Hierarchical**: Tables contain cells, lists contain items
- **Metadata-rich**: Confidence scores, fonts, bounding boxes
- **LLM-friendly**: Structured input for enhancement

#### 3. Processor Chain

**Pattern**: Chain of Responsibility

```rust
pub trait Processor {
    fn process(&self, document: Document) -> Result<Document>;
    fn name(&self) -> &str;
}

impl ProcessorChain {
    pub fn builder() -> ProcessorBuilder;
    pub fn process(&self, document: Document) -> Result<Document> {
        let mut doc = document;
        for processor in &self.processors {
            doc = processor.process(doc)?;
        }
        Ok(doc)
    }
}
```

**Why chain?**

- **Composable**: Enable/disable processors via config
- **Testable**: Each processor isolated
- **Debuggable**: Inspect document state between stages

---

## Basic Usage

### Quick Start

```rust
use edgequake_pdf::{PdfExtractor, PdfConfig};
use edgequake_llm::providers::mock::MockProvider;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create LLM provider (required for optional enhancement)
    let provider = Arc::new(MockProvider::new());

    // Initialize extractor
    let extractor = PdfExtractor::new(provider);

    // Load PDF bytes
    let pdf_bytes = std::fs::read("research-paper.pdf")?;

    // Extract to Markdown
    let markdown = extractor.extract_to_markdown(&pdf_bytes).await?;

    println!("Extracted Markdown:\n{}", markdown);
    Ok(())
}
```

**Source**: [`edgequake-pdf/src/lib.rs:52-67`](../../../edgequake/crates/edgequake-pdf/src/lib.rs#L52-L67)

### Get Detailed Results

```rust
use edgequake_pdf::{PdfExtractor, PdfConfig};
use edgequake_llm::providers::mock::MockProvider;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = Arc::new(MockProvider::new());
    let extractor = PdfExtractor::new(provider);

    let pdf_bytes = std::fs::read("document.pdf")?;

    // Get full extraction result
    let result = extractor.extract_full(&pdf_bytes).await?;

    // Access metadata
    println!("Pages: {}", result.page_count);
    println!("Title: {}", result.metadata.title);
    println!("Status: {}", result.status_summary());

    // Access per-page content
    for page in &result.pages {
        println!("Page {}: {} chars", page.page_number + 1, page.text.len());
    }

    // Access extracted images
    for img in &result.images {
        println!("Image {}: {} ({})", img.id, img.mime_type, img.description.as_deref().unwrap_or("no description"));
    }

    // Check for errors (graceful degradation)
    for error in &result.page_errors {
        eprintln!("Warning: Page {} failed - {}", error.page, error.error);
    }

    Ok(())
}
```

**Source**: [`edgequake-pdf/src/extractor.rs:308-340`](../../../edgequake/crates/edgequake-pdf/src/extractor.rs#L308-L340)

### Custom Configuration

```rust
use edgequake_pdf::{PdfExtractor, PdfConfig, ExtractionMode, OutputFormat};
use edgequake_llm::providers::mock::MockProvider;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = Arc::new(MockProvider::new());

    // Customize extraction config
    let config = PdfConfig {
        mode: ExtractionMode::Text, // vs Vision, Hybrid
        output_format: OutputFormat::Markdown,
        enhance_tables: false, // Disable LLM for speed
        enhance_readability: false,
        max_pages: Some(100), // Limit pages
        include_page_numbers: true,
        ..Default::default()
    };

    let extractor = PdfExtractor::with_config(provider, config);

    let pdf_bytes = std::fs::read("document.pdf")?;
    let markdown = extractor.extract_to_markdown(&pdf_bytes).await?;

    println!("{}", markdown);
    Ok(())
}
```

**Source**: [`edgequake-pdf/src/config.rs:189-260`](../../../edgequake/crates/edgequake-pdf/src/config.rs#L189-L260)

---

## Table Detection

### How It Works

EdgeQuake uses **spatial clustering** to detect tables from text block positions:

```
┌──────────────────────────────────────────────────────────────────┐
│                    TABLE DETECTION ALGORITHM                     │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  INPUT: Text blocks with bounding boxes                          │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │ Block("Name",  bbox=(50, 100, 100, 110), font_size=10)    │   │
│  │ Block("Age",   bbox=(200, 100, 240, 110), font_size=10)   │   │
│  │ Block("City",  bbox=(350, 100, 400, 110), font_size=10)   │   │
│  │ Block("Alice", bbox=(50, 85, 100, 95), font_size=10)      │   │
│  │ Block("25",    bbox=(200, 85, 220, 95), font_size=10)     │   │
│  │ Block("NYC",   bbox=(350, 85, 380, 95), font_size=10)     │   │
│  └───────────────────────────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │ STEP 1: Group Blocks by Y-Coordinate (ROWS)               │   │
│  │                                                           │   │
│  │ Algorithm:                                                │   │
│  │   for each block in sorted_by_y1(blocks):                 │   │
│  │     for each existing row:                                │   │
│  │       if vertical_overlap(block, row[0]) > 0.5 * min_height: 
│  │         row.add(block)                                    │   │
│  │         break                                             │   │
│  │     if not added:                                         │   │
│  │       create new_row([block])                             │   │
│  │                                                           │   │
│  │ Result:                                                   │   │
│  │   Row 0 (y≈100): [Name, Age, City]                        │   │
│  │   Row 1 (y≈85):  [Alice, 25, NYC]                         │   │
│  │                                                           │   │
│  │ WHY 0.5 overlap: Blocks on same row have >50% vertical    │   │
│  │ overlap. Handles slight misalignment from PDF extraction. │   │
│  └─────────────────────┬─────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │ STEP 2: Sort Each Row by X-Coordinate (LEFT-TO-RIGHT)     │   │
│  │                                                           │   │
│  │ for each row in rows:                                     │   │
│  │   row.sort_by_x1()                                        │   │
│  │                                                           │   │
│  │ Result:                                                   │   │
│  │   Row 0: [Name@x=50, Age@x=200, City@x=350]               │   │
│  │   Row 1: [Alice@x=50, 25@x=200, NYC@x=350]                │   │
│  └─────────────────────┬─────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │ STEP 3: Find Table Extent (Consecutive Multi-Block Rows)  │   │
│  │                                                           │   │
│  │ Starting from first row with >1 blocks:                   │   │
│  │   Extend table while:                                     │   │
│  │     • Next row has >1 blocks, AND                         │   │
│  │     • Max gap between blocks < 150pt (not columns)        │   │
│  │   OR:                                                     │   │
│  │     • Next row is single block aligned with table columns │   │
│  │       (merged cell or spanning header)                    │   │
│  │                                                           │   │
│  │ WHY 150pt threshold: Typical table cell gap is 10-50pt,   │   │
│  │ multi-column layout gap is 150-300pt. This distinguishes  │   │
│  │ tables from two-column text.                              │   │
│  │                                                           │   │
│  │ WHY 0.8 alignment: Single-block rows must have 80%        │   │
│  │ X-overlap with existing columns to be part of table.      │   │
│  └─────────────────────┬─────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │ STEP 4: Validate Table Likelihood                         │   │
│  │                                                           │   │
│  │ Requirements (OODA fix 2026-01-04):                       │   │
│  │   • At least 3 rows total                                 │   │
│  │   • At least one row with >1 blocks                       │   │
│  │                                                           │   │
│  │ WHY 3 rows minimum: Avoid false positives from short      │   │
│  │ multi-line phrases. Real tables have multiple data rows.  │   │
│  │                                                           │   │
│  │ REJECTED: Prior threshold was 6 rows, missed small tables.│   │
│  └─────────────────────┬─────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │ STEP 5: Create Table Block                                │   │
│  │                                                           │   │
│  │ table_block = Block {                                     │   │
│  │   block_type: Table,                                      │   │
│  │   bbox: union_of_all_cells,                               │   │
│  │   children: [                                             │   │
│  │     Cell("Name"), Cell("Age"), Cell("City"),              │   │
│  │     Cell("Alice"), Cell("25"), Cell("NYC")                │   │
│  │   ]                                                       │   │
│  │ }                                                         │   │
│  └─────────────────────┬─────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  OUTPUT: Table Block                                             │
│  ┌────────────────────────────────────────────────────────┐      │
│  │ | Name  | Age | City |                                 │      │
│  │ |-------|-----|------|                                 │      │
│  │ | Alice | 25  | NYC  |                                 │      │
│  └────────────────────────────────────────────────────────┘      │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘

EDGE CASES HANDLED:
1. **Ragged tables** (uneven columns): Accepted, aligned best-effort
2. **Merged cells**: Single block spanning multiple columns detected
3. **Multi-column layouts**: Skipped via page.columns.len() > 1 check
4. **Complex merged cells**: May produce incorrect structure
5. **Nested tables**: Not supported (children flattened)
```

**Source**: [`edgequake-pdf/src/processors/table_detection.rs:20-300`](../../../edgequake/crates/edgequake-pdf/src/processors/table_detection.rs#L20-L300)

### Code Example: Accessing Tables

```rust
use edgequake_pdf::{PdfExtractor, BlockType};
use edgequake_llm::providers::mock::MockProvider;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = Arc::new(MockProvider::new());
    let extractor = PdfExtractor::new(provider);

    let pdf_bytes = std::fs::read("financial-report.pdf")?;

    // Extract Document for block-level access
    let doc = extractor.extract_document(&pdf_bytes).await?;

    // Find all tables
    for page in &doc.pages {
        for block in &page.blocks {
            if block.block_type == BlockType::Table {
                println!("Table on page {}", page.number);
                println!("  Cells: {}", block.children.len());
                println!("  Bounding box: {:?}", block.bbox);

                // Access cells
                for cell in &block.children {
                    println!("    Cell: {}", cell.text);
                }
            }
        }
    }

    Ok(())
}
```

**Source**: [`edgequake-pdf/src/extractor.rs:224-240`](../../../edgequake/crates/edgequake-pdf/src/extractor.rs#L224-L240)

### When Table Detection Fails

**Scenario 1: Multi-Column Layout Detected as Table**

**Problem**: Two-column academic paper with side-by-side text looks like table rows.

**Solution**: `TableDetectionProcessor` skips pages with `page.columns.len() > 1`

```rust
// In table_detection.rs:
if page.columns.len() > 1 {
    tracing::info!("Skipping multi-column page ({} columns)", page.columns.len());
    continue;
}
```

**Source**: [`edgequake-pdf/src/processors/table_detection.rs:63-68`](../../../edgequake/crates/edgequake-pdf/src/processors/table_detection.rs#L63-L68)

**Scenario 2: Complex Merged Cells**

**Problem**: Table with cells spanning multiple rows/columns produces incorrect structure.

**Workaround**: Use `TextTableReconstructionProcessor` to parse text-based tables:

```rust
use edgequake_pdf::processors::TextTableReconstructionProcessor;

let processor = TextTableReconstructionProcessor::new();
let doc = processor.process(doc)?;
```

**Source**: [`edgequake-pdf/src/processors/table_detection.rs:300-450`](../../../edgequake/crates/edgequake-pdf/src/processors/table_detection.rs#L300-L450)

---

## Layout Analysis

### Multi-Column Detection

EdgeQuake uses the **XY-Cut algorithm** to detect columns:

```
┌──────────────────────────────────────────────────────────────────┐
│                        XY-CUT ALGORITHM                          │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  PURPOSE: Recursively split page into columns/regions            │
│                                                                  │
│  INPUT: Page with text blocks                                    │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  Page (8.5" x 11")                                         │  │
│  │  ┌─────────────────────┐  ┌─────────────────────┐          │  │
│  │  │ Column 1            │  │ Column 2            │          │  │
│  │  │ Text blocks here... │  │ Text blocks here... │          │  │
│  │  └─────────────────────┘  └─────────────────────┘          │  │
│  └────────────────────────────────────────────────────────────┘  │
│                        │                                         │
│                        ▼                                         │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │ STEP 1: Project Blocks onto X-Axis                        │   │
│  │                                                           │   │
│  │ Create histogram of horizontal positions:                 │   │
│  │                                                           │   │
│  │   X-axis: 0     100    200    300    400    500    600    │   │
│  │           │      │      │      │      │      │      │     │   │
│  │   Density: ████ ____ ████ ____ ████ ██████ ████ ____ ████ │   │
│  │           (Col1)     (Gap)    (Col2)                      │   │
│  │                                                           │   │
│  │ WHY: Large gaps in X-projection indicate column boundaries│   │
│  └─────────────────────┬─────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │ STEP 2: Find Vertical Cut (Largest Gap)                   │   │
│  │                                                           │   │
│  │ Scan histogram for widest gap:                            │   │
│  │   gap_threshold = page_width * 0.1  // 10% of page        │   │
│  │   if max_gap_width > gap_threshold:                       │   │
│  │     cut_x = gap_center                                    │   │
│  │                                                           │   │
│  │ WHY 10%: Column gap is typically 5-15% of page width.     │   │
│  │ Smaller gaps are whitespace within paragraphs.            │   │
│  └─────────────────────┬─────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │ STEP 3: Split Page at Cut                                 │   │
│  │                                                           │   │
│  │ Left region:  blocks where bbox.x2 < cut_x                │   │
│  │ Right region: blocks where bbox.x1 > cut_x                │   │
│  │                                                           │   │
│  │ Recursively apply XY-Cut to each region:                  │   │
│  │   • Try vertical cuts first (columns)                     │   │
│  │   • Try horizontal cuts (sections) if no vertical gap     │   │
│  │   • Stop when no gaps > threshold                         │   │
│  └─────────────────────┬─────────────────────────────────────┘   │
│                        │                                         │
│                        ▼                                         │
│  OUTPUT: Column Bounding Boxes                                   │
│  ┌────────────────────────────────────────────────────────┐      │
│  │ columns = [                                            │      │
│  │   BBox { x1: 50, x2: 280, y1: 50, y2: 750 },           │      │
│  │   BBox { x1: 320, x2: 550, y1: 50, y2: 750 }           │      │
│  │ ]                                                      │      │
│  └────────────────────────────────────────────────────────┘      │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

**Source**: [`edgequake-pdf/src/layout/xy_cut.rs:1-200`](../../../edgequake/crates/edgequake-pdf/src/layout/xy_cut.rs#L1-L200)

### Reading Order Detection

Once columns are detected, `ReadingOrderDetector` establishes block sequence:

**Algorithm**:

1. **Single column**: Top-to-bottom by Y1-coordinate
2. **Multi-column**: Process each column top-to-bottom, left-to-right column order

```rust
impl ReadingOrderDetector {
    pub fn detect_order(&self, page: &Page) -> Vec<usize> {
        if page.columns.len() <= 1 {
            // Single column: sort by Y
            self.sort_by_y(&page.blocks)
        } else {
            // Multi-column: process column-by-column
            let mut order = Vec::new();
            for column in &page.columns {
                let blocks_in_column = self.blocks_in_bbox(&page.blocks, column);
                order.extend(self.sort_by_y(&blocks_in_column));
            }
            order
        }
    }
}
```

**Source**: [`edgequake-pdf/src/layout/reading_order.rs:50-100`](../../../edgequake/crates/edgequake-pdf/src/layout/reading_order.rs#L50-L100)

---

## Processing Pipeline

### Processor Chain Example

```rust
use edgequake_pdf::processors::*;

// Build custom processor chain
let chain = ProcessorChain::builder()
    .add(MarginFilterProcessor::new())
    .add(StyleDetectionProcessor::new())
    .add(HeaderDetectionProcessor::new())
    .add(ListDetectionProcessor::new())
    .add(TableDetectionProcessor::new())
    .add(BlockMergeProcessor::new())
    .build();

// Process document
let processed_doc = chain.process(raw_doc)?;
```

**Source**: [`edgequake-pdf/src/processors/builder.rs:20-60`](../../../edgequake/crates/edgequake-pdf/src/processors/builder.rs#L20-L60)

### Available Processors

| Processor                     | Purpose                    | Configuration                                  |
| ----------------------------- | -------------------------- | ---------------------------------------------- |
| `MarginFilterProcessor`       | Remove headers/footers     | `top_margin: 0.1, bottom_margin: 0.1`          |
| `StyleDetectionProcessor`     | Detect bold/italic/fonts   | None                                           |
| `HeaderDetectionProcessor`    | Identify section headers   | `font_size_threshold: 2.0` (2pt above average) |
| `ListDetectionProcessor`      | Detect bullets/numbers     | `patterns: ["•", "-", "*", r"\d+\."]`          |
| `TableDetectionProcessor`     | Detect spatial tables      | `min_rows: 3, max_gap: 150.0`                  |
| `CaptionDetectionProcessor`   | Find "Table 1", "Figure 2" | `patterns: ["Table", "Figure", "Fig"]`         |
| `CodeBlockDetectionProcessor` | Identify code (monospace)  | `monospace_fonts: ["Courier", "Monaco"]`       |
| `BlockMergeProcessor`         | Merge adjacent paragraphs  | `max_gap: 20.0`                                |
| `GarbledTextFilterProcessor`  | Remove non-printable chars | None                                           |
| `HyphenContinuationProcessor` | Join hyphenated words      | None                                           |
| `LlmEnhanceProcessor`         | LLM-powered cleanup        | `provider: Arc<dyn LLMProvider>`               |

### Graceful Degradation

**Problem**: A single corrupted page shouldn't fail entire document extraction.

**Solution**: `ExtractionResult` tracks per-page errors:

```rust
pub struct ExtractionResult {
    pub page_count: usize,
    pub markdown: String,        // Only successful pages
    pub pages: Vec<PageContent>, // Only successful pages
    pub page_errors: Vec<(usize, String)>, // Failed pages
    // ...
}
```

**Usage**:

```rust
let result = extractor.extract(&pdf_bytes).await?;

if !result.page_errors.is_empty() {
    eprintln!("Warning: {} pages failed extraction", result.page_errors.len());
    for (page_num, error) in &result.page_errors {
        eprintln!("  Page {}: {}", page_num, error);
    }
}

// Still use successfully extracted pages
println!("Extracted {} / {} pages", result.pages.len(), result.page_count);
```

**Source**: [`edgequake-pdf/src/extractor.rs:90-110`](../../../edgequake/crates/edgequake-pdf/src/extractor.rs#L90-L110)

**Why this design?**

- **Real-world PDFs are messy**: Corrupt fonts, bad encodings, malformed content
- **Partial data better than none**: 9/10 pages extracted > complete failure
- **User choice**: Application decides acceptable failure rate

---

## Advanced Topics

### LLM Enhancement

Enable LLM-powered text cleanup:

```rust
use edgequake_pdf::{PdfExtractor, PdfConfig};
use edgequake_llm::providers::openai::OpenAIProvider;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use real LLM provider
    let provider = Arc::new(OpenAIProvider::new("your-api-key")?);

    // Enable LLM enhancement
    let mut config = PdfConfig::default();
    config.enable_llm_enhance = true;

    let extractor = PdfExtractor::with_config(provider, config);

    let pdf_bytes = std::fs::read("noisy-scan.pdf")?;
    let markdown = extractor.extract_to_markdown(&pdf_bytes).await?;

    // LLM cleaned up OCR errors, normalized formatting
    println!("{}", markdown);
    Ok(())
}
```

**What LLM Enhancement Does**:

- Fix OCR errors (common character misrecognitions)
- Normalize formatting (inconsistent spacing, capitalization)
- Clean garbled text (encoding issues)
- **Not used for**: Content generation or summarization (violation of extraction principle)

**Source**: [`edgequake-pdf/src/processors/llm_enhance.rs:1-100`](../../../edgequake/crates/edgequake-pdf/src/processors/llm_enhance.rs#L1-L100)

### Vision Model Extraction

For image-heavy or complex layouts, use vision models:

```rust
let mut config = PdfConfig::default();
config.mode = ExtractionMode::Vision;

let extractor = PdfExtractor::with_config(provider, config);
```

**How it works**:

1. Render PDF pages to images (150 DPI by default)
2. Send images to vision model (GPT-4V, Claude-3, etc.)
3. Extract structured content from model response
4. Fall back to text extraction if vision fails (Hybrid mode)

**Use cases**:

- Forms with complex layouts
- Documents with significant graphical elements
- Scanned documents (low-quality OCR)

**Source**: [`edgequake-pdf/src/vision.rs:1-200`](../../../edgequake/crates/edgequake-pdf/src/vision.rs#L1-L200)

### Performance Tuning

#### Text Mode (Fast)

```rust
let config = PdfConfig {
    mode: ExtractionMode::Text,
    enhance_tables: false,
    enhance_readability: false,
    ..Default::default()
};
```

**Trade-offs**:

- ✅ 3-5x faster extraction
- ✅ No LLM costs
- ❌ Lower quality structure detection
- ❌ May miss complex tables

#### Hybrid Mode (Balanced)

```rust
let config = PdfConfig {
    mode: ExtractionMode::Hybrid,
    quality_threshold: 0.5, // Switch to vision if quality < 50%
    enhance_tables: true,
    ..Default::default()
};
```

**Trade-offs**:

- ✅ Best structure preservation
- ✅ Automatic fallback to vision
- ✅ Only uses LLM when needed
- ❌ Variable cost (depends on PDF quality)

#### Batch Processing

Process multiple PDFs in parallel:

```rust
use tokio::task::JoinSet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = Arc::new(MockProvider::new());
    let extractor = Arc::new(PdfExtractor::new(provider));

    let pdf_files = vec!["doc1.pdf", "doc2.pdf", "doc3.pdf"];
    let mut tasks = JoinSet::new();

    for pdf_file in pdf_files {
        let extractor = Arc::clone(&extractor);
        let pdf_file = pdf_file.to_string();

        tasks.spawn(async move {
            let bytes = std::fs::read(&pdf_file)?;
            extractor.extract_to_markdown(&bytes).await
        });
    }

    while let Some(result) = tasks.join_next().await {
        match result? {
            Ok(markdown) => println!("Success: {} chars", markdown.len()),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}
```

**Concurrency considerations**:

- CPU-bound: PDF parsing (lopdf)
- IO-bound: LLM API calls
- **Recommended**: Parallelize at document level, not page level (reduces LLM call overhead)

---

## Troubleshooting

### Common Issues

#### 1. No Text Extracted

**Symptoms**:

```rust
let result = extractor.extract_full(&pdf_bytes).await?;
assert!(result.markdown.is_empty()); // Empty!
```

**Causes**:

- PDF contains only images (scanned document)
- PDF uses unsupported font encoding
- PDF content stream is corrupted

**Solutions**:

1. Enable vision mode:

   ```rust
   config.mode = ExtractionMode::Vision;
   ```

2. Enable image OCR:

   ```rust
   config.image_ocr.enabled = true;
   ```

3. Check page errors:
   ```rust
   for error in &result.page_errors {
       eprintln!("Page {}: {}", error.page, error.error);
   }
   ```

#### 2. Table Not Detected

**Symptoms**: Table content extracted as plain text, not Markdown table.

**Causes**:

- Multi-column page (detector skips these)
- Table has <3 rows (below threshold)
- Large gap between columns (>150pt, detected as multi-column layout)

**Solutions**:

1. Check column count:

   ```rust
   let doc = extractor.extract_document(&pdf_bytes)?;
   for page in &doc.pages {
       if page.columns.len() > 1 {
           println!("Page {}: {} columns detected", page.number, page.columns.len());
       }
   }
   ```

2. Use `TextTableReconstructionProcessor` (parses text patterns):

   ```rust
   use edgequake_pdf::processors::TextTableReconstructionProcessor;

   let mut chain = ProcessorChain::builder()
       .add(TableDetectionProcessor::new())
       .add(TextTableReconstructionProcessor::new()) // Fallback
       .build();
   ```

3. Lower row threshold (custom processor):
   ```rust
   // Requires forking the crate - consider contributing!
   ```

#### 3. Encoding Issues (Garbled Text)

**Symptoms**: Text contains � or mojibake (incorrect characters).

**Causes**:

- PDF uses custom font encoding without ToUnicode map
- Encoding detection heuristics failed

**Solutions**:

1. Enable LLM enhancement (can fix common errors):

   ```rust
   config.enhance_readability = true;
   ```

2. Use vision model (bypasses text extraction):

   ```rust
   config.mode = ExtractionMode::Vision;
   ```

3. Manual encoding fix (post-processing):
   ```rust
   let markdown = result.markdown
       .replace("ï¬", "fi")  // Common ligature error
       .replace("â€™", "'"); // Smart quote error
   ```

#### 4. Performance Issues

**Symptoms**: Extraction takes >30 seconds for 100-page document.

**Causes**:

- LLM enhancement enabled (slow API calls)
- Vision model enabled (image rendering + API calls)
- Complex layout with many tables

**Solutions**:

1. Disable enhancements:

   ```rust
   config.enhance_readability = false;
   config.enhance_tables = false;
   ```

2. Use Text mode:

   ```rust
   config.mode = ExtractionMode::Text;
   ```

3. Limit pages:
   ```rust
   config.max_pages = Some(50);
   ```

**Benchmarks**:

- Text mode: ~1 page/sec (CPU-bound)
- Hybrid mode: ~0.3-0.5 pages/sec (depends on quality)
- Vision mode: ~0.1 pages/sec (rendering + API)

---

## Comparison

### EdgeQuake vs Alternatives

| Feature                | EdgeQuake          | PyPDF2      | pdfplumber   | Camelot        | Marker       |
| ---------------------- | ------------------ | ----------- | ------------ | -------------- | ------------ |
| Language               | Rust               | Python      | Python       | Python         | Python       |
| Structure Preservation | ✅ Block-based     | ❌ Raw text | ⚠️ Basic     | ❌ Tables only | ✅ Excellent |
| Table Detection        | ✅ Spatial + Text  | ❌          | ✅           | ✅             | ✅           |
| Multi-Column           | ✅ XY-Cut          | ❌          | ⚠️ Heuristic | ❌             | ✅           |
| Graceful Degradation   | ✅ Per-page errors | ❌          | ❌           | ❌             | ⚠️           |
| LLM Enhancement        | ✅ Optional        | ❌          | ❌           | ❌             | ✅ Required  |
| Vision Models          | ✅ Optional        | ❌          | ❌           | ❌             | ✅           |
| Speed (100 pages)      | ~100 sec           | ~10 sec     | ~300 sec     | ~200 sec       | ~150 sec     |
| Memory (100 pages)     | ~200 MB            | ~50 MB      | ~400 MB      | ~300 MB        | ~500 MB      |
| API Costs (100 pages)  | $0-5               | $0          | $0           | $0             | $2-10        |
| Customization          | ✅ Processor chain | ❌          | ⚠️ Limited   | ❌             | ❌ Black box |

**When to use EdgeQuake**:

- ✅ Need structure preservation (tables, headers, lists)
- ✅ Multi-column academic papers
- ✅ Graceful degradation required
- ✅ Integration with Rust-based RAG pipeline
- ✅ Want LLM enhancement but not required

**When to use alternatives**:

- **PyPDF2**: Simple text extraction, speed critical, Python ecosystem
- **pdfplumber**: Complex table extraction, willing to wait
- **Camelot**: Table-only extraction, bordered tables
- **Marker**: Don't need customization, willing to pay LLM costs

---

## References

### Source Code

- **Main crate**: [`edgequake/crates/edgequake-pdf/`](../../../edgequake/crates/edgequake-pdf/)
- **Extractor**: [`src/extractor.rs`](../../../edgequake/crates/edgequake-pdf/src/extractor.rs)
- **Table detection**: [`src/processors/table_detection.rs`](../../../edgequake/crates/edgequake-pdf/src/processors/table_detection.rs)
- **Layout analysis**: [`src/layout/`](../../../edgequake/crates/edgequake-pdf/src/layout/)
- **Schema**: [`src/schema/`](../../../edgequake/crates/edgequake-pdf/src/schema/)

### Test Examples

- **Basic tests**: [`tests/basic_tests.rs`](../../../edgequake/crates/edgequake-pdf/tests/basic_tests.rs)
- **Table tests**: [`tests/table_tests.rs`](../../../edgequake/crates/edgequake-pdf/tests/table_tests.rs)
- **Layout tests**: [`tests/layout_tests.rs`](../../../edgequake/crates/edgequake-pdf/tests/layout_tests.rs)
- **Test data**: [`test-data/`](../../../edgequake/crates/edgequake-pdf/test-data/)

### Related Documentation

- [Architecture Overview](../architecture/overview.md)
- [Document Ingestion Tutorial](../tutorials/document-ingestion.md)
- [API Reference: Extended API](../api-reference/extended-api.md)
- [Troubleshooting Common Issues](../troubleshooting/common-issues.md)

### External Resources

- [lopdf crate](https://crates.io/crates/lopdf): PDF parsing library
- [Marker project](https://github.com/VikParuchuri/marker): Inspiration for block schema
- [XY-Cut algorithm paper](https://ieeexplore.ieee.org/document/568477): Layout detection
- [PDF Reference 1.7](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf): PDF specification

---

**Version**: 1.0  
**Last Updated**: 2026-01-29  
**Maintainer**: EdgeQuake Team
