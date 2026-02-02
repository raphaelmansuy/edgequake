# OODA Iteration 01 - Act

## Implementation Summary

### Commit: `345bc157`
**Message:** `fix(pdf): OCR layer detection uses bimodal Y distribution`

### Files Changed

#### 1. `edgequake/crates/edgequake-pdf/src/extraction_engine.rs`
**Lines:** 250-320 (OCR layer detection logic)

**Before (9 lines):**
```rust
let actual_max_y = elements.iter().map(|e| e.y).fold(0.0f32, |a, b| a.max(b));
let actual_min_y = elements.iter().map(|e| e.y).fold(f32::MAX, |a, b| a.min(b));
let has_ocr_layer = actual_max_y > page_height * 2.5;
```

**After (48 lines):**
```rust
// WHY: Sort Y coordinates and look for bimodal gap > 0.8 * page_height
// This detects real OCR layers (two distinct text regions) while
// handling CTM-transformed PDFs with scaled-up Y coordinates.
let mut y_values: Vec<f32> = elements.iter().map(|e| e.y).collect();
y_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

let mut has_ocr_layer = false;
let mut ocr_split_point = 0.0f32;

for i in 1..y_values.len() {
    let gap = y_values[i] - y_values[i - 1];
    if gap > page_height * 0.8 {
        has_ocr_layer = true;
        ocr_split_point = (y_values[i - 1] + y_values[i]) / 2.0;
        break;
    }
}

// Filter OCR layer based on which side has more elements
if has_ocr_layer {
    let below_split = elements.iter().filter(|e| e.y < ocr_split_point).count();
    let above_split = elements.iter().filter(|e| e.y >= ocr_split_point).count();
    
    if below_split > above_split {
        elements.retain(|e| e.y < ocr_split_point);
    } else {
        elements.retain(|e| e.y >= ocr_split_point);
    }
}
```

### Verification Results

| PDF | Before | After | Status |
|-----|--------|-------|--------|
| Qwen.pdf | 0 bytes | 629 bytes | ✅ FIXED |
| Beyond Transformer | 17,759 bytes | 17,759 bytes | ✅ No regression |
| Agentic Platform | 94,896 bytes | 94,896 bytes | ✅ No regression |

### Test Coverage Added

**File:** `tests/type3_font_extraction.rs`

```rust
#[tokio::test]
async fn test_type3_font_extraction_qwen() {
    // Validates Type3 font extraction with CTM transforms
    // Expected: At least 500 bytes extracted
}

#[tokio::test]
async fn test_type3_font_document_structure() {
    // Validates document structure (at least 5 blocks)
}
```

### Diagnostic Tools Added

**File:** `src/bin/trace_content.rs`
- Standalone binary for content stream debugging
- Traces CTM transforms, font lookups, text extraction
- Usage: `cargo run --bin trace_content -- <pdf_path>`

### Remaining Work
- [ ] Improve Qwen.pdf text fragmentation (block merging)
- [ ] Add more Type3 font test cases
- [ ] Document CTM transform handling in architecture docs
