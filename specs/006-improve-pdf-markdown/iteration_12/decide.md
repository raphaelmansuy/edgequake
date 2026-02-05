# OODA-12: Decide - Add WHY Comments

## Decision

Add WHY comments to undocumented constants in block_classifier.rs.

## Implementation Plan

### Change 1: Heading level ratios (lines 133, 135)
```rust
let ratio = dominant_size / body_font_size;
// WHY (OODA-12): Heading level based on size ratio to body text.
// - 2.0x: Very large (double body) = major heading (#)
// - 1.7x: Large (70% bigger) = secondary heading (##)
// - 1.5x: Medium = default to # (conservative)
let level = if ratio >= 2.0 {
    1 // Very large = #
} else if ratio >= 1.7 {
    2 // Large = ##
} else {
    1 // Title = # (most conservative)
};
```

### Change 2: Uppercase ratio (line 291)
```rust
// WHY (OODA-12): 50% uppercase threshold for all-caps section detection.
// True all-caps = 100%, but OCR/extraction may have errors.
// 50% catches "ABSTRACT", "REFERENCES" with some lowercase mixed in.
alpha_count > 0 && (uppercase_count as f32 / alpha_count as f32) >= 0.5
```

## Risk Assessment

- **Risk**: Low - comments only
- **Benefit**: High - documents heading classification logic

## Success Criteria

- [ ] All constants have WHY comments
- [ ] Tests pass
- [ ] No clippy warnings
