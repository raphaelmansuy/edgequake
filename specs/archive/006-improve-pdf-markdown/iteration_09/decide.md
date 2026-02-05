# OODA-09: Decide - Add WHY Comments for Magic Numbers

## Decision

Add WHY comments to all undocumented magic numbers in text_grouping.rs.

## Implementation Plan

### Change 1: Line 307 (100.0pt top zone)

```rust
// WHY: 100pt (~13% of US Letter height) captures title/author zone.
// Elements in this region are logged for debugging header classification.
if elem.y < 100.0 {
```

### Change 2: Lines 407, 413 (15-80pt author zone)

```rust
// WHY: Author zone is 15-80pt from page top.
// - 15pt: Below header margin
// - 80pt: Above abstract (typically starts ~100pt)
// US Letter 792pt tall, so this is ~2-10% of page.
let in_author_zone = elem.y > 15.0 && elem.y < 80.0;
```

### Change 3: Lines 566-567 (30pt gap threshold)

```rust
// WHY: 30pt gap (~4% of page height) indicates section boundary.
// Single-spaced text has ~12-14pt line height, so 30pt = 2+ blank lines.
// This separates main content from bottom sections (references, acknowledgments).
let (left_main, left_bottom) = self.split_by_vertical_gap(left_lines, 30.0);
```

### Change 4: Line 422 (20pt with short text)

```rust
// WHY: Short text (<30 chars) below 20pt might be page number or header.
|| (elem.text.len() < 30 && elem.y > 20.0);
```

## Risk Assessment

- **Risk**: Low - comments only, no logic changes
- **Benefit**: High - improves code documentation

## Success Criteria

- [ ] All magic numbers have WHY comments
- [ ] All tests still pass
- [ ] No clippy warnings
