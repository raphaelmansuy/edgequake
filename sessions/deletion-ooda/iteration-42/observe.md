# OODA-42: Observe

## Observation

EdgeQuake supports async processing mode:
- `async_processing: true` → returns immediately, processes in background
- `async_processing: false` → waits for processing to complete

## Gap

No explicit tests for async processing interaction with deletion.

## Evidence

Current tests use `async_processing: false`. Need to verify deletion works correctly with async documents.
