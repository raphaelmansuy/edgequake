# OODA-13 Observation: Cross-Column Reference Interleaving

## Date: 2025-01-27

## Focus: v2 PDF Structure Score Stuck at 53.6%

## Key Finding

The REFERENCES section (pages 9-10) has **44 references in two columns**, but only **5 list items** are detected in the generated markdown. References from left and right columns are being **interleaved and merged** at the line/block level.

## Evidence

### Gold File Analysis

```
Path: test-data/real_dataset/v2_2512.25072v1.gold.md
Total reference list items: 44
Format: "* [N] Author, Title, Venue, Year."
- Left column: [1]-[22]
- Right column: [23]-[44]
```

### Generated Output Analysis

```
Path: /tmp/v2_extracted.md
Total reference list items: 5
Detected items: [1], [3], [8], [13], [36]
```

### Debug Logging Results

Only 17 reference elements detected at column separation:

```
REF-LEFT: Y=327.2 X=58.0 boundary=320.0 text='[1] L. Sentis...'
REF-LEFT: Y=345.3 X=58.0 boundary=320.0 text='[2] X. Cheng...'
...
REF-LEFT: Y=606.6 X=54.0 boundary=320.0 text='[12] K. Black...'
```

**ZERO** `REF-RIGHT` elements detected - all 17 refs classified as LEFT.

Page 10 refs [39]-[44] appear at Y=17-179 with X=54, boundary=300.

## Interleaving Evidence in Generated Markdown

```markdown
- [1] L. Sentis and O. Khatib, "A whole-body control framework for
  humanoids operating in human environments," in ICRA., 2006. Teleoperation with immersive active visual feedback," in CoRL, 2024.
```

Note: `[1]` ends mid-sentence, then `[2]` content continues without list marker.

Later in file:

```markdown
- [8] E. Hsieh, W.-H. Hsieh, Y.-J. Wang, T. Lin, J. Malik, K. Sreenath,
  and H. Qi, "Learning dexterous manipulation skills from imperfect - [31] J. Li, X. Cheng, T. Huang, S. Yang...
  simulations," arXiv:2512.02011, 2025.
```

Note: `[31]` (right column) appears **between** `[8]` continuation lines (left column), with large leading whitespace.

## Root Cause Hypothesis

1. **Column Detection Works**: Pages correctly detected as 2-column (OODA-12 messages confirm)
2. **Column Separation Works**: Elements correctly classified as left (X < boundary) or right (X > boundary)
3. **Problem is AFTER separation**: In `group_two_column_layout()`:

   ```rust
   // Column separation happens here
   left_column.push(elem);  // or right_column.push(elem);
   ...
   // Then columns are re-combined:
   let (left_main, left_footer) = Self::group_single_column_layout(...);
   let (right_main, right_footer) = Self::group_single_column_layout(...);
   (left_main + right_main, left_footer + right_footer)  // <-- CONCATENATION
   ```

4. **Y-Band Grouping Merges Across Boundaries**: When `left_main + right_main` is combined, subsequent block building still groups by Y proximity, causing elements from different columns at similar Y positions to merge.

## Missing References Analysis

- Refs [1]-[12]: 12 detected in left column
- Refs [13]-[22]: MISSING - should be in left column but not detected
- Refs [23]-[38]: MISSING - should be in right column but no REF-RIGHT logs
- Refs [39]-[44]: 5 detected in left column (page 10)

Total: 17 detected / 44 expected = 38.6% detection rate

## Files Involved

- `src/backend/text_grouping.rs:278-350` - Column separation logic
- `src/backend/text_grouping.rs:350-400` - group_single_column_layout Y-band grouping
- `src/backend/block_builder.rs` - Block construction from lines
- `src/backend/extraction_engine.rs:630-656` - OODA-12 Y-sort skip (working)

## Next Steps (Orient Phase)

1. Trace why refs [13]-[38] are not being detected at all
2. Check if column boundary (320) is too high - some refs may fall in gap
3. Verify the block builder preserves column separation
4. Consider marking elements with column ID for downstream processing
