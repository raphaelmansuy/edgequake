# Iteration 07: Decide

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Decision: Add Mixed Style Line Test

Add `test_mixed_style_line()` to verify complete mixed-style line handling.

### Test Implementation

```rust
#[test]
fn test_mixed_style_line() {
    // Create chars: "Hi" (bold) + "there" (italic)
    // Verify: 2 spans with correct styles
}
```

### Rationale

1. **Higher confidence** - Tests full span creation pipeline
2. **Catches regressions** - Would catch OODA-02/03 bugs
3. **Quick to run** - No I/O, pure logic test
