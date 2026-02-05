# Iteration 06: Orient

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Analysis: Remaining Documentation Gap

### Identified Issue

The `can_append()` method had all magic numbers documented except one:

- ✓ 0.5pt font size tolerance (documented)
- ✓ 0.3 \* font_size y-tolerance (documented)
- ✓ 0.25 \* font_size space threshold (documented)
- ✗ 0.3 \* avg_char_width overlap tolerance (NOT documented)

### Why This Matters

Kerning in proportional fonts causes characters to overlap:

- "AV" has significant negative sidebearing
- "To" has the 'o' tucked under the 'T'
- Without understanding this, developers might "fix" the tolerance

### Solution

Add WHY comment explaining kerning-based overlap tolerance.
