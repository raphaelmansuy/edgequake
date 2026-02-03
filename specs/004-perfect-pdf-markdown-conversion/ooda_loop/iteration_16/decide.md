# OODA-16: Decide Phase

## Decision

**Enable table detection for multi-column pages with stricter criteria.**

Instead of:

```rust
if page.columns.len() > 1 {
    continue;  // Skip entirely
}
```

Use:

```rust
// Process all pages, but use stricter criteria for multi-column
let strict_mode = page.columns.len() > 1;
```

## Implementation Plan

### Step 1: Remove unconditional skip

Replace the `continue` with a mode flag that enables stricter table detection.

### Step 2: Add stricter row grouping for multi-column pages

In `group_blocks_by_row`, when in strict mode:

- Require Y-alignment within 2pt (instead of 10pt tolerance)
- This distinguishes precise table rows from approximate column alignment

### Step 3: Add text length filter

In `is_likely_table`, when in strict mode:

- Calculate average text length per block in candidate table rows
- Require avg_length < 100 characters (tables have short cells)

### Step 4: Keep 3+ row requirement

Already exists, no change needed.

## Code Changes

### File: `src/processors/table_detection.rs`

**Change 1**: Remove skip, add strict_mode flag

```rust
// Old:
if page.columns.len() > 1 {
    continue;
}

// New:
let strict_mode = page.columns.len() > 1;
```

**Change 2**: Pass strict_mode to row grouping

```rust
let rows = self.group_blocks_by_row(page, strict_mode);
```

**Change 3**: Stricter Y-tolerance in strict mode

```rust
fn group_blocks_by_row(&self, page: &Page, strict_mode: bool) -> Vec<Vec<usize>> {
    // ...
    let y_tolerance = if strict_mode { 2.0 } else { 10.0 };
    // ...
}
```

**Change 4**: Text length check in is_likely_table

```rust
fn is_likely_table(&self, table_rows: &[usize], rows: &[Vec<usize>],
                   page: &Page, strict_mode: bool) -> bool {
    if strict_mode {
        // Check average text length
        let avg_len = self.calculate_avg_text_length(table_rows, rows, page);
        if avg_len > 100.0 {
            return false; // Too long for table cells
        }
    }
    // ... existing checks ...
}
```

## Expected Impact

| Document        | Before    | Expected After | Reason                                  |
| --------------- | --------- | -------------- | --------------------------------------- |
| AlphaEvolve     | 81.2%     | ~85%           | Table 1 detected                        |
| Other multi-col | unchanged | unchanged      | Strict criteria prevent false positives |

## Risk Mitigation

- If false positives occur, tighten text length threshold (100 → 80)
- If tables still missed, relax Y-tolerance (2pt → 5pt)
- Monitor all documents, not just AlphaEvolve

## Commit Message

```
OODA-16: Enable table detection in multi-column layouts

- Remove unconditional skip for multi-column pages
- Add strict_mode with tighter Y-tolerance (2pt vs 10pt)
- Add text length filter (<100 chars) for table cell detection
- Preserves precision while detecting tables like AlphaEvolve Table 1
```
