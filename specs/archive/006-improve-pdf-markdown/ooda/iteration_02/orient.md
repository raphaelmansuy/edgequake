# Iteration 02: Orient

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Analysis of Findings

### Critical Bug Identified: Style-blind `can_append()`

The `Span::can_append()` method (pymupdf_structs.rs:77-97) checks:

- Same page ✓
- Same font size ✓
- Same font name ✓
- Horizontal adjacency ✓
- Vertical alignment ✓

**Missing check**: Font style (bold/italic) flags!

### Impact

```text
Input characters:
  'T' (is_bold=true)
  'h' (is_bold=true)
  'i' (is_bold=false)  ← Different style!
  's' (is_bold=false)

Current behavior:
  Span { text: "This", font_is_bold: Some(true) }  ← First char wins
  Markdown output: **This**  ← Wrong! "is" should not be bold

Expected behavior:
  Span { text: "Th", font_is_bold: Some(true) }
  Span { text: "is", font_is_bold: Some(false) }
  Markdown output: **Th**is  ← Correct
```

### First Principles Analysis

**Question**: Should a span contain mixed styles?

**Answer**: No. A span should be homogeneous in style:

- Same font name
- Same font size
- Same font style (bold/italic)

This is how PyMuPDF4LLM works - each "span" in the DICT format has a single
`flags` value that applies to all characters in the span.

### Solution Options

#### Option A: Add style check to `can_append()`

**Pros**:

- Simple fix (2-line change)
- Correct behavior immediately

**Cons**:

- May create more spans (slightly more memory)
- Could fragment words if style changes mid-word (rare edge case)

#### Option B: Track style changes and split spans post-hoc

**Pros**:

- Handles edge cases like mid-word style changes

**Cons**:

- More complex
- Unnecessary for most PDFs

### Recommendation: Option A

Add style check to `can_append()`. The edge case of mid-word style changes
is rare and the simpler solution is more maintainable.

---

## Risk Assessment

| Risk                   | Impact | Likelihood | Mitigation          |
| ---------------------- | ------ | ---------- | ------------------- |
| More spans created     | Low    | High       | Acceptable tradeoff |
| Mid-word fragmentation | Low    | Low        | Monitor in tests    |
| Performance regression | Low    | Low        | Profile if issues   |

---

_Iteration 02 - Orient complete_
_Next: Decide - Plan the can_append() fix_
