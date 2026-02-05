# OODA-26 Act: Add Rotation Detection Tests

## Actions Taken

Added 4 unit tests for `is_rotated_ctm()` in `src/backend/content_parser.rs`:

1. **test_normal_text_not_rotated** - Identity matrix, returns false
2. **test_90_ccw_rotation** - 90° counter-clockwise, returns true
3. **test_90_cw_rotation** - 90° clockwise, returns true
4. **test_small_angle_not_rotated** - 5° skew, returns false

## Results

| Metric                           | Before | After    |
| -------------------------------- | ------ | -------- |
| Tests                            | 473    | 477 (+4) |
| content_parser.rs rotation tests | 0      | 4        |
| Clippy warnings                  | 0      | 0        |

## Test Coverage

The tests validate the rotation detection threshold (0.1) is appropriate:

- Identity matrix [1,0,0,1,tx,ty]: a=1, d=1 → NOT rotated ✓
- 90° CCW [0,1,-1,0,tx,ty]: a=0, d=0 → rotated ✓
- 90° CW [0,-1,1,0,tx,ty]: a=0, d=0 → rotated ✓
- 5° skew [0.996,0.087,-0.087,0.996,tx,ty]: a≈1, d≈1 → NOT rotated ✓

## WHY This Matters

Proper rotation detection prevents arXiv watermarks (rotated 90°) from being merged with body text, improving extraction quality for academic papers.

## Files Modified

- `src/backend/content_parser.rs` - Added rotation detection tests
