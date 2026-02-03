# OODA-17: Orient Phase

## First Principles Analysis

### Why Titles Wrap in PDFs

Academic paper titles are typically:
1. Centered on the page
2. Use large fonts (14-18pt)
3. Have limited horizontal space due to centering
4. Long titles (10+ words) often wrap to 2-3 lines

The PDF stores each wrapped line as separate text objects with different Y-coordinates.

### What Makes These Lines "One Title"

```
Line 1: "Fundamentals of Building Autonomous LLM"
Line 2: "Agents"
```

These are ONE title because:
1. **Same font size**: Both 14.3pt
2. **Close Y-spacing**: 17.9pt apart (~1.25× font size = normal line spacing)
3. **Title zone**: Both at top of first page
4. **Large font**: Both above body text size
5. **Content coherence**: "Agents" completes the phrase (not a new section)

### Line Spacing Thresholds (First Principles)

For a 14pt font:
- Normal line spacing = 1.2-1.5 × font_size = 16.8-21pt
- Paragraph break = 2+ × font_size = 28pt+

For multi-line title detection:
- Y-gap < 1.5 × font_size = SAME PARAGRAPH/TITLE
- Y-gap > 2.0 × font_size = NEW ELEMENT

Using 1.5× as threshold:
- 14.3pt font × 1.5 = 21.45pt
- Our gap is 17.9pt < 21.45pt → MERGE

### Algorithm for Merging Spanning Lines

```
Input: spanning_lines = [[line1_elements], [line2_elements], ...]

For each consecutive pair of lines:
  If gap < 1.5 × font_size:
    Merge into single line
  Else:
    Keep as separate lines

Output: merged_spanning_lines
```

## Implementation Location

The best place to merge is in `TextGrouper::group_two_column_layout()` after `group_single_column_layout(spanning_elements)` is called.

```rust
// Before returning spanning_lines
let spanning_lines = self.group_single_column_layout(spanning_elements);
let merged_spanning_lines = self.merge_title_lines(spanning_lines);
```

## Edge Cases

1. **Multiple titles**: Paper may have main title + subtitle
   - Solution: Only merge if font size matches exactly

2. **Author names**: Often in title zone with large font
   - Solution: Require continuous Y-progression (no gaps)

3. **Conference header**: "Published at ICLR 2025" at top
   - Solution: Already filtered as header content

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Merge non-title content | Require font size match within 0.5pt |
| Over-merging | Use strict Y-gap threshold (1.5× font) |
| Miss wrapped titles | May need to relax to 2× font for some PDFs |

## Decision

Implement `merge_title_lines()` in TextGrouper with:
- Y-gap threshold: 1.5 × avg_font_size
- Font size match: within 1pt
- Apply only to spanning lines (title zone content)
