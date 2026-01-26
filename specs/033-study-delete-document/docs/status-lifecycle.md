# Document Status Lifecycle

**OODA-02**: Status-based deletion safety

---

## Status State Machine

```ascii
┌─────────────────────────────────────────────────────────┐
│              Document Status Lifecycle                   │
└─────────────────────────────────────────────────────────┘

    Upload Request
          │
          ├─────────────────────────────────────────────┐
          │                                             │
          ▼                                             ▼
    async_processing: true                    async_processing: false
          │                                             │
          ▼                                             ▼
    ┌──────────┐                              ┌────────────┐
    │ "pending"│                              │"processing"│
    └────┬─────┘                              └─────┬──────┘
         │                                          │
         │ Background task                          │ Inline processing
         │ picks up document                        │ completes
         ▼                                          │
    ┌────────────┐                                  │
    │"processing"│                                  │
    └────┬───┬───┘                                  │
         │   │                                      │
    Success  Failure                                │
         │   │                                      │
         ▼   ▼                                      ▼
    ┌────────────┐  ┌──────────┐         ┌────────────────┐
    │"completed" │  │ "failed" │         │   "processed"  │
    └─────┬──────┘  └────┬─────┘         │   (legacy)     │
          │              │               └───────┬────────┘
          │              │                       │
          └──────────────┴───────────────────────┘
                         │
                         ▼
                  DELETE Allowed
                         │
                         ▼
                  [Document Removed]
```

---

## Status Definitions

| Status       | Description                       | Delete Allowed? |
| ------------ | --------------------------------- | --------------- |
| `pending`    | Queued for async processing       | ❌ NO (409)     |
| `processing` | Currently being processed         | ❌ NO (409)     |
| `completed`  | Processing finished successfully  | ✅ YES          |
| `processed`  | Legacy status (same as completed) | ✅ YES          |
| `failed`     | Processing failed with error      | ✅ YES          |
| `unknown`    | Status not set (legacy documents) | ✅ YES          |

---

## Deletion Safety Rules

### WHY Status Checking Matters

Deleting a document while it's being processed can cause:

1. **Race Condition**: Background task writes data while deletion removes it
2. **Orphaned Data**: Entities/edges created AFTER deletion check starts
3. **Partial Deletion**: Some entities exist, others don't
4. **Data Corruption**: Inconsistent state in knowledge graph

### OODA-02 Safety Implementation

**Location**: `edgequake/crates/edgequake-api/src/handlers/documents.rs`

```rust
// OODA-02: Safety check - prevent deletion of documents still being processed
match document_status.as_str() {
    "pending" => {
        return Err(ApiError::Conflict(
            "Cannot delete document with status 'pending'. \
             Wait for processing to complete or cancel the task."
        ));
    }
    "processing" => {
        return Err(ApiError::Conflict(
            "Cannot delete document with status 'processing'. \
             Wait for processing to complete or cancel the task."
        ));
    }
    "completed" | "processed" | "failed" | "unknown" => {
        // OK to delete
    }
    other => {
        // Unknown status - allow deletion with warning
        tracing::warn!("Unknown document status: {}", other);
    }
}
```

---

## User Actions by Status

### Document in "pending" Status

**Scenario**: User uploaded document with `async_processing: true`, task queued but not started.

**User Options**:

1. Wait for processing to start and complete
2. Cancel the pending task (future: via task API)
3. Force delete (future: admin override)

**Error Response**:

```json
{
  "status": 409,
  "code": "CONFLICT",
  "message": "Cannot delete document 'doc-123' with status 'pending'. The document is queued for processing. Please wait for processing to complete or cancel the task."
}
```

### Document in "processing" Status

**Scenario**: Background task actively extracting entities and relationships.

**User Options**:

1. Wait for processing to complete
2. Cancel the processing task (future: via task API)
3. Check task status via `/api/v1/tasks/:task_id`

**Error Response**:

```json
{
  "status": 409,
  "code": "CONFLICT",
  "message": "Cannot delete document 'doc-123' with status 'processing'. The document is currently being processed. Please wait for processing to complete or cancel the task."
}
```

### Document in "failed" Status

**Scenario**: Processing failed, partial data may exist.

**Behavior**:

- Deletion proceeds normally
- Cascade logic cleans up any partial entities/edges
- Reference counting protects shared data
- All orphaned data removed

**Response**:

```json
{
  "status": 200,
  "deleted": true,
  "chunks_deleted": 3,
  "entities_affected": 2,
  "relationships_affected": 1
}
```

---

## Test Coverage

| Test Case                              | Status  | File                     |
| -------------------------------------- | ------- | ------------------------ |
| Delete pending document rejected       | ✅ PASS | e2e_document_deletion.rs |
| Delete processing document rejected    | ✅ PASS | e2e_document_deletion.rs |
| Delete completed document allowed      | ✅ PASS | e2e_document_deletion.rs |
| Delete failed document allowed         | ✅ PASS | e2e_document_deletion.rs |
| Multi-document shared entity preserved | ✅ PASS | e2e_document_deletion.rs |
| Orphaned edge cleanup                  | ✅ PASS | e2e_document_deletion.rs |

---

## Future Enhancements

### Task Cancellation API (Deferred)

```
POST /api/v1/tasks/:task_id/cancel
```

This would:

1. Set task status to "cancelling"
2. Background processor checks for cancellation
3. Abort processing and clean up partial data
4. Set document status to "cancelled"
5. Allow deletion

### Force Delete (Admin Only)

```
DELETE /api/v1/admin/documents/:id?force=true
```

This would:

1. Skip status check
2. Cancel any running background task
3. Clean up all data
4. Audit log the force deletion

---

## Related Documentation

- [OODA Loop Iteration 02](../ooda_loop/iteration_02/act.md)
- [Gap Analysis](../ooda_loop/iteration_02/orient.md)
- [Summary](./summary.md)
