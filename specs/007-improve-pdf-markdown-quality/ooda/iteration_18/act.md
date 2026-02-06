# OODA Iteration 18 - Act

## Changes Made

### 1. Cross-Empty-Line Join Length Guard

**File:** `src/renderers/markdown.rs` line ~1328
**Change:** Added `MAX_FRAGMENT_LINE_LEN` (30 chars) guard to cross-empty-line join logic.

Before: Any two lines matching `should_join_lines()` were joined across `\n\n` paragraph breaks.
After: Only lines ≤30 chars can be joined across `\n\n` breaks.

**WHY:** `render_text()` adds `\n\n` between blocks. Long lines represent complete
sentences whose paragraph boundaries must be preserved. Only short lines are word
fragments from narrow PDF text boxes that genuinely split a word across blocks.

### 2. Column Alignment Check in Paragraph Continuation

**File:** `src/renderers/markdown.rs` line ~393
**Change:** Added `MAX_COLUMN_X_DRIFT` (50pt) check comparing `prev.bbox.x1` vs `curr.bbox.x1`.

Before: Blocks from different columns (x1=318 vs x1=78) were joined as paragraph continuations.
After: Blocks with >50pt left margin difference are rejected as continuations.

**WHY:** After multi-column layout reordering, blocks from different columns become
adjacent in the render list. They have different X positions and must NOT be merged
as paragraph continuations—they belong to different text flows.

### 3. New Tests (6 added)

- `test_join_broken_lines_no_cross_paragraph_join_long_line` - Long lines don't join across `\n\n`
- `test_join_broken_lines_short_fragment_cross_empty` - Short fragments DO still join
- `test_join_broken_lines_medium_fragment_cross_empty` - Medium fragments still join
- `test_join_broken_lines_preserves_paragraph_structure` - Realistic multi-block paragraphs preserved
- `test_paragraph_continuation_different_columns` - Right→Left column blocks NOT joined
- `test_paragraph_continuation_same_column` - Same-column blocks still joined

## Test Results

```
test result: ok. 562 passed; 0 failed; 0 ignored; 0 measured
```

## Quality Improvement Evidence

### Two-Column Test PDF (003_two_columns.pdf)

**Before (IT17):**

```
...interleave the lines with SOTA extraction requires understanding the some space and tests the vertical flow.spatial layout of the page.
```

**After (IT18):**

```
...interleave the lines with SOTA extraction requires understanding the
some space and tests the vertical flow.
spatial layout of the page.
```

Blocks from different columns now correctly separated with line breaks.

### Elitizon PDF

**Before:** "readiness." was concatenated with previous paragraph
**After:** "readiness." on separate line, properly separated from different-column content

## Commit

```
OODA-IT18: Cross-paragraph join guard and column alignment check
```
