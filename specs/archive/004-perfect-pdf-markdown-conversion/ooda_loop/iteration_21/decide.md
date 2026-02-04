# OODA-21 Decide: Implement Paragraph Detection

## Date: 2025-02-03

## Decision

Implement paragraph detection in `TableDetectionProcessor` to exclude long text blocks from table candidates.

## Rationale

**First Principles Analysis:**

1. Tables have SHORT cells with DATA, not prose
2. Paragraphs span wide portions of page width
3. Paragraphs have many characters (>60)
4. Marking paragraphs as table rows destroys reading order and structure

**Markitdown Reference:**

```python
is_paragraph = line_width > page_width * 0.55 and len(combined_text) > 60
```

## Specific Changes

### 1. Add `is_paragraph()` function to `table_detection.rs`

```rust
/// Detect if a block is a paragraph (not a table cell).
///
/// WHY: Tables contain short data cells, not prose.
/// Markitdown uses: width > 55% page AND chars > 60.
///
/// First Principles:
/// - Tables have columnar structure with short cell content
/// - Paragraphs span significant page width with flowing text
/// - 55% threshold: Columns are typically 40-45% of page width
/// - 60 char threshold: Table cells rarely exceed 60 characters
fn is_paragraph(block: &Block, page_width: f32) -> bool {
    let block_width = block.bbox.x2 - block.bbox.x1;
    let text_len = block.text.chars().count();

    block_width > page_width * 0.55 && text_len > 60
}
```

### 2. Modify `find_table_extent()` to skip paragraphs

Before adding a row to table extent, check if any block in the row is a paragraph.
If so, stop the table extent.

### 3. Modify `detect_tables()` to validate table candidates

Before creating a table, validate that cells are appropriately sized.

## Expected Impact

| Metric              | Before | After (Expected) | Change |
| ------------------- | ------ | ---------------- | ------ |
| Structural Fidelity | 80.3%  | 84-86%           | +4-6%  |
| Text Preservation   | 81.3%  | 81-82%           | ~0%    |
| Overall             | 80.8%  | 83-85%           | +3-4%  |

## Test Plan

1. Run smoke tests: `cargo test --package edgequake-pdf --test quick_smoke`
2. Run feature tests: `cargo test --package edgequake-pdf --test basic_features --features slow-tests`
3. Run comprehensive tests: `cargo test --package edgequake-pdf --test comprehensive_quality --features comprehensive-tests`
4. Compare quality metrics before/after

## Rollback Plan

If quality degrades, revert the changes with:

```bash
git revert HEAD
```

## Commit Message

```
OODA-21: Add paragraph detection to table processor

WHY: Tables should contain short data cells, not prose paragraphs.
Blocks with width > 55% page AND > 60 chars are paragraphs.

WHAT: Added is_paragraph() check to find_table_extent() to stop
table detection when encountering paragraphs.

EVIDENCE: Markitdown uses same threshold (55%/60 chars).

EXPECTED: +4-6% structural fidelity improvement.
```
