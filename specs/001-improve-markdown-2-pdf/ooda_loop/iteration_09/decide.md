# OODA Iteration 09 – Decide

**Date:** 2026-02-07

## Decisions

1. **Add `classify_blocks_page_aware()`** to `TextGrouper` that estimates page_height per page from block coordinates.

2. **Update pipeline** to call `classify_blocks_page_aware()` instead of `classify_blocks()`.

3. **Remove dead code**: Delete `style_text()` and `get_style_type()` to eliminate clippy warnings.

4. **No new tests needed**: Existing footnote tests cover the classification; pipeline integration is tested via fast_quality.
