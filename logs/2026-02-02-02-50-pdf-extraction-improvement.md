# Task Log: PDF Extraction Improvement

**Date:** 2026-02-02
**Session:** PDF to Markdown conversion improvement for Beyond Transformers PDF

## Summary

Fixed a critical issue where PDF content extraction was losing ~91% of text elements due to overly aggressive Y-coordinate filtering. The fix more than doubled the extracted content.

## Actions

1. **Identified Root Cause**: Y-bounds filtering was rejecting elements with Y > 110% of page height
   - PDF uses CTM (Current Transformation Matrix) that shifts coordinates
   - Page 1: Elements at Y range 664.5 to 1298.7 (but page height is 792)
   - Most content was being filtered out

2. **Fixed Y-Bounds Filtering** (`extraction_engine.rs`):
   - Old: Fixed bounds at `page_height * 1.15` max
   - New: Dynamic bounds based on actual element distribution
   - OCR layer detection: Only filter if max_y > 2.5x page height
   - Normal PDFs: Keep all elements, trust CTM normalization

3. **Fixed Word Fragmentation** (`element_processing.rs`):
   - Changed space insertion threshold from 1.0x to 1.5x char_width
   - Prevents false splits like "D iagnose" → "Diagnose"
   - Added WHY comment explaining the rationale

4. **Removed Debug Logging**:
   - CHAIN-TRACE (was looking for wrong PDF)
   - PAGE1-BLOCK, PAGE1-COLUMNS
   - ENG-RAW, ENG-FILTER, MERGE-CHECK

## Decisions

- **Keep all elements for PDFs with CTM shifts**: Rather than trying to guess the "correct" page bounds, we now keep all elements within a reasonable range and rely on Y-normalization to fix coordinates
- **OCR layer threshold at 2.5x page height**: OCR layers are typically placed at exactly 2x or more of page height, so 2.5x is a safe threshold to detect them

## Results

| Metric             | Before      | After        | Improvement   |
| ------------------ | ----------- | ------------ | ------------- |
| Lines extracted    | 265         | 596          | +125% (2.2x)  |
| File size          | 6,331 bytes | 17,759 bytes | +180% (2.8x)  |
| Author bios        | Truncated   | Complete     | ✓             |
| Word fragmentation | "D iagnose" | "Diagnose"   | ✓             |
| Tests passing      | 408         | 408          | No regression |

## Remaining Issues

1. **Two-column Q&A tables**: Page 2 has borderless table layout not detected
2. **Reading order**: Interleaved left/right content in some sections
3. **Bold+Italic markers**: Some italic text rendered as `***` instead of `*`

## Next Steps

1. Consider semantic table detection for borderless layouts
2. Improve column detection for structured Q&A formats
3. Review font weight detection for italic fonts

## Lessons/Insights

- CTM transforms can significantly shift coordinates - never assume Y=0 is at page origin
- Filter by content distribution, not absolute page bounds
- Character-by-character PDFs have ~10% position jitter - be generous with spacing thresholds
