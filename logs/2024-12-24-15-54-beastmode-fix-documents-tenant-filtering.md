# Task Log: Fix Documents List Tenant Filtering

**Date:** 2024-12-24 15:54
**Mode:** Beastmode

## Summary

Fixed the Documents list endpoint (`/api/v1/documents`) to filter by tenant context, completing the multi-tenancy filtering implementation.

## Issue

The Documents list API was returning documents from ALL tenants regardless of the current tenant/workspace selection. This was inconsistent with the Graph and Query APIs which were already filtering correctly.

## Changes Made

### 1. Added tenant context to document metadata storage

**File:** [handlers/documents.rs](edgequake/crates/edgequake-api/src/handlers/documents.rs)

- Added `workspace_id_for_storage` and `tenant_id_for_storage` extraction from `TenantContext`
- Added `tenant_id` and `workspace_id` fields to initial document metadata JSON (line ~155)
- Added `tenant_id` and `workspace_id` fields to completed document metadata JSON (line ~305)

### 2. Added TenantContext to list_documents function

- Updated `list_documents` signature to accept `TenantContext` parameter
- Added debug logging for tenant context

### 3. Added tenant filtering logic

- Extended internal `DocMetadata` struct with `tenant_id` and `workspace_id` fields
- Added extraction of tenant_id/workspace_id from stored metadata
- Created `matches_tenant_context` closure to filter documents
- Applied filtering when building document list (both for documents with chunks and pending documents)

## Code Changes Summary

```rust
// DocMetadata struct now includes:
tenant_id: Option<String>,
workspace_id: Option<String>,

// list_documents now accepts TenantContext:
pub async fn list_documents(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
) -> ApiResult<Json<ListDocumentsResponse>>

// Filtering logic:
let matches_tenant_context = |meta: &DocMetadata| -> bool {
    if let Some(ref filter_ws) = filter_workspace_id {
        if meta.workspace_id.as_ref() != Some(filter_ws) {
            return false;
        }
    }
    if let Some(ref filter_tid) = filter_tenant_id {
        if meta.tenant_id.as_ref() != Some(filter_tid) {
            return false;
        }
    }
    true
};
```

## Testing

- ✅ `cargo check --package edgequake-api` - Compiles with only warnings
- ✅ `cargo test --package edgequake-api list_documents` - 6 tests pass
- ✅ `cargo test --package edgequake-api` - All 217 tests pass

## Behavior After Fix

- Documents uploaded with tenant context will have `tenant_id` and `workspace_id` stored in metadata
- `list_documents` will filter results to only show documents belonging to the current tenant/workspace
- Existing documents without tenant metadata will only appear when no tenant filter is applied

## Actions

- [x] Add tenant_id/workspace_id to initial document metadata
- [x] Add tenant_id/workspace_id to completed document metadata
- [x] Add TenantContext to list_documents function signature
- [x] Extend DocMetadata struct with tenant fields
- [x] Extract tenant fields from stored metadata
- [x] Add filtering logic to document list building
- [x] Verify all tests pass

## Next Steps

- Re-run E2E browser tests to confirm documents are now filtered by tenant
- Consider adding tenant filtering to other document endpoints (get_document, delete_document)
