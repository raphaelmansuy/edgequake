# Iteration 04: Decide

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Decision: Extend Existing Test

### Approach

Add monospace rejection tests to the existing `test_span_rejects_different_style()` function.

### Test Cases to Add

```rust
// OODA-04: Test monospace span rejection
// Create a span starting with monospace text
let mut mono_span = Span::new(0);
let mono_char = RawChar {
    char: 'x',
    is_bold: false,
    is_italic: false,
    is_monospace: true,
    ...
};
mono_span.append(&mono_char);

// Try to append non-monospace character
let non_mono_char = RawChar {
    char: 'y',
    is_monospace: false,
    ...
};
assert!(!mono_span.can_append(&non_mono_char));  // Should reject

// Same monospace style should be accepted
let same_mono_char = RawChar {
    char: 'z',
    is_monospace: true,
    ...
};
assert!(mono_span.can_append(&same_mono_char));  // Should accept
```

### File to Modify

`layout/pymupdf_structs.rs` - `test_span_rejects_different_style()` (around line 890)
