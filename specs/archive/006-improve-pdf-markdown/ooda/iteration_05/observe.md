# Iteration 05: Observe

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Focus: Documentation and WHY Comments Audit

### Current WHY Comment Coverage

```text
File                          WHY Count
─────────────────────────────────────────
backend/extraction_engine.rs  18
backend/text_grouping.rs      11
layout/pymupdf_grouper.rs     18
backend/lattice.rs            12
layout/pymupdf_structs.rs     6
backend/element_processing.rs 5
backend/spatial.rs            5
layout/block_classifier.rs    5
backend/pdfium.rs             4
...
```

### Files with Low WHY Coverage

1. **`extractor.rs`** (1 WHY comment, 856 lines)
2. **`config.rs`** (1 WHY comment, 803 lines)
3. **`bin.rs`** (1 WHY comment)
4. **`vision.rs`** (1 WHY comment)

### Areas Needing Documentation

1. **Font Style Data Flow** - Need ASCII diagram showing:
   - PDFium → RawChar → Span → Markdown
   - What each component contributes

2. **Span Merging Algorithm** - Key decisions:
   - Why font size tolerance of 0.5?
   - Why y-tolerance of 0.3 \* font_size?
   - Why space_threshold of 0.25 \* font_size?

3. **Backend Selection** - Already documented in mod.rs, but could add more context
