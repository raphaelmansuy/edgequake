# IT41 — Decide

## Actions

1. **Modify `Span::can_append()` in `pymupdf_structs.rs`**:
   - Add `is_url_punctuation()` helper function
   - Apply 33% threshold only for URL/path punctuation
   - Keep 22% threshold for general punctuation and letters

2. **Validate**:
   - LightRAG URLs should be intact (https://github.com/)
   - Elitizon "AI Agent Design & Building" should have spaces around &
   - "Executive summary" should still work

## Code Change

```rust
// URL/path punctuation that should bind tightly to adjacent characters
fn is_url_punctuation(c: char) -> bool {
    matches!(c, ':' | '/' | '.' | '@' | '-' | '_')
}

let is_url_boundary = last_char.map(is_url_punctuation).unwrap_or(false)
    || is_url_punctuation(ch.char);

let space_threshold = if self.font_is_monospace.unwrap_or(false) || is_url_boundary {
    // Monospace OR URL punctuation: use higher threshold
    self.font_size * 0.33
} else {
    // Proportional non-URL: lower threshold for word detection
    self.font_size * 0.22
};
```

## Scope

- Single file: `src/layout/pymupdf_structs.rs`
- ~15 lines modified
- Focused fix with no changes to other components

## Risk Assessment

**Low risk**:

- Refines existing logic without changing overall approach
- URL punctuation list is conservative (common cases only)
- General punctuation still uses word-boundary-friendly threshold
