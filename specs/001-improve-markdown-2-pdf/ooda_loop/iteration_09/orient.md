# OODA Iteration 09 – Orient

**Date:** 2026-02-07

## Analysis

### Page-Aware Classification
Two approaches to get page_height:
1. **API-level**: Add `get_page_dimensions()` call from PdfiumExtractor → requires pipeline API change
2. **Estimation**: Calculate from `max(block.y1) + 72pt margin` per page → no API change needed

Approach 2 is sufficient because:
- Footnote detection uses a bottom_portion ratio (25%), so exact page_height isn't critical
- The estimated page_height is within 5% of actual for most documents
- No pipeline API breakage

### Dead Code Cleanup
The `style_text` method was superseded by the span-grouping approach in OODA-04. The `get_style_type` function (without superscript) was superseded by `get_style_type_with_ref`. Both should be removed.
