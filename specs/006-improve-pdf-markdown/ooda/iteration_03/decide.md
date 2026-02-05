# Iteration 03: Decide

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Decision: Implement PDFium-based Monospace Detection

### Rationale

1. **Higher Accuracy**: Font descriptor flag is ~99% accurate vs ~70% for name matching
2. **Follows OODA-02 Pattern**: Same structure used for bold/italic successfully
3. **Backward Compatible**: Fallback to name matching preserves legacy behavior
4. **Zero API Cost**: PDFium already provides this, just need to extract it

### Selected Approach

**Hybrid Detection with Priority**:

```rust
pub fn is_monospace(&self) -> bool {
    // OODA-03: Prefer font descriptor flag (99% accurate)
    if let Some(is_mono) = self.font_is_monospace {
        return is_mono;
    }
    // Fallback: font name pattern matching (70% accurate)
    // WHY: Legacy data or lopdf backend may not have descriptor
    self.font_name
        .as_ref()
        .map(|n| { /* pattern matching */ })
        .unwrap_or(false)
}
```

### Implementation Order

1. **RawChar** - Add field first (foundation)
2. **pdfium.rs** - Extract from PDFium (data source)
3. **Span** - Add field and update methods (consumer)
4. **Tests** - Fix existing + add new test
5. **Clippy + Commit**

### Risk Assessment

| Risk | Mitigation |
|------|------------|
| API missing in pdfium-render | Verified exists: `font_is_fixed_pitch()` |
| Whitespace inheritance | Use `last_is_monospace` tracking (same as bold/italic) |
| Test failures | Fix all struct initializers with new field |
