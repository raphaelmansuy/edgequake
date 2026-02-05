# OODA-25 Decide: Add Tests to Elements Module

## Decision

Add unit tests to `elements.rs` for the `RawChar` helper methods.

## Implementation Plan

### Tests to Add

1. **test_raw_char_dimensions** - Basic width/height calculation
2. **test_raw_char_center_point** - Center coordinate calculation
3. **test_raw_char_zero_size** - Edge case: point-sized char
4. **test_raw_char_large_coordinates** - Real-world PDF values

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_char(x0: f32, y0: f32, x1: f32, y1: f32) -> RawChar { ... }

    #[test] fn test_raw_char_dimensions() { ... }
    #[test] fn test_raw_char_center_point() { ... }
    #[test] fn test_raw_char_zero_size() { ... }
    #[test] fn test_raw_char_large_coordinates() { ... }
}
```

## Expected Outcome

- Tests: 469 → 473 (+4)
- Coverage: Fills gap in elements.rs
- Documentation: Tests show expected behavior
