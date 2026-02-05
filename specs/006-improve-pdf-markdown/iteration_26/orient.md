# OODA-26 Orient: Analysis of Rotation Detection Testing Gap

## Context

The `is_rotated_ctm()` function detects text rotated 90 degrees (like arXiv watermarks in margins). This is critical for proper text grouping - rotated text should NOT be merged with body text.

## Risk Assessment

| Factor          | Risk   | Mitigation                         |
| --------------- | ------ | ---------------------------------- |
| False positives | High   | Could reject legitimate text       |
| False negatives | Medium | Would merge watermarks into body   |
| Edge cases      | Medium | Near-threshold values need testing |

## The Function Logic

```rust
fn is_rotated_ctm(ctm: &[f32; 6]) -> bool {
    let a = ctm[0].abs();
    let d = ctm[3].abs();
    a < 0.1 && d < 0.1  // Both diagonal elements near zero = 90° rotation
}
```

**Matrix interpretation:**

- Normal: [1, 0, 0, 1, tx, ty] → a=1, d=1 → NOT rotated
- 90° CCW: [0, 1, -1, 0, tx, ty] → a=0, d=0 → rotated
- 90° CW: [0, -1, 1, 0, tx, ty] → a=0, d=0 → rotated

## Alignment with Mission

Mission 006 goals:

- ✅ Improve test coverage → Adding tests for rotation detection
- ✅ Clean code → Tests validate edge case handling
- ✅ Quality extraction → Prevents watermark contamination

## Decision

Add 4 unit tests for `is_rotated_ctm()` covering normal and rotated matrices.
