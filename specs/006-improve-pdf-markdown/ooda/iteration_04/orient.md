# Iteration 04: Orient

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Analysis: Test Structure

### Existing Test Pattern

The `test_span_rejects_different_style` test follows this pattern:

```rust
// 1. Create span with initial style
let mut span = Span::new(0);
let styled_char = RawChar { is_bold: true, ... };
span.append(&styled_char);

// 2. Try to append different style
let different_char = RawChar { is_bold: false, ... };
assert!(!span.can_append(&different_char));

// 3. Verify same style accepted
let same_char = RawChar { is_bold: true, ... };
assert!(span.can_append(&same_char));
```

### Test Cases Needed for Monospace

1. **Monospace → Non-monospace**: Should reject
2. **Non-monospace → Monospace**: Should reject
3. **Monospace → Monospace**: Should accept
4. **Non-monospace → Non-monospace**: Should accept

### Integration Point

Add to existing `test_span_rejects_different_style` or create new `test_span_rejects_different_monospace`.

**Decision**: Add to existing test (follows DRY principle, single test for all style checks).
