# Decide – OODA-19: Add Font Analysis Edge Case Tests

## Decision

Add tests for median calculation edge cases and document-level font detection.

## Tests to Add

```rust
#[test]
fn test_median_even_count() {
    let analyzer = FontAnalyzer::new();
    // Even count: median is middle element (n/2)
    assert_eq!(analyzer.calculate_median(vec![10.0, 11.0, 12.0, 13.0]), 12.0);
}

#[test]
fn test_median_single_element() {
    let analyzer = FontAnalyzer::new();
    assert_eq!(analyzer.calculate_median(vec![14.0]), 14.0);
}

#[test]
fn test_median_two_elements() {
    let analyzer = FontAnalyzer::new();
    // Two elements: n/2 = 1, so second element
    assert_eq!(analyzer.calculate_median(vec![10.0, 14.0]), 14.0);
}

#[test]
fn test_median_with_outliers() {
    let analyzer = FontAnalyzer::new();
    // Outliers don't affect median (unlike mean)
    // [4, 4, 4, 10, 12, 12, 12, 48, 72] median = 12
    assert_eq!(
        analyzer.calculate_median(vec![4.0, 48.0, 12.0, 72.0, 4.0, 12.0, 12.0, 4.0, 10.0]),
        12.0
    );
}

#[test]
fn test_valid_size_boundary() {
    let analyzer = FontAnalyzer::new();
    // Boundary conditions
    assert!(analyzer.is_valid_size(4.0)); // Min valid
    assert!(analyzer.is_valid_size(72.0)); // Max valid
    assert!(!analyzer.is_valid_size(3.9)); // Just below min
    assert!(!analyzer.is_valid_size(72.1)); // Just above max
}
```

## Location

Add to `src/processors/font_analysis.rs` in the `#[cfg(test)] mod tests` section.
