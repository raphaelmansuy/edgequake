# OODA-13: Orient - Delete Document Fix Analysis

## First Principles

### Why This Bug Existed

1. **Dual ID representation**: Workspace has both string name ("default") and UUID
2. **Inconsistent storage**: Documents store string, but operations need UUID
3. **Missing mapping**: `get_workspace_vector_storage_strict()` in documents.rs lacked the "default" → UUID mapping

### Impact Assessment

| Impact | Severity | Description |
|--------|----------|-------------|
| Delete blocked | High | Users cannot delete documents in "default" workspace |
| Cleanup stuck | High | Cancelled documents accumulate |
| Data orphans | Medium | Cannot clean up stale data |

## Proposed Fix

### Change 1: Add "default" → UUID Mapping

**File**: `documents.rs:get_workspace_vector_storage_strict()`

```rust
// Map "default" to well-known UUID
let effective_workspace_id = if workspace_id == "default" || workspace_id.is_empty() {
    "00000000-0000-0000-0000-000000000003"
} else {
    workspace_id
};
```

### Change 2: Add "cancelled" to Allowed Statuses

**File**: `documents.rs:delete_document()`

```rust
"completed" | "processed" | "partial_failure" | "failed" | "cancelled" | "unknown" => {
    // OK to delete
}
```

## Risk Assessment

| Change | Risk | Mitigation |
|--------|------|------------|
| Mapping "default" | Low | Well-defined default UUID, same as processor.rs |
| Adding "cancelled" | Low | Cancelled is a terminal state, safe to delete |

## Alternatives Considered

1. **Store UUID everywhere**: Too invasive, would break existing documents
2. **Create migration**: Complex, not needed for simple mapping
3. **Document the workaround**: Poor UX, users shouldn't need to know internals

**Chosen**: Add mapping in code (simplest, consistent with processor.rs)
