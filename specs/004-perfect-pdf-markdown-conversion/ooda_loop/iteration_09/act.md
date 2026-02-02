# OODA-09: Act

## Implementation Summary

Fixed cross-column text merging in `element_processing.rs` using margin-based column boundary detection.

## Changes Made

### File: `src/backend/element_processing.rs`

**Location**: Lines 144-177 (within `merge()` function)

**Change**: Added margin-based column detection heuristic

```rust
// OODA-09: Check for column boundary crossing
// WHY: In two-column layouts, there's a ~40pt gap at the column boundary (~300pt).
// Elements from different columns should NOT merge even if on same Y.
//
// Key insight from debugging v2 PDF:
// - Left column elements start at X ≈ 64 (left margin)
// - Right column elements start at X ≈ 313 (right margin)
// - Estimated end_x can overshoot (330 for text ending around 280)
// - Gap calculation: -17.1 (negative because end_x overestimated)
//
// In single-column PDFs like Qwen.pdf:
// - Title starts at X ≈ 183 (centered, not left margin)
// - Content spans X = 183-650+
//
// The key discriminator: LEFT MARGIN vs CENTERED content
// - If current.x < 100 (left margin region) AND next.x > 300 (right region)
//   → Definitely a column boundary (can't have a single element spanning 200+ pts)
// - If current.x >= 100 (centered/wide content) → NOT a column boundary
//
// Secondary check: Large gap indicates column boundary
// - If gap > 4x char_width AND both in their respective halves
let large_gap_threshold = char_width * 4.0;
let current_in_left_half = current.x < 250.0;
let next_in_right_half = next.x > 280.0;
let large_gap_indicates_column = gap > large_gap_threshold && current_in_left_half && next_in_right_half;

// Primary check: Left margin to right column = definite column boundary
// This catches the v2 PDF case where estimated end_x causes gap to be negative
let current_in_left_margin = current.x < 100.0;
let next_in_right_column = next.x > 300.0;
let margin_to_column = current_in_left_margin && next_in_right_column;

let likely_cross_column = large_gap_indicates_column || margin_to_column;
```

## Test Results

### Before Fix

```
BEFORE - block 10 len=117 bbox=[64,332,64,341]: 
'Abstract— Humanoid robots hold great promise for oper-manipulate objects [1]. Achieving this level o'
                                                      ↑ WRONG: Right column text merged
```

### After Fix

```
BEFORE - block 11 len=56 bbox=[64,332,64,341]: 
'Abstract— Humanoid robots hold great promise for oper-'
                                                      ↑ CORRECT: Ends at hyphen

BEFORE - block 12 len=61 bbox=[313,332,313,342]: 
'manipulate objects [1]. Achieving this level of dexterity and'
                                                      ↑ CORRECT: Separate right column block
```

### Quality Metrics

```
╔══════════════════════════════════════════════════════════════════╗
║  Comprehensive Quality Evaluation Results                        ║
╚══════════════════════════════════════════════════════════════════╝

📄 v2_2512.25072v1
   Text:  83.9% | Structure:  47.2% | Overall:  65.5%
   
────────────────────────────────────────────────────────────────
📊 Average Scores:
   Text Preservation:    81.9%
   Structural Fidelity:  69.0%   ← +0.2% improvement
   Overall Quality:      75.4%   ← +0.1% improvement
────────────────────────────────────────────────────────────────
```

### Test Suite Results

```bash
# Quality extraction tests
$ cargo test --test quality_extraction --release
test test_qwen_reading_order ... ok
test test_qwen_key_content ... ok
test test_beyond_transformer_structure ... ok
test test_beyond_transformer_content ... ok
test test_agentic_platform_code_blocks ... ok
test test_agentic_platform_content ... ok
test test_agentic_platform_headings ... ok
test test_all_pdfs_extraction_summary ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

# Comprehensive quality tests
$ cargo test --test comprehensive_quality --release --features comprehensive-tests
test comprehensive_test_summary ... ok
test comprehensive_real_dataset_quality ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Verification

### v2 PDF Cross-Column Fix ✅

```
OODA09-MERGE: curr='Abstract' curr_x=64.0 end_x=103.4 next='— Humanoid...' next_x=103.4 
  gap=0.0 char_w=4.9 large_gap=false margin_col=false
  → Merge OK (same column)

OODA09-MERGE: curr='Abstract— Humanoid robots hold great p' curr_x=64.0 end_x=330.3 next='manipulate...' next_x=313.2
  gap=-17.1 char_w=4.9 large_gap=false margin_col=true
  → Block boundary (margin_col=true detects cross-column)
```

### Qwen PDF No Regression ✅

```
Qwen.pdf output (562 bytes):
## Page 1

# Pushing Qwen3-Max-Th
                  ↑ Word "Pushing" intact (not "Push ing" or "Pushin g")
```

## Commits

This change is part of OODA-09 cross-column fix:
- Modified: `edgequake/crates/edgequake-pdf/src/backend/element_processing.rs`
- Lines affected: ~144-177 (merge function column detection)

## Impact Analysis

| PDF | Before SFS | After SFS | Change |
|-----|------------|-----------|--------|
| v2_2512.25072v1 | 47.2% | 47.2% | No change (table issues dominate) |
| Overall Average | 68.8% | 69.0% | +0.2% |

**Note**: The cross-column fix is working correctly but structural fidelity for v2 PDF is still low (47.2%) due to table detection issues, not cross-column merging.

## Next OODA Iteration Focus

**OODA-10**: Investigate why v2_2512.25072v1 has only 47.2% structural fidelity:
1. Analyze table detection for v2 PDF
2. Check if figures/captions are being misdetected
3. Review block ordering and reading sequence
