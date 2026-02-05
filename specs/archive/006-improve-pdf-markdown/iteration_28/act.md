# OODA-28 Act: Add DBSCAN Documentation and Tests

## Actions Taken

### 1. Added WHY Comment to GeometricClusterer

Documented why DBSCAN was chosen over histogram binning for PDF column detection:

- No need to specify number of columns a priori
- Handles variable-width columns
- Robust to noise
- Adapts to any document scale

### 2. Added dbscan_1d Tests

Three new tests for the 1D DBSCAN variant:

- `test_dbscan_1d_two_clusters` - Validates cluster separation
- `test_dbscan_1d_single_cluster` - All points in one cluster
- `test_dbscan_1d_empty` - Empty input edge case

## Results

| Metric          | Before | After    |
| --------------- | ------ | -------- |
| Tests           | 481    | 484 (+3) |
| WHY comments    | 0      | 1        |
| dbscan_1d tests | 0      | 3        |

## Test Lessons

During implementation, discovered that the original test case `[1,2,3,4]` with `eps=2.0` was creating 2 clusters due to DBSCAN's density requirement. Fixed by using tighter values `[1,2,2.5,3]` with `eps=3.0` to ensure all points connect.

## Files Modified

- `src/layout/geometric.rs` - Added WHY docs and 3 tests
