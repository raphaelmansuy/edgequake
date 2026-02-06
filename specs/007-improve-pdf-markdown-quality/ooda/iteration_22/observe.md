# OODA Iteration 22 – Observe

## What happened

After OODA-IT21 fixed reading order, the markdown output showed **missing word spaces** at two levels:

### Level 1: Same-line inter-span gaps (HEADERS)

Headers like "AI Services" rendered as "AIServices", "Executive summary" as "Executivesummary".

**Root cause**: The TextGrouper's `chars_to_spans()` strips whitespace characters and creates separate `Span` objects per word. `Line::text()` reconstructs spaces by checking inter-span horizontal gaps (threshold: `avg_size * 0.15`). But when we convert to `schema::TextSpan` in `pdfium_backend.rs`, we were pushing spans sequentially without inserting space `TextSpan` objects between them. `render_spans_styled()` concatenates span texts directly, producing "AI" + "Services" = "AIServices".

### Level 2: Cross-line word boundaries (PARAGRAPHS)

Paragraph text like "build\nvs-buy" rendered as "buildvs-buy", "agentic\nworkflows" as "agenticworkflows".

**Root cause**: Between lines, we inserted `TextSpan::plain("\n")`. In `consolidate_spans()`, this `\n` is a "plain joiner" (`.trim().is_empty()` is true for `\n`) and gets absorbed into the previous styled span. So "build" (bold) + "\n" (plain) → "build\n" (bold). Then in `render_spans_styled()`:

1. `content` = "build\n"
2. `trimmed` = content.trim() = "build" (trim removes trailing \n)
3. `trailing_space` = content.ends_with(' ') = **false** (ends with '\n', not ' ')
4. Result: "**build**" with NO trailing space
5. Next span "vs-buy" follows immediately → "**build**vs-buy"

## Evidence

```
# Before fix (IT21 output):
"buildvs-buy"
"frompilot"
"thatproduce"
"agenticworkflows"
"helpteams"
"evaluationprotocol"

# After IT22 Level 1 fix (inter-span spaces):
"AI Services" ✓
"Executive summary" ✓
But Level 2 issues persisted

# After IT22 Level 2 fix (line separator \n → space):
"build vs-buy" ✓
"from pilot" ✓
"that produce" ✓
"agentic workflows" ✓
"help teams" ✓
"evaluation protocol" ✓
```

## Metrics

- All 569 tests pass before and after both fixes
- Zero new clippy warnings
