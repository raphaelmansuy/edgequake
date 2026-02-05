# OODA-31 Decide: Add Reading Order Tests

## Decision

Add 4 unit tests to reading_order.rs:

1. **test_reading_order_iter** - Test iterator method
2. **test_detector_default** - Test Default trait
3. **test_detector_with_tolerances** - Test custom tolerance constructor
4. **test_from_xy_cut_order** - Test XY-cut conversion

## Implementation

```rust
#[test]
fn test_reading_order_iter() {
    let order = ReadingOrder::new(vec![2, 0, 3, 1]);
    let collected: Vec<usize> = order.iter().collect();
    assert_eq!(collected, vec![2, 0, 3, 1]);
}

#[test]
fn test_detector_default() {
    let detector = ReadingOrderDetector::default();
    // WHY: Default line tolerance is 3.0 (matches pymupdf4llm)
    assert!((detector.line_tolerance - 3.0).abs() < 0.001);
}

#[test]
fn test_detector_with_tolerances() {
    let detector = ReadingOrderDetector::with_tolerances(5.0, 25.0);
    assert!((detector.line_tolerance - 5.0).abs() < 0.001);
}

#[test]
fn test_from_xy_cut_order() {
    let detector = ReadingOrderDetector::new();
    let xy_order = vec![3, 1, 2, 0];
    let reading_order = detector.from_xy_cut_order(&xy_order);
    assert_eq!(reading_order.order, vec![3, 1, 2, 0]);
    assert!((reading_order.confidence - 1.0).abs() < 0.001);
}
```

## Expected Outcome

- Tests: 490 → 494 (+4)
- All ReadingOrder and ReadingOrderDetector constructors/methods tested
