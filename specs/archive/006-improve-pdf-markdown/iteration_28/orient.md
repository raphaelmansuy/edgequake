# OODA-28 Orient: Analysis of Geometric Module

## Context

DBSCAN (Density-Based Spatial Clustering of Applications with Noise) is used for column detection because:

- No need to specify number of clusters a priori (unlike k-means)
- Handles arbitrary shapes and noise points
- Works with variable-width columns common in PDFs

## Risk Assessment

| Factor            | Risk   | Mitigation                          |
| ----------------- | ------ | ----------------------------------- |
| Complex algorithm | Medium | WHY comments explain key decisions  |
| Magic numbers     | High   | Document why specific values chosen |
| Edge cases        | Medium | Add tests for degenerate inputs     |

## Key Decisions to Document

1. **min_samples=3**: Minimum 3 points to form cluster core
   - WHY: Avoids noise from single outlier text spans
2. **10th percentile for eps**: Adaptive epsilon calculation
   - WHY: Captures tight clusters while ignoring outliers

3. **1D DBSCAN variant**: Separate implementation
   - WHY: More efficient for single-axis clustering (columns)

## Alignment with Mission

Mission 006 goals:

- ✅ High signal WHY comments → Adding to core algorithm
- ✅ Improve test coverage → Adding dbscan_1d tests
- ✅ Clean code → Documentation improves maintainability

## Decision

1. Add WHY comment to DBSCAN explaining the choice over histogram binning
2. Add 3 tests for `dbscan_1d()` function
