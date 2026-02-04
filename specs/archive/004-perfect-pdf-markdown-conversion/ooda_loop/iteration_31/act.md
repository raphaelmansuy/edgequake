# OODA-31 ACT: Fixed Compiler Warnings

## Summary

Fixed 5 compiler warnings in the edgequake-pdf crate to establish a clean baseline for optimization work.

## Changes Made

### 1. extraction_engine.rs - Removed duplicate variable

**Line 276**: Removed first declaration of `original_y_range` (was unused, second declaration on line 288 is the one actually used).

```rust
// Before:
let original_y_range = actual_max_y - actual_min_y;

// OODA-19: Compute effective X bounds...

let original_y_range = actual_max_y - actual_min_y; // duplicate

// After:
// OODA-19: Compute effective X bounds...

let original_y_range = actual_max_y - actual_min_y; // single declaration
```

### 2. layout_processing.rs:472 - Removed unused mut

**Line 472**: Changed `let mut result = blocks;` to `let result = blocks;`

### 3. table_detection.rs:403 - Fixed unused assignment

**Line 403→423**: Removed initial `false` assignment, added `let` to actual assignment:

```rust
// Before:
let mut has_author_pattern = false;
...
has_author_pattern = combined_text.contains('@') || ...

// After:
// (no initial declaration)
...
let has_author_pattern = combined_text.contains('@') || ...
```

### 4. lattice.rs:331 - Added #[allow(dead_code)]

**Line 331**: Added `#[allow(dead_code)]` to `filter_lines` method (useful for debugging but not currently called in hot path).

### 5. text_grouping.rs:935 - Added #[allow(dead_code)]

**Line 935**: Added `#[allow(dead_code)]` to `sort_line_by_runs` method (optimization utility that may be used in future performance work).

## Test Results

```bash
$ cargo build --lib
Finished `dev` profile in 0.12s

$ cargo test --test quick_smoke
running 4 tests
test smoke_test_summary ... ok
test smoke_headers_and_lists ... ok
test smoke_sample_pdf ... ok
test smoke_simple_text ... ok

test result: ok. 4 passed; 0 failed
```

## Verification

```
Compiler warnings before: 5
Compiler warnings after: 0 (lib only - bin has 3 separate warnings)
```

## Files Modified

1. `src/backend/extraction_engine.rs` - Line 276
2. `src/processors/layout_processing.rs` - Line 472
3. `src/processors/table_detection.rs` - Lines 403, 423
4. `src/backend/lattice.rs` - Line 331
5. `src/backend/text_grouping.rs` - Line 935

## Next Steps

- OODA-32: Create micro-tests for instant feedback loop
- Focus on text, tables, columns, fonts, structure
