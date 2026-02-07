# OODA Iteration 08 – Orient

**Date:** 2026-02-07

## Analysis

### Timing Fix

Debug builds are 5-10x slower than release. Parallel test execution adds further contention from pdfium library loading + IO. The original thresholds (500ms-3s) were designed for release builds. Debug parallel execution shows ~40s for multi-page PDFs, so 60s is a safe threshold.

### Footnote Integration

Two changes needed:

1. **Structural**: Add `Footnote` to `BlockType`, integrate into renderer match arm
2. **Classification**: Add footnote detection step to `classify_block()` using the existing `footnote.rs` detection logic

Risk: Footnote detection uses `y1` coordinate in page-bottom check. The `y1` in the layout `Block` struct uses PDF coordinates where smaller y = bottom of page. This aligns with `is_footnote()` checking `block.y1 > bottom_threshold` (low y1 = bottom of page).

### Page Height Dependency

Footnote detection requires `page_height` which the current `classify_block(block, body_font_size)` doesn't receive. Solution: Add `page_height` parameter with backward-compatible wrapper.
