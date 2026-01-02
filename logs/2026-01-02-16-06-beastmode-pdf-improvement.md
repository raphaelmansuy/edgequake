# PDF→Markdown Improvement Session Log

## Session Summary

**Date:** 2026-01-02  
**Duration:** OODA Loops 18-22 (continuation from previous session)  
**Baseline Score:** 27.2/100  
**Final Score:** 34.2/100 (+7.0 points, +25.7% relative improvement)

## Actions Performed

1. **Fixed spans.clear() bug in BlockMergeProcessor** (Loop 18)

   - Root cause: `cur.spans.clear()` at line 1493 was discarding all span styling info after block merges
   - Fix: Removed the clear() call since `Block.merge()` properly extends spans
   - Impact: Bold patterns improved from 11 to 63 in test documents

2. **Reverted processor reorder experiment** (Loop 19)

   - Attempted to move BlockMergeProcessor before header detection
   - Result: Score decreased, reverted

3. **Tested bold keyword fallback for headers** (Loop 20)

   - Added detection for bold text matching section keywords
   - Result: Caused over-detection, reverted to simpler approach

4. **Removed debug statements** (Loop 21)

   - Cleaned up `eprintln!` debug statements in sota_backend.rs and markdown.rs

5. **Tested SectionPatternProcessor** (Loop 22)

   - Exported and added SectionPatternProcessor to pipeline
   - Result: Caused over-detection (41→64 headers), disabled

6. **Ran full test suite**
   - All 173 tests pass
   - Code verified with clippy

## Key Code Changes

### [processor.rs](edgequake/crates/edgequake-pdf/src/processors/processor.rs)

- Removed `cur.spans.clear()` after block merge (lines ~1493-1494)
- This preserves bold/italic styling through the merge process

### [sota_backend.rs](edgequake/crates/edgequake-pdf/src/backend/sota_backend.rs)

- Enhanced bold font detection with SFBX (SF Bold Extended) and CMBX (Computer Modern Bold) patterns
- Removed debug eprintln! statements

### [markdown.rs](edgequake/crates/edgequake-pdf/src/renderers/markdown.rs)

- Removed debug eprintln! statements

### [mod.rs](edgequake/crates/edgequake-pdf/src/processors/mod.rs)

- Added SectionPatternProcessor to exports (for future use)

### [extractor.rs](edgequake/crates/edgequake-pdf/src/extractor.rs)

- Added import for SectionPatternProcessor (disabled in pipeline)

## Per-Document Scores (Final)

| Document              | Table | Style | Composite |
| --------------------- | ----- | ----- | --------- |
| 2900_Goyal_et_al      | 0.7%  | 39.6% | 35.2      |
| AlphaEvolve           | 0.9%  | 53.4% | 40.7      |
| agent_2510.09244v1    | 0.2%  | 60.1% | 43.1      |
| ccn_2512.21804v1      | 0.0%  | 5.6%  | 21.2      |
| one_tool_2512.20957v2 | 9.0%  | 20.0% | 30.6      |

## Decisions Made

1. Bold keyword fallback was removed as it caused over-detection in some documents
2. SectionPatternProcessor causes over-detection and is disabled
3. Table accuracy remains low (2.2%) - requires fundamental table extraction improvements
4. ccn document has structural issues that need more investigation

## Next Steps

1. Investigate table extraction processor (currently disabled)
2. Look into multi-line title merging for documents like agent_2510
3. Consider adding more LaTeX font pattern detection
4. Investigate why ccn document has such low style accuracy
5. Fix SectionPatternProcessor to be less aggressive

## Lessons/Insights

- `spans.clear()` was a critical bug that prevented bold rendering
- Bold detection working correctly in font parsing (SFBX fonts detected)
- Aggressive header detection can hurt score if it over-detects
- Table accuracy is the main limiting factor for further score improvement
- SectionPatternProcessor needs refinement before it can be used
