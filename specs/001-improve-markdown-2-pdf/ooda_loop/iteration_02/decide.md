# OODA Iteration 02: Decide

**Date**: 2026-02-06
**Mission Re-read**: Confirmed

## Decisions

1. Create `src/renderers/pua_filter.rs` with `is_pua_char()` and `filter_pua()`
2. Export from `src/renderers/mod.rs`
3. Integrate into `pymupdf_renderer.rs` at both render_line_styled and render_line_plain
4. Skip empty spans after PUA filtering (prevent blank output)
5. 13 unit tests covering all PUA ranges, boundaries, and edge cases
