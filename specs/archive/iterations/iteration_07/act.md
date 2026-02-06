# OODA-07 Act: Author Line Fragment Rescue

## Changes Implemented

### File: `edgequake/crates/edgequake-pdf/src/backend/text_grouping.rs`

**Change**: Added OODA-07 rescue logic after the main element classification loop.

**Problem**: Author names that span across the column boundary get fragmented:

- `Haozhi Qi, Yen-Jen Wang, ... Koushil Sreenath ∗` at X=96 (left column)
- `, Jitendra Malik †` at X=433 (right column)

Both have Y ≈ 27-31 (same visual line), but the second fragment is classified as right column because X > column_boundary (300). This causes `, Jitendra Malik` to appear in the middle of body text instead of with other authors.

**Solution**: Post-classification rescue for elements that meet ALL criteria:

1. There ARE spanning elements (indicating a title page)
2. Element's Y > 15 (authors are below title, not at Y=0 where body text starts)
3. Element's Y < 60 (authors are near top, not in body)
4. There's a left_column element at same Y (±font_size tolerance)
5. Element looks like continuation (starts with comma, or contains † ∗ symbols)

**WHY these constraints**:

- Y > 15: Titles are at Y≈0, body text can also start at Y=0
- Y < 60: Authors don't go past Y=60 typically
- Same-Y check: Author fragments must be on the same visual line
- Continuation check: `, Jitendra Malik` starts with comma

### Code Added (lines 390-451)

```rust
// OODA-07: Only rescue elements that meet ALL of these criteria...
for elem in right_column.drain(..) {
    let should_rescue = if !spanning_elements.is_empty() {
        let in_author_zone = elem.y > 15.0 && elem.y < 60.0;

        if in_author_zone {
            let y_tolerance = elem.font_size.max(5.0);
            let has_left_sibling = left_column.iter().any(|left| {
                (left.y - elem.y).abs() < y_tolerance
                    && left.y > 15.0
                    && left.y < 60.0
            });

            let looks_like_continuation = elem.text.trim().starts_with(',')
                || elem.text.contains('†')
                || elem.text.contains('∗')
                || (elem.text.len() < 30 && elem.y > 20.0);

            has_left_sibling && looks_like_continuation
        } else {
            false
        }
    } else {
        false
    };

    if should_rescue {
        rescued_to_left.push(elem);
    } else {
        keep_in_right.push(elem);
    }
}
```

## Test Results

### Passing Tests

- `test_arxiv_paper_extraction`: PASS (two-column reading order preserved)
- All other fast_quality tests: PASS

### Pre-existing Failure (not caused by OODA-07)

- `test_qwen_reading_order`: FAIL (word splitting issue: "Push ing" vs "Pushing")
  - This was failing before OODA-07 changes

### Specific Improvement Verified

**v2_2512.25072v1** author line now in correct order:

```
Before OODA-07:
Block 0: Title
Block 1: Authors (without Jitendra Malik)
...
Block 10: ", Jitendra Malik †"  ← WRONG: appears after INTRODUCTION

After OODA-07:
Block 0: Title
Block 1: Authors (without Jitendra Malik)
Block 2: ", Jitendra Malik †"  ← CORRECT: appears right after other authors
Block 3: UC Berkeley
```

## Quality Metrics

| Metric    | Before | After | Delta |
| --------- | ------ | ----- | ----- |
| Quality   | 0.724  | 0.724 | 0%    |
| ROUGE-L   | 0.700  | 0.700 | 0%    |
| Word F1   | 0.897  | 0.897 | 0%    |
| Structure | 0.602  | 0.602 | 0%    |
| Format    | 0.470  | 0.470 | 0%    |

**Note**: While the fix corrects specific author line ordering, it doesn't significantly move the aggregate metrics because:

1. Author lines are a small fraction of total content
2. The larger ROUGE-L gap is caused by other reading order issues
3. The fix is intentionally conservative to avoid breaking two-column body text

## Commit

Commit message: `OODA-07: Rescue author line fragments from right column to fix reading order`
