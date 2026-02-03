# OODA-23 Act: Cross-Column Hyphenation Fix

## Implementation Summary

### Changes Made

1. **layout_processing.rs** - Added `merge_cross_column_hyphenation()` function:
   - Post-processing step in `merge_page_blocks()` for multi-column layouts
   - Finds blocks ending with hyphen in column N
   - Finds blocks starting with lowercase in column N+1
   - Validates linguistic plausibility (rejects "reposito" + "tory" = "repositotory")
   - Merges pairs that form valid word continuations

2. **block.rs** - Extended fragment ending detection:
   - Added "to", "ro", "po" to fragment endings list
   - WHY: "reposito-ries" should be treated as continuation, not compound
   - Prevents "reposito-" from being treated as compound word prefix

### Algorithm Details

The cross-column hyphenation fix works in two phases:

**Phase 1: Block Merge Post-Processing (layout_processing.rs)**

- Groups blocks by column
- For each column pair (N, N+1):
  - Find blocks in column N ending with "-"
  - Find potential continuations in column N+1 (starts with lowercase)
  - Validate: reject if fragment + continuation creates repeated syllables
  - Merge valid pairs using Block.merge()

**Phase 2: Hyphenation Handling (block.rs)**

- Block.merge() detects hyphenation pattern (ends with "-", next starts lowercase)
- Distinguishes compound words ("long-horizon") from continuations ("reposito-ries")
- Extended `is_fragment_ending` to include common Latin suffixes: ti, ni, fi, si, gi, vi, ci, to, ro, po

### Results

Before fix:
```
22:...reposito-
24:Figure 1.Illustration...
28:ries remains limited
```

After fix:
```
20:...repositories remains limited...
```

### Validation

- Smoke tests: PASS
- Specific PDF (one_tool_2512.20957v2.pdf): hyphenation correctly resolved
