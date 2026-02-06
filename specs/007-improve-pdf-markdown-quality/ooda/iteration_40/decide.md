# IT40 — Decide

## Actions

1. **Modify `Span::can_append()` in `pymupdf_structs.rs`**:
   - Make space threshold font-aware
   - Monospace: 33% (unchanged)
   - Proportional: 22%

2. **Update WHY comments** to explain font-aware logic

3. **Run tests** — verify no regressions

4. **Validate output**:
   - Elitizon: spaces restored ("Executive summary")
   - LightRAG: no regressions (already uses explicit spaces)

## Scope

- Single file change: `src/layout/pymupdf_structs.rs`
- ~10 lines modified
- Focused fix for missing spaces in proportional fonts

## Code Change

```rust
// OODA-IT40: Font-aware space threshold
// WHY: Monospace fonts have wide inter-char spacing (25-28%), proportional don't (5-15%)
let space_threshold = if self.font_is_monospace.unwrap_or(false) {
    // Monospace: inter-char spacing can reach 28%, space ~26%
    // Keep 33% to avoid false word boundaries (OODA-IT32)
    self.font_size * 0.33
} else {
    // Proportional: inter-char spacing ~5-15%, space ~20-25%
    // Use 22% to catch word boundaries while preserving kerned pairs
    self.font_size * 0.22
};
```

## Risk Assessment

**Low risk**:
- Monospace behavior unchanged (33% threshold preserved)
- Proportional fonts get lower threshold (22%) — closer to original 25%
- Change is isolated to span grouping, no downstream effects
