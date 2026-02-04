# OODA-40: Adopt pymupdf4llm Gold Standard

## Overview

This iteration establishes pymupdf4llm as the reference gold standard generator
for PDF-to-Markdown quality evaluation, replacing hand-crafted gold files.

## Key Changes

1. **New gold standard source**: pymupdf4llm v0.2.9
2. **Baseline F1**: 0.686 average across 7 test documents
3. **Target F1**: ≥0.90 to claim production quality
4. **Scripts added**:
   - `scripts/generate_gold_pymupdf4llm.py` - Generate gold files
   - `scripts/compare_against_pymupdf.py` - Evaluate against gold

## Why pymupdf4llm?

1. **Production-proven**: Used by major RAG pipelines
2. **Pure extraction**: No ML models, deterministic
3. **Faithful to PDF structure**: Preserves physical layout
4. **Multi-column handling**: Correct reading order algorithm

## Quality Gaps Identified

| Issue                   | Impact | Root Cause                            | Status   |
| ----------------------- | ------ | ------------------------------------- | -------- |
| Author name spacing     | Medium | merge_line() word gap detection       | ✅ FIXED |
| Two-column interleaving | High   | Column separation in text_grouping.rs | Pending  |
| Missing content         | High   | Over-aggressive footer detection      | Pending  |
| Section header levels   | Low    | BlockType classification              | Pending  |

## Fix Applied: Author Name Spacing (OODA-40a)

### Problem
Author names like "Zhaoxi ZhangYitong DuanYanzhi Zhang" were merged without spaces
because `merge_line()` was calculating word gap as:
```rust
let spacing = elem.x - prev.x;  // Wrong: start-to-start distance
```

### Solution
Added `width` field to `TextElement` and updated gap calculation:
```rust
// Now calculates actual visual gap between elements
let gap = elem.x - (prev.x + prev.width);
```

### Files Modified
- `src/backend/elements.rs` - Added `width: f32` field
- `src/backend/content_parser.rs` - Populate width from font size estimate
- `src/backend/text_grouping.rs` - Use gap calculation instead of spacing
- `src/backend/*_tests` - Updated test helper functions

### Results After Fix
Author names now properly spaced:
- Before: "Alexander NovikovNgân VuMarvin Eisenberger"
- After: "Alexander Novikov, Ngân Vu, Marvin Eisenberger"

F1 score remains at 0.686 (main issue is two-column interleaving, not spacing).

## Next Steps

1. OODA-41: Fix two-column interleaving by adopting pymupdf4llm's bbox-based approach
2. OODA-42: Tune footer/header detection thresholds
3. OODA-43: Improve column boundary detection

## Baseline Scores (per document)

| Document               | F1    | Notes                             |
| ---------------------- | ----- | --------------------------------- |
| agent_2510.09244v1     | 0.814 | Best score, simple layout         |
| ccn_2512.21804v1       | 0.807 | Technical paper, good extraction  |
| 2900_Goyal_et_al       | 0.722 | Academic paper                    |
| v2_2512.25072v1        | 0.689 | Two-column issues                 |
| AlphaEvolve            | 0.620 | Complex layout, column interleave |
| one_tool_2512.20957v2  | 0.596 | Multi-column degradation          |
| 01_2512.25075v1        | 0.552 | Worst: heavy two-column issues    |
