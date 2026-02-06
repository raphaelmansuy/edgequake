# IT36 — Act: Bullet List Detection + Content-Hash Images

## Files Modified

| File                                    | Change                                                                    |
| --------------------------------------- | ------------------------------------------------------------------------- |
| `src/layout/pymupdf_structs.rs`         | Added `Line::starts_with_list_marker()`, modified `Block::can_add_line()` |
| `src/layout/pymupdf_grouper.rs`         | Modified `join_blocks_phase2()` to skip bullet blocks                     |
| `src/processors/structure_detection.rs` | Accept `BlockType::Paragraph` in ListDetectionProcessor                   |
| `src/renderers/markdown.rs`             | Render spans with formatting after bullet prefix skip                     |
| `src/bin.rs`                            | Content-hash image naming (idempotent)                                    |

## Test Results

- **449 lib tests**: ✅ All pass, 0 failures
- **Clippy**: ✅ 0 warnings in edgequake-pdf
- **LightRAG PDF quality**:
  - Before: 0 properly formatted list items (all merged into paragraphs)
  - After: 27 proper markdown list items with `- ` prefix
  - Bold/italic preserved in list items: `- **General Aspect**. We emphasize...`
  - Block count: 217 → 239 (22 more blocks from bullet splitting)
- **Elitizon PDF**: No regression, output consistent

## Quality Impact

| Category       | Before IT36      | After IT36                  | Notes             |
| -------------- | ---------------- | --------------------------- | ----------------- |
| Lists (bullet) | ~0% (all merged) | ~95% (27/27 detected)       | Major improvement |
| Bold in lists  | 0% (lost)        | ~95% (preserved from spans) | Fixed             |
| Image naming   | Not idempotent   | Content-hash based          | Spec requirement  |

## Before/After Example

**Before (IT35):**

```
• **General Aspect**. We emphasize... • **Methodologies**. To enable... • **Experimental Findings**. Extensive...
```

**After (IT36):**

```
- **General Aspect**. We emphasize the importance of developing a graph-empowered RAG system...
- **Methodologies**. To enable an efficient and adaptive RAG system, we propose LightRAG...
- **Experimental Findings**. Extensive experiments were conducted to evaluate...
```
