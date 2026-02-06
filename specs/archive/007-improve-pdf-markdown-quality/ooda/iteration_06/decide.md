# Iteration 06: DECIDE - Action Plan

## Decision

Add end-to-end rendering test to `renderers/markdown.rs` that verifies:

1. Bold spans produce `**bold**` output
2. Italic spans produce `*italic*` output
3. Mixed bold+italic produces `***bold italic***`
4. Consolidated spans don't fragment into `**a** **b**`

## Implementation

### Test Location

`src/renderers/markdown.rs` - Add to existing `#[cfg(test)]` module

### Test Code Structure

```rust
#[test]
fn test_render_styled_spans_bold_italic() {
    // Create spans with different styles
    let bold_span = TextSpan::styled("bold", FontStyle { weight: Some(700), ..default });
    let normal_span = TextSpan::plain(" normal ");
    let italic_span = TextSpan::styled("italic", FontStyle { italic: true, ..default });

    // Create block with spans
    let mut block = Block::new(BlockType::Paragraph, bbox);
    block.spans = vec![bold_span, normal_span, italic_span];

    // Render
    let renderer = MarkdownRenderer::new();
    let output = renderer.render(&doc);

    // Assert
    assert!(output.contains("**bold**"));
    assert!(output.contains("*italic*"));
}
```

## Success Criteria

1. Test passes
2. No fragmentation like `**b** **old**`
3. Whitespace preserved correctly
