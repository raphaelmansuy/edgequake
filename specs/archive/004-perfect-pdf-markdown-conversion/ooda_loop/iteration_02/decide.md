# OODA Iteration 02 - Decide

## Decision: Move Flip Detection Before OCR Filtering

### Chosen Approach

**Move flip detection to occur BEFORE OCR filtering**, using the original Y coordinate range.

### Rationale

1. **Semantic correctness**: The PDF's coordinate system is a property of the document, not of the filtered content
2. **Robustness**: Original Y range gives reliable flip detection regardless of what gets filtered
3. **Minimal change**: Only requires reordering existing logic, not new algorithms

### Implementation Plan

1. After parsing text elements, compute original Y range (before any filtering)
2. Detect flip: `is_flipped = original_y_range > page_height * 1.5`
3. Apply OCR layer filtering (unchanged)
4. Normalize coordinates using `is_flipped` from step 2

### Code Changes

```rust
// BEFORE (broken):
let (min_y, max_y, is_flipped) = {
    let ys: Vec<f32> = filtered_elements.iter()  // ← FILTERED!
        .map(|e| e.y)
        .collect();
    // ...
};

// AFTER (fixed):
// Step 1: Detect flip from ORIGINAL coordinates
let (original_min_y, original_max_y) = {
    let ys: Vec<f32> = elements.iter()  // ← ORIGINAL!
        .map(|e| e.y)
        .collect();
    // ...
};
let is_flipped = (original_max_y - original_min_y) > page_height * 1.5;

// Step 2: Filter (unchanged)
// Step 3: Normalize using is_flipped from step 1
```

### Risk Assessment

- **Low risk**: Logic reordering, not new algorithms
- **Testable**: Existing test PDFs verify correct behavior
- **Backward compatible**: Normal PDFs (non-flipped) unaffected

### Success Criteria

- Qwen.pdf: "Pushing" appears before "Beyond" in output
- All existing test PDFs: No regression in extraction quality
- All tests: Pass with no failures
