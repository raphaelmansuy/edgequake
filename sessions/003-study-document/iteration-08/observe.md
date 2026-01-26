# Iteration 08: OBSERVE - Reprocess Endpoints Missing Cleanup Logic

## Date
2025-01-28

## Focus Area
Verify that `reprocess_failed` and `recover_stuck` endpoints clean up partial data before requeueing.

## Observation Method
Code analysis of reprocessing endpoints in `documents.rs`.

## Key Findings

### 1. CHANGE-IT03-02 Was Not Implemented

In iteration 03, we identified that `reprocess_failed` and `recover_stuck` should clean up partial data before requeueing documents. However, this cleanup logic was **never implemented**.

### 2. Current reprocess_failed Flow (documents.rs:2891-3032)

```
┌──────────────────────────────────────────────────┐
│               reprocess_failed()                 │
├──────────────────────────────────────────────────┤
│ 1. Find failed documents by scanning KV metadata │
│ 2. For each failed document:                     │
│    a. Update status to "pending"                 │
│    b. Set new track_id                           │
│    c. Create new TextInsertData task             │
│    d. Enqueue to task manager                    │
│ 3. Return requeued count                         │
│                                                  │
│ ❌ NO CLEANUP of:                                │
│    - Partial entities from failed attempt        │
│    - Partial edges from failed attempt           │
│    - Partial chunk embeddings                    │
└──────────────────────────────────────────────────┘
```

### 3. Current recover_stuck Flow (documents.rs:3034-3200)

Same issue - no cleanup before requeueing.

```
┌──────────────────────────────────────────────────┐
│               recover_stuck()                    │
├──────────────────────────────────────────────────┤
│ 1. Find documents stuck in "processing" state   │
│    for longer than threshold (default 30 min)   │
│ 2. For each stuck document:                     │
│    a. Update status to "pending"                │
│    b. Create new TextInsertData task            │
│    c. Enqueue to task manager                   │
│ 3. Return recovered count                       │
│                                                  │
│ ❌ NO CLEANUP of:                                │
│    - Partial entities from interrupted attempt  │
│    - Partial edges from interrupted attempt     │
│    - Partial chunk embeddings                   │
└──────────────────────────────────────────────────┘
```

### 4. Problem Scenario

```
Timeline:
T1: Document A uploaded, processing starts
T2: Processing fails at 60% (some entities created)
T3: User calls reprocess_failed
T4: Document A reprocessed from scratch
T5: Now entities exist TWICE (from T2 and T4)

Result:
- Duplicate entities with same name
- source_ids now contains document_id twice
- Delete document A → entities still exist (reference count > 1 but actually only one doc)
```

### 5. Existing cleanup_document_data Function

Looking at the delete flow, I found `cleanup_document_data` helper function:

```rust
// documents.rs - delete_document function
// Lines ~1420-1600 contain the cleanup logic
```

Let me verify if there's a reusable helper:

### 6. Code Search for Cleanup Functions

No `cleanup_document_graph_data` or similar helper function exists. The cleanup logic is embedded directly in `delete_document` function.

## Risk Assessment

| Severity | Impact | Scenario |
|----------|--------|----------|
| **HIGH** | Data duplication | Reprocessed documents create duplicate entities |
| **HIGH** | Incorrect deletion | Delete after reprocess may leave orphaned data |
| **MEDIUM** | Storage bloat | Partial data accumulates from failed attempts |

## Evidence

**File**: `documents.rs:2891-3032` - `reprocess_failed` function
- No call to any cleanup function
- Only updates status and requeues

**File**: `documents.rs:3034-3200` - `recover_stuck` function
- Same pattern - no cleanup

## Conclusion

**GAP-08 CONFIRMED**: Reprocess endpoints do not clean up partial data before requeueing documents. This violates the mission requirement:

> "Ensure there is reprocessing mechanism for failed documents. Ensure deleting a failed document cleans up all partial data."

We need to:
1. Extract cleanup logic from `delete_document` into reusable helper
2. Call cleanup before requeueing in `reprocess_failed`
3. Call cleanup before requeueing in `recover_stuck`
