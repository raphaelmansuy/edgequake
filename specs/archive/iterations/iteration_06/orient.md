# OODA-06 Orient: Line Preservation Strategy

## Analysis

### Why Lines Matter

pymupdf4llm preserves PDF line breaks because:

1. **Visual structure**: Academic papers have ~70-80 char lines
2. **Hyphenation**: Words broken across lines with `-` need proper handling
3. **Reference matching**: Line-by-line comparison in ROUGE-L benefits from aligned lines
4. **Human readability**: Long single-line paragraphs are hard to read

### Current Pipeline Flow

```
PDF → RawChar → Spans → Lines → Blocks → Markdown
                             ↓
                   [render_lines_inline]
                             ↓
                    join(" ") ← PROBLEM
```

### Proposed Change

Simple fix in `render_lines_inline`:

- Change from: `.join(" ")`
- Change to: `.join("\n")`

### Considerations

1. **Headers**: Should still be single-line (no change needed - different render path)
2. **Lists**: Continuation lines need special handling (already handled separately)
3. **Code blocks**: Already rendered line-by-line (no change needed)

### Expected Impact

| Metric    | Current | Expected | Rationale                             |
| --------- | ------- | -------- | ------------------------------------- |
| Structure | 0.453   | ~0.65    | Lines ratio will improve dramatically |
| ROUGE-L   | 0.701   | ~0.75    | Better line-level alignment           |
| Quality   | 0.702   | ~0.75    | Weighted improvement                  |

### Risk Assessment

- **Low risk**: Single character change (`" "` → `"\n"`)
- **No logic change**: Just output formatting
- **Easy revert**: If quality drops, revert is trivial
