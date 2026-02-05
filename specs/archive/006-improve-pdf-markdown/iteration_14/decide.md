# OODA-14: Decide - Add Heading Level Tests

## Decision

Add `test_heading_level_classification` test to block_classifier.rs.

## Implementation Plan

```rust
#[test]
fn test_heading_level_classification() {
    let classifier = BlockClassifier::new();
    let body_size = 10.0;

    // Helper to create a block with given font size
    fn make_block(font_size: f32) -> Block {
        Block::from_line(Line {
            spans: vec![Span {
                text: "Test Heading".to_string(),
                font_size,
                // ... other fields
            }],
            // ... line bounds
        })
    }

    // H1: ratio >= 2.0 (20pt / 10pt = 2.0)
    assert!(matches!(
        classifier.classify_block(&make_block(20.0), body_size),
        BlockType::Header(1)
    ));

    // H2: ratio >= 1.7 (18pt / 10pt = 1.8)
    assert!(matches!(
        classifier.classify_block(&make_block(18.0), body_size),
        BlockType::Header(2)
    ));

    // H1 conservative: ratio >= 1.5 (16pt / 10pt = 1.6)
    assert!(matches!(
        classifier.classify_block(&make_block(16.0), body_size),
        BlockType::Header(1)
    ));

    // Paragraph: ratio < 1.5 (10pt / 10pt = 1.0)
    assert!(matches!(
        classifier.classify_block(&make_block(10.0), body_size),
        BlockType::Paragraph
    ));
}
```

## Risk Assessment

- **Risk**: Low - adding test coverage
- **Benefit**: High - validates heading classification logic

## Success Criteria

- [ ] New test covers H1, H2, and Paragraph cases
- [ ] All tests pass
- [ ] No clippy warnings
