# Iteration 02: Decide

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Prioritized Action Plan

### Change 1: Add Style Check to `can_append()`

**Target**: `layout/pymupdf_structs.rs:77-97`

Add style comparison after font name check:

```rust
// Same font style (bold/italic)
// WHY: A span must have homogeneous style for correct markdown rendering.
// Without this check, "**Th**is" could become "**This**" if 'i' and 's'
// are incorrectly appended to a bold span.
if self.font_is_bold != Some(ch.is_bold) {
    return false;
}
if self.font_is_italic != Some(ch.is_italic) {
    return false;
}
```

### Change 2: Add Test for Style-based Span Splitting

Add a unit test to verify spans are correctly split when style changes.

### Implementation Order

```
1. Modify can_append() in pymupdf_structs.rs
2. Add unit test
3. Run cargo test -p edgequake-pdf
4. Verify clippy still passes
```

---

## Success Criteria

- [ ] `can_append()` rejects characters with different bold/italic flags
- [ ] Unit test verifies span splitting behavior
- [ ] All existing tests pass
- [ ] Zero clippy warnings

---

*Iteration 02 - Decide complete*
*Next: Act - Implement the changes*
