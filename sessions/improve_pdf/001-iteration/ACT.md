# Iteration 001 - ACT Phase

## Action Taken

**Problem Fixed**: Reading order inversion in PDF → Markdown conversion

## Root Cause

The `ReadingOrderDetector` in `src/layout/reading_order.rs` was sorting blocks by **ascending Y** coordinate, which in PDF coordinates (where Y=0 is at BOTTOM of page) resulted in content appearing **bottom-to-top** instead of top-to-bottom.

### Files Modified

1. **`src/layout/reading_order.rs`**
   - `single_column_order()`: Changed Y comparison from `y_a.partial_cmp(&y_b)` to `y_b.partial_cmp(&y_a)` (descending Y)
   - `sort_by_position()`: Same fix - changed to descending Y sort
   - `merge_column_orders()`: Fixed Y comparisons for spanning elements (use `>` instead of `<`)
   - Updated 3 unit tests to use correct PDF coordinate expectations

## Code Changes Summary

```rust
// BEFORE (wrong - ascending Y = bottom-to-top):
y_a.partial_cmp(&y_b).unwrap()

// AFTER (correct - descending Y = top-to-bottom):
y_b.partial_cmp(&y_a).unwrap()
```

## Test Results

- **102 unit tests**: ✅ All passing
- **Integration tests**: ✅ All passing
- **Total**: 171 tests passing

## Validation Results

### Simple Test Suite

- Composite Score: **92.7/100** (unchanged - simple tests unaffected)
- Table Accuracy: 100%
- Style Accuracy: 84.3%

### Real Dataset Evaluation

- Reading order now **CORRECT** - content flows top-to-bottom
- Before: Output started with footnotes at bottom of page
- After: Output starts with title at top of page

### Sample Output (2900_Goyal_et_al.pdf)

**BEFORE** (inverted):

```
1Link of code
...footnote content...
```

**AFTER** (correct):

```
LLMs4OL 2025: The 2nd Large Language Models for Ontology Learning Challenge...
### Clustering-Based Ontology Learning Using LLMs
Pankaj Kumar Goyal...
```

## Remaining Issues for Next Iteration

1. **Ligature handling**: "specifc" instead of "specific" (fi ligature not decoded)
2. **Word spacing**: Some concatenated words like "silpnlp" instead of "silp_nlp"
3. **Style detection**: 84.3% accuracy - room for improvement
4. **Two-column interleaving**: Some content may still interleave incorrectly

## Next Steps

Iteration 002 will focus on ligature decoding in `cmap_parser.rs` or `font_encoding.rs`.
