# OODA-18 Decide: Accept Current State, Focus on Other Improvements

## Decision

Accept the current Y-sorted output as baseline and focus on other quality improvements.

## Rationale

1. **Time investment vs. reward**: Updating gold files is high effort and may not yield immediate quality gains

2. **Other opportunities exist**: There are other issues affecting quality that don't require gold file changes:
   - Margin content filtering (arxiv watermarks)
   - Header/footer detection
   - Table text extraction (empty table blocks)
   - Multi-line text merging

3. **Reading order is "acceptable"**: While interleaved, the text IS present and readable

## Alternative Focus Areas

### Immediate Opportunities

1. **Margin content filtering**: Remove arxiv identifiers, page numbers
2. **Empty table blocks**: Fix `text=''` issue in table rendering
3. **Hyphenation handling**: Improve word continuation across lines

### Future Consideration

- Create a separate "reading order" evaluation metric
- Eventually update gold files for correct reading order
