# Iteration 04: Observe

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Focus: Add Monospace Span Rejection Test

### OODA-02 Pattern Reference

In OODA-02, we added `test_span_rejects_different_style()` which tests:

- Bold span rejects non-bold character
- Italic span rejects non-italic character
- Same style is accepted

### Gap Identified

OODA-03 added monospace style checking to `can_append()`, but no test verifies:

- Monospace span rejects non-monospace character
- Non-monospace span rejects monospace character
- Same monospace style is accepted

### Current Test Coverage

**File**: `layout/pymupdf_structs.rs:804-898`

```rust
#[test]
fn test_span_rejects_different_style() {
    // Tests bold rejection ✓
    // Tests italic rejection ✓
    // Missing: monospace rejection ✗
}
```

### Expected Test Behavior

```text
Input Characters:
  'c'(mono=true) + 'o'(mono=true) + 'd'(mono=false) + 'e'(mono=false)

Expected Spans:
  Span 1: "co" (monospace)
  Span 2: "de" (not monospace)

Without OODA-03 check:
  Single Span: "code" (all monospace = WRONG)
```
