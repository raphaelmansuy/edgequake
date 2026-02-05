# OODA-42: Observe - Pipeline Architecture Analysis

## Date: 2026-02-05

## Objective

Deep analysis of the dual extraction pipeline architecture to understand:

1. What code belongs to LEGACY (lopdf) vs NEW (pdfium) pipeline
2. What code is SHARED between both pipelines
3. Dependencies and coupling between modules
4. Lines of code inventory per pipeline

---

## ASCII Architecture Diagram

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                     EDGEQUAKE-PDF EXTRACTION ARCHITECTURE                     │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                          ENTRY POINT: extractor.rs                       │ │
│  │                                                                         │ │
│  │   PdfExtractor::new() ─────────► Backend Selection (compile-time)       │ │
│  │        │                              │                                  │ │
│  │        │  #[cfg(feature="lopdf")]     │  #[cfg(not(feature="lopdf"))]   │ │
│  │        │           │                  │           │                      │ │
│  │        ▼           ▼                  ▼           ▼                      │ │
│  │   ExtractionEngine (default)     MockBackend (fallback)                 │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                      │                                       │
│                 ┌────────────────────┴────────────────────┐                  │
│                 │                                         │                  │
│  ╔══════════════▼════════════════╗   ╔═══════════════════▼════════════════╗ │
│  ║   LEGACY PIPELINE (lopdf)     ║   ║    NEW PIPELINE (pdfium)           ║ │
│  ║   default = ["lopdf"]         ║   ║    pdfium = ["dep:pdfium-render"]  ║ │
│  ╠═══════════════════════════════╣   ╠════════════════════════════════════╣ │
│  ║                               ║   ║                                    ║ │
│  ║  ┌───────────────────────┐   ║   ║  ┌────────────────────────────┐   ║ │
│  ║  │  extraction_engine.rs │   ║   ║  │     pdfium.rs (304 lines)  │   ║ │
│  ║  │      (1302 lines)     │   ║   ║  │                            │   ║ │
│  ║  │                       │   ║   ║  │  PdfiumExtractor           │   ║ │
│  ║  │  ExtractionEngine     │   ║   ║  │   └─ extract_chars()       │   ║ │
│  ║  │   └─ extract()        │   ║   ║  │   └─ font weight flags     │   ║ │
│  ║  │   └─ font name match  │   ║   ║  │   └─ accurate bounding     │   ║ │
│  ║  └───────────┬───────────┘   ║   ║  └──────────┬─────────────────┘   ║ │
│  ║              │               ║   ║             │                      ║ │
│  ║  ┌───────────▼───────────┐   ║   ║  ┌──────────▼─────────────────┐   ║ │
│  ║  │  content_parser.rs    │   ║   ║  │ pymupdf_pipeline.rs        │   ║ │
│  ║  │      (664 lines)      │   ║   ║  │     (276 lines)            │   ║ │
│  ║  │                       │   ║   ║  │                            │   ║ │
│  ║  │  ContentParser        │   ║   ║  │ PymupdfPipeline            │   ║ │
│  ║  │   └─ parse_operators  │   ║   ║  │  └─ convert_file()         │   ║ │
│  ║  │   └─ text matrix calc │   ║   ║  │  └─ convert_bytes()        │   ║ │
│  ║  └───────────┬───────────┘   ║   ║  └──────────┬─────────────────┘   ║ │
│  ║              │               ║   ║             │                      ║ │
│  ║  ┌───────────▼───────────┐   ║   ║  ┌──────────▼─────────────────┐   ║ │
│  ║  │  element_processing   │   ║   ║  │ pymupdf_grouper.rs         │   ║ │
│  ║  │      (389 lines)      │   ║   ║  │     (1362 lines)           │   ║ │
│  ║  │                       │   ║   ║  │                            │   ║ │
│  ║  │  ElementProcessor     │   ║   ║  │ TextGrouper                │   ║ │
│  ║  │   └─ filter_elements  │   ║   ║  │  └─ group() -> Blocks      │   ║ │
│  ║  │   └─ detect_style     │   ║   ║  │  └─ classify_blocks()      │   ║ │
│  ║  └───────────┬───────────┘   ║   ║  └──────────┬─────────────────┘   ║ │
│  ║              │               ║   ║             │                      ║ │
│  ║  ┌───────────▼───────────┐   ║   ║  ┌──────────▼─────────────────┐   ║ │
│  ║  │  text_grouping.rs     │   ║   ║  │ pymupdf_renderer.rs        │   ║ │
│  ║  │      (1492 lines)     │   ║   ║  │     (TBD lines)            │   ║ │
│  ║  │                       │   ║   ║  │                            │   ║ │
│  ║  │  TextGrouper          │   ║   ║  │ MarkdownRenderer           │   ║ │
│  ║  │   └─ group_into_lines │   ║   ║  │  └─ render()               │   ║ │
│  ║  │   └─ merge_line       │   ║   ║  │  └─ apply styles           │   ║ │
│  ║  └───────────┬───────────┘   ║   ║  └──────────┬─────────────────┘   ║ │
│  ║              │               ║   ║             │                      ║ │
│  ║  ┌───────────▼───────────┐   ║   ║             │                      ║ │
│  ║  │  block_builder.rs     │   ║   ╚═════════════▼══════════════════════╝ │
│  ║  │      (398 lines)      │   ║                 │                       │
│  ║  │                       │   ║                 │                       │
│  ║  │  BlockBuilder         │   ║                 │                       │
│  ║  │   └─ build_blocks()   │   ║                 │                       │
│  ║  └───────────┬───────────┘   ║                 │                       │
│  ║              │               ║                 │                       │
│  ║  ┌───────────▼───────────┐   ║                 │                       │
│  ║  │  column_detection.rs  │   ║                 │                       │
│  ║  │      (714 lines)      │   ║                 │                       │
│  ║  │                       │   ║                 │                       │
│  ║  │  ColumnDetector       │   ║                 │                       │
│  ║  │   └─ detect_columns   │   ║                 │                       │
│  ║  │   └─ histogram proj.  │   ║                 │                       │
│  ║  └───────────────────────┘   ║                 │                       │
│  ║                               ║                 │                       │
│  ╚═══════════════════════════════╝                 │                       │
│                                                    │                       │
│  ┌─────────────────────────────────────────────────▼─────────────────────┐ │
│  │                       SHARED MODULES (both pipelines use)             │ │
│  ├───────────────────────────────────────────────────────────────────────┤ │
│  │                                                                       │ │
│  │  elements.rs (96 lines)       - RawChar, TextElement structs          │ │
│  │  spatial.rs (339 lines)       - R-tree spatial indexing               │ │
│  │  mock.rs (45 lines)           - MockBackend for testing               │ │
│  │                                                                       │ │
│  │  schema/                      - Document, Page, Block, BoundingBox    │ │
│  │  processors/                  - Post-processing (headers, tables)     │ │
│  │  renderers/markdown.rs        - Final Markdown output                 │ │
│  │                                                                       │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                   LOPDF-ONLY SUPPORT MODULES (to deprecate)           │  │
│  ├───────────────────────────────────────────────────────────────────────┤  │
│  │                                                                       │  │
│  │  font_handling.rs (616 lines)  - Font dictionary parsing              │  │
│  │  encodings.rs (1317 lines)     - Character encoding tables            │  │
│  │  glyph_list.rs (391 lines)     - Adobe glyph name mapping             │  │
│  │  truetype_cmap.rs (201 lines)  - TrueType cmap table parsing          │  │
│  │  lattice.rs (1712 lines)       - Table detection via line patterns    │  │
│  │                                                                       │  │
│  │  Total: 4,237 lines to deprecate/remove                               │  │
│  │                                                                       │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Lines of Code Inventory

### LEGACY Pipeline (lopdf) - 8,379 lines

| File                  | Lines | Purpose              | Status        |
| --------------------- | ----- | -------------------- | ------------- |
| extraction_engine.rs  | 1,302 | Main lopdf backend   | TO DEPRECATE  |
| text_grouping.rs      | 1,492 | Line/block grouping  | TO DEPRECATE  |
| content_parser.rs     | 664   | PDF operator parsing | TO DEPRECATE  |
| column_detection.rs   | 714   | Two-column detection | KEEP (shared) |
| font_handling.rs      | 616   | Font dict parsing    | TO DEPRECATE  |
| element_processing.rs | 389   | Element filtering    | TO DEPRECATE  |
| block_builder.rs      | 398   | Block construction   | TO DEPRECATE  |
| lattice.rs            | 1,712 | Table detection      | TO DEPRECATE  |
| encodings.rs          | 1,317 | Encoding tables      | TO DEPRECATE  |
| glyph_list.rs         | 391   | Adobe glyph names    | TO DEPRECATE  |
| truetype_cmap.rs      | 201   | TrueType parsing     | KEEP (useful) |

### NEW Pipeline (pdfium) - 1,942 lines

| File                | Lines | Purpose                | Status          |
| ------------------- | ----- | ---------------------- | --------------- |
| pdfium.rs           | 304   | PDFium char extraction | KEEP (core)     |
| pymupdf_pipeline.rs | 276   | High-level pipeline    | KEEP (core)     |
| pymupdf_grouper.rs  | 1,362 | Text grouping          | KEEP (refactor) |

### SHARED Modules - 935 lines

| File        | Lines | Purpose         | Status |
| ----------- | ----- | --------------- | ------ |
| elements.rs | 96    | RawChar struct  | KEEP   |
| spatial.rs  | 339   | R-tree indexing | KEEP   |
| mock.rs     | 45    | Testing backend | KEEP   |
| mod.rs      | 106   | Module exports  | UPDATE |

---

## Critical Finding: Font Style Detection

### LEGACY (lopdf) - UNRELIABLE

```rust
// font_handling.rs - guesses from font name strings
fn is_bold(&self) -> bool {
    let name_lower = self.base_font_name.to_lowercase();
    name_lower.contains("bold") ||
    name_lower.contains("bd") ||
    name_lower.contains("medi") ||  // Added OODA-09
    name_lower.contains("semi")
}

// Problem: Fonts like "ArialMT" won't match
// Result: Missed bold text → Quality loss
```

### NEW (pdfium) - ACCURATE

```rust
// pdfium.rs - reads actual font descriptor flags
let is_bold = char_obj.font_weight().map_or(false, |w| match w {
    PdfFontWeight::Weight700Bold
    | PdfFontWeight::Weight800
    | PdfFontWeight::Weight900 => true,
    PdfFontWeight::Custom(n) => n >= 700,
    _ => false,
});

// Result: Accurate bold detection from PDF metadata
```

---

## Data Flow Comparison

### LEGACY Pipeline Flow

```
PDF bytes
    │
    ▼
┌──────────────────┐
│ lopdf::Document  │  Parse PDF structure
└────────┬─────────┘
         │
    ▼    ▼
┌──────────────────┐
│ ContentParser    │  Parse content streams, text operators
│  └─ Tj, TJ, Tm   │  Extract text matrix positions
└────────┬─────────┘
         │
    ▼    ▼
┌──────────────────┐
│ TextElement[]    │  Font name + estimated positions
│  (UNRELIABLE)    │  Width estimation factor = 0.48
└────────┬─────────┘
         │
    ▼    ▼
┌──────────────────┐
│ TextGrouper      │  Group elements → lines → blocks
│ ColumnDetector   │  Detect multi-column layouts
└────────┬─────────┘
         │
    ▼    ▼
┌──────────────────┐
│ BlockBuilder     │  Build schema::Block from groups
└────────┬─────────┘
         │
    ▼    ▼
┌──────────────────┐
│ Document IR      │  schema::Document with Pages/Blocks
└────────┬─────────┘
         │
    ▼    ▼
┌──────────────────┐
│ ProcessorChain   │  Headers, tables, lists, cleanup
└────────┬─────────┘
         │
    ▼    ▼
┌──────────────────┐
│ MarkdownRenderer │  Final Markdown output
└──────────────────┘
```

### NEW Pipeline Flow (pdfium)

```
PDF bytes
    │
    ▼
┌──────────────────┐
│ PDFium (C++)     │  Google's Chromium PDF engine
│ pdfium-render    │  Rust bindings
└────────┬─────────┘
         │
    ▼    ▼
┌──────────────────┐
│ RawChar[]        │  Accurate character positions
│  is_bold: bool   │  Font descriptor flags (700+)
│  is_italic: bool │  Font italic flag
│  x0,y0,x1,y1     │  Tight bounding boxes
└────────┬─────────┘
         │
    ▼    ▼
┌──────────────────┐
│ TextGrouper      │  Char → Span → Line → Block
│ (pymupdf_grouper)│  Style-aware grouping
└────────┬─────────┘
         │
    ▼    ▼
┌──────────────────┐
│ classify_blocks  │  Header, List, Code detection
└────────┬─────────┘
         │
    ▼    ▼
┌──────────────────┐
│ MarkdownRenderer │  Style-preserving output
│ (pymupdf_render) │  **bold**, *italic*, # headers
└──────────────────┘
```

---

## Key Observations

1. **lopdf is default but inferior** - The Cargo.toml has `default = ["lopdf"]` but lopdf produces lower quality due to font name matching

2. **pdfium requires runtime library** - Needs `libpdfium.dylib` at runtime, limiting deployment

3. **Two parallel text grouping implementations**:
   - `backend/text_grouping.rs` (1492 lines) for lopdf
   - `layout/pymupdf_grouper.rs` (1362 lines) for pdfium
   - Violates DRY principle

4. **Evaluation uses the DEFAULT backend** - eval_comprehensive.py calls the binary which uses lopdf, not pdfium!

5. **Mission spec says "Eliminate lopdf"** - Direct quote: "Eliminate other backends such as lopdf"

---

## Next Steps (Orient)

1. Verify which backend eval_comprehensive.py actually uses
2. Plan the deprecation strategy for lopdf modules
3. Document what needs to happen to make pdfium the default
4. Estimate LOC reduction from removing lopdf support
