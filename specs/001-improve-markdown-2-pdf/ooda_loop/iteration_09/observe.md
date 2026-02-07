# OODA Iteration 09 – Observe

**Date:** 2026-02-07
**Mission Re-read:** Confirmed (`specs/001-improve-markdown-2-pdf.md`)

## Observations

### 1. Footnote Detection Not Active in Pipeline

OODA-08 added footnote detection infrastructure but `classify_blocks()` passed `page_height=0.0`, disabling it in the actual pipeline. The `pymupdf_pipeline.rs` calls `grouper.classify_blocks()` without page context.

### 2. Dead Code Warnings

Clippy reports 2 warnings in edgequake-pdf:

- `style_text` method unused (superseded by `render_line_styled`)
- `get_style_type` function unused (superseded by `get_style_type_with_ref`)

### 3. Page Height Availability

The `RawChar` and `Block` structs contain y coordinates in PDF space (y=0 at bottom). Page height can be estimated from `max(block.y1) + margin` per page without requiring API changes.

### 4. Test Baseline: 507 unit + 7 integration = 514 passing
