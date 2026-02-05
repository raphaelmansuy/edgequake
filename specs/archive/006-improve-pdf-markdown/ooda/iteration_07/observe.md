# Iteration 07: Observe

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Focus: Mixed Font Styles in Single Line

### Current Test Coverage

The existing `test_span_rejects_different_style` tests:

- Bold → Non-bold: ✓
- Italic → Non-italic: ✓
- Monospace → Non-monospace: ✓

### Gap Identified

No integration test for a full line with mixed styles:

```text
Input:  "This is **bold** and _italic_ and `code` text"
Expected Spans:
  - "This is " (plain)
  - "bold" (bold)
  - " and " (plain)
  - "italic" (italic)
  - " and " (plain)
  - "code" (monospace)
  - " text" (plain)
```

### Why This Matters

Unit tests verify individual span rejection, but integration tests ensure:

1. Complete pipeline handles mixed styles
2. Markdown output is correct
3. No regressions when changes are made
