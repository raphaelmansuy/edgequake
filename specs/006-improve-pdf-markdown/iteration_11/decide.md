# OODA-11: Decide - Add WHY Comments

## Decision

Add WHY comments to undocumented constants in markdown.rs.

## Implementation Plan

### Change 1: line 248 (list indentation)
```rust
// WHY (OODA-11): 72pt = 1 inch = standard PDF left margin.
// 20pt ≈ 0.28" per level = standard typographic indent step.
// Formula: (indent - margin) / step_size = nesting level
let lvl = ((indent - 72.0).max(0.0) / 20.0).floor() as usize;
```

### Change 2: line 601 (table row Y-tolerance)
```rust
// WHY (OODA-11): 10pt Y-tolerance for same-row detection.
// Matches other tolerances (block_gap, line joining) in codebase.
// Cells on same row should have Y positions within 10pt.
if (y - prev_y).abs() > 10.0 {
```

## Risk Assessment

- **Risk**: Low - comments only
- **Benefit**: High - clarifies list/table rendering logic

## Success Criteria

- [ ] All constants have WHY comments
- [ ] Tests pass
- [ ] No clippy warnings
