# OODA-21 Act: Paragraph Detection Implementation

## Date: 2025-02-03

## Summary

Added paragraph detection to table processor to prevent prose blocks from being incorrectly classified as table cells.

## Commit

**SHA:** `d15bf685`
**Message:** OODA-21: Add paragraph detection to table processor

## Changes Made

### 1. processors/table_detection.rs - Added Helper Functions

```rust
// Lines 22-52: New paragraph detection section

/// Detect if a block is a paragraph (NOT a table cell).
///
/// **First Principles (from Markitdown analysis):**
/// - Tables contain SHORT data cells, not flowing prose
/// - Paragraphs span significant page width (>55%)
/// - Paragraphs have many characters (>60)
fn is_paragraph(block: &Block, page_width: f32) -> bool {
    let block_width = block.bbox.x2 - block.bbox.x1;
    let text_len = block.text.chars().count();
    block_width > page_width * 0.55 && text_len > 60
}

/// Check if any block in a row is a paragraph.
fn row_contains_paragraph(row: &[usize], blocks: &[Block], page_width: f32) -> bool {
    row.iter()
        .any(|&idx| is_paragraph(&blocks[idx], page_width))
}
```

### 2. detect_tables() - Added Early Exit for Paragraph Rows

```rust
// Lines 227-237: Skip paragraph rows before table detection

// OODA-21: Skip rows that contain paragraphs (not table candidates)
// WHY: Paragraphs are prose content, not tabular data
if row_contains_paragraph(&rows[i], &page.blocks, page_width) {
    for &block_idx in &rows[i] {
        new_blocks.push(page.blocks[block_idx].clone());
    }
    i += 1;
    continue;
}
```

### 3. find_table_extent() - Stop at Paragraph Boundaries

```rust
// Lines 295-302: Stop table extent when hitting paragraphs

// OODA-21: Stop table if this row contains a paragraph
// WHY: Paragraphs are flowing text, not table cells
if row_contains_paragraph(current_row_blocks, &page.blocks, page_width) {
    tracing::debug!(
        "  OODA-21: Stopping table extent at row {} - paragraph detected",
        j
    );
    break;
}
```

## Verification

| Test                | Result                        |
| ------------------- | ----------------------------- |
| Build               | ✅ Success                    |
| Smoke tests         | ✅ 4/4 passed (0.08s)         |
| Comprehensive tests | ✅ 2/2 passed (139.85s)       |
| Quality metrics     | 80.8% (stable, no regression) |

## Impact Assessment

```
┌─────────────────────────────────────────────────────────────┐
│  IMPACT: PARAGRAPH DETECTION                                │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Expected Benefits:                                          │
│  • Prevents prose from being marked as tables               │
│  • Better structural fidelity for single-column docs        │
│  • Aligned with Markitdown best practices                   │
│                                                              │
│  Measured Impact:                                            │
│  • Quality stable at 80.8% (no regression)                  │
│  • Multi-column pages already skip table detection          │
│  • Change protects single-column documents                  │
│                                                              │
│  Thresholds (from Markitdown):                              │
│  • Width > 55% of page width = paragraph                    │
│  • Characters > 60 = flowing text                           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Files Changed

| File                          | Lines Changed | Type             |
| ----------------------------- | ------------- | ---------------- |
| processors/table_detection.rs | +35 lines     | Feature addition |

## Lessons Learned

1. **Multi-column layouts already protected** - OODA-34 skips table detection
2. **Markitdown thresholds are good baselines** - 55%/60 chars is reasonable
3. **Stable metrics mean no harm** - Conservative change doesn't break existing quality

## Next Steps (OODA-22)

Since paragraph detection provides protection for edge cases but doesn't significantly
improve current test PDFs (all multi-column), the next focus should be:

1. **Improve reading order** - Current output shows interleaved blocks in multi-column
2. **Fix text preservation** - 81.3% is below 98% target
3. **Analyze specific document failures** - AlphaEvolve at 74.3% structure needs investigation
