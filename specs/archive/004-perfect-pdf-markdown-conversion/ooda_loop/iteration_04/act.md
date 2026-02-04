# OODA Iteration 04 - Act

## Implementation: Y Coordinate Normalization Fix

### Modified File

`edgequake/crates/edgequake-pdf/src/backend/extraction_engine.rs`
Lines: 386-398

### Change Applied

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
+     // Normal PDF coordinate system: lower Y = bottom of page (like a graph)
+     // To convert to document order (Y=0 at top), we flip: normalized_y = max_y - y
+     // This makes content at max_y (visual top of page) become Y=0
+     // WHY (OODA-04): Previously used `y - min_y` which kept Y=0 at bottom,
+     // causing reversed reading order (bottom content sorted first).
+     // All downstream sorting (text_grouping.rs, reading_order.rs) expects
+     // ascending Y = top-to-bottom document order.
+     elements
+         .into_iter()
+         .map(|mut e| {
+             e.y = max_y - e.y;
+             e
+         })
+         .collect()
+ }
```

### Commit

```
d5a30713 fix(pdf): correct Y normalization for non-flipped PDFs
```

### Test Results

All 8 quality tests pass:

```
test test_qwen_key_content ... ok
test test_qwen_reading_order ... ok
test test_beyond_transformer_content ... ok
test test_beyond_transformer_structure ... ok
test test_agentic_platform_code_blocks ... ok
test test_agentic_platform_content ... ok
test test_agentic_platform_headings ... ok
test test_all_pdfs_extraction_summary ... ok

test result: ok. 8 passed; 0 failed
```

### Validation Output

#### agentfail_2601.22984v1.pdf (BEFORE → AFTER)

**BEFORE**: Text from bottom of page appeared first
**AFTER**: Title appears correctly at top

```markdown
# Why Your Deep Research Agent Fails?

## On Hallucination Evaluation in Full Research Trajectory

Yuhao Zhan Tianyu Fan Linxuan Huang 1

### Abstract

Diagnosing the failure mechanisms of Deep Research Agents (DRAs) remains...
```

#### hotmess_2601.23045v1.pdf (AFTER)

Reading order now correct (introduction paragraphs follow abstract):

```markdown
Alexander Hagele ¨ ∗1,2 Aryo Pradipta Gema 1,3 Henry Sleight 4 Ethan Perez 5
...
There are an increasing number of predictions that AI will soon be more capable than human...
```

Note: Title "THE HOT MESS OF AI" is still missing - likely a header detection issue (separate from Y normalization).

#### Apple-Sandbox-Guide-v1.0.pdf (AFTER)

```markdown
Apple 's Sandbox Guide

v1.0

13 2011
```

### Remaining Issues

1. **Title Detection**: hotmess PDF title may be in a header/footer zone being filtered
2. **Text Concatenation**: "humanbeings" instead of "human beings" - space detection issue
3. **TOC Corruption**: Apple Sandbox Guide Page 2 still shows garbled characters ("55555...")

### Next Steps (OODA-05)

1. Investigate missing titles in hotmess PDF
2. Add tests for the 3 new PDFs
3. Fix text concatenation (word boundary detection)
