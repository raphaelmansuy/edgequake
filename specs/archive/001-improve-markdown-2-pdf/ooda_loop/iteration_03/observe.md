# OODA Iteration 03: Observe

**Date**: 2026-02-06
**Mission File Re-read**: Confirmed - specs/001-improve-markdown-2-pdf.md

## Observations

1. **BlockType::ListItem is flat**: No nesting level encoded. All list items render at the same indentation regardless of their position in the document.
2. **Usage scope**: `BlockType::ListItem` is referenced in 30+ places across grouper, renderer, and test files. Changing the enum variant (e.g., `ListItem(u8)`) would be invasive.
3. **pymupdf4llm reference**: `document_layout.py:97-151` implements `create_list_item_levels()` which returns a separate dictionary `{index: level}` keyed by block index — hierarchy is computed externally, not stored in the block type.
4. **Algorithm principle**: The `x0` coordinate (left edge) of each list item determines its nesting depth. Items indented further right are deeper in the hierarchy.
5. **Contiguous segments matter**: pymupdf4llm groups consecutive list items into segments, breaking on non-list blocks or column changes. Levels are assigned independently per segment.
6. **Threshold**: A 10pt difference in `x0` between sorted items triggers a level increase.

## Key Files Examined

- `src/layout/pymupdf_structs.rs:65-94` — BlockType enum, ListItem has no level
- `src/layout/pymupdf_renderer.rs:199-288` — render_list_item() uses flat "- " prefix
- `src/layout/pymupdf_grouper.rs:1-50` — Block grouping, no hierarchy pass
- `zz-explore/pymupdf4llm/.../document_layout.py:97-151` — Gold standard algorithm
