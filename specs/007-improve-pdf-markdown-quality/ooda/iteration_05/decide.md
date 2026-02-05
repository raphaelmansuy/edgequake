# Iteration 05: DECIDE - Action Plan

## Decision

Modify `convert_text_block_to_schema_block()` in `pdfium_backend.rs` to:

1. Add `TextSpan` and `FontStyle` to imports
2. Iterate through `text_block.lines` and their spans
3. Convert each `layout::Span` to `schema::TextSpan` with style preservation
4. Populate `block.spans` with the styled spans
5. Add line break handling between lines

## Implementation Plan

### Step 1: Update imports

```rust
use crate::schema::{
    Block, BlockId, BlockType, BoundingBox, Document, ExtractionMethod, Page,
    TextSpan, FontStyle,  // ADD THESE
};
```

### Step 2: Add helper function

```rust
/// Convert layout::Span to schema::TextSpan with style preservation.
fn convert_span_to_text_span(span: &crate::layout::pymupdf_structs::Span) -> TextSpan {
    let mut style = FontStyle::default();
    style.weight = if span.font_is_bold.unwrap_or(false) {
        Some(700)
    } else {
        None
    };
    style.italic = span.font_is_italic.unwrap_or(false);
    style.size = Some(span.font_size as f64);
    style.family = span.font_name.clone();

    TextSpan::styled(span.text.clone(), style)
}
```

### Step 3: Update convert_text_block_to_schema_block

Add span population after setting `block.text`:

```rust
// Populate spans with styled TextSpan objects
for (line_idx, line) in text_block.lines.iter().enumerate() {
    for span in &line.spans {
        let text_span = convert_span_to_text_span(span);
        block.spans.push(text_span);
    }
    // Add space between lines (except last)
    if line_idx < text_block.lines.len() - 1 && !line.spans.is_empty() {
        block.spans.push(TextSpan::plain(" "));
    }
}
```

## Risk Assessment

| Risk                   | Likelihood | Mitigation                     |
| ---------------------- | ---------- | ------------------------------ |
| Break existing tests   | Low        | Run full test suite            |
| Performance impact     | Low        | Span iteration is O(n)         |
| Renderer compatibility | Low        | Renderer already handles spans |

## Success Criteria

1. Tests pass
2. Markdown output shows `**bold**` and `*italic*` correctly
3. No regression in multi-column or table handling
