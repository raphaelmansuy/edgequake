# OODA-13: Act - Delete Document Fix Implementation

## Changes Made

### 1. Added "default" → UUID Mapping

**File**: [documents.rs](edgequake/crates/edgequake-api/src/handlers/documents.rs#L108-L117)

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

### 2. Added "cancelled" to Deletion Status Check

**File**: [documents.rs](edgequake/crates/edgequake-api/src/handlers/documents.rs#L2062)

```rust
"completed" | "processed" | "partial_failure" | "failed" | "cancelled" | "unknown" => {
    // OK to delete
    // OODA-13: Added "cancelled" status to explicitly allow deletion after task cancellation
```

## Test Results

### Before Fix

```bash
curl -X DELETE /api/v1/documents/d578ab1e-...

# Response:
{
  "code": "BAD_REQUEST",
  "message": "Invalid workspace ID 'default': invalid character..."
}
```

### After Fix

```bash
curl -X DELETE /api/v1/documents/d578ab1e-...

# Response:
{
  "document_id": "d578ab1e-9651-4a5d-904e-f9ae2663cf85",
  "deleted": true,
  "chunks_deleted": 1,
  "entities_affected": 0,
  "relationships_affected": 0
}
```

## Evidence

- Document count: 5 → 4 ✅
- Cancelled document deleted successfully ✅
- API returns 200 OK with deletion summary ✅

## Mission Criterion Addressed

✅ **"Ensure delete document works fully"**

- Deletion now works for documents in "default" workspace
- Cancelled documents can be deleted
- Cascade deletion cleans up chunks

**PDF Storage Cleanup**: This document didn't have PDF data. PDF cleanup will be
verified with a PDF document deletion in a subsequent iteration.

## Commit

```
OODA-13: Fix document deletion for "default" workspace

- Map "default" workspace_id to UUID 00000000-0000-0000-0000-000000000003
- Add "cancelled" status to allowed deletion statuses
- Fix: Documents with workspace_id="default" can now be deleted

WHY: Documents store workspace_id="default" but deletion requires UUID
for vector storage lookup. This mapping aligns with processor.rs behavior.
```
