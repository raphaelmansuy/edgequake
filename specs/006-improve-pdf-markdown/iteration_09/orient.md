# OODA-09: Orient - Document Magic Numbers in text_grouping.rs

## Analysis

### Page Layout Context
US Letter page: 612pt × 792pt (8.5" × 11" at 72 DPI)
A4 page: 595pt × 842pt (210mm × 297mm at 72 DPI)

### Magic Number Meanings

1. **100.0pt** (line 307): "top of page" zone
   - ~12.6% of page height
   - Used to log elements in top region
   - Reasoning: Title, authors, abstract are in top ~100pt

2. **15.0pt** (author zone bottom bound)
   - ~1.9% of page height
   - Below this is header/footer zone
   - Reasoning: Headers typically < 15pt from edge

3. **80.0pt** (author zone upper bound)
   - ~10.1% of page height
   - Author names typically in 15-80pt range
   - Reasoning: Title at top, then authors, then abstract

4. **30.0pt** (vertical gap threshold)
   - ~3.8% of page height
   - Used to detect section boundaries
   - Reasoning: Single-spaced text has ~12-14pt line height,
     so 30pt gap = 2+ blank lines = section break

5. **20.0pt** (line 422, combined with text length)
   - Slightly below author zone
   - Used with `text.len() < 30` for short text detection

## Prioritization

Most critical to document:
1. **30.0pt gap threshold** - affects section ordering
2. **100.0pt top zone** - affects header/body classification
3. **15.0/80.0pt author zone** - affects author detection

## Hypothesis

Adding WHY comments will:
- Improve code maintainability
- Help future developers understand layout assumptions
- Document PDF page coordinate conventions
