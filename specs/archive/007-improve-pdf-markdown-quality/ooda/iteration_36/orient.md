# IT36 — Orient: Four-Layer Fix

## Architecture

```text
                    IT36 Fix Points
                    ═══════════════

  Layer 1: Line.starts_with_list_marker()  ← NEW method on Line
           │
  Layer 2: Block.can_add_line()  ── rejects bullet-starting lines
           │
  Layer 3: join_blocks_phase2()  ── skips merge when next block starts with bullet
           │
  Layer 4: ListDetectionProcessor ── accepts Paragraph blocks (not just Text)
           │
  Layer 5: render_list_item()    ── renders spans (preserves bold/italic) after prefix skip
```

## Key Decisions

1. **Bullet detection at Line level** (not just processor level):
   By adding `starts_with_list_marker()` to `Line`, both `can_add_line` and
   `join_blocks_phase2` can prevent merging without circular dependencies.

2. **Subset of bullet characters** (not full 530+ set):
   `starts_with_list_marker()` uses ~30 common bullets. The full set is only
   needed in `ListDetectionProcessor` for classification. Block splitting only
   needs common ones (•, ◦, ▪, etc.) to avoid false positives.

3. **Content-hash image naming**:
   Spec requires idempotent naming. Using `DefaultHasher` on pixel data for
   `img_{:016x}.png` format. Skip saving if file exists (caching).

## Compatibility

- `BlockType::Text` (lopdf) and `BlockType::Paragraph` (pdfium) both accepted
- No changes to schema or block types — purely behavioral fix
