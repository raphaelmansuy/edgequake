# Task Log: Column Detection Threshold Fix
**Date:** 2026-01-04 13:08  
**Session:** Continuation of footer separation testing

## Context
After implementing footer separation fix (commit 42cbfcf), tested SpaceTimePilot PDF and discovered page 13 had cross-column text mixing due to column detection failure.

## Problem Discovery
SpaceTimePilot page 13 cross-column garbling:
```
LINE-XRANGE: Y=174.2 X=[70.5,329.2] range=258.8 
text='Each trajectory is rendered into a 120-frame sequence atFor temporal-control tra'
```

## Root Cause Analysis

### Diagnostic Logging
Added COL-DEBUG info-level logging to column_detection.rs to trace detection algorithm:
```
COL-DEBUG: Column detection start: 38 elements, page_width=612.0
COL-DEBUG: Projection gaps found: [27.5, 197.5]
Detected SINGLE-COLUMN layout (left_starts=29, right_starts=8, balance=0.28)
```

### Why Column Detection Failed
1. **Low element count:** Only 38 elements on page 13 (near end of document)
2. **No center gap:** Projection histogram gaps at [27.5, 197.5] - no gap near center (306.0)
3. **Zone-based fallback triggered:**
   - left_zone_end = 612 × 0.45 = 275.4
   - right_zone_start = 612 × 0.50 = 306.0
   - left_starts = 29 elements (X < 275.4)
   - right_starts = 8 elements (X > 306.0)
   - balance = 8/29 = **0.28 < 0.30** ← FAILED THRESHOLD

4. **Threshold too strict:** balance_ratio > 0.30 requirement failed even though both columns had content (>= 5 each)

### Why 0.30 Threshold is Inconsistent
- Projection-based detection uses `balance > 0.25`
- Zone-based fallback used `balance > 0.30`
- This inconsistency caused false negatives for pages with slightly uneven distribution

## Solution Implemented
Changed line 210 in `src/backend/column_detection.rs`:
```rust
// OLD: if left_starts >= 5 && right_starts >= 5 && balance_ratio > 0.3 {
// NEW: if left_starts >= 5 && right_starts >= 5 && balance_ratio > 0.25 {
```

**WHY 0.25:**
- Matches projection-based threshold for consistency
- Still requires both columns >= 5 elements (prevents false positives)
- balance=0.28 now passes (right on edge but reasonable)

## Verification

### Page 13 After Fix
```
COL-DEBUG: Column detection start: 38 elements, page_width=612.0
Detected TWO-COLUMN layout with boundary at 299.9
```

No more LINE-XRANGE warning at Y=174.2!

### Generated Markdown Quality
**Before:**
```
...sequence atFor temporal-control tra...
```
(merged left+right columns)

**After:**
```
Line 838: Each trajectory is rendered into a 120-frame sequence at
...
Line 844: For temporal-control training, we could sample any time
```
(proper separation)

### Test Results
- ✅ All 398 tests pass
- ✅ SpaceTimePilot: chars=51897, f1=0.845 (maintained)
- ✅ v2_2512.25072v1: No "ap-paradigm" garbling (footer fix still works)
- ✅ LINE-XRANGE warnings reduced from >20 to 5

## Additional Changes
1. **COL-DEBUG logging:** Promoted key debug! messages to info! with COL-DEBUG prefix for diagnostics
2. **Markdown renderer:** Fixed unused loop variable warning (i -> _i)

## Git Commit
```
commit f46ebff
fix(pdf): Lower zone-based column detection threshold to 0.25, add COL-DEBUG logging
```

## Impact Summary
| Metric | Before | After |
|--------|--------|-------|
| Page 13 detection | SINGLE-COLUMN | TWO-COLUMN ✅ |
| Cross-column mixing | Yes (X=[70.5,329.2]) | No ✅ |
| Tests passing | 398 | 398 ✅ |
| SpaceTimePilot f1 score | 0.845 | 0.845 ✅ |
| Footer separation | Working | Working ✅ |

## Lessons Learned
1. **Threshold consistency matters:** Projection-based and zone-based should use same balance threshold
2. **Diagnostic logging is critical:** COL-DEBUG traces revealed exact failure point
3. **Test real-world PDFs:** Synthetic tests might not catch edge cases like low element count pages
4. **Balance threshold sweet spot:** 0.25 works well (not too strict, not too permissive)
5. **Zone-based is a robust fallback:** Works when projection histogram fails (no center gap)

## Follow-up Tasks
- [x] Fix column detection threshold
- [x] Verify footer separation still works
- [x] Run full test suite
- [x] Test on real dataset
- [ ] Consider removing COL-DEBUG logging once stable
- [ ] Monitor for other pages with similar issues
