# OODA Iteration 22 – Orient

## Analysis

Two distinct mechanisms cause missing word spaces in pdfium-rendered markdown:

### Mechanism 1: Missing inter-span space TextSpans

**Pipeline trace**: `PdfiumExtractor` → chars → `TextGrouper.chars_to_spans()` strips whitespace chars → separate `Span` per word → `convert_text_block_to_schema_block()` pushes `TextSpan` per Span → `render_spans_styled()` concatenates.

The `chars_to_spans()` function intentionally strips space characters and creates word-boundary spans. `Line::text()` adds spaces back using gap detection (`avg_size * 0.15` threshold). But the schema-level span conversion didn't replicate this gap analysis.

### Mechanism 2: Newline joiner absorption + trim

**Pipeline trace**: `TextSpan::plain("\n")` between lines → `consolidate_spans()` absorbs into previous styled span → `render_spans_styled()` trims content → trailing `\n` lost.

The `consolidate_spans()` function treats any `TextSpan` where `.text.trim().is_empty()` as a "plain joiner" — including `\n`. It absorbs the joiner into the previous span. Then `render_spans_styled()` calls `content.trim()` which strips the trailing `\n`. The `trailing_space` preservation only checks `content.ends_with(' ')`, not `content.ends_with('\n')`.

### Why space is the correct inter-line separator for spans

In PDFs, line breaks within a text block represent **soft wraps** from the page layout engine — they're column-width artifacts, not semantic line breaks. When rendering to markdown (which reflows), these should become spaces. The `block.text` field already gets normalized to spaces during processor chain processing. Using `" "` for the inter-line span separator aligns the span representation with block.text semantics.

The `render_text()` span validity check uses `split_whitespace()` normalization, which treats both `\n` and ` ` identically, so this change doesn't affect the validity check.

## Options

1. **Fix at source**: Insert space `TextSpan` between same-line spans (gap detection) AND change inter-line separator from `\n` to ` `
2. **Fix in renderer**: Modify `render_spans_styled()` to handle `\n` joiners by preserving whitespace
3. **Fix in consolidation**: Make `consolidate_spans()` convert `\n` joiners to spaces before absorption

## Recommendation

Option 1: Fix at source. This is the most principled approach — fix the span data at the point where it's generated, so all downstream consumers see correct word-separated text.
