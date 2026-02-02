# OODA Iteration 01 - Decide

## Decision: Replace Absolute Threshold with Bimodal Distribution Detection

### Options Considered

#### Option A: Remove OCR Layer Detection Entirely
- **Pros:** Simple fix, no false positives
- **Cons:** Would regress on scanned PDFs with actual OCR layers
- **Decision:** REJECTED - Need OCR layer detection for scanned documents

#### Option B: Increase Threshold Multiplier
- **Pros:** Simple parameter change
- **Cons:** CTM scales vary; no single threshold works for all PDFs
- **Decision:** REJECTED - Doesn't address root cause

#### Option C: Bimodal Y Distribution Detection
- **Pros:** Works with any coordinate system; detects actual OCR layers by finding gaps
- **Cons:** More complex implementation
- **Decision:** SELECTED - Robust solution that handles transformed coordinates

### Selected Approach: Bimodal Y Distribution Detection

Instead of checking if `max_y > threshold`, detect OCR layers by finding a **gap** in Y coordinates:

```rust
// NEW ALGORITHM
1. Collect all Y coordinates from text elements
2. Sort Y coordinates in ascending order
3. Scan for gaps > 0.8 * page_height
4. If found: split_point = midpoint of gap
5. Filter elements based on which side of split has more elements
```

**Why This Works:**
- Real OCR layers have text at two distinct Y ranges (original + overlay)
- CTM-transformed PDFs have continuous Y distribution (no gap)
- Gap detection is coordinate-system independent

### Risk Assessment

| Risk | Mitigation |
|------|------------|
| Edge case: wide Y distribution in normal PDF | Gap must be > 0.8 * page_height (very wide) |
| Edge case: PDF with exactly 2 text regions | Rare; acceptable trade-off |
| Regression on existing PDFs | Run full test suite before merge |

### Success Criteria
- [x] Qwen.pdf extracts > 500 bytes
- [x] Beyond Transformer PDF still works
- [x] Agentic Platform PDF still works
- [x] All existing tests pass
