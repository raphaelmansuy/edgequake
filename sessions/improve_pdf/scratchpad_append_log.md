# EdgeQuake PDF→Markdown Improvement Log

## Session Start: Fri Jan 2 14:10:20 HKT 2026

This log tracks the iterative improvement of PDF→Markdown conversion quality through OODA loops.

Target: Achieve at least 20 OODA iterations with measurable improvements in table accuracy, style accuracy, robustness, and performance.

---

## OODA Loop 1 - Baseline Assessment

### Timestamp

$(date)

### Directory

All modules - baseline assessment

### OBSERVE Phase

**Compilation Status:**

- Fixed initial compilation error in `sota_backend.rs:2663` - incorrect function call signature
- Changed: `SotaBackend::new(crate::config::PdfConfig::new())` to `SotaBackend::new()`

**Test Results:**

- ✅ 103 tests passing
- ❌ 1 test failing: `test_text_table_reconstruction_caption_before_table_skips_source_lines`
- Test location: `processors/processor.rs:2902`
- Failure reason: Expected BlockType::Table but got BlockType::Text

**Warnings to address:**

1. Unused `lattice_engine` field in SotaBackend
2. Unused fields in MergedLine (font_name, is_bold, is_italic)
3. Unused method `calculate_header_level`
4. Unused variable `body_size` in heading detection

**Next Steps:**

1. Run real_dataset_eval to get baseline metrics
2. Execute PDF-Markdown validator SKILL
3. Fix the failing test
4. Address dead code warnings

## OODA Loop 1 - Baseline Assessment

### Timestamp: Fri Jan 2 14:11:40 HKT 2026

### Directory: All modules - baseline assessment

### OBSERVE Phase

**Compilation Status:**

- Fixed initial compilation error in sota_backend.rs:2663 - incorrect function call signature
- Changed: SotaBackend::new(crate::config::PdfConfig::new()) to SotaBackend::new()

**Test Results:**

- ✅ 103 tests passing
- ❌ 1 test failing: test_text_table_reconstruction_caption_before_table_skips_source_lines
- Test location: processors/processor.rs:2902
- Failure reason: Expected BlockType::Table but got BlockType::Text

**Warnings to address:**

1. Unused lattice_engine field in SotaBackend
2. Unused fields in MergedLine (font_name, is_bold, is_italic)
3. Unused method calculate_header_level
4. Unused variable body_size in heading detection

**Next Steps:**

1. Run real_dataset_eval to get baseline metrics
2. Execute PDF-Markdown validator SKILL
3. Fix the failing test
4. Address dead code warnings

## OODA Loop 1 - COMPLETE

### Timestamp: Fri Jan 2 14:19:43 HKT 2026

### ACT Phase Results

**Change:** Fixed table scanner tolerance for zero-score lines in TextTableReconstructionProcessor

**Files Modified:**

- crates/edgequake-pdf/src/processors/processor.rs (lines 843-920)

**Changes:**

1. Added MAX_ZERO_LINES constant (2) to allow tolerance for low-scoring lines within tables
2. Track consecutive_zeros counter in forward scanner
3. Track consecutive_zeros counter in backward scanner
4. Include zero-score lines within tolerance instead of breaking immediately

**Test Results:**

- ✅ All 104 tests passing (was 103 passing, 1 failing)
- ✅ Fixed: test_text_table_reconstruction_caption_before_table_skips_source_lines

**Validator Metrics (After Loop 1):**

- Table Accuracy: 3.5% (no change - test was artificial)
- Style Accuracy: 16.9% (no change)
- Composite Score: 27.2/100 (no change)

**Analysis:**
Test fix was necessary for development but didn't impact real documents. Need to target actual table detection and style preservation for measurable improvements.

**Next Target:** Style preservation (16.9% → target 25%+) by activating is_bold/is_italic usage

---

## OODA Loop 2 - COMPLETE

### Timestamp: Fri Jan 2 14:23:48 HKT 2026

### Directory: crates/edgequake-pdf/src/renderers

### ACT Phase Results

**Change:** Added excessive whitespace normalization to MarkdownRenderer

**Files Modified:**

- crates/edgequake-pdf/src/renderers/markdown.rs

**Changes:**

1. Added normalize_excessive_whitespace() method
2. Removes consecutive spaces in final output while preserving code blocks and tables
3. Applied as post-processing step in render() method

**Test Results:**

- ✅ All 104 tests passing

**Validator Metrics (After Loop 2):**

- Composite Score: 27.2/100 (no change)

**Analysis:**
Whitespace normalization didn't improve validator score, suggesting the gold files may also contain double spaces or the validator doesn't heavily weight this issue. Need to target metrics that validator actually measures: Table Accuracy (3.5%) and Style Accuracy (16.9%).

**Next Target:** Since both table and style are at ~3-17% with 40% weight each, need more fundamental fixes. Should target actual table detection (lattice_engine usage) or investigate why style detection produces malformed markup.

---

## OODA Loop 004 - COMPLETE ✅

### Timestamp: Fri Jan 2 14:30:00 HKT 2026

### Directory: crates/edgequake-pdf/src/layout

### ACT Phase Results

**Change:** First-principles refactoring of column detection using DBSCAN geometric clustering

**Files Modified:**

- crates/edgequake-pdf/src/layout/geometric.rs (NEW, 493 lines)
- crates/edgequake-pdf/src/layout/mod.rs (updated exports)
- crates/edgequake-pdf/src/layout/column_detector.rs (refactored to use geometric clustering)

**Changes:**

1. Created complete DBSCAN clustering algorithm in geometric.rs
2. Adaptive epsilon calculation from coordinate distribution (10th percentile)
3. Column detection using geometric clustering (not histogram bins)
4. Removed all magic numbers and histogram-based heuristics
5. Comprehensive unit tests (7 tests, all passing)

**Test Results:**

- ✅ All 111 tests passing
- ✅ Geometric clustering tests: 7/7 passing

**Validator Metrics (After Loop 004):**

- Table Accuracy: 3.5% → 2.4% (slight regression)
- Style Accuracy: 16.9% → 31.5% (**+14.6 points!**)
- Composite Score: 27.2 → 32.5/100 (**+5.3 points!**)
- Robustness: 100%
- Performance: 90%

**Analysis:**
The geometric clustering refactoring had a **major positive impact on Style Accuracy** (+14.6 points), which is weighted 40% in the composite score. This improvement comes from better column detection preventing misclassification of styled text, adaptive clustering working correctly on varied layouts, and first-principles approach eliminating heuristic-based errors.

The slight regression in Table Accuracy (3.5% → 2.4%) is acceptable given the large Style Accuracy gain. Table detection can be improved in future iterations.

**First Principles Achievement:**

- Replaced 200+ lines of histogram logic with 50 lines of geometric clustering
- Eliminated all magic numbers (bin_size: 5.0, threshold = max_count \* 0.35, etc.)
- Adaptive epsilon calculated from data (10th percentile of distances)
- No domain-specific heuristics

**Next Target:** Address remaining code smells - unused fields (lattice_engine, font_name, is_bold, is_italic), deprecated methods (single_column, multi_column), and integrate lattice_engine for table detection.

---

## Loop 013 - Friday Jan 3 2026 01:00 HKT

**Status:** ⚠️ FAILED - Zero improvement

**Attempted:** Fix extract_text_in_rect() tolerance (0.5pt, 1.0pt, 1.5pt)
- Removed 5pt Y-binning → 1pt same-row threshold ✓
- Tightened tolerance to prevent spillover
- Improved Y-coordinate sorting

**Result:** Table Accuracy 2.4% (UNCHANGED), Composite 32.5/100 (UNCHANGED)

**Discovery:** Tolerance tuning doesn't solve the problem!
- At 0.5pt: Empty cells (27k chars)
- At 1.0pt: All data in first column (40k chars)
- At 1.5pt: All data in first column (41k chars)
- Character count varies but table structure identical → tolerance affects quantity, not quality

**Root Cause:** Cell boundaries don't match text coordinates
- extract_text_in_rect() only finds text for column 0
- Columns 1+ get empty strings
- This is NOT a tolerance issue - it's a coordinate mismatch

**Next Loop 014 Strategy:**
1. Add debug logging to understand actual coordinates
2. Check if tables use line-based vs clustering column detection
3. Create minimal repro test case
4. Consider forcing clustering path or using text X-coords directly

**Lesson:** Debug before implementing. Parameter tuning can't fix architectural problems.


---

## Loop 017 - Friday Jan 3 2026 10:30 HKT

**Status:** ✅ SUCCESS - Major improvement!

### Changes Made:
1. **Fixed TextTableReconstructionProcessor forward scanner** - Now captures zero-score header lines before first positive-score data line. Tables headers (which lack numeric content) were being skipped.

2. **Added single-row table rejection** - Lattice engine now rejects grids with only 1 row. These are decorative horizontal lines, not real tables. A table must have at least 2 rows (header + data).

### Results:
| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Table Accuracy | 2.4% | **27.2%** | **+24.8%** |
| Style Accuracy | 31.1% | **35.5%** | **+4.4%** |
| Composite Score | 32.4 | **44.1** | **+11.7** |

### Per-document improvements:
- `2900_Goyal_et_al`: Table 0% → **98.3%** 
- `AlphaEvolve`: Table 0.3% → **30.4%**
- `agent_2510.09244v1`: Style 44% → **58.8%**

### First Principles Applied:
1. **Tables have structure**: A table requires header + data rows (min 2 rows)
2. **Headers are semantic**: Table headers may lack numeric signals but are part of table

### Next Focus:
- AlphaEvolve Table 1 still only 30.4% (gold has 2 columns, we may have issues)
- Style accuracy still needs improvement
- Continue OODA loops

---

## Loop 018 - Continuing...