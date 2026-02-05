# OODA-27 Decide: Add Safe Truncate Tests

## Decision

Add unit tests for the `safe_truncate()` function to validate UTF-8 boundary handling.

## Implementation Plan

### Tests to Add

1. **test_safe_truncate_ascii** - ASCII string truncation
2. **test_safe_truncate_multibyte_boundary** - Multi-byte char at boundary
3. **test_safe_truncate_short_string** - String shorter than max
4. **test_safe_truncate_empty** - Empty string edge case

### Test Cases

```rust
// ASCII: standard truncation
assert_eq!(safe_truncate("Hello World", 5), "Hello");

// Multi-byte: Euro sign is 3 bytes, can't truncate at byte 2
assert_eq!(safe_truncate("€100", 2), ""); // Must go back to 0

// Short string: no truncation needed
assert_eq!(safe_truncate("Hi", 10), "Hi");

// Empty: edge case
assert_eq!(safe_truncate("", 5), "");
```

## Expected Outcome

- Tests: 477 → 481 (+4)
- Coverage: Validates UTF-8 boundary logic
