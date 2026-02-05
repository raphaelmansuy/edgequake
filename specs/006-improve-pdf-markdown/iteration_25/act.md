# OODA-25 Act: Add Tests to Elements Module

## Actions Taken

Added 4 unit tests to `src/backend/elements.rs`:

1. **test_raw_char_dimensions** - Validates width/height calculation
2. **test_raw_char_center_point** - Validates center coordinate calculation
3. **test_raw_char_zero_size** - Edge case for point-sized characters
4. **test_raw_char_large_coordinates** - Real-world PDF coordinate values

## Results

| Metric            | Before | After    |
| ----------------- | ------ | -------- |
| Tests             | 469    | 473 (+4) |
| elements.rs tests | 0      | 4        |
| Clippy warnings   | 0      | 0        |

## Test Details

All tests use a helper function `make_char()` to reduce boilerplate:

```rust
fn make_char(x0: f32, y0: f32, x1: f32, y1: f32) -> RawChar {
    RawChar { char: 'A', x0, y0, x1, y1, ... }
}
```

Tests validate:

- Basic arithmetic: width = x1 - x0, height = y1 - y0
- Center computation: (min + max) / 2
- Zero dimensions don't cause division errors
- Large coordinates (real PDF values) work correctly

## Files Modified

- `src/backend/elements.rs` - Added tests module
