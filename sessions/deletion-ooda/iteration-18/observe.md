# OODA-18 Observe: Document Reprocessing Deep Analysis

## Mission Requirement

From specs/033-study-delete-document/003-study-document.md:

> "Impact of reprocessing a document must be fully studied, and handled correctly."
> "What happens when reprocessing a document that was partially processed, failed processing, or is in the middle of processing?"
> "Ensure no dangling data remains, no shared data is deleted."

## Current Reprocessing Tests (e2e_document_deletion.rs)

### Existing Tests

1. **test_reprocess_cleans_partial_graph_data** (~line 1850)
   - Tests that reprocessing cleans up old entities before re-extracting
   - Verifies old entities are removed

2. **test_reprocess_preserves_shared_entities** (~line 1926)
   - Tests that shared entities from other documents are preserved
   - Verifies reference tracking works during reprocess

3. **test_recover_stuck_cleans_partial_graph_data** (~line 2008)
   - Tests recovery of stuck/processing documents
   - Verifies partial data is cleaned

## Gaps Identified

### Missing Edge Cases

1. **Reprocess during active processing**
   - What if document is PROCESSING and reprocess is requested?
   - Should reject or queue?

2. **Reprocess of FAILED document**
   - Should clean up partial data from failed attempt
   - Current tests may not cover this explicitly

3. **Reprocess with changed content**
   - Old entities should be removed
   - New entities from new content should appear

4. **Reprocess with identical content**
   - Idempotent behavior expected
   - Entity counts should remain same

5. **Concurrent reprocess requests**
   - What happens if two reprocess requests come simultaneously?
   - Race condition potential

6. **Reprocess of document with embeddings**
   - Old embeddings should be deleted
   - New embeddings should be created

## Code Analysis: Reprocess Flow

### Location: handlers/documents.rs

The reprocess endpoint likely:

1. Checks document status
2. Cleans up old chunks/entities/embeddings
3. Re-runs pipeline
4. Updates document status

Let me verify the actual implementation...
