# IT36 — Decide: Implementation Plan

1. Add `Line::starts_with_list_marker()` in `pymupdf_structs.rs`
   - Check common bullet characters (•, ◦, ▪, ▸, ►, ●, ○, ■, □, etc.)
   - Check numbered patterns ("1. ", "2) ")
   - Validate: bullet must be followed by space/end/uppercase/asterisk

2. Modify `Block::can_add_line()` in `pymupdf_structs.rs`
   - Reject lines where `starts_with_list_marker()` returns true
   - Force each bullet item into its own block

3. Modify `join_blocks_phase2()` in `pymupdf_grouper.rs`
   - Skip merge when next block's first line starts with a list marker
   - Prevents re-merging after initial split

4. Fix `ListDetectionProcessor` in `structure_detection.rs`
   - Accept `BlockType::Paragraph` in addition to `BlockType::Text`
   - Include Paragraph blocks in min_x calculation

5. Fix `render_list_item()` in `markdown.rs`
   - Skip bullet prefix spans, render remaining with formatting
   - Preserves bold/italic from span styling

6. Fix image naming in `bin.rs`
   - Use `DefaultHasher` on image pixel data → `img_{:016x}.png`
   - Skip saving if file already exists (idempotent)
