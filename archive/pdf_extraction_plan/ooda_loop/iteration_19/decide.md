# OODA-19 Decide: Implement Adaptive X Bounds

## Date

2026-02-03

## Decision

Implement adaptive X bounds filtering in `extraction_engine.rs` to prevent text truncation.

## Specific Changes

### 1. Add X bounds computation (after line 263)

```rust
// First pass: get actual element bounds (both X and Y)
let actual_min_x = elements.iter().map(|e| e.x).fold(f32::INFINITY, f32::min);
let actual_max_x = elements.iter().map(|e| e.x).fold(f32::NEG_INFINITY, f32::max);

// OODA-19: Compute effective X bounds for filtering
let effective_x_max = page_width.max(actual_max_x);
let effective_x_min = 0.0f32.min(actual_min_x);
```

### 2. Update filter logic (lines 346-352)

```rust
.filter(|e| {
    e.x >= effective_x_min - x_margin
        && e.x <= effective_x_max + x_margin
        && e.y >= y_lower_bound
        && e.y <= y_upper_bound
})
```

### 3. Update debug logging

```rust
debug!(
    "ENG-FILTER: y_bounds=({:.1}, {:.1}), x_bounds=({:.1}, {:.1}), elem_count_before={}",
    y_lower_bound, y_upper_bound, effective_x_min - x_margin, effective_x_max + x_margin, elements.len()
);
```

## Expected Outcomes

1. Text elements at x=673.9 ("ocus on ") and x=722.4 ("agentic") will be preserved
2. Block 2 will contain complete text: "...with a focus on agentic..."
3. No regressions on existing test suite (415 tests)

## Rollback Plan

If unexpected side effects occur, revert to nominal page_width filtering with increased margin:

```rust
let x_margin = 200.0; // Increased from 50.0
```

## Verification

1. Run `cargo test --package edgequake-pdf --lib`
2. Extract AI_Services\_\_Elitizon.pdf and verify "focus on agentic" is present
3. Run quality metrics tests
