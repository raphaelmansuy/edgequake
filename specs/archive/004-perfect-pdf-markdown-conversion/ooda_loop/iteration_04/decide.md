# OODA Iteration 04 - Decide

## Decision: Fix Y Coordinate Normalization

### What

Modify `extraction_engine.rs` to normalize Y coordinates correctly for **all** PDFs, not just flipped ones.

### Where

File: `edgequake/crates/edgequake-pdf/src/backend/extraction_engine.rs`
Lines: 386-394

### Change

```diff
- } else {
-     // Normal coordinate system: lower Y = bottom of page
-     // Normalize by shifting: normalized_y = visual_y - min_y
-     // This makes content at min_y become Y=0 (but still bottom-first)
-     // Then text_grouping handles Y-sorting for reading order
-     elements
-         .into_iter()
-         .map(|mut e| {
-             e.y -= min_y;
-             e
-         })
-         .collect()
- }
+ } else {
+     // Normal PDF coordinate system: lower Y = bottom of page
+     // To convert to document order (Y=0 at top), we flip:
+     // normalized_y = max_y - visual_y
+     // This makes content at max_y (visual top) become Y=0
+     // WHY: All downstream sorting expects ascending Y = top-to-bottom
+     elements
+         .into_iter()
+         .map(|mut e| {
+             e.y = max_y - e.y;
+             e
+         })
+         .collect()
+ }
```

### Expected Impact

1. **hotmess\_\*.pdf**: Reading order corrected (abstract first, body later)
2. **agentfail\_\*.pdf**: Reading order corrected
3. **Apple-Sandbox-\*.pdf**: Reading order corrected
4. **Qwen.pdf**: Should still work (uses flipped=true branch)

### Validation Plan

1. Run existing quality tests: `cargo test -p edgequake-pdf quality_extraction`
2. Re-run conversion on 3 new test PDFs
3. Verify abstract appears before body in output
4. Compare with markitdown baseline

### Rollback

If regressions detected, revert the single line change.

### Commit Message

```
fix(pdf): correct Y normalization for non-flipped PDFs

OODA-04: Normal PDFs had reversed reading order because Y coordinates
were shifted (y - min_y) instead of flipped (max_y - y). This made
Y=0 at the bottom of the page, causing ascending Y sort to put
bottom content first.

Now both flipped and normal PDFs normalize to Y=0 at top, which
matches the assumption in text_grouping.rs and reading_order.rs.

Fixes reading order for:
- hotmess_2601.23045v1.pdf
- agentfail_2601.22984v1.pdf
- Apple-Sandbox-Guide-v1.0.pdf
```
