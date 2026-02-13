# Analysis - Iteration 21

## Gap: No E2E tests for lineage methods in any SDK

Mission explicitly requires E2E metadata tests for each SDK. Without these tests,
we can't verify that SDK types and paths match the real API for lineage endpoints.

## Approach

Add lineage E2E tests following existing patterns in each SDK:
- 3 tests per SDK: document lineage, document metadata, chunk lineage
- Non-destructive (read-only operations on existing documents)
- Graceful handling when no documents exist or chunks are not found
