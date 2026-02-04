# OODA-31 DECIDE: Fix Compiler Warnings

## Decision

Fix all 5 compiler warnings to improve code quality and maintainability.

## Specific Changes

### 1. Fix `unused_variables: original_y_range` in extraction_engine.rs:276

**Action:** Prefix with underscore or remove duplicate declaration
**File:** `src/backend/extraction_engine.rs`
**Line:** 276

### 2. Fix `unused_mut` in layout_processing.rs:472

**Action:** Remove `mut` keyword
**File:** `src/processors/layout_processing.rs`
**Line:** 472

### 3. Fix `unused_assignments: has_author_pattern` in table_detection.rs:403

**Action:** Remove unused assignment or use the variable
**File:** `src/processors/table_detection.rs`
**Line:** 403

### 4. Fix `dead_code: filter_lines` in lattice.rs:331

**Action:** Add `#[allow(dead_code)]` or remove if truly unused
**File:** `src/backend/lattice.rs`
**Line:** 331

### 5. Fix `dead_code: sort_line_by_runs` in text_grouping.rs:935

**Action:** Add `#[allow(dead_code)]` or remove if truly unused
**File:** `src/backend/text_grouping.rs`
**Line:** 935

## Rationale

- **Quick win**: Takes <5 minutes, shows immediate progress
- **Clean baseline**: Zero warnings before starting optimization work
- **CI health**: Prevents warning accumulation

## Expected Outcome

```
warning: `edgequake-pdf` (lib) generated 5 warnings → 0 warnings
```

## Test Plan

```bash
cargo build --package edgequake-pdf 2>&1 | grep -c warning
# Expected: 0

cargo test --package edgequake-pdf --test quick_smoke
# Expected: 4 tests pass
```
