# OODA Iteration 02 - Act

## Implementation: Early Flip Detection Fix

### Changes Made

#### File: `extraction_engine.rs`

**1. Added early flip detection before OCR filtering (~line 260-280)**

```rust
// Step 1: Detect coordinate system flip from ORIGINAL coordinates
let (original_min_y, original_max_y) = {
    let ys: Vec<f32> = elements.iter().map(|e| e.y).collect();
    if ys.is_empty() {
        (0.0, page_height)
    } else {
        let min_y = ys.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_y = ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        (min_y, max_y)
    }
};

let original_y_range = original_max_y - original_min_y;
let is_flipped = original_y_range > page_height * 1.5;

info!(
    "ENG-COORD: original Y range {:.1} to {:.1} (span={:.1}), page_height={:.1}, is_flipped={}",
    original_min_y, original_max_y, original_y_range, page_height, is_flipped
);
```

**2. Simplified normalization stage (~line 340-360)**

Normalization now uses the `is_flipped` flag from early detection:

```rust
// Normalize Y coordinates using pre-computed is_flipped flag
let normalized_y = if is_flipped {
    max_y - e.y  // Flip: higher original Y = lower normalized Y
} else {
    e.y - min_y  // Normal: keep original ordering
};
```

### Verification Results

#### 1. Qwen.pdf - Reading Order Correct
```
Before: "Beyond its Limits" appeared first (wrong)
After:  "Pushing Qwen3-Max-Th" appears first (correct)
```

#### 2. Log Output
```
ENG-COORD: original Y range 265.6 to 2452.5 (span=2186.9), page_height=792.0, is_flipped=true
```

#### 3. All Test PDFs
| Document | Before | After | Status |
|----------|--------|-------|--------|
| Qwen.pdf | Wrong order | Correct order | ✅ Fixed |
| Beyond Transformer | 17,759 bytes | 17,759 bytes | ✅ No regression |
| Agentic Platform | 94,896 bytes | 94,896 bytes | ✅ No regression |

#### 4. Test Suite
```
cargo test --package edgequake-pdf
# All tests pass including new type3_font_extraction tests
```

### Commits

1. `11348e09` - feat(pdf): add Type3 font regression tests and OODA iteration 01 docs
2. `81043e72` - fix(pdf): detect flipped coordinate systems from negative CTM

### Lessons Learned

1. **Order matters**: When detection depends on filtering, detection must occur BEFORE filtering
2. **Use original data**: Document-level properties should be derived from original data, not filtered subsets
3. **Trace the pipeline**: Adding logging at each stage helped identify exactly where the logic broke

### Next Steps

1. Convert debug logging (`info!`) back to `debug!` for production
2. Investigate truncated title text: "Pushing Qwen3-Max-Th" missing "inking"
3. Analyze text fragmentation patterns for further quality improvements
