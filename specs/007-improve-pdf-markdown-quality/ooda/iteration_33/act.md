# IT33 — Act

## Changes Made

### 1. Improved `table_like_score` scoring

**File**: `src/processors/table_detection.rs`

Added three new scoring dimensions:
- **Percentage-rich blocks** (+3): >50% of lines match `N.N%` pattern
- **Numeric-line blocks** (+2): >50% of lines are pure numeric
- **Short multiline blocks** (+1): ≥3 lines, avg length ≤20 chars

### 2. New helper functions

**File**: `src/processors/table_detection.rs`

- `is_percentage_value(s)` — Detects "32.4%", "100%", ".5%" patterns
- `is_numeric_or_pct(s)` — Combines float parse with percentage check
- `strip_numeric_decorators(s)` — Removes `,` and `%` in single pass (fixes clippy consecutive-replace warning)
- `parse_linearized_grid(lines)` — Parses [label, val, val, ..., label, val, ...] into Vec<Vec<String>>

### 3. Column-oriented table reconstruction

**File**: `src/processors/table_detection.rs`

New `try_column_reconstruction()` function:
1. Collects all blocks after caption with `table_like_score ≥ 2`
2. Flattens all lines from matching blocks
3. Calls `parse_linearized_grid()` to detect row boundaries (label followed by N numeric values)
4. Returns `Some(Vec<Vec<String>>)` if valid grid found (≥2 rows, ≥2 cols)

### 4. Updated `scan_for_table` flow

```
scan_for_table(page, caption_idx)
  ├── try_column_reconstruction()  ← NEW: tries linearized grid first
  │   └── if Some(rows) → build Table from rows
  └── parse_rows()                 ← EXISTING: fallback to row-oriented parsing
```

### 5. Fixed `parse_numeric_suffix`

Now uses `strip_numeric_decorators()` to handle `%` and `,` before float parsing.

### 6. Unit tests added (9 new)

| Test | Validates |
|------|-----------|
| `test_table_like_score_percentage_blocks` | Percentage blocks score ≥3 |
| `test_table_like_score_short_multiline` | Short multiline blocks get bonus |
| `test_is_percentage_value` | Pattern matching for N.N% |
| `test_is_numeric_or_pct` | Combined numeric/percentage check |
| `test_parse_linearized_grid` | Regular grid parsing |
| `test_parse_linearized_grid_uneven` | Graceful handling of uneven data |
| `test_parse_linearized_grid_no_pattern` | Returns empty for non-grid data |
| `test_parse_numeric_suffix_with_percentages` | % suffix handling |
| `test_strip_numeric_decorators` | Character stripping |

## Test Results

- **449 lib tests pass** (was 440, +9 new)
- **0 failures**
- **clippy**: No new warnings (fixed consecutive-replace warning)

## Quality Impact

| Table | Before | After | Notes |
|-------|--------|-------|-------|
| Table 1 (lighrag p7) | Plain text dump | Markdown table (16×5) | Column reconstruction |
| Table 2 (lighrag p8) | Plain text dump | Markdown table (12 children) | Improved scoring |
| Table 4 (lighrag p12) | Working | Working (via column path) | No regression |
| Table 3 (lighrag p10) | Fails | Fails | Complex case study — future iteration |
| Table 5 (lighrag p14) | Fails | Fails | Complex case study — future iteration |
| Elitizon tables | Working | Working | No regression |
