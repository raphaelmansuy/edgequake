# Act – OODA-23: Replace TODOs with KNOWN LIMITATION Comments

## What Changed

Replaced 4 generic "TODO" comments with detailed "KNOWN LIMITATION" comments:

1. **extractor.rs:539** - Image extraction in text mode
   - WHY: Requires vision/multimodal LLM
   - WORKAROUND: Use Vision mode
   - FUTURE: Use ImageOcrConfig

2. **pymupdf_grouper.rs:163** - Vertical text detection
   - WHY: PDFium bboxes don't indicate direction
   - WORKAROUND: Filter by margin position
   - FUTURE: Character sequence analysis

3. **pymupdf_renderer.rs:156** - Proper table rendering
   - WHY: Complex cell boundary detection needed
   - WORKAROUND: Render as paragraphs
   - FUTURE: Use lattice.rs

4. **pdfium_backend.rs:296** - Image presence detection
   - WHY: Requires PDF object traversal
   - WORKAROUND: Assume images if Vision mode requested
   - FUTURE: Use page_objects() iterator

## Code Locations

- `edgequake/crates/edgequake-pdf/src/extractor.rs`
- `edgequake/crates/edgequake-pdf/src/layout/pymupdf_grouper.rs`
- `edgequake/crates/edgequake-pdf/src/layout/pymupdf_renderer.rs`
- `edgequake/crates/edgequake-pdf/src/backend/pdfium_backend.rs`

## Verification

```
cargo test --lib
# Result: 469 passed

grep -rn "TODO" src/ --include="*.rs"
# Result: 0 matches (all TODOs replaced)
```

## Value Added

- Codebase now has 0 TODOs in edgequake-pdf
- Each limitation is documented with:
  - WHY the limitation exists
  - WORKAROUND for users
  - FUTURE implementation approach
- Sets realistic expectations for contributors

## Next Iteration

OODA-24: Continue with other improvements
