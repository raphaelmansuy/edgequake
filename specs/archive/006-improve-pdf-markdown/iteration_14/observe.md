# OODA-14: Observe - Missing Heading Level Tests

## Current State

The `test_block_classifier` test only checks:

- H1 detection (24pt font on 12pt body = 2.0x ratio)
- List item detection

## Gap Identified

Missing tests for:

1. H2 detection (1.7-2.0x ratio)
2. Paragraph detection (< 1.5x ratio)
3. Edge case at exactly 1.5x threshold
4. Edge case at exactly 1.7x threshold

## Evidence

From the classification logic:

```rust
let level = if ratio >= 2.0 {
    1 // Very large = #
} else if ratio >= 1.7 {
    2 // Large = ##
} else {
    1 // Title = # (most conservative)
};
```

Current test only covers ratio >= 2.0 case (24pt / 12pt = 2.0).

## Data Needed

- H2 test: Need 17-19pt font on 10pt body (1.7-1.9x ratio)
- Paragraph test: Need 10pt font on 10pt body (1.0x ratio)
