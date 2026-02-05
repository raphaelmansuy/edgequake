# Iteration 01: Observe

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Territory Mapping

### Directory Structure

```
edgequake/crates/edgequake-pdf/src/
├── backend/
│   ├── mod.rs                    # Backend trait definition
│   ├── pdfium_backend.rs         # NEW: PdfiumBackend (recommended)
│   ├── pdfium.rs                 # PDFium extractor wrapper
│   ├── extraction_engine.rs      # LEGACY: lopdf-based (deprecated)
│   ├── font_handling.rs          # Font detection (shared)
│   ├── content_parser.rs         # PDF content stream parser
│   ├── text_grouping.rs          # Text grouping logic
│   ├── block_builder.rs          # Block construction
│   ├── column_detection.rs       # Multi-column detection
│   ├── element_processing.rs     # Element processing
│   ├── elements.rs               # TextElement, RawChar types
│   ├── encodings.rs              # Character encodings
│   ├── glyph_list.rs             # Glyph→Unicode mapping
│   ├── lattice.rs                # Table detection
│   ├── truetype_cmap.rs          # TrueType font parsing
│   ├── spatial.rs                # Spatial utilities
│   └── mock.rs                   # Mock backend for tests
├── layout/
│   ├── mod.rs                    # Layout module exports
│   ├── pymupdf_grouper.rs        # Text grouping (pymupdf4llm style)
│   ├── pymupdf_renderer.rs       # Markdown rendering
│   ├── pymupdf_structs.rs        # Block/Line/Span types
│   ├── column_detector.rs        # Column detection
│   ├── block_classifier.rs       # Block type classification
│   ├── reading_order.rs          # Reading order detection
│   ├── xy_cut.rs                 # XY-cut algorithm
│   └── geometric.rs              # Geometric utilities
├── renderers/
│   ├── mod.rs
│   ├── markdown.rs               # Markdown output
│   └── json.rs                   # JSON output
├── pipeline/                     # Processing pipeline
├── processors/                   # Document processors
└── lib.rs                        # Public API
```

### Two Extraction Pipelines Identified

#### 1. PdfiumBackend (RECOMMENDED - pdfium_backend.rs + pdfium.rs)

- Uses `pdfium-render` crate (bindings to Google's PDFium)
- **Font style detection**: Uses `font_is_italic()` and `font_weight()` from PDFium API
- **Status**: Active, recommended in docs
- **Lines**: ~475 (pdfium_backend.rs) + ~306 (pdfium.rs)

```rust
// pdfium.rs:166-178 - Font style extraction
let is_italic = char_obj.font_is_italic();
let is_bold = char_obj.font_weight().is_some_and(|w| {
    matches!(
        w,
        PdfFontWeight::Weight700Bold
            | PdfFontWeight::Weight800
            | PdfFontWeight::Weight900
    ) || matches!(w, PdfFontWeight::Custom(n) if n >= 700)
});
```

#### 2. ExtractionEngine (DEPRECATED - extraction_engine.rs)

- Uses `lopdf` crate (pure Rust PDF parsing)
- **Font style detection**: Uses font name pattern matching + FontDescriptor flags
- **Status**: Deprecated since v0.2.0, marked for removal in v0.3.0
- **Lines**: ~1347

```rust
// font_handling.rs:56-88 - Font style from FontDescriptor + name patterns
let is_bold = if let Some(bold_flag) = flags_bold {
    bold_flag
} else {
    lower_name.contains("bold")
        || lower_name.contains("black")
        || lower_name.contains("heavy")
        || lower_name.contains("sfbx")   // SF Bold Extended
        || lower_name.contains("cmbx")   // Computer Modern Bold Extended
        || lower_name.contains("medi")   // Medium weight
        || lower_name.contains("-bold")
};
```

### Font Style Propagation Chain

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    FONT STYLE DATA FLOW                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  PDFIUM PIPELINE:                                                       │
│  ════════════════                                                       │
│                                                                         │
│  pdfium.rs:extract_chars_from_page()                                    │
│      │                                                                  │
│      │ char_obj.font_is_italic() → bool                                 │
│      │ char_obj.font_weight() → PdfFontWeight                           │
│      ▼                                                                  │
│  RawChar { is_bold: bool, is_italic: bool, ... }                        │
│      │                                                                  │
│      │ pdfium_backend.rs → TextGrouper                                  │
│      ▼                                                                  │
│  layout/pymupdf_grouper.rs:group()                                      │
│      │                                                                  │
│      │ Groups RawChar → Span → Line → Block                             │
│      ▼                                                                  │
│  pymupdf_structs.rs::Span { flags: u32, ... }                           │
│      │                                                                  │
│      │ flags: bit 0 = superscript, bit 1 = italic, bit 2 = serifed,     │
│      │        bit 3 = monospaced, bit 4 = bold                          │
│      ▼                                                                  │
│  pymupdf_renderer.rs:render_block_markdown()                            │
│      │                                                                  │
│      │ Converts flags → **bold**, *italic*, `code`                      │
│      ▼                                                                  │
│  Markdown Output                                                        │
│                                                                         │
│  LOPDF PIPELINE (DEPRECATED):                                           │
│  ════════════════════════════                                           │
│                                                                         │
│  extraction_engine.rs                                                   │
│      │                                                                  │
│      │ font_handling.rs:FontInfo::from_dict()                           │
│      │ - FontDescriptor.Flags bit 7 → italic                            │
│      │ - FontDescriptor.ItalicAngle != 0 → italic                       │
│      │ - FontDescriptor.Weight >= 700 → bold                            │
│      │ - Font name patterns → bold/italic (fallback)                    │
│      ▼                                                                  │
│  TextElement { is_bold: bool, is_italic: bool, ... }                    │
│      │                                                                  │
│      │ text_grouping.rs                                                 │
│      ▼                                                                  │
│  Block { block_type, ... }                                              │
│      │                                                                  │
│      │ ⚠️ STYLE INFORMATION LOST HERE!                                  │
│      │ schema::Block does not carry per-span styles                     │
│      ▼                                                                  │
│  renderers/markdown.rs                                                  │
│      │                                                                  │
│      │ No style information available                                   │
│      ▼                                                                  │
│  Markdown Output (no bold/italic)                                       │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### Clippy Warnings Analysis

**Total warnings in edgequake-pdf**: 14

| File                       | Warning           | Severity |
| -------------------------- | ----------------- | -------- |
| `bin/diagnose_fonts.rs:80` | collapsible_match | Low      |
| `bin/test_decode.rs:53`    | collapsible_match | Low      |
| `bin/debug_page1.rs:5`     | unused import     | Low      |
| `bin/debug_page1.rs:85`    | dead_code         | Low      |
| `bin/trace_content.rs:70`  | unused_variables  | Low      |
| `bin/trace_content.rs:124` | unused_mut        | Low      |

Most warnings are in binary tools, not core library code.

### Feature Flags

```toml
# Cargo.toml features
[features]
default = ["pdfium", "lopdf"]
pdfium = ["pdfium-render"]        # Recommended backend
lopdf = ["dep:lopdf"]             # Legacy backend (deprecated)
vision = []                       # Vision-based extraction
```

### Key Findings

1. **Two distinct pipelines exist**: PdfiumBackend uses `pdfium-render` directly, ExtractionEngine uses `lopdf`
2. **Font style detection differs**:
   - Pdfium: `font_is_italic()`, `font_weight()` - API-based, accurate
   - Lopdf: FontDescriptor flags + name patterns - manual parsing, less reliable
3. **Style propagation differs**:
   - Pdfium: Styles flow through `Span.flags` to `pymupdf_renderer.rs`
   - Lopdf: Styles stored in `TextElement` but lost when converting to `schema::Block`
4. **Deprecation in progress**: ExtractionEngine marked deprecated since v0.2.0
5. **Binary tools have clippy warnings**: Not critical but should be fixed
6. **DRY violations**: Font detection logic exists in both `font_handling.rs` and `pdfium.rs`

### Files Examined

| File                           | Lines | Purpose              |
| ------------------------------ | ----- | -------------------- |
| `backend/mod.rs`               | 167   | PdfBackend trait     |
| `backend/pdfium_backend.rs`    | 475   | PdfiumBackend impl   |
| `backend/pdfium.rs`            | 306   | PDFium extractor     |
| `backend/extraction_engine.rs` | 1347  | Legacy lopdf backend |
| `backend/font_handling.rs`     | 617   | Font parsing         |
| `lib.rs`                       | 141   | Public API           |

---

_Iteration 01 - Observe complete_
_Next: Orient - Analyze findings and define solutions_
