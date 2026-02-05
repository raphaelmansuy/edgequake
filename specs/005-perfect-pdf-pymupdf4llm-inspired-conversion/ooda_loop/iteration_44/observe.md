# OODA-44: Observe - lopdf Modules for Deprecation

## Date: 2026-02-05

## Observation

The lopdf-based modules are now LEGACY. With PdfiumBackend as the preferred backend, we should mark lopdf modules as deprecated to guide future development toward the pdfium pipeline.

---

## Legacy Modules (lopdf feature)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                        lopdf Module Inventory                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  extraction_engine.rs  (1302 lines)  - Main lopdf backend                   │
│  block_builder.rs      (344 lines)   - Block construction                   │
│  column_detection.rs   (520 lines)   - Column layout detection              │
│  content_parser.rs     (800+ lines)  - PDF content stream parsing           │
│  element_processing.rs (400+ lines)  - Text element processing              │
│  encodings.rs          (800+ lines)  - PDF text encodings                   │
│  font_handling.rs      (600+ lines)  - Font metrics and analysis            │
│  glyph_list.rs         (1500+ lines) - Adobe glyph name→unicode mapping     │
│  lattice.rs            (550 lines)   - Table structure detection            │
│  text_grouping.rs      (800+ lines)  - Text grouping logic                  │
│  truetype_cmap.rs      (300+ lines)  - TrueType font cmap parsing           │
│                                                                             │
│  TOTAL: ~8,379 lines of legacy code                                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Modules to KEEP (shared between pipelines)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Shared Modules (no deprecation)                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  elements.rs    - RawChar struct used by BOTH pipelines                     │
│  spatial.rs     - LineSpatialIndex used for layout analysis                 │
│  mock.rs        - MockBackend for testing                                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Deprecation Strategy

1. **Add `#[deprecated]` attributes** to lopdf module items
2. **Keep modules compilable** - don't break existing code
3. **Add migration notes** pointing to pdfium equivalents
4. **Future removal** - Schedule for v0.3.0 after testing

---

## ExtractionEngine Analysis

The main entry point `ExtractionEngine` should be deprecated with a clear message:

```rust
#[deprecated(
    since = "0.2.0",
    note = "Use PdfiumBackend instead for more accurate font style detection. \
            ExtractionEngine relies on font name pattern matching which is unreliable. \
            PdfiumBackend uses PDFium's font descriptor flags for accurate bold/italic detection."
)]
pub struct ExtractionEngine { ... }
```

---

## Impact Assessment

- **Breaking change**: No (lopdf still compiles and works)
- **Warnings in builds**: Yes (deprecation warnings)
- **User migration**: Point to PdfiumBackend
- **API stability**: Maintained through PdfBackend trait
