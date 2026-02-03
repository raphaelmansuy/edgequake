# OODA-19 Orient: Adaptive X Bounds Strategy

## Date

2026-02-03

## Analysis of Root Cause

### The Problem Pattern

The existing code applied a **rigid X bounds filter**:

```rust
e.x >= -x_margin && e.x <= page_width + x_margin
```

This assumed text coordinates would always fall within `[-50, 662]` for a 612pt page.

### Why This Assumption Fails

1. **CTM Transforms**: PDF content streams can include coordinate transforms that shift the origin
2. **Non-standard page definitions**: Some PDFs define MediaBox differently than CropBox
3. **Scaled content**: Content may be scaled without updating page dimensions

### Existing Smart Approach for Y Bounds

The code already handles Y bounds intelligently:

```rust
// First pass: get actual element bounds
let actual_min_y = elements.iter().map(|e| e.y).fold(f32::INFINITY, f32::min);
let actual_max_y = elements.iter().map(|e| e.y).fold(f32::NEG_INFINITY, f32::max);
```

Then uses these actual bounds for OCR layer detection and filtering.

## Solution Strategy

### Apply Same Logic to X Bounds

Compute actual X range from extracted elements:

```rust
let actual_min_x = elements.iter().map(|e| e.x).fold(f32::INFINITY, f32::min);
let actual_max_x = elements.iter().map(|e| e.x).fold(f32::NEG_INFINITY, f32::max);
```

Use effective bounds that encompass both nominal and actual:

```rust
let effective_x_max = page_width.max(actual_max_x);
let effective_x_min = 0.0f32.min(actual_min_x);
```

### Filter with Effective Bounds

```rust
.filter(|e| {
    e.x >= effective_x_min - x_margin
        && e.x <= effective_x_max + x_margin
        && e.y >= y_lower_bound
        && e.y <= y_upper_bound
})
```

## Trade-offs Considered

### Risk: Including OCR Ghost Text

If there's an OCR layer with X coordinates far off-page, we might include it.

**Mitigation**: OCR layers typically have different Y coordinates (detected by existing bimodal Y distribution logic). The X filter is less critical for OCR rejection.

### Risk: Performance Impact

Computing min/max X adds a single pass over elements.

**Mitigation**: Elements array is already iterated for Y bounds. Negligible overhead.

## Decision

→ Implement adaptive X bounds using actual element coordinates
→ Maintain x_margin buffer for edge tolerance
→ Add debug logging to track effective bounds
