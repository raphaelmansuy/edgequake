# OODA-18 Orient: Document Reprocessing Analysis

## Current Coverage Assessment

### Existing Tests (VERIFIED)

| Test | Coverage | Status |
|------|----------|--------|
| `test_reprocess_cleans_partial_graph_data` | Failed doc cleanup | ✅ Covered |
| `test_reprocess_preserves_shared_entities` | Shared entity safety | ✅ Covered |
| `test_recover_stuck_cleans_partial_graph_data` | Stuck doc cleanup | ✅ Covered |

### Implementation Analysis

The `reprocess_failed` handler (documents.rs:3095):
1. Finds documents with `status: "failed"`
2. Calls `cleanup_document_graph_data()` BEFORE requeueing
3. Updates status to "pending"
4. Creates new task for processing queue

This is correct behavior per GAP-08 requirements.

## Identified Gaps

### Gap A: Idempotent Reprocess Test
No test verifies that reprocessing identical content produces same result.
- WHY MATTERS: Ensures deterministic behavior

### Gap B: Concurrent Reprocess Test
No test for two simultaneous reprocess requests.
- WHY MATTERS: Race conditions could corrupt source_ids

### Gap C: Reprocess with Changed Content
No test for reprocessing after content update.
- WHY MATTERS: Old entities must be removed, new ones created

### Gap D: PROCESSING → Reprocess Rejection
No test verifying reprocess request is rejected for PROCESSING documents.
- WHY MATTERS: Could cause data corruption

## First Principles Assessment

### Core Invariants

1. **Source ID Integrity**: After reprocess, `source_ids` should contain exactly one reference per document
2. **No Duplicates**: Entity deduplication must work correctly on reprocess
3. **Shared Safety**: Entities shared with other documents must not be deleted
4. **Cleanup First**: Partial data must be cleaned before requeueing

### Current Implementation Strengths

- Cleanup happens BEFORE requeueing (good)
- Uses `cleanup_document_graph_data()` which handles source_ids correctly
- Error handling logs but continues (resilient)

### Current Implementation Weaknesses

- No guard against concurrent reprocess requests
- No verification that document is actually FAILED before cleanup

## Priority for OODA-18

Based on signal value and risk:

1. **HIGH**: Add test for PROCESSING document rejection
2. **MEDIUM**: Add test for concurrent reprocess (race condition)
3. **LOW**: Add idempotency test (nice to have)

## Decision

Focus on HIGH priority: Test that reprocess is rejected for PROCESSING documents.

This is a safety-critical gap that could cause data corruption if not handled.
