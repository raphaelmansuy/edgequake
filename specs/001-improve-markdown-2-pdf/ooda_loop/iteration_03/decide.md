# OODA Iteration 03: Decide

**Date**: 2026-02-06
**Mission Re-read**: Confirmed

## Decisions

1. **New file**: Create `src/layout/list_hierarchy.rs` with `compute_list_levels(blocks: &[Block]) -> HashMap<usize, u8>`.
2. **Algorithm**:
   - Scan blocks sequentially, collecting contiguous `BlockType::ListItem` segments (break on non-list or column change).
   - Within each segment, sort by `x0` coordinate.
   - Assign level 0 to the first item. If next item's `x0 > prev_x0 + 10.0`, increment level; otherwise keep same level.
   - Return map of `{block_index: level}`.
3. **Integration**: In `pymupdf_renderer.rs`, call `compute_list_levels()` at the start of `render()`. Look up each list item's level from the map.
4. **Indentation**: `render_list_item()` receives `level: u8`. Indent = `"  "` repeated `level` times (2-space indent per level), then `"- "` prefix.
5. **Default**: Blocks not in the map default to level 0 (flat, preserving current behavior).
6. **Export**: Add `pub mod list_hierarchy` to `src/layout/mod.rs`.
7. **Tests**: 7 unit tests — single item, flat list, nested 2-level, nested 3-level, segment break on paragraph, column break, empty input.
