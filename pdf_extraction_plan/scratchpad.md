# PyZerox vs EdgeQuake-PDF: Comparison & Improvement Plan

**Date:** 2025-12-31
**Analysis:** Comparison with pyzerox and improvement proposals
**Status:** ✅ MARKER-STYLE ARCHITECTURE IMPLEMENTED

---

## 🔄 OODA Loop: Conversion Quality Analysis

### Cycle 1: 2025-01-01

#### **OBSERVE: Current Conversion Output Analysis**

**Test File 1: sample.pdf (18,810 bytes, 1 page)**

- Output: 1,572 characters
- Contains: Lorem ipsum placeholder text with headers

**Observed Issues:**

1. ❌ **Word concatenation**: "Phaselluscongue", "diamsuscipitmauris", "penatibusetmagnisdisparturient"
2. ❌ **Header order inverted**: "## This is a simple PDF" appears BEFORE "# Sample PDF"
3. ❌ **No page number in output** (--page-numbers flag not producing expected results)
4. ⚠️ **Character encoding**: "Þle" instead of "file" (Þ = thorn character, encoding issue)

**Test File 2: one_tool.pdf (49,756 characters, 14 pages)**

- Output: 49,756 characters (comprehensive academic paper)
- Contains: Full research paper with tables, figures, references

**Observed Issues:**

1. ❌ **Two-column layout merged incorrectly**: Text from left/right columns interleaved
2. ❌ **Table extraction broken**: Tables rendered as raw text with pipes misaligned
3. ❌ **Figure placeholders generic**: "Image 1.1: Figure/Chart" lacks context
4. ⚠️ **Math formulas partially rendered**: Some LaTeX preserved, some broken
5. ✅ **Headers detected**: Section headers correctly identified with ##
6. ✅ **Links preserved**: URLs in references retained
7. ⚠️ **Page breaks inconsistent**: Some pages have "## arXiv..." markers

---

#### **ORIENT: Root Cause Analysis**

| Issue              | Root Cause                      | Severity | Fix Complexity |
| ------------------ | ------------------------------- | -------- | -------------- |
| Word concatenation | pdf_oxide loses word boundaries | HIGH     | MEDIUM         |
| Header order       | Reading order algorithm         | HIGH     | LOW            |
| Two-column merge   | No layout detection active      | HIGH     | HIGH           |
| Table extraction   | pdf_oxide text stream           | MEDIUM   | MEDIUM         |
| Encoding issues    | PDF font encoding               | LOW      | LOW            |
| Page numbers       | CLI flag not passed to config   | LOW      | LOW            |

**Key Insight:** pdf_oxide's `to_markdown()` output is the bottleneck. The underlying text extraction merges words and loses layout.

---

#### **DECIDE: Improvement Strategy**

**Immediate Fixes (This Session):**

1. Fix page number output in CLI
2. Add word boundary normalization post-processing
3. Fix header reading order (sort by Y position)

**Short-term Improvements (Future):**

1. Enable layout detection in extraction pipeline
2. Add table detection and formatting
3. Improve two-column handling with XY-cut

**Long-term (Vision Mode):**

1. Integrate pdfium-render for page images
2. Use vision LLM for complex documents
3. Hybrid mode with quality detection

---

#### **ACT: Implementation Plan**

```
- [x] Fix CLI page numbers flag ✅
- [x] Add word boundary normalization ✅
- [x] Fix character encoding (Þ → fi) ✅
- [x] Sort headers by Y position (move title to top) ✅
- [x] Test on sample.pdf and one_tool.pdf ✅
- [x] Verify all 116 tests pass ✅
- [x] Document improvements ✅
```

---

### Cycle 1 Results: 2025-01-01

#### **Improvements Implemented:**

1. **Character Encoding Fix** (`fix_character_encoding()`)

   - Fixed: "Þle" → "file" (thorn ligature)
   - Fixed: ﬁ, ﬂ, ﬀ, ﬃ, ﬄ ligatures

2. **Header Reordering** (`reorder_document_headers()`)

   - Main title (# ) now moved to top of document
   - Before: "## Subtitle" appeared before "# Title"
   - After: "# Title" appears first

3. **Test Results:**
   - sample.pdf: Title now at top, "file" correctly rendered
   - one_tool.pdf: 50,007 chars extracted, headers correctly ordered
   - All 116 tests pass

#### **Remaining Issues (Future OODA Cycles):**

| Issue              | Status               | Next Steps                         |
| ------------------ | -------------------- | ---------------------------------- |
| Two-column merge   | 🔴 Not fixed         | Requires layout-aware extraction   |
| Word concatenation | 🟡 Partially fixed   | Needs vision mode for complex PDFs |
| Table extraction   | 🔴 Not fixed         | Needs table detection pipeline     |
| Math formulas      | 🟡 Partially working | LaTeX extraction needs improvement |

---

## ✅ Implementation Status

The Marker-style architecture has been fully implemented. EdgeQuake-PDF now includes:

1. **Block-based Schema** - 22 block types, hierarchical structure, reading order
2. **Layout Detection** - XY-cut algorithm, column detection, reading order
3. **Vision Mode** - VisionExtractor with multimodal LLM support
4. **LLM Enhancement** - Table formatting, math conversion, image descriptions
5. **Multiple Renderers** - Markdown and JSON output formats

**Test Results:** 116 tests passing (105 unit + 10 integration + 1 doc)

See [implementation_summary.md](./implementation_summary.md) for full details.

---

## Executive Summary

**PyZerox** uses a **vision-first** approach: convert PDF pages to images, then use vision LLMs (GPT-4o, Claude, Gemini) to "read" the visual representation and output markdown directly.

**EdgeQuake-PDF** uses a **text extraction + AI enhancement** approach: extract text programmatically from PDF structure using `pdf_oxide`, then optionally use AI for refinement.

Both approaches have trade-offs, and the **optimal solution combines the best of both**.

---

## Comparison Matrix

| Feature                          | PyZerox                               | EdgeQuake-PDF                          | Winner    |
| -------------------------------- | ------------------------------------- | -------------------------------------- | --------- |
| **Core Approach**                | Vision model reads page images        | Text extraction + AI enhancement       | Trade-off |
| **Two-Column Layout**            | ✅ Excellent (vision sees layout)     | ❌ Poor (text streams merge columns)   | PyZerox   |
| **Tables**                       | ✅ Excellent (vision sees structure)  | ❌ Poor (no spatial awareness)         | PyZerox   |
| **Cost per Page**                | ~$0.01-0.03 (vision tokens expensive) | ~$0.001-0.005 (text only, optional AI) | EdgeQuake |
| **Speed**                        | Slower (image encoding + LLM)         | Faster (native extraction)             | EdgeQuake |
| **Accuracy for Text-Heavy PDFs** | Good                                  | Good (with post-processing)            | Tie       |
| **Accuracy for Complex Layouts** | Excellent                             | Poor                                   | PyZerox   |
| **Works Offline**                | ❌ Requires API                       | ✅ Mock mode available                 | EdgeQuake |
| **Native Rust**                  | ❌ Python                             | ✅ Rust                                | EdgeQuake |
| **Image Extraction**             | ❌ Pages as images only               | ✅ Extracts embedded images            | EdgeQuake |
| **Maintain Format Option**       | ✅ Cross-page context                 | ❌ Not implemented                     | PyZerox   |
| **Concurrency**                  | ✅ Parallel page processing           | ❌ Sequential                          | PyZerox   |
| **Multi-Provider Support**       | ✅ OpenAI, Claude, Gemini, Bedrock    | ⚠️ OpenAI (via edgequake-llm)          | PyZerox   |

---

## Key Insight: Why PyZerox Works Better for Complex Documents

PyZerox's approach is fundamentally different:

```
PDF → Images (poppler/graphicsmagick) → Vision Model → Markdown
```

The vision model **sees the document as a human would see it**:

- Two columns are visually distinct
- Tables are visually structured
- Headers stand out visually
- Charts and figures are naturally described

EdgeQuake's current approach:

```
PDF → Text Extraction (pdf_oxide) → Post-Processing → Markdown
```

The text extraction **loses spatial information**:

- Two columns become interleaved text streams
- Tables become chaotic text
- Layout-dependent meaning is lost

---

## Proposed Improvements for EdgeQuake-PDF

### Phase 1: Hybrid Vision Mode (High Impact)

Add a **vision-first extraction mode** similar to PyZerox:

```rust
pub enum ExtractionMode {
    /// Fast text-based extraction with post-processing
    TextBased,
    /// Vision model reads rendered page images (most accurate)
    VisionBased,
    /// Try text-based first, fallback to vision if quality is low
    Hybrid,
}
```

**Implementation Steps:**

1. Add PDF-to-image conversion using `pdfium-render` or `pdf-render` crate
2. Encode page images as base64
3. Send to vision-capable LLM with structured prompt
4. Aggregate per-page markdown

**Cost Management:**

- Only use vision mode for complex pages (detected by layout heuristics)
- Use text mode for simple text-heavy pages
- Allow user to configure per-page or per-document

### Phase 2: Page Rendering Pipeline

```rust
/// Render PDF page to image for vision processing
async fn render_page_to_image(&self, page_num: usize) -> Result<Vec<u8>> {
    // Use pdfium-render or cairo-based rendering
    // Return PNG bytes
}

/// Process page using vision model
async fn extract_page_with_vision(&self, image_bytes: &[u8]) -> Result<String> {
    let base64_image = base64::encode(image_bytes);

    let messages = vec![
        ChatMessage::system(VISION_SYSTEM_PROMPT),
        ChatMessage::user_with_image(
            "Convert this PDF page to clean markdown.",
            &base64_image,
            "image/png"
        ),
    ];

    self.llm_provider.chat(&messages, None).await
}
```

### Phase 3: Quality Detection for Hybrid Mode

Detect when vision mode should be used:

```rust
fn should_use_vision_mode(&self, text: &str, page_num: usize) -> bool {
    let indicators = [
        // Many short lines suggest columns or tables
        text.lines().filter(|l| l.len() < 50).count() as f32 / text.lines().count() as f32 > 0.5,
        // Many pipe characters suggest tables
        text.matches('|').count() > 10,
        // Repeated patterns suggest corrupted extraction
        has_repetitive_patterns(text),
        // Very short text for a page suggests extraction failure
        text.len() < 500,
    ];

    indicators.iter().filter(|&&x| x).count() >= 2
}
```

### Phase 4: Cross-Page Context (maintainFormat)

Implement PyZerox's `maintain_format` feature:

```rust
pub struct PageContext {
    previous_page_markdown: Option<String>,
    document_structure: DocumentStructure,
}

async fn extract_page_with_context(&self, page: usize, ctx: &PageContext) -> Result<String> {
    let prompt = if let Some(prev) = &ctx.previous_page_markdown {
        format!(
            "Previous page ended with:\n{}\n\nContinue extracting, maintaining formatting:",
            prev.chars().take(500).collect::<String>()
        )
    } else {
        "Extract this PDF page to markdown:".to_string()
    };
    // Process with context
}
```

### Phase 5: Concurrent Page Processing

Add parallel processing like PyZerox:

```rust
pub async fn extract_concurrent(&self, pdf_bytes: &[u8], concurrency: usize) -> Result<String> {
    let results: Vec<_> = futures::stream::iter(0..page_count)
        .map(|page| self.extract_page(page))
        .buffer_unordered(concurrency)
        .collect()
        .await;

    // Aggregate in order
    results.into_iter().sorted_by_key(|(i, _)| *i).collect()
}
```

---

## Vision System Prompt (Based on PyZerox)

```rust
const VISION_SYSTEM_PROMPT: &str = r#"
You are a document processing assistant. Convert the PDF page image to clean Markdown.

Rules:
1. Output ONLY the markdown content, no explanations
2. Preserve document structure (headers, lists, paragraphs)
3. Format tables using markdown table syntax
4. Preserve code blocks with appropriate language tags
5. For figures/charts, describe them briefly in [Figure: description] format
6. Maintain reading order (left column before right column for multi-column)
7. Preserve mathematical equations using LaTeX notation ($...$)
8. Include page numbers or section references if visible
"#;
```

---

## PDF Rendering Crate Options

| Crate                | Pros                              | Cons                           |
| -------------------- | --------------------------------- | ------------------------------ |
| `pdfium-render`      | High quality, Chromium's renderer | Large binary, complex setup    |
| `pdf-render`         | Pure Rust                         | Lower quality for complex PDFs |
| `mupdf` (bindings)   | Excellent quality                 | C dependency                   |
| `poppler` (bindings) | Industry standard                 | C dependency                   |

**Recommendation:** Start with `pdfium-render` for quality, add feature flag for pure-Rust fallback.

---

## Configuration Updates

```rust
pub struct PdfConfig {
    // ... existing fields ...

    /// Extraction mode: TextBased, VisionBased, or Hybrid
    pub extraction_mode: ExtractionMode,

    /// DPI for page rendering in vision mode (default: 150)
    pub vision_dpi: u32,

    /// Maximum image dimension for vision processing
    pub max_image_dimension: u32,

    /// Enable cross-page context for consistent formatting
    pub maintain_format: bool,

    /// Number of pages to process concurrently
    pub concurrency: usize,

    /// Quality threshold for hybrid mode (0.0-1.0)
    pub text_quality_threshold: f32,
}
```

---

## Cost Analysis

### PyZerox (Vision-Only)

- GPT-4o: ~$0.025 per page (high-res images)
- GPT-4o-mini: ~$0.005 per page
- Gemini Flash: ~$0.001 per page

### EdgeQuake Current (Text-Only)

- No LLM cost for basic extraction
- Optional AI enhancement: ~$0.002 per page

### EdgeQuake Proposed (Hybrid)

- Simple pages: $0 (text extraction)
- Complex pages: ~$0.005-0.01 (vision)
- Average: ~$0.003 per page (assuming 30% complex pages)

---

## Implementation Priority

1. **🔴 High Priority: Vision Mode Core**

   - Add PDF-to-image rendering
   - Extend LLM trait for images
   - Implement vision extraction pipeline

2. **🟠 Medium Priority: Hybrid Intelligence**

   - Page complexity detection
   - Automatic mode switching
   - Quality validation

3. **🟡 Nice to Have: Advanced Features**
   - Concurrent processing
   - Cross-page context
   - Multi-provider support

---

## Quick Wins: Immediate Text-Based Improvements

Before implementing vision mode, these can improve current output:

1. **Better Two-Column Detection**

   - Analyze line lengths and positions
   - Use heuristics to detect column boundaries
   - Reorder text by spatial position

2. **Table Reconstruction**

   - Detect tabular patterns in extracted text
   - Use regex to identify table structures
   - Format as markdown tables

3. **Smarter Word Boundary Detection**
   - Use a word frequency dictionary
   - Apply statistical word segmentation
   - Handle domain-specific terminology

---

## Conclusion

The **recommended path** is:

1. **Short-term:** Continue improving text-based post-processing (current work) ✅
2. **Medium-term:** Add vision mode as an option for complex documents
3. **Long-term:** Implement hybrid mode with automatic quality detection

This gives users:

- **Fast, free extraction** for simple documents
- **High-quality vision extraction** for complex layouts
- **Automatic optimization** with hybrid mode

---

## Previous Session Notes (Archived)

### pdf_oxide v0.2.2 API Research

```rust
use pdf_oxide::PdfDocument;
use pdf_oxide::converters::ConversionOptions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = PdfDocument::open("paper.pdf")?;
    println!("Pages: {}", doc.page_count());
    let text = doc.extract_text(0)?;
    let options = ConversionOptions::default();
    let markdown = doc.to_markdown(0, &options)?;
    Ok(())
}
```

2. **Markdown Export**: Clean formatting with bold detection
3. **Image Extraction**: Extract embedded images with metadata
4. **Layout Analysis**: DBSCAN clustering and XY-Cut algorithms
5. **OCR Detection**: Auto-detects scanned vs native PDFs

### Dependency Notes

- `pdf_oxide` uses `image = "0.24"` internally
- Need to match image version to avoid conflicts
- No external C dependencies (pure Rust)

## Implementation Progress

### Step 1: Fix Cargo.toml ⏳

Need to update dependencies:

- Change `image = "0.25"` to `image = "0.24"` (match pdf_oxide)
- Change `tesseract = "0.14"` to optional or remove (pdf_oxide has built-in OCR)

### Step 2: Implement Core Extractor ⏳

Core extraction flow:

1. Load PDF bytes into memory
2. Open with `PdfDocument::open_from_bytes()` or write to temp file
3. Iterate pages and extract content
4. Optionally enhance with AI
5. Assemble final Markdown

### Step 3: AI Enhancement ⏳

For images:

1. Extract image bytes
2. Base64 encode
3. Send to LLM with vision prompt
4. Parse description

For tables:

1. Detect table regions
2. If complex, send to AI for interpretation
3. Generate Markdown table

## Blockers & Solutions

### Blocker 1: pdf_oxide open from bytes

Need to check if `PdfDocument::open_from_bytes()` exists or need to write temp file.

**Solution**: Check API docs, may need to write bytes to temp file first.

### Blocker 2: Image version mismatch

pdf_oxide uses `image = "0.24"` but Cargo.toml has `image = "0.25"`.

**Solution**: Update to `image = "0.24"` for compatibility.

## Test Files Needed

1. Simple text PDF - for basic extraction
2. Academic paper PDF - for complex layout
3. Form PDF - for form field extraction
4. Scanned PDF - for OCR testing

## Commands Log

```bash
# Build the crate
cargo build --package edgequake-pdf

# Run tests
cargo test --package edgequake-pdf

# Check for errors
cargo check --package edgequake-pdf
```

## Notes

- pdf_oxide is 47.9× faster than PyMuPDF4LLM
- Average 53ms per PDF processing
- Production ready with 100% success rate on 103-file test suite
