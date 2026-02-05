# Iteration 06: Act

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Changes Implemented

### Added Kerning Overlap WHY Comment

**File**: `layout/pymupdf_structs.rs:199-205`

```rust
// If gap is negative (overlapping or backwards), reject
// unless it's minor overlap from kerning
// WHY 0.3 * avg_char_width: Kerning in proportional fonts can cause
// characters to overlap slightly (e.g., "AV", "To"). Allowing 30%
// overlap tolerance preserves kerned pairs while rejecting truly
// overlapping text (which indicates layout issues or vertical text).
let avg_char_width = (self.x1 - self.x0) / self.text.len().max(1) as f32;
```

## Verification

```bash
$ cargo test -p edgequake-pdf --lib
test result: ok. 450 passed; 0 failed
```

## Impact

All magic numbers in `can_append()` are now documented:

| Threshold | WHY | OODA |
|-----------|-----|------|
| 0.5pt font size | PDF rounding errors | OODA-05 |
| 0.3 * font_size y | Subscript detection | OODA-05 |
| 0.25 * font_size space | Word boundary | (existing) |
| 0.3 * avg_width overlap | Kerning tolerance | OODA-06 |
