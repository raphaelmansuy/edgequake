# Iteration 08: DECIDE - Clippy Fixes & Observations

## Decision

### This Iteration: Code Quality Fixes

Fix Clippy warnings identified during table detection investigation:

1. **Fix loop variable warning** (pymupdf_grouper.rs:583)
   - Change `for i in start..=end` to slice iteration
   - Impact: None on functionality, cleaner code

2. **Fix collapsible if** (pymupdf_grouper.rs:247)
   - Combine nested if into single condition
   - Impact: None on functionality, cleaner code

### Deferred: Table Detection Improvements

Table detection requires more complex changes:

- Enabling whitespace-based detection
- Adding false positive filtering
- Integrating with existing pipeline

These should be addressed in a dedicated iteration (IT09+).

## Implementation Plan

```
[ ] Read pymupdf_grouper.rs line 583 (loop warning)
[ ] Fix loop to use slice iteration
[ ] Read pymupdf_grouper.rs line 247 (if collapse)
[ ] Fix nested if to combined condition
[ ] Run clippy to verify fixes
[ ] Run tests to ensure no regressions
[ ] Commit changes
```

## Expected Outcomes

- Zero Clippy warnings for edgequake-pdf crate
- All 515 tests passing
- Cleaner code following idiomatic Rust patterns
