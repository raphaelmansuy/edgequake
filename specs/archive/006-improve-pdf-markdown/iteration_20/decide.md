# Decide – OODA-20: Add Percentile Edge Case Tests

## Tests to Add

```rust
#[test]
fn test_percentile_single_element() {
    let data = vec![42.0];
    // Any percentile of single element returns that element
    assert_eq!(DocumentStats::percentile(&data, 0.0), 42.0);
    assert_eq!(DocumentStats::percentile(&data, 0.5), 42.0);
    assert_eq!(DocumentStats::percentile(&data, 1.0), 42.0);
}

#[test]
fn test_percentile_two_elements() {
    let data = vec![10.0, 20.0];
    // p=0.0 → index 0 (10.0)
    // p=0.5 → index 0.5 → 0 (10.0)
    // p=1.0 → index 1 (20.0)
    assert_eq!(DocumentStats::percentile(&data, 0.0), 10.0);
    assert_eq!(DocumentStats::percentile(&data, 1.0), 20.0);
}

#[test]
fn test_percentile_interpolation() {
    // Array [0, 1, 2, 3, 4, 5, 6, 7, 8, 9] (10 elements)
    let data: Vec<f32> = (0..10).map(|i| i as f32).collect();
    // p=0.1 → index 0.9 → 0 (floor) → 0.0
    assert_eq!(DocumentStats::percentile(&data, 0.1), 0.0);
    // p=0.3 → index 2.7 → 2 (floor) → 2.0
    assert_eq!(DocumentStats::percentile(&data, 0.3), 2.0);
    // p=0.5 → index 4.5 → 4 (floor) → 4.0
    assert_eq!(DocumentStats::percentile(&data, 0.5), 4.0);
    // p=0.9 → index 8.1 → 8 (floor) → 8.0
    assert_eq!(DocumentStats::percentile(&data, 0.9), 8.0);
}

#[test]
fn test_percentile_large_array() {
    // 100 elements: 1.0 to 100.0
    let data: Vec<f32> = (1..=100).map(|i| i as f32).collect();
    // p=0.10 → index 9.9 → 9 → 10.0 (value at index 9)
    assert_eq!(DocumentStats::percentile(&data, 0.10), 10.0);
    // p=0.50 → index 49.5 → 49 → 50.0
    assert_eq!(DocumentStats::percentile(&data, 0.50), 50.0);
    // p=0.90 → index 89.1 → 89 → 90.0
    assert_eq!(DocumentStats::percentile(&data, 0.90), 90.0);
}
```

## Location

Add to `src/processors/stats.rs` test module after existing tests.
