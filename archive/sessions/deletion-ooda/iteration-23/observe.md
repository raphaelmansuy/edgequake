# OODA-23 Observe: Metrics History E2E Tests

## Mission Context

OODA-20/21/22 implemented metrics recording and history API.
Now we need tests to verify the functionality works end-to-end.

## Test Requirements

### 1. Test Metrics Snapshot Recording

- Upload a document → verify snapshot recorded
- Delete a document → verify snapshot recorded
- Check that trigger_type is "event"

### 2. Test Metrics History Query

- Get history with default pagination
- Get history with custom limit/offset
- Verify reverse chronological order

### 3. Test Edge Cases

- Empty history for new workspace
- History isolation between workspaces

## Test Strategy

Since we're using in-memory storage for tests, the metrics
recording will use the stub implementation. We need to:

1. Test the API endpoint returns correct structure
2. Test pagination parameters work
3. For real testing, need PostgreSQL integration tests

## Files to Create

1. `e2e_metrics_history.rs` - New test file
