# OODA-08: Decide - Monospace Style Transition Test

## Decision

Add `test_monospace_style_chars_to_spans` test to `layout/pymupdf_grouper.rs`.

## Implementation Plan

1. Add new test function after `test_mixed_style_chars_to_spans`
2. Use existing `make_styled_char` helper (scoped to test, need to recreate)
3. Create chars: "Hi" (normal) + "code" (monospace) + "!" (normal)
4. Assert 3 spans with correct text and is_monospace flags

## Test Code Structure

```rust
#[test]
fn test_monospace_style_chars_to_spans() {
    let grouper = TextGrouper::new();

    // Helper function (same as OODA-07)
    fn make_styled_char(...) -> RawChar { ... }

    // "Hi" normal + "code" monospace + "!" normal
    let chars = vec![
        make_styled_char('H', x, y, 12.0, false, false, false),
        make_styled_char('i', x, y, 12.0, false, false, false),
        make_styled_char('c', x, y, 12.0, false, false, true),  // monospace
        make_styled_char('o', x, y, 12.0, false, false, true),
        make_styled_char('d', x, y, 12.0, false, false, true),
        make_styled_char('e', x, y, 12.0, false, false, true),
        make_styled_char('!', x, y, 12.0, false, false, false), // back to normal
    ];

    let spans = grouper.chars_to_spans(&chars);

    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].text, "Hi");
    assert_eq!(spans[0].font_is_monospace, Some(false));
    assert_eq!(spans[1].text, "code");
    assert_eq!(spans[1].font_is_monospace, Some(true));
    assert_eq!(spans[2].text, "!");
    assert_eq!(spans[2].font_is_monospace, Some(false));
}
```

## Risk Assessment

- **Risk**: Low - straightforward test addition
- **Mitigation**: Test already verified at unit level (OODA-04)

## Success Criteria

- [ ] Test compiles
- [ ] Test passes
- [ ] All 451+ tests pass
- [ ] No clippy warnings
