# OODA Loop Iteration 38 - Investigate one_tool_2512 Low F1

## Date

2025-02-04

## Observe

`one_tool_2512.20957v2` has the lowest F1 score (0.753) in the evaluation dataset:

- **Precision**: 0.670 (low - generating 27% more content than expected)
- **Recall**: 0.861 (moderate - missing some content)

### Symptoms

1. **Author names merged**: `'Zhaoxi ZhangYitong DuanYanzhi Zhang'` instead of `'Zhaoxi Zhang, Yitong Duan, Yanzhi Zhang'`
2. **Text fragmentation**: Sentences broken mid-word and content from different locations interleaved
3. **Math fragments appearing as text**: `*ii,j i,j*`, `*t∈T (a)*`, `*π(a|s)θtt*`
4. **Header level mismatches**: Gold has `## Abstract`, generated has `### Abstract`
5. **Page numbers included**: "12", "14" appearing as content
6. **Duplicate content**: Prompt template appears twice

### File Size Analysis

- Gold: 31,169 bytes, 4,929 words
- Generated: 39,728 bytes, 5,982 words
- Excess: 21% more words than expected

## Orient

### Root Cause Analysis

1. **Author Name Spacing**:
   - Block 3 in extraction: `'Zhaoxi ZhangYitong DuanYanzhi Zhang'`
   - This is at the text element level before block building
   - Likely cause: PDF has no explicit spaces between author names (common in some LaTeX templates)
   - Possible fix: Heuristic detection of CamelCase patterns in author/title regions

2. **Two-Column Layout Issues**:
   - Page 1: `TWO-COLUMN layout with boundary at 295.0`
   - Page 14: `SINGLE-COLUMN layout` (incorrectly classified?)
   - Content from columns appears to be mixing

3. **Math Content Extraction**:
   - LaTeX math symbols are being extracted as text fragments
   - These add noise and reduce precision
   - Should be filtered or rendered differently

4. **Page Number Inclusion**:
   - Page footers contain page numbers
   - These should be filtered by margin detection
   - Current margin filters may not be strict enough

## Decide

Priority fixes for OODA-38:

1. **Investigate author name spacing** - Check if this is a font metrics issue or missing space characters
2. **Check page 14 single-column classification** - Verify if this is causing duplicate content
3. **Investigate math fragment filtering** - Consider filtering blocks that look like LaTeX fragments

## Act

[To be completed - investigation in progress]

## Files to Investigate

- `src/backend/text_extraction.rs` - Character spacing detection
- `src/backend/text_grouping.rs` - Column classification
- `src/processors/margin_filter.rs` or equivalent - Page number filtering
