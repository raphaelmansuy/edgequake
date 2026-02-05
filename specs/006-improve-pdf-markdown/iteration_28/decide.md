# OODA-28 Decide: Add DBSCAN Documentation and Tests

## Decision

Add WHY documentation to the core DBSCAN implementation and tests for `dbscan_1d()`.

## Implementation Plan

### 1. Add WHY Comment to GeometricClusterer

Explain why DBSCAN was chosen over histogram binning for PDF column detection.

### 2. Add Tests for dbscan_1d()

```rust
#[test]
fn test_dbscan_1d_simple() {
    // Two clear clusters
    let values = vec![1.0, 1.5, 2.0, 10.0, 10.5, 11.0];
    let clusters = dbscan_1d(&values, 2.0, 2);
    assert_eq!(clusters.len(), 2);
}

#[test]
fn test_dbscan_1d_single_cluster() {
    let values = vec![1.0, 2.0, 3.0, 4.0];
    let clusters = dbscan_1d(&values, 2.0, 2);
    assert_eq!(clusters.len(), 1);
}

#[test]
fn test_dbscan_1d_empty() {
    let values: Vec<f32> = vec![];
    let clusters = dbscan_1d(&values, 2.0, 2);
    assert!(clusters.is_empty());
}
```

## Expected Outcome

- Tests: 481 → 484 (+3)
- WHY comments: 0 → 1
- Coverage: dbscan_1d now has tests
