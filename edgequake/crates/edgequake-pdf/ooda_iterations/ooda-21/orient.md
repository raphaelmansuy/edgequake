# OODA-21: ArXiv Metadata Extraction - ORIENT

## Problem Classification
- Type: Metadata extraction and positioning
- Scope: ArXiv papers only (specific pattern)
- Priority: Medium (affects multiple test documents)

## First Principles Analysis

### What is the arXiv watermark?
- 90-degree rotated text in left margin of page 1
- Format: `arXiv:YYMM.NNNNN[vN] [category] DD Mon YYYY`
- Example: `arXiv:2510.09244v1 [cs.AI] 10 Oct 2025`

### Why filter vs extract?
The current approach filters ALL rotated text. But:
1. ArXiv identifiers are valuable metadata
2. Gold files expect them at document top
3. Other rotated text (like vertical labels) should still be filtered

### Solution Strategy: Selective Extraction
1. Detect rotated text (keep OODA-19 detection)
2. Pattern-match for arXiv identifiers
3. If arXiv: Extract as document metadata, place at top
4. If not arXiv: Filter as before

## Risk Assessment
- Risk: False positive matching non-arXiv text
- Mitigation: Use strict regex `arXiv:\d{4}\.\d{4,5}v?\d*`

## Alternative Approaches
1. **Parse PDF metadata** - arXiv ID is in /arXivID metadata field
   - Pro: More reliable than text extraction
   - Con: Requires metadata parsing, not all PDFs have it
2. **Extract rotated text as footnote** - Place at document end
   - Pro: Preserves all rotated content
   - Con: Doesn't match gold expectation
3. **Selective text extraction** - Pattern match arXiv, filter rest
   - Pro: Matches gold expectation exactly
   - Con: Specific to arXiv format

**Chosen:** Approach 3 (selective extraction) for immediate fix,
consider Approach 1 for future enhancement.
