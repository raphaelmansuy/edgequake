# OODA-27 Act: Add Safe Truncate Tests

## Actions Taken

Added 4 unit tests for `safe_truncate()` in `src/backend/block_builder.rs`:

1. **test_safe_truncate_ascii** - ASCII string truncation
2. **test_safe_truncate_multibyte_boundary** - Euro sign boundary handling
3. **test_safe_truncate_short_string** - No truncation needed
4. **test_safe_truncate_box_drawing** - Box drawing character boundaries

## Results

| Metric | Before | After |
|--------|--------|-------|
| Tests | 477 | 481 (+4) |
| safe_truncate tests | 0 | 4 |
| Clippy warnings | 0 | 0 |

## Test Coverage

The tests validate UTF-8 boundary handling:
- ASCII: Direct truncation at byte position ✓
- Multi-byte (Euro '€'=3 bytes): Falls back to char boundary ✓
- Short strings: No truncation needed ✓
- Box drawing ('─'=3 bytes): Validates non-ASCII boundaries ✓

## WHY This Matters

Without safe truncation, log statements like:
```rust
safe_truncate(text, 80)
```
could panic if byte 80 falls in the middle of a multi-byte character. This is common in academic PDFs with special characters.

## Files Modified

- `src/backend/block_builder.rs` - Added safe_truncate tests
