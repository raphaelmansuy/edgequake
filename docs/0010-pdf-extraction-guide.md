# PDF Extraction Guide

> **Implements**: [FEAT1001-FEAT1025](features.md#advanced-pdf-features-feat10xx)  
> **Enforces**: [BR1001-BR1026](business_rules.md#pdf-processing-rules-br10xx)
>
> Comprehensive guide to EdgeQuake's state-of-the-art PDF extraction system

**Module**: `edgequake-pdf` (~26,000 lines) | **Status**: ✅ Production Ready

> **Code Reference**: [edgequake/crates/edgequake-pdf/](../edgequake/crates/edgequake-pdf/)

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Extraction Pipeline](#extraction-pipeline)
4. [Backend Engines](#backend-engines)
5. [Processor Chain](#processor-chain)
6. [Table Detection](#table-detection)
7. [Image Extraction & OCR](#image-extraction--ocr)
8. [Formula Detection](#formula-detection)
9. [Markdown Rendering](#markdown-rendering)
10. [Configuration](#configuration)
11. [Quality Metrics](#quality-metrics)
12. [Troubleshooting](#troubleshooting)

---

## Overview

The EdgeQuake PDF extraction system converts PDF documents to Markdown with structure preservation, enabling high-quality RAG ingestion. It achieves:

- **92.7% quality score** on synthetic test dataset (120 gold files)
- **95%+ reading order accuracy** for single-column documents
- **Table detection** for both ruled (lattice) and borderless (stream) tables
- **Multi-column layout** detection with correct reading order
- **Image extraction** with optional LLM-based OCR
- **Formula detection** with LaTeX conversion (beta)

### Key Capabilities

| Feature                   | Status    | Description                              |
| ------------------------- | --------- | ---------------------------------------- |
| Text extraction           | ✅ Stable | Character-level PDF parsing via lopdf    |
| Layout analysis           | ✅ Stable | Multi-column detection, reading order    |
| Table detection (lattice) | ✅ Stable | Line-based grid detection                |
| Table detection (stream)  | ✅ Stable | Whitespace-based column detection        |
| Heading detection         | ✅ Stable | Font size/weight analysis                |
| List detection            | ✅ Stable | Bullet/numbering pattern recognition     |
| Image extraction          | ✅ Stable | Embedded image export to PNG/JPEG        |
| Image OCR (LLM)           | ✅ Stable | Vision model text extraction             |
| Formula detection         | 🔧 Beta   | LaTeX conversion from PDF math           |
| Code block detection      | ✅ Stable | Monospace font pattern matching          |

---

## Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────────────────────┐
│                     PDF Extraction Architecture                          │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────┐                                                          │
│  │ PDF Bytes  │                                                          │
│  └─────┬──────┘                                                          │
│        │                                                                 │
│        ▼                                                                 │
│  ┌────────────────────────────────────────────────────────────────┐     │
│  │                    SotaBackend                                  │     │
│  │  • Parse PDF structure (lopdf)                                 │     │
│  │  • Extract text, fonts, images                                 │     │
│  │  • Build Document schema                                       │     │
│  │  • Font analysis (size, weight, style)                         │     │
│  │  • Bounding box calculation                                    │     │
│  └───────────────────────────┬────────────────────────────────────┘     │
│                              │                                           │
│                              ▼                                           │
│  ┌────────────────────────────────────────────────────────────────┐     │
│  │                   ProcessorChain                                │     │
│  │  ┌─────────────────────────────────────────────────────────┐   │     │
│  │  │ 1. LayoutProcessor                                       │   │     │
│  │  │    • Multi-column detection                             │   │     │
│  │  │    • Reading order determination                        │   │     │
│  │  │    • Block merging                                      │   │     │
│  │  └─────────────────────────────────────────────────────────┘   │     │
│  │  ┌─────────────────────────────────────────────────────────┐   │     │
│  │  │ 2. TableDetectionProcessor                               │   │     │
│  │  │    • Lattice engine (line-based)                        │   │     │
│  │  │    • Stream engine (whitespace-based)                   │   │     │
│  │  │    • Cell alignment preservation                        │   │     │
│  │  └─────────────────────────────────────────────────────────┘   │     │
│  │  ┌─────────────────────────────────────────────────────────┐   │     │
│  │  │ 3. HeaderDetectionProcessor                              │   │     │
│  │  │    • Font size ratio analysis (>1.2x body)              │   │     │
│  │  │    • Font weight detection (bold)                       │   │     │
│  │  │    • Length limits (<200 chars)                         │   │     │
│  │  └─────────────────────────────────────────────────────────┘   │     │
│  │  ┌─────────────────────────────────────────────────────────┐   │     │
│  │  │ 4. StyleDetectionProcessor                               │   │     │
│  │  │    • Bold/italic detection                              │   │     │
│  │  │    • Code block detection (monospace)                   │   │     │
│  │  │    • List item patterns                                 │   │     │
│  │  └─────────────────────────────────────────────────────────┘   │     │
│  │  ┌─────────────────────────────────────────────────────────┐   │     │
│  │  │ 5. PostProcessor                                         │   │     │
│  │  │    • Hyphen continuation fixing                         │   │     │
│  │  │    • Garbled text filtering                             │   │     │
│  │  │    • Whitespace normalization                           │   │     │
│  │  └─────────────────────────────────────────────────────────┘   │     │
│  └───────────────────────────┬────────────────────────────────────┘     │
│                              │                                           │
│                              ▼                                           │
│  ┌────────────────────────────────────────────────────────────────┐     │
│  │                 MarkdownRenderer                                │     │
│  │  • Generate Markdown syntax                                    │     │
│  │  • Preserve structure (headings, lists, tables)               │     │
│  │  • Handle special cases (code, images)                         │     │
│  └───────────────────────────┬────────────────────────────────────┘     │
│                              │                                           │
│                              ▼                                           │
│                      ┌──────────────┐                                    │
│                      │  Markdown    │                                    │
│                      │   Output     │                                    │
│                      └──────────────┘                                    │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### Module Responsibilities

| Module                  | Lines | Responsibility                          | Key Algorithms               |
| ----------------------- | ----- | --------------------------------------- | ---------------------------- |
| `SotaBackend`           | ~3000 | PDF parsing, font analysis              | Font size clustering, bbox   |
| `LatticeEngine`         | ~600  | Table detection via line grids          | Grid intersection detection  |
| `ProcessorChain`        | ~3000 | Content transformation pipeline         | Processor trait, chaining    |
| `MarkdownRenderer`      | ~800  | Final Markdown generation               | Syntax tree rendering        |
| `ImageOcrProcessor`     | ~500  | Vision LLM image understanding          | LLM prompt engineering       |
| `LayoutProcessor`       | ~640  | Multi-column detection, reading order   | Geometric clustering         |
| `HeaderDetector`        | ~570  | Heading classification                  | Font size ratio thresholds   |
| `TableDetector`         | ~730  | Table cell reconstruction               | Cell boundary detection      |
| `TextCleanup`           | ~670  | Hyphen/garbled text fixing              | Regex pattern matching       |

---

## Extraction Pipeline

### High-Level Flow

```rust
// Located: edgequake/crates/edgequake-pdf/src/extractor.rs

pub struct PdfExtractor {
    backend: SotaBackend,
    processor_chain: ProcessorChain,
    renderer: MarkdownRenderer,
    config: PdfExtractionConfig,
}

impl PdfExtractor {
    pub async fn extract(&self, pdf_bytes: &[u8]) -> Result<ExtractionResult> {
        // STAGE 1: Parse PDF and extract raw content
        let raw_document = self.backend.extract(pdf_bytes)?;
        
        // STAGE 2: Process through transformation pipeline
        let processed_document = self.processor_chain.process(raw_document)?;
        
        // STAGE 3: Render to Markdown
        let markdown = self.renderer.render(&processed_document)?;
        
        Ok(ExtractionResult {
            markdown,
            pages: processed_document.pages.len(),
            quality_score: self.calculate_quality(&processed_document),
        })
    }
}
```

### Extraction Stages

#### Stage 1: PDF Parsing (SotaBackend)

**Input**: Raw PDF bytes  
**Output**: `Document` schema with pages, blocks, text runs

```rust
// WHY: lopdf provides low-level PDF object access
// ALTERNATIVE: pdfium (C++ binding, heavier)

pub struct SotaBackend {
    pdf_doc: LopdfDocument,
    font_info: HashMap<String, FontInfo>,
}

impl SotaBackend {
    pub fn extract(&mut self, pdf_bytes: &[u8]) -> Result<Document> {
        // 1. Parse PDF structure
        self.pdf_doc = LopdfDocument::load_mem(pdf_bytes)?;
        
        // 2. Extract font information for all pages
        self.analyze_fonts()?;
        
        // 3. Extract text with positioning
        let pages = self.extract_pages()?;
        
        // 4. Deduplicate overlapping text
        let deduped = self.deduplicate_text(pages)?;
        
        Ok(Document { pages: deduped, metadata: self.extract_metadata()? })
    }
}
```

**Key Operations:**
- **Font Analysis**: Cluster font sizes to determine body text size
- **Text Extraction**: Character-by-character with bounding boxes
- **Deduplication**: Merge overlapping text elements (BR1021)
- **Metadata**: Title, author, page count, creation date

#### Stage 2: Processing Pipeline

Each processor transforms the document schema:

```rust
#[async_trait]
pub trait Processor: Send + Sync {
    fn name(&self) -> &str;
    async fn process(&self, document: Document) -> Result<Document>;
}

pub struct ProcessorChain {
    processors: Vec<Box<dyn Processor>>,
}

impl ProcessorChain {
    pub async fn process(&self, mut document: Document) -> Result<Document> {
        for processor in &self.processors {
            document = processor.process(document).await?;
            tracing::debug!("Processed with {}", processor.name());
        }
        Ok(document)
    }
}
```

**Standard Chain Order** (BR1020):
1. `LayoutProcessor` - Establish reading order
2. `TableDetectionProcessor` - Identify table structures
3. `HeaderDetectionProcessor` - Classify headings
4. `StyleDetectionProcessor` - Detect bold/italic/code
5. `PostProcessor` - Clean up artifacts

#### Stage 3: Markdown Rendering

**Input**: Processed `Document`  
**Output**: Markdown string

```rust
pub struct MarkdownRenderer {
    config: RenderConfig,
}

impl MarkdownRenderer {
    pub fn render(&self, document: &Document) -> Result<String> {
        let mut output = String::new();
        
        for page in &document.pages {
            for block in &page.blocks {
                match &block.block_type {
                    BlockType::Heading(level) => {
                        output.push_str(&format!("{} {}\n\n", "#".repeat(*level), block.text));
                    }
                    BlockType::Table(rows) => {
                        output.push_str(&self.render_table(rows)?);
                    }
                    BlockType::CodeBlock => {
                        output.push_str(&format!("```\n{}\n```\n\n", block.text));
                    }
                    BlockType::List(items) => {
                        for item in items {
                            output.push_str(&format!("- {}\n", item));
                        }
                        output.push('\n');
                    }
                    BlockType::Paragraph => {
                        output.push_str(&format!("{}\n\n", block.text));
                    }
                }
            }
        }
        
        Ok(output)
    }
}
```

---

## Backend Engines

### SotaBackend - Primary Extraction Engine

> **Code Reference**: [backend/sota_backend.rs](../edgequake/crates/edgequake-pdf/src/backend/sota_backend.rs)

**Purpose**: Parse PDF and extract text with precise positioning

**Key Algorithms:**

#### 1. Font Size Clustering

```rust
// WHY: Identify body text size to detect headings
// ALGORITHM: Find most common font size across document

pub fn analyze_font_sizes(&self, pages: &[Page]) -> FontSizeStats {
    let mut size_counts: HashMap<NotNan<f32>, usize> = HashMap::new();
    
    for page in pages {
        for text_run in &page.text_runs {
            *size_counts.entry(text_run.font_size).or_insert(0) += 1;
        }
    }
    
    // Most common size is body text
    let body_size = size_counts.iter()
        .max_by_key(|(_, count)| *count)
        .map(|(size, _)| **size)
        .unwrap_or(12.0);
    
    FontSizeStats {
        body_size,
        heading_threshold: body_size * 1.2, // BR1010
        max_size: size_counts.keys().map(|s| **s).max().unwrap_or(72.0),
    }
}
```

#### 2. Bounding Box Deduplication

```rust
// WHY: PDF rendering sometimes overlays identical text
// ENFORCES: BR1021 - Deduplication by bounding box

pub fn deduplicate_text_runs(&self, runs: Vec<TextRun>) -> Vec<TextRun> {
    let mut unique_runs = Vec::new();
    
    for run in runs {
        let is_duplicate = unique_runs.iter().any(|existing: &TextRun| {
            // Same text and >80% bbox overlap
            existing.text == run.text && 
            bbox_overlap(&existing.bbox, &run.bbox) > 0.8
        });
        
        if !is_duplicate {
            unique_runs.push(run);
        }
    }
    
    unique_runs
}
```

#### 3. Reading Order Determination

```rust
// WHY: PDF has no inherent text order, must be inferred
// ALGORITHM: Top-to-bottom, left-to-right within columns

pub fn sort_text_runs_by_reading_order(&self, runs: &mut [TextRun]) {
    runs.sort_by(|a, b| {
        // Sort by Y position first (top to bottom)
        if (a.bbox.y0 - b.bbox.y0).abs() > 5.0 {
            a.bbox.y0.partial_cmp(&b.bbox.y0).unwrap()
        } else {
            // Same line: sort by X position (left to right)
            a.bbox.x0.partial_cmp(&b.bbox.x0).unwrap()
        }
    });
}
```

---

## Processor Chain

### LayoutProcessor

> **Code Reference**: [processors/layout_processing.rs](../edgequake/crates/edgequake-pdf/src/processors/layout_processing.rs)

**Purpose**: Detect multi-column layouts and establish correct reading order

**Algorithm**:
1. Cluster text runs by X position to detect columns
2. Sort columns left-to-right
3. Within each column, sort top-to-bottom
4. Merge adjacent text runs into blocks

```rust
pub struct LayoutProcessor {
    column_detection_threshold: f32, // Default: 50.0 points
}

impl Processor for LayoutProcessor {
    async fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            // 1. Detect columns by X-position clustering
            let columns = self.detect_columns(&page.text_runs)?;
            
            // 2. Sort columns left-to-right
            let sorted_columns = self.sort_columns(columns);
            
            // 3. Flatten column order
            page.text_runs = sorted_columns.into_iter()
                .flat_map(|col| col.runs)
                .collect();
            
            // 4. Merge adjacent runs into blocks
            page.blocks = self.merge_into_blocks(&page.text_runs)?;
        }
        
        Ok(document)
    }
}
```

### TableDetectionProcessor

> **Code Reference**: [processors/table_detection.rs](../edgequake/crates/edgequake-pdf/src/processors/table_detection.rs)

**Purpose**: Detect and reconstruct table structures

**Supports Two Modes**:
1. **Lattice** - Line-based grid detection (ruled tables)
2. **Stream** - Whitespace-based column detection (borderless tables)

See [Table Detection](#table-detection) section for details.

### HeaderDetectionProcessor

> **Code Reference**: [processors/structure_detection.rs](../edgequake/crates/edgequake-pdf/src/processors/structure_detection.rs)

**Purpose**: Classify text blocks as headings based on font properties

**Algorithm**:

```rust
pub struct HeadingClassifier {
    body_font_size: f32,
    heading_size_ratio: f32, // BR1010: Default 1.2
    max_heading_length: usize, // BR1011: Default 200
}

impl HeadingClassifier {
    pub fn classify_heading(&self, block: &TextBlock) -> Option<usize> {
        // Rule 1: Font size must be >= 1.2x body
        let size_ratio = block.font_size / self.body_font_size;
        if size_ratio < self.heading_size_ratio {
            return None;
        }
        
        // Rule 2: Length must be <= 200 characters
        if block.text.len() > self.max_heading_length {
            return None;
        }
        
        // Rule 3: Should be bold (optional)
        let is_bold = block.font_weight > 500.0;
        
        // Determine heading level (1-6) based on size ratio
        let level = match size_ratio {
            r if r >= 2.0 => 1,
            r if r >= 1.8 => 2,
            r if r >= 1.5 => 3,
            r if r >= 1.3 => 4,
            r if r >= 1.2 => 5,
            _ => 6,
        };
        
        Some(level)
    }
}
```

**Heading Detection Thresholds**:

| Ratio    | Level | Example Size (12pt body) |
| -------- | ----- | ------------------------ |
| ≥2.0x    | H1    | 24pt+                    |
| ≥1.8x    | H2    | 21.6pt                   |
| ≥1.5x    | H3    | 18pt                     |
| ≥1.3x    | H4    | 15.6pt                   |
| ≥1.2x    | H5    | 14.4pt                   |
| <1.2x    | Body  | 12pt                     |

---

## Table Detection

### Lattice Engine (Line-Based)

> **Code Reference**: [backend/lattice.rs](../edgequake/crates/edgequake-pdf/src/backend/lattice.rs)

**Use Case**: Tables with visible borders/lines

**Algorithm**:
1. Extract horizontal and vertical line segments from PDF
2. Find intersections to build grid
3. Assign text to cells based on bounding box containment
4. Reconstruct table structure

```rust
pub struct LatticeEngine {
    min_line_length: f32, // Default: 20.0 points
    intersection_tolerance: f32, // Default: 5.0 points
}

impl LatticeEngine {
    pub fn detect_tables(&self, page: &Page) -> Result<Vec<Table>> {
        // 1. Extract line segments
        let h_lines = self.extract_horizontal_lines(page)?;
        let v_lines = self.extract_vertical_lines(page)?;
        
        // 2. Find line intersections
        let intersections = self.find_intersections(&h_lines, &v_lines);
        
        // 3. Build cell grid
        let cells = self.build_cell_grid(&intersections, &h_lines, &v_lines)?;
        
        // 4. Assign text to cells
        for text_run in &page.text_runs {
            if let Some(cell) = self.find_containing_cell(&cells, &text_run.bbox) {
                cell.text.push_str(&text_run.text);
            }
        }
        
        // 5. Convert grid to table structure
        self.cells_to_table(cells)
    }
}
```

**Intersection Detection** (WHY: Line detection is core to lattice):

```rust
fn find_intersections(
    &self,
    h_lines: &[Line],
    v_lines: &[Line],
) -> Vec<Point> {
    let mut intersections = Vec::new();
    
    for h_line in h_lines {
        for v_line in v_lines {
            // Check if lines intersect within tolerance
            if (h_line.y - v_line.x).abs() < self.intersection_tolerance &&
               (v_line.y - h_line.x).abs() < self.intersection_tolerance {
                intersections.push(Point {
                    x: v_line.x,
                    y: h_line.y,
                });
            }
        }
    }
    
    intersections
}
```

### Stream Engine (Whitespace-Based)

**Use Case**: Tables without borders (whitespace-aligned columns)

**Algorithm**:
1. Analyze whitespace distribution to detect column boundaries
2. Group text runs by column
3. Detect row boundaries by Y-position gaps
4. Reconstruct table structure

```rust
pub fn detect_stream_table(&self, text_runs: &[TextRun]) -> Result<Option<Table>> {
    // 1. Find column boundaries by X-position gaps
    let columns = self.detect_column_boundaries(text_runs)?;
    
    if columns.len() < 2 {
        return Ok(None); // Not a table
    }
    
    // 2. Group text runs by column
    let mut column_runs: Vec<Vec<TextRun>> = vec![Vec::new(); columns.len()];
    for run in text_runs {
        if let Some(col_idx) = self.find_column(&columns, run.bbox.x0) {
            column_runs[col_idx].push(run.clone());
        }
    }
    
    // 3. Detect row boundaries by Y-position gaps
    let rows = self.detect_row_boundaries(&column_runs)?;
    
    // 4. Build table structure
    Ok(Some(self.build_table(rows, columns.len())?))
}
```

**Column Boundary Detection**:

```rust
// WHY: Whitespace gaps indicate column separators
// THRESHOLD: 20+ points of whitespace

fn detect_column_boundaries(&self, runs: &[TextRun]) -> Result<Vec<f32>> {
    // Sort by X position
    let mut sorted = runs.to_vec();
    sorted.sort_by(|a, b| a.bbox.x0.partial_cmp(&b.bbox.x0).unwrap());
    
    let mut boundaries = vec![];
    let mut last_x_end = sorted[0].bbox.x1;
    
    for run in &sorted[1..] {
        let gap = run.bbox.x0 - last_x_end;
        
        // Large gap indicates column boundary
        if gap > 20.0 {
            boundaries.push((last_x_end + run.bbox.x0) / 2.0);
        }
        
        last_x_end = last_x_end.max(run.bbox.x1);
    }
    
    Ok(boundaries)
}
```

---

## Image Extraction & OCR

### Image Extraction

> **Code Reference**: [image_extraction.rs](../edgequake/crates/edgequake-pdf/src/image_extraction.rs)

**Purpose**: Extract embedded images from PDF pages

```rust
pub struct ImageExtractor {
    config: ImageOcrConfig,
}

impl ImageExtractor {
    pub fn extract_page_images(
        &self,
        pdf_doc: &LopdfDocument,
        page_id: ObjectId,
        page_num: usize,
    ) -> Result<Vec<ImageData>> {
        let images = pdf_doc.get_page_images(page_id)?;
        let mut extracted = Vec::new();
        
        for (img_idx, img_obj) in images.iter().enumerate() {
            // 1. Decode image data
            let decoded = self.decode_image(img_obj)?;
            
            // 2. Convert to PNG/JPEG
            let image_data = self.convert_to_standard_format(&decoded)?;
            
            // 3. Check size limit (BR1024)
            if image_data.bytes.len() > self.config.max_image_size_bytes {
                tracing::warn!("Skipping oversized image: {} bytes", image_data.bytes.len());
                continue;
            }
            
            // 4. Base64 encode for LLM
            let base64 = BASE64.encode(&image_data.bytes);
            
            extracted.push(ImageData {
                page_num,
                image_index: img_idx,
                format: image_data.format,
                data: base64,
            });
        }
        
        Ok(extracted)
    }
}
```

### LLM-Based OCR

> **Code Reference**: [image_ocr.rs](../edgequake/crates/edgequake-pdf/src/image_ocr.rs)

**Purpose**: Use vision LLM to extract text from images

**Supported Models**:
- OpenAI GPT-4o-mini with vision
- OpenAI GPT-4o
- LM Studio vision models (OpenAI-compatible API)

```rust
pub struct ImageOcrProcessor {
    llm_provider: Arc<dyn VisionLLMProvider>,
    rate_limiter: RateLimiter,
}

impl ImageOcrProcessor {
    pub async fn process_image(&self, image: &ImageData) -> Result<String> {
        // Enforce rate limit (BR1026)
        self.rate_limiter.wait().await;
        
        // Build vision prompt
        let prompt = self.build_ocr_prompt();
        
        // Call vision LLM
        let response = self.llm_provider.analyze_image(
            &image.data,
            &prompt,
        ).await?;
        
        // Extract text from response
        self.extract_text_from_response(&response)
    }
    
    fn build_ocr_prompt(&self) -> String {
        r#"
        Extract all text from this image.
        
        Requirements:
        - Preserve formatting (paragraphs, lists, tables)
        - Output in Markdown format
        - If the image contains a chart or diagram, describe it briefly
        - If no text is present, output "NO_TEXT"
        
        Output:
        "#.to_string()
    }
}
```

**Rate Limiting** (BR1026):

```rust
pub struct RateLimiter {
    max_rpm: usize, // Requests per minute
    last_request: Instant,
}

impl RateLimiter {
    pub async fn wait(&mut self) {
        let min_interval = Duration::from_millis(60_000 / self.max_rpm as u64);
        let elapsed = self.last_request.elapsed();
        
        if elapsed < min_interval {
            let wait_time = min_interval - elapsed;
            tokio::time::sleep(wait_time).await;
        }
        
        self.last_request = Instant::now();
    }
}
```

---

## Formula Detection

> **Code Reference**: [formula/](../edgequake/crates/edgequake-pdf/src/formula/)

**Status**: 🔧 Beta  
**Purpose**: Detect mathematical formulas and convert to LaTeX

**Planned Approach**:
1. Detect math mode indicators (symbols, font names)
2. Extract formula bounding box
3. Use vision LLM to convert to LaTeX
4. Embed in Markdown as `$...$` or `$$...$$`

**Current Limitations**:
- Formula detection not yet in production
- Complex formulas may require manual review
- Vision LLM LaTeX accuracy varies by model

---

## Markdown Rendering

> **Code Reference**: [renderers/](../edgequake/crates/edgequake-pdf/src/renderers/)

### MarkdownRenderer

**Purpose**: Convert processed `Document` schema to final Markdown

```rust
pub struct MarkdownRenderer {
    config: RenderConfig,
}

pub struct RenderConfig {
    pub preserve_line_breaks: bool,      // Default: false
    pub table_alignment: TableAlignment, // Default: Left
    pub code_fence_lang: String,         // Default: ""
    pub image_embed_mode: ImageMode,     // Default: Base64
}

impl MarkdownRenderer {
    pub fn render(&self, document: &Document) -> Result<String> {
        let mut output = String::new();
        
        // Render title if present
        if let Some(title) = &document.metadata.title {
            output.push_str(&format!("# {}\n\n", title));
        }
        
        // Render each page
        for page in &document.pages {
            output.push_str(&self.render_page(page)?);
        }
        
        Ok(output)
    }
    
    fn render_page(&self, page: &Page) -> Result<String> {
        let mut output = String::new();
        
        for block in &page.blocks {
            match &block.block_type {
                BlockType::Heading(level) => {
                    output.push_str(&self.render_heading(*level, &block.text));
                }
                BlockType::Table(rows) => {
                    output.push_str(&self.render_table(rows)?);
                }
                BlockType::CodeBlock => {
                    output.push_str(&self.render_code_block(&block.text));
                }
                BlockType::List(items) => {
                    output.push_str(&self.render_list(items));
                }
                BlockType::Image(image_ref) => {
                    output.push_str(&self.render_image(image_ref)?);
                }
                BlockType::Paragraph => {
                    output.push_str(&format!("{}\n\n", block.text));
                }
            }
        }
        
        Ok(output)
    }
}
```

### Table Rendering

**Markdown Table Format**:

```markdown
| Header 1 | Header 2 | Header 3 |
|----------|----------|----------|
| Cell 1   | Cell 2   | Cell 3   |
| Cell 4   | Cell 5   | Cell 6   |
```

**Algorithm**:

```rust
fn render_table(&self, rows: &[TableRow]) -> Result<String> {
    let mut output = String::new();
    
    // Determine column widths
    let col_count = rows.iter().map(|r| r.cells.len()).max().unwrap_or(0);
    let col_widths = self.calculate_column_widths(rows, col_count);
    
    // Render header row (first row)
    if let Some(header) = rows.first() {
        output.push('|');
        for (i, cell) in header.cells.iter().enumerate() {
            output.push_str(&format!(" {:<width$} |", cell.text, width = col_widths[i]));
        }
        output.push('\n');
        
        // Render separator
        output.push('|');
        for width in &col_widths {
            output.push_str(&format!("{:-<width$}|", "", width = width + 2));
        }
        output.push('\n');
    }
    
    // Render data rows
    for row in rows.iter().skip(1) {
        output.push('|');
        for (i, cell) in row.cells.iter().enumerate() {
            output.push_str(&format!(" {:<width$} |", cell.text, width = col_widths[i]));
        }
        output.push('\n');
    }
    
    output.push('\n');
    Ok(output)
}
```

---

## Configuration

### PdfExtractionConfig

> **Code Reference**: [config.rs](../edgequake/crates/edgequake-pdf/src/config.rs)

```rust
#[derive(Debug, Clone)]
pub struct PdfExtractionConfig {
    // Backend settings
    pub enable_lattice: bool,              // Default: true
    pub enable_stream: bool,               // Default: true
    pub deduplication_threshold: f32,      // Default: 0.8 (80% overlap)
    
    // Processor settings
    pub heading_size_ratio: f32,           // Default: 1.2 (BR1010)
    pub max_heading_length: usize,         // Default: 200 (BR1011)
    pub column_detection_threshold: f32,   // Default: 50.0 points
    
    // Image OCR settings
    pub enable_image_ocr: bool,            // Default: false
    pub max_image_size_bytes: usize,       // Default: 10MB (BR1024)
    pub vision_rate_limit_rpm: usize,      // Default: 10 (BR1026)
    
    // Formula detection
    pub enable_formula_detection: bool,    // Default: false
    
    // Render settings
    pub preserve_line_breaks: bool,        // Default: false
    pub table_alignment: TableAlignment,   // Default: Left
}

impl Default for PdfExtractionConfig {
    fn default() -> Self {
        Self {
            enable_lattice: true,
            enable_stream: true,
            deduplication_threshold: 0.8,
            heading_size_ratio: 1.2,
            max_heading_length: 200,
            column_detection_threshold: 50.0,
            enable_image_ocr: false,
            max_image_size_bytes: 10 * 1024 * 1024,
            vision_rate_limit_rpm: 10,
            enable_formula_detection: false,
            preserve_line_breaks: false,
            table_alignment: TableAlignment::Left,
        }
    }
}
```

### Environment Variables

| Variable                      | Default | Description                     |
| ----------------------------- | ------- | ------------------------------- |
| `PDF_HEADING_SIZE_RATIO`      | `1.2`   | Font size ratio for headings    |
| `PDF_MAX_HEADING_LENGTH`      | `200`   | Max heading length (characters) |
| `PDF_ENABLE_IMAGE_OCR`        | `false` | Enable vision LLM OCR           |
| `PDF_VISION_RATE_LIMIT_RPM`   | `10`    | Vision API requests per minute  |
| `PDF_COLUMN_DETECT_THRESHOLD` | `50.0`  | Column gap detection (points)   |

### Usage Example

```rust
use edgequake_pdf::{PdfExtractor, PdfExtractionConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Configure extraction
    let config = PdfExtractionConfig {
        enable_image_ocr: true,
        heading_size_ratio: 1.3, // Stricter heading detection
        vision_rate_limit_rpm: 5, // Slower for free tier
        ..Default::default()
    };
    
    // Create extractor
    let extractor = PdfExtractor::new(config)?;
    
    // Extract PDF
    let pdf_bytes = std::fs::read("document.pdf")?;
    let result = extractor.extract(&pdf_bytes).await?;
    
    println!("Extracted {} pages", result.pages);
    println!("Quality score: {:.1}%", result.quality_score * 100.0);
    println!("Markdown length: {} chars", result.markdown.len());
    
    std::fs::write("output.md", result.markdown)?;
    
    Ok(())
}
```

---

## Quality Metrics

### Test Suite Results

**Synthetic Dataset** (120 gold files):
- **Success Rate**: 95.0% (114/120 PDFs extracted)
- **Average Quality Score**: 92.7/100
- **Reading Order Accuracy**: 96.2%
- **Table Detection F1**: 0.89 (lattice), 0.72 (stream)

**Real-World Dataset** (5 academic papers):
- **Success Rate**: 100% (5/5 PDFs extracted)
- **Average Similarity**: 36.1% (vs gold standard)
- **Best**: 65.2% (11-page paper with simple layout)
- **Worst**: 12.4% (44-page paper with complex multi-column)

### Quality Score Calculation

```rust
pub fn calculate_quality_score(&self, document: &Document) -> f32 {
    let mut score = 0.0;
    let mut weight_sum = 0.0;
    
    // Structure preservation (40%)
    let structure_score = self.measure_structure_preservation(document);
    score += structure_score * 0.4;
    weight_sum += 0.4;
    
    // Reading order accuracy (30%)
    let order_score = self.measure_reading_order_accuracy(document);
    score += order_score * 0.3;
    weight_sum += 0.3;
    
    // Table accuracy (20%)
    let table_score = self.measure_table_accuracy(document);
    score += table_score * 0.2;
    weight_sum += 0.2;
    
    // Character fidelity (10%)
    let char_score = self.measure_character_fidelity(document);
    score += char_score * 0.1;
    weight_sum += 0.1;
    
    score / weight_sum
}
```

### Validation Metrics

> **Tool**: [pdf-markdown-validator](../../.github/skills/pdf-markdown-validator/)

| Metric               | Weight | Description                       |
| -------------------- | ------ | --------------------------------- |
| Levenshtein Distance | 30%    | Character-level similarity        |
| Table Accuracy       | 30%    | Cell count and alignment match    |
| Style Preservation   | 20%    | Headings, lists, code blocks      |
| Robustness           | 20%    | Error handling, edge cases        |

---

## Troubleshooting

### Common Issues

#### 1. Poor Reading Order

**Symptom**: Text appears jumbled or out of sequence  
**Cause**: Multi-column layout not detected properly  
**Solution**:
```rust
let config = PdfExtractionConfig {
    column_detection_threshold: 30.0, // Lower threshold for narrower columns
    ..Default::default()
};
```

#### 2. Missing Tables

**Symptom**: Tables not detected or cells misaligned  
**Cause**: Borderless tables not detected by stream engine  
**Solution**:
- Ensure `enable_stream: true`
- Check if text alignment is consistent (stream requires aligned columns)
- Try lattice mode if table has faint lines

#### 3. Incorrect Headings

**Symptom**: Body text classified as headings or vice versa  
**Cause**: Font size ratio threshold too low/high  
**Solution**:
```rust
let config = PdfExtractionConfig {
    heading_size_ratio: 1.3, // Stricter (fewer false positives)
    // or
    heading_size_ratio: 1.1, // Looser (catch more headings)
    ..Default::default()
};
```

#### 4. Image OCR Failures

**Symptom**: Images not extracted or OCR returns garbage  
**Cause**: Vision API rate limits or model limitations  
**Solution**:
```rust
let config = PdfExtractionConfig {
    vision_rate_limit_rpm: 5, // Reduce rate to avoid throttling
    max_image_size_bytes: 5 * 1024 * 1024, // Reduce size for faster processing
    ..Default::default()
};
```

#### 5. Garbled Text

**Symptom**: Random characters or encoding issues  
**Cause**: Custom font encoding not properly mapped  
**Solution**:
- Check PDF font encoding (CMap)
- Enable PostProcessor cleanup
- File a bug report with the PDF for encoding investigation

### Debug Logging

Enable debug logging to diagnose issues:

```rust
use tracing_subscriber;

tracing_subscriber::fmt()
    .with_env_filter("edgequake_pdf=debug")
    .init();

let result = extractor.extract(&pdf_bytes).await?;
```

**Key Debug Output**:
- Font analysis results
- Column detection boundaries
- Table detection decisions
- Processor chain execution

### Performance Tuning

**Slow Extraction**:
- Disable image OCR if not needed: `enable_image_ocr: false`
- Disable formula detection: `enable_formula_detection: false`
- Reduce processor chain (custom `ProcessorChain`)

**High Memory Usage**:
- Process PDFs in streaming mode (page-by-page)
- Reduce `max_image_size_bytes`
- Enable early deduplication

---

## Related Documents

- [Features Registry - FEAT1001-FEAT1025](features.md#advanced-pdf-features-feat10xx)
- [Business Rules - BR1001-BR1026](business_rules.md#pdf-processing-rules-br10xx)
- [Architecture Overview - PDF Section](0002-architecture-overview.md#edgequake-pdf---pdf-extraction)
- [API Reference - Document Upload](0003-api-reference.md#document-endpoints)
- [Configuration Reference](0007-configuration-reference.md)

---

**Next Steps**:
- [Query Engine Guide →](0009-algorithms-reference.md#query-modes-and-retrieval-strategies)
- [Deployment Guide →](0006-deployment-guide.md)
- [Testing Guide →](../edgequake/crates/edgequake-pdf/TEST_PROTOCOL.md)
