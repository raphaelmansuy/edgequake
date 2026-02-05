# OODA-13: Decide - Add WHY Comment

## Decision

Add WHY comment for the space width ratio in pdfium.rs.

## Implementation Plan

### Change: line 266 (space width)

```rust
// WHY (OODA-13): Space width = 25% of font size is a conservative estimate.
// Proportional fonts: 0.2-0.3 of em. Monospace: ~0.6 of em.
// 0.25 works well for word boundary detection in both font types.
last_x1 + fs * 0.25,
```

## Risk Assessment

- **Risk**: Very low - single comment addition
- **Benefit**: Documents space width synthesis logic

## Success Criteria

- [ ] Comment added
- [ ] Tests pass
- [ ] No clippy warnings
