# OODA-10: Decide - Add WHY Comments

## Decision

Add WHY comments to all undocumented constants in pymupdf_grouper.rs.

## Implementation Plan

### Change 1: column_overlap: 0.5 (line 110)
```rust
// WHY: 50% horizontal overlap threshold for same-column detection.
// Two blocks are "same column" if X ranges overlap by 50%+.
// This handles indented paragraphs while preventing adjacent column merging.
column_overlap: 0.5,
```

### Change 2: COLUMN_GAP_THRESHOLD (line 302)
Already has inline comment. Add WHY prefix:
```rust
// WHY: 10pt is less than typical column gutter (14-20pt) but larger than
// word gaps (<5pt). Provides margin for detection uncertainty.
const COLUMN_GAP_THRESHOLD: f32 = 10.0;
```

### Change 3: page_width < 100.0 (line 497)
```rust
// WHY: 100pt ≈ 1.4 inches is too small for readable content.
// Typical pages: US Letter (612pt), A4 (595pt).
if page_width < 100.0 {
```

## Risk Assessment

- **Risk**: Low - comments only
- **Benefit**: High - improves maintainability

## Success Criteria

- [ ] All constants have WHY comments
- [ ] Tests pass
- [ ] No clippy warnings
