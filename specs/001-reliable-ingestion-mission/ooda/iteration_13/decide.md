# OODA-13: Decide - Delete Document Fix Implementation

## Decision

Fix document deletion for "default" workspace by:

1. Adding "default" → UUID mapping in `get_workspace_vector_storage_strict()`
2. Adding "cancelled" to allowed deletion statuses

## Implementation Details

### Change 1: Workspace Mapping

**File**: `edgequake/crates/edgequake-api/src/handlers/documents.rs:108`

```rust
// OODA-13: Handle "default" workspace by mapping to the well-known UUID
// WHY: Documents created via default workspace are stored with workspace_id="default"
// but deletion/operations need a valid UUID for vector storage lookup.
// Default workspace UUID: 00000000-0000-0000-0000-000000000003
let effective_workspace_id = if workspace_id == "default" || workspace_id.is_empty() {
    "00000000-0000-0000-0000-000000000003"
} else {
    workspace_id
};
```

### Change 2: Cancelled Status

**File**: `edgequake/crates/edgequake-api/src/handlers/documents.rs:2062`

```rust
"completed" | "processed" | "partial_failure" | "failed" | "cancelled" | "unknown" => {
    // OK to delete
    // OODA-13: Added "cancelled" status to explicitly allow deletion after task cancellation
```

## Test Plan

1. Build with changes: `cargo build -p edgequake-api` ✅
2. Restart backend
3. Delete cancelled document
4. Verify deletion cascades (entities, relationships, vectors)

## Success Criteria

- [ ] Document deletion returns 200 OK
- [ ] Document gone from list
- [ ] Entities removed or updated
- [ ] Vector embeddings cleaned up
