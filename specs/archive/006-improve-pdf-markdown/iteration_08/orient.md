# OODA-08: Orient - Monospace Style Transition Test

## Analysis

### Why This Matters

Monospace detection is critical for code rendering in Markdown. The pipeline should:

1. Detect monospace font from PDFium (`font_is_fixed_pitch()`)
2. Create separate spans for monospace text
3. Render monospace spans with backticks in Markdown

### Current Test Coverage

| Level                        | Bold/Italic | Monospace  |
| ---------------------------- | ----------- | ---------- |
| Unit (can_append)            | ✅ OODA-02  | ✅ OODA-04 |
| Integration (chars_to_spans) | ✅ OODA-07  | ❌ Missing |

### Test Design

Test case: "Hi `code` !" (normal, monospace, normal)

This tests:

1. Normal → monospace transition (should split)
2. Monospace → normal transition (should split again)
3. Three spans total with correct is_monospace flags

## Alternatives

1. **Add to existing test** - Could extend test_mixed_style_chars_to_spans
   - Con: Test becomes too long, harder to debug failures
2. **New dedicated test** - test_monospace_style_chars_to_spans
   - Pro: Clear, focused, easy to identify failures
   - Selected: This approach

## Hypothesis

Adding `test_monospace_style_chars_to_spans` will pass because:

- RawChar.is_monospace is properly extracted (OODA-03)
- Span.can_append() checks is_monospace (OODA-04)
- chars_to_spans() uses can_append() to decide when to split
