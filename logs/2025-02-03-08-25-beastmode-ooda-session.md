# OODA Session Log - 2025-02-03-08:25

## Session Summary

Continuing OODA iterations for PDF-to-Markdown quality improvement. Target: 95%+.

## Actions Performed

1. **OODA-31 Completed**: Fixed list prefix parsing bug
   - **Bug**: Numbered list items like `2.Examine reasoning architectures, such as Chain-of-Thought (CoT) and Tree-` were rendered as `2. and Tree-`
   - **Root Cause**: Code used `raw_text.find(") ")` which matched the `)` in `(CoT) ` at position 64, extracting wrong content
   - **Fix**: Rewrote list prefix parsing to find digits first, then check immediately following characters for delimiters
   - **Commit**: `3596eaa3 OODA-31: Fix list prefix parsing to only match at text start`

2. **Bold Detection Analysis** (OODA-32 attempt)
   - Investigated missing bold formatting (gold has 53 bold lines, extracted has 1)
   - Found Nimbus fonts use "Medi" suffix for medium weight (=bold)
   - Added `-medi` detection but this caused over-bolding (section titles also use Medi font)
   - **Reverted**: Adding Medi detection caused quality to drop from 87.3% to 86.0%
   - **Learning**: Font weight detection is complex - same font used for both titles and emphasis

3. **Quality Analysis**
   - Current overall quality: 87.3%
   - Text Preservation: 85.4%
   - Structural Fidelity: 89.2%
   - Gap to 95% target: 7.7 percentage points

## Key Findings

### Sources of Quality Loss

1. **Table extraction** (major impact)
   - Tables are fragmented and malformed
   - Column headers missing (e.g., `Function-level` → missing)
   - Gold has clean aligned tables, we produce fragments

2. **Column interleaving** (medium impact)
   - Two-column layouts sometimes still have interleaved content
   - Figure captions mixing with body text

3. **Case preservation**
   - Some terms extracted in wrong case (`Function-level` → `function-level`)
   - Quality test is case-sensitive

4. **Over-segmentation**
   - Gold: 490 lines, Extracted: 1544 lines (3x)
   - Paragraphs being split into too many lines

## Current Status

- **All 415 tests pass**
- **Quality tests pass at 87.3%**
- **OODA-31 committed successfully**

## Next Steps

1. Improve table detection and rendering (biggest impact)
2. Better column reading order for specific layouts
3. Reduce paragraph fragmentation
4. Investigate per-document quality breakdown

## Decisions

- Do NOT include "medi" in bold detection - causes over-bolding of headings
- Focus on table extraction for biggest quality gains
- Keep word-level case-sensitive comparison (changing test would be cheating)

## Lessons

- Font weight naming is inconsistent across font families
- Same font can be used semantically for different purposes (headings vs emphasis)
- Table extraction from PDFs is fundamentally complex
- Quality improvement diminishing returns as baseline gets higher
