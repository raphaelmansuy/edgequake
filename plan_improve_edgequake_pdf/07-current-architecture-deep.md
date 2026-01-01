# Current Architecture Analysis

## Overview

The `edgequake-pdf` crate is a sophisticated PDF-to-Markdown converter with AI enhancement capabilities. It follows a pipeline architecture with six major components.

## Component Breakdown

### 1. Backend Layer (`src/backend/`)

**Purpose:** Abstract PDF extraction from specific libraries

**Files:**
- `mod.rs` (13 lines) - `PdfBackend` trait definition
- `pdfium.rs` (494 lines) - Pdfium-based implementation
- `mock.rs` (46 lines) - Testing implementation

**Trait Definition:**
```rust
#[async_trait]
pub trait PdfBackend: Send + Sync {
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document>;
    fn get_info(&self, pdf_bytes: &[u8]) -> Result<PdfInfo>;
}
```

**Current Implementations:**

| Backend | extract() Behavior | get_info() Behavior |
|---------|-------------------|---------------------|
| PdfiumBackend | Extracts text with positions<br>Creates blocks<br>**Runs layout analysis**<br>**Sorts by reading order** | Returns page count<br>PDF version<br>File size |
| MockBackend | Returns pre-built Document | Returns static info |

**Issue Identified:** 
- `PdfiumBackend` does layout analysis (columns, reading order) during extraction
- `MockBackend` does not
- Inconsistent abstraction

### 2. Schema Layer (`src/schema/`)

**Purpose:** Document object model representing the intermediate representation

**Files:**
- `document.rs` (600 lines) - Document, Page, DocumentMetadata, TOC
- `block.rs` (489 lines) - Block structure with hierarchy
- `block_types.rs` (186 lines) - BlockType enum and traits
- `geometry.rs` (450 lines) - BoundingBox, Point, geometric operations

**Key Types:**
```
Document
├── metadata: DocumentMetadata
├── pages: Vec<Page>
└── toc: Vec<TocEntry>

Page
├── number: usize
├── width, height: f32
├── blocks: Vec<Block>
├── columns: Vec<BoundingBox>
└── margins: Option<PageMargins>

Block
├── id: BlockId
├── block_type: BlockType
├── bbox: BoundingBox
├── text: String
├── spans: Vec<TextSpan>
├── children: Vec<Block>
└── metadata: HashMap<String, String>
```

**Strength:** Rich, hierarchical model with geometry support

### 3. Layout Analysis (`src/layout/`)

**Purpose:** Detect document structure (columns, reading order, regions)

**Files:**
- `mod.rs` (324 lines) - LayoutAnalyzer, LayoutAnalysis
- `xy_cut.rs` (552 lines) - Recursive XY-Cut algorithm
- `column_detector.rs` (402 lines) - Column detection via projection
- `reading_order.rs` (406 lines) - Z-order + multi-column ordering

**Algorithm:** XY-Cut recursive segmentation
```
              Page
               |
          Horizontal Cut
           /         \
      Column 1     Column 2
         |            |
    Vertical Cut  Vertical Cut
      /    \        /    \
   Block1 Block2 Block3 Block4
```

**Usage:**
- `PdfiumBackend` calls `LayoutAnalyzer.analyze()` and `sort_by_reading_order()`
- `LayoutProcessor` **also** calls the same methods

**Issue Identified:** Layout analysis runs TWICE (backend + processor)

### 4. Processing Pipeline (`src/processors/`)

**Purpose:** Transform and enhance extracted document

**Files:**
- `processor.rs` (989 lines) - PostProcessor trait + 7 implementations
- `llm_enhance.rs` (388 lines) - AI-powered enhancement
- `builder.rs` (351 lines) - ProcessorChain fluent API
- `provider.rs` (99 lines) - Content providers (file, bytes)

**Processors (in order):**
1. **LayoutProcessor** - Applies layout analysis (DUPLICATE!)
2. **TableDetectionProcessor** - Identifies tables from structure
3. **HeaderDetectionProcessor** - Detects headings from fonts
4. **CaptionDetectionProcessor** - Finds captions near images/tables
5. **ListDetectionProcessor** - Identifies bulleted/numbered lists
6. **CodeBlockDetectionProcessor** - Detects code via fonts
7. **BlockMergeProcessor** - Merges adjacent text blocks
8. **PostProcessor** - Normalizes whitespace

**Strengths:**
- Composable pipeline
- Each processor has single responsibility
- Trait-based extensibility

**Issue Identified:**
- LayoutProcessor always runs even if backend already did layout
- Creates redundant work

### 5. Rendering Layer (`src/renderers/`)

**Purpose:** Convert Document to output formats

**Files:**
- `markdown.rs` (595 lines) - Markdown with styles (Standard/Minimal/Verbose)
- `json.rs` (132 lines) - JSON export

**Markdown Styles:**
- **Standard:** `# Heading`, `**bold**`, `_italic_`, tables
- **Minimal:** Plain text, minimal formatting
- **Verbose:** Full metadata, block IDs, debugging info

**Strength:** Multiple output formats with configurable verbosity

### 6. Orchestration (`src/extractor.rs`, `src/config.rs`)

**Purpose:** Coordinate the full extraction pipeline

**Flow:**
```rust
async fn extract_to_markdown(&self, pdf_bytes: &[u8]) -> Result<String> {
    // 1. Backend extraction
    let mut doc = self.backend.extract(pdf_bytes).await?;
    
    // 2. Processing pipeline
    doc = self.apply_processors(doc).await?;
    
    // 3. Rendering
    let renderer = MarkdownRenderer::new(...);
    Ok(renderer.render(&doc))
}
```

**Pipeline Assembly (extractor.rs:218):**
```rust
ProcessorChain::new()
    .add(LayoutProcessor::new())         // ← FIRST!
    .add(TableDetectionProcessor::new())
    .add(HeaderDetectionProcessor::new())
    .add(CaptionDetectionProcessor::new())
    .add(ListDetectionProcessor::new())
    .add(CodeBlockDetectionProcessor::new())
    .add(BlockMergeProcessor::new())
    .add(PostProcessor::new())
```

## Data Flow Diagram

```
┌───────────┐
│ PDF Bytes │
└─────┬─────┘
      │
      ▼
┌─────────────────────────────────────┐
│ PdfBackend::extract()               │
│ ┌─────────────────────────────────┐ │
│ │ 1. Load PDF (pdfium)            │ │
│ │ 2. Extract characters + pos     │ │
│ │ 3. Group into words (gaps)      │ │
│ │ 4. Group into lines (y-coord)   │ │
│ │ 5. Detect columns (x-gaps)      │ │
│ │ 6. Create blocks                │ │
│ │ 7. ❌ RUN LAYOUT ANALYSIS       │ │ ← ISSUE!
│ │ 8. ❌ SORT BY READING ORDER     │ │ ← ISSUE!
│ └─────────────────────────────────┘ │
└─────┬───────────────────────────────┘
      │
      ▼
┌─────────────────────┐
│ Document (sorted)   │ ← Already analyzed!
└─────┬───────────────┘
      │
      ▼
┌─────────────────────────────────────┐
│ LayoutProcessor::process()          │
│ ┌─────────────────────────────────┐ │
│ │ 1. ❌ RUN LAYOUT ANALYSIS AGAIN │ │ ← DUPLICATE!
│ │ 2. ❌ SORT BY READING ORDER     │ │ ← DUPLICATE!
│ └─────────────────────────────────┘ │
└─────┬───────────────────────────────┘
      │
      ▼
┌─────────────────────┐
│ Other Processors    │
│ • TableDetection    │
│ • HeaderDetection   │
│ • BlockMerge        │
│ • etc.              │
└─────┬───────────────┘
      │
      ▼
┌─────────────────────┐
│ Renderer            │
│ • Markdown          │
│ • JSON              │
└─────┬───────────────┘
      │
      ▼
┌─────────────────────┐
│ Output String       │
└─────────────────────┘
```

## Critical Issues

### Issue #1: Layout Analysis Duplication
- **Location:** `backend/pdfium.rs:426` + `processors/processor.rs:87`
- **Impact:** Performance penalty, wasted CPU
- **Root Cause:** Unclear separation of concerns

### Issue #2: Inconsistent Backend Abstraction
- **PdfiumBackend:** Returns fully analyzed document
- **MockBackend:** Returns raw document
- **Impact:** Processors must handle both cases

### Issue #3: Tight Coupling
- Backend directly instantiates `LayoutAnalyzer`
- Cannot swap layout algorithm without modifying backend
- Hard to test layout independently

## Strengths

1. ✅ **Clean Trait Abstraction:** `PdfBackend` trait allows multiple implementations
2. ✅ **Rich Document Model:** Block-based hierarchy with geometry
3. ✅ **Sophisticated Layout:** XY-Cut + reading order + columns
4. ✅ **Extensible Pipeline:** Processor trait with composable chain
5. ✅ **Multiple Output Formats:** Markdown (3 styles) + JSON
6. ✅ **Good Test Coverage:** 98 unit tests passing
7. ✅ **CLI Tool:** Usable binary with clap

## Weaknesses

1. ❌ **Duplicate Layout Analysis:** Runs twice per document
2. ❌ **Backend Overreach:** Does more than extraction
3. ❌ **Inconsistent Abstraction:** Backends behave differently
4. ❌ **Fixed Pipeline:** Cannot easily remove/reorder processors
5. ❌ **Vision Module Orphaned:** `vision.rs` exists but not integrated
6. ❌ **No Streaming:** Must process entire document in memory

## Lines of Code Breakdown

```
Component              Files    Lines    %
────────────────────────────────────────────
Schema                 4        1,725    22%
Processors             4        1,737    22%
Layout                 4        1,684    21%
Renderers              2          727     9%
Backend                3          553     7%
Extractor              1          296     4%
Config                 1          312     4%
Vision                 1          464     6%
Other                  -          422     5%
────────────────────────────────────────────
Total                  26       7,920   100%
```

## Next Steps

See [02-proposed-architecture.md](./02-proposed-architecture.md) for the refactored design.
