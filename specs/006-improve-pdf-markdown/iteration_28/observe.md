# OODA-28 Observe: Geometric Module Documentation Gap

## Current State

The `geometric.rs` module (600 lines) implements DBSCAN clustering for column detection but has 0 WHY comments despite containing complex algorithms.

## File Analysis

- **Size**: 600 lines
- **WHY comments**: 0
- **Tests**: 7
- **Total lib tests**: 481

## Key Functions Lacking WHY Comments

1. `dbscan()` - Core clustering algorithm, why DBSCAN for PDFs?
2. `range_query()` - Why squared distance optimization?
3. `calculate_eps_from_distribution()` - Why 10th percentile?
4. `dbscan_1d()` - Why separate 1D implementation?

## Observations

1. The module is well-structured but assumes reader knows DBSCAN
2. Magic numbers like `min_samples: 3` and percentile 0.10 lack explanation
3. The choice of DBSCAN over histogram binning is mentioned but not why

## Test Coverage Gaps

1. No test for `dbscan_1d()` function
2. No test for degenerate cases (1-2 points)

## Recommendation

1. Add WHY comments to key functions explaining algorithm choices
2. Add tests for `dbscan_1d()` function
3. Add tests for edge cases (very few points)
