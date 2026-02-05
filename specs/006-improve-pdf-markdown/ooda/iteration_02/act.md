# Iteration 02: Act

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Changes Implemented

### 1. Added Style Check to `can_append()`

**File**: `layout/pymupdf_structs.rs:73-98`

Added style comparison to ensure spans are split when font style changes:

```rust
// OODA-02: Same font style (bold/italic)
// WHY: A span must have homogeneous style for correct markdown rendering.
// Without this check, mixed bold/normal text would all become bold.
if let Some(span_bold) = self.font_is_bold {
    if span_bold != ch.is_bold {
        return false;
    }
}
if let Some(span_italic) = self.font_is_italic {
    if span_italic != ch.is_italic {
        return false;
    }
}
```

### 2. Added Unit Test for Style-based Span Splitting

**File**: `layout/pymupdf_structs.rs:768-860`

New test `test_span_rejects_different_style()` verifies:
- Span rejects characters with different bold flag
- Span rejects characters with different italic flag
- Span accepts characters with same style

### 3. Enhanced Documentation

Added OODA-02 reference and detailed docstring explaining the bug this fixes:

```rust
/// ## OODA-02: Style Check
///
/// Characters with different bold/italic flags MUST create separate spans.
/// Without this check, mixed-style text would inherit the first char's style:
/// ```text
/// Input: 'T'(bold) 'h'(bold) 'i'(normal) 's'(normal)
/// Bad:   Span{text:"This", is_bold:true} → "**This**" (WRONG)
/// Good:  Span{text:"Th", is_bold:true}, Span{text:"is", is_bold:false}
///        → "**Th**is" (CORRECT)
/// ```
```

---

## Verification

### Test Results

```
cargo test -p edgequake-pdf test_span_rejects_different_style
test layout::pymupdf_structs::tests::test_span_rejects_different_style ... ok
```

### Full Test Suite

```
cargo test -p edgequake-pdf
test result: ok. 450 passed; 0 failed; 0 ignored
```

### Clippy

```
cargo clippy -p edgequake-pdf
0 warnings for edgequake-pdf
```

---

## Files Modified

| File | Lines Changed | Type |
|------|--------------|------|
| `src/layout/pymupdf_structs.rs` | +80 | Fix + Test |

---

## Success Criteria Checklist

- [x] `can_append()` rejects characters with different bold/italic flags
- [x] Unit test verifies span splitting behavior
- [x] All existing tests pass (450)
- [x] Zero clippy warnings

---

## Commit Ready

```
OODA-02: Fix style-blind span merging causing incorrect markdown

Bug: can_append() did not check bold/italic flags when merging
characters into spans. This caused mixed-style text like "**Th**is"
to be rendered as "**This**" because the non-bold "is" was incorrectly
merged into the bold span.

Fix: Added style comparison to can_append() that rejects characters
with different bold or italic flags from the current span.

Added test: test_span_rejects_different_style() verifies:
- Spans reject characters with different bold flag
- Spans reject characters with different italic flag
- Spans accept characters with same style

450 tests pass, zero clippy warnings.
```

---

*Iteration 02 - Act complete*
*Next: Iteration 03 - Continue improvements*
