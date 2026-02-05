# OODA-26 Decide: Add Rotation Detection Tests

## Decision

Add unit tests for `is_rotated_ctm()` to validate rotation detection logic.

## Implementation Plan

### Tests to Add

1. **test_normal_text_not_rotated** - Identity matrix, should return false
2. **test_90_ccw_rotation** - Counter-clockwise rotation, should return true
3. **test_90_cw_rotation** - Clockwise rotation, should return true
4. **test_small_angle_not_rotated** - Slight skew, should return false

### Test Cases

```rust
// Normal text: [1, 0, 0, 1, 0, 0]
assert!(!ContentParser::is_rotated_ctm(&[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]));

// 90° CCW: [0, 1, -1, 0, 0, 0]
assert!(ContentParser::is_rotated_ctm(&[0.0, 1.0, -1.0, 0.0, 0.0, 0.0]));

// 90° CW: [0, -1, 1, 0, 0, 0]
assert!(ContentParser::is_rotated_ctm(&[0.0, -1.0, 1.0, 0.0, 0.0, 0.0]));

// Slight skew: [0.98, 0.1, -0.1, 0.98, 0, 0]
assert!(!ContentParser::is_rotated_ctm(&[0.98, 0.1, -0.1, 0.98, 0.0, 0.0]));
```

## Expected Outcome

- Tests: 473 → 477 (+4)
- Validation of critical rotation detection function
