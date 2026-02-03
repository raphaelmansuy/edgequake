# OODA-24 Orient: Structural Quality Gap Analysis

## Comparison: Extracted vs Gold (one_tool_2512.20957v2)

### Gold File Strengths
1. **Metadata extraction**: Authors, Affiliation extracted as structured fields
2. **Clean Abstract**: Marked with `## Abstract` header
3. **Figure handling**: Figure descriptions in blockquotes, separate from text flow
4. **No page numbers**: Page numbers filtered out
5. **Better paragraph structure**: Logical grouping of sentences

### Our Extraction Issues
1. **Page numbers appear**: "2" appearing as standalone lines (line 6, 8)
2. **Figure captions inline**: "Figure 1.Illustration of a LLM navigating..."
3. **Missing metadata structure**: No Authors/Affiliation extraction
4. **arXiv ID placement**: Appears mid-document instead of as metadata
5. **Paragraph fragmentation**: Some sentences split across blocks

## Root Cause Analysis

### Issue 1: Page Numbers in Output
**Cause**: Page numbers not filtered during extraction
**Location**: Likely in block classification or header/footer detection
**Impact**: Medium - adds noise to output

### Issue 2: Figure Captions Inline
**Cause**: Figure captions not recognized as special block type
**Location**: `structure_detection.rs` or `processor.rs`
**Impact**: High - disrupts reading flow

### Issue 3: Missing Metadata
**Cause**: No metadata extraction pipeline (Authors, Affiliation)
**Location**: Would need new processor
**Impact**: Medium - affects document structure

### Issue 4: Paragraph Fragmentation
**Cause**: Block merge not aggressive enough for multi-column layouts
**Location**: `layout_processing.rs`
**Impact**: High - affects readability

## Priority Analysis

| Issue | Impact | Effort | Priority |
|-------|--------|--------|----------|
| Page numbers | Medium | Low | P2 |
| Figure captions | High | Medium | P1 |
| Missing metadata | Medium | High | P3 |
| Paragraph fragmentation | High | Medium | P1 |

## First Principles: Figure Caption Handling

Academic PDFs have figures with captions that:
1. Are visually distinct (italics, smaller font, or specific position)
2. Often appear in the middle of text columns
3. Should NOT interrupt paragraph flow in markdown

The gold file handles this by:
- Extracting figure description as blockquote
- Placing it AFTER the current paragraph, not inline

Our current approach:
- Treats figure captions as regular text blocks
- Reading order places them by Y position (correct geometrically but wrong semantically)

**Solution**: Detect figure captions and defer them in reading order
