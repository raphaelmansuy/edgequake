# OODA-13: Observe - Delete Document with Default Workspace

## Mission Criterion

> "Ensure delete document works fully (including PDF storage cleanup)"

## Initial Test (FAILED)

```bash
# Attempt to delete a cancelled document
curl -X DELETE http://localhost:8080/api/v1/documents/d578ab1e-9651-4a5d-904e-f9ae2663cf85
```

**Error Response**:
```json
{
  "code": "BAD_REQUEST",
  "message": "Invalid workspace ID 'default': invalid character: expected an optional prefix of `urn:uuid:` followed by [0-9a-fA-F-], found `u` at 5. Document ingestion requires a valid workspace."
}
```

## Root Cause Analysis

### The Problem

Documents stored with `workspace_id: "default"` fail deletion because:

1. Document metadata stores: `"workspace_id": "default"`
2. `get_workspace_vector_storage_strict()` tries to parse "default" as UUID
3. UUID parsing fails → API returns BadRequest

### Discovery Path

```
documents.rs:get_workspace_vector_storage_strict()
  └─ Uuid::parse_str("default")
     └─ Err: "invalid character... found 'u' at 5"
```

### Code Discrepancy

**processor.rs** (correctly handles "default"):
```rust
// processor.rs:404
if workspace_id.is_empty() || workspace_id == "default" {
    // Uses default storage
}
```

**documents.rs** (was missing this check):
```rust
// documents.rs:111
let workspace_uuid = match Uuid::parse_str(workspace_id) {
    // No handling for "default" string!
}
```

## Default Workspace Mapping

From `workspace_service_impl.rs:77`:
```rust
let default_workspace_id = Uuid::parse_str("00000000-0000-0000-0000-000000000003")
```

So "default" → `00000000-0000-0000-0000-000000000003`

## Secondary Issue: Missing "cancelled" Status

The deletion status check was also missing "cancelled":

```rust
// Before (missing cancelled)
"completed" | "processed" | "partial_failure" | "failed" | "unknown" => { OK }

// After (added cancelled)
"completed" | "processed" | "partial_failure" | "failed" | "cancelled" | "unknown" => { OK }
```
