# OODA Loop Iteration 01 - Decide

## Priority Order

| Priority | Change                             | Files                      | Impact      |
| -------- | ---------------------------------- | -------------------------- | ----------- |
| P1       | Update max_workspaces to 500       | multitenancy.rs, tenant.rs | Low risk    |
| P2       | Update max_document_size to 50MB   | state.rs, config.rs        | Low risk    |
| P3       | Implement workspace delete cascade | workspaces.rs handler      | High value  |
| P4       | Verify document delete cascade     | Already working            | Verify only |

## Specific Changes

### Change 1: Update TenantPlan::default_max_workspaces()

**File**: `edgequake/crates/edgequake-core/src/types/multitenancy.rs:192-199`

**Before**:

```rust
pub fn default_max_workspaces(&self) -> usize {
    match self {
        TenantPlan::Free => 2,
        TenantPlan::Basic => 5,
        TenantPlan::Pro => 20,
        TenantPlan::Enterprise => 100,
    }
}
```

**After**:

```rust
pub fn default_max_workspaces(&self) -> usize {
    match self {
        TenantPlan::Free => 10,
        TenantPlan::Basic => 100,
        TenantPlan::Pro => 500,     // Target: 500 workspaces
        TenantPlan::Enterprise => 500, // Target: 500 workspaces
    }
}
```

### Change 2: Update TenantPlan in edgequake-auth (duplicate)

**File**: `edgequake/crates/edgequake-auth/src/tenant.rs:64-71`

Same changes as above for consistency.

### Change 3: Update max_document_size to 50MB

**File**: `edgequake/crates/edgequake-api/src/state.rs:262`

**Before**: `max_document_size: 10 * 1024 * 1024`
**After**: `max_document_size: 50 * 1024 * 1024` (52,428,800 bytes)

### Change 4: Update body_limit to 50MB

**File**: `edgequake/crates/edgequake-core/src/config.rs:239`

**Before**: `body_limit: 10 * 1024 * 1024`
**After**: `body_limit: 50 * 1024 * 1024`

### Change 5: Implement workspace cascade delete

**File**: `edgequake/crates/edgequake-api/src/handlers/workspaces.rs`

**Function**: `delete_workspace` handler

**New Logic**:

```rust
pub async fn delete_workspace(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(workspace_id = %workspace_id, "Starting workspace cascade delete");

    let workspace_id_str = workspace_id.to_string();

    // 1. Clear vector storage for this workspace
    let vectors_cleared = state
        .vector_storage
        .clear_workspace(&workspace_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to clear workspace vectors: {}", e)))?;

    tracing::info!(workspace_id = %workspace_id, vectors_cleared = vectors_cleared, "Cleared vector storage");

    // 2. Delete all documents and their data from KV storage
    let all_keys = state.kv_storage.keys().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let workspace_keys: Vec<String> = all_keys
        .into_iter()
        .filter(|key| {
            // Check if key belongs to this workspace (stored in metadata)
            key.ends_with("-metadata") || key.ends_with("-content") || key.contains("-chunk-")
        })
        .collect();

    // For each metadata key, check if it belongs to this workspace
    // and delete associated content and chunks
    let mut documents_deleted = 0;
    for key in workspace_keys.iter().filter(|k| k.ends_with("-metadata")) {
        if let Ok(Some(metadata)) = state.kv_storage.get_by_id(key).await {
            if let Some(ws_id) = metadata.get("workspace_id").and_then(|v| v.as_str()) {
                if ws_id == workspace_id_str || ws_id == "default" {
                    let doc_id = key.trim_end_matches("-metadata");
                    // Delete metadata
                    state.kv_storage.delete(key).await.ok();
                    // Delete content
                    state.kv_storage.delete(&format!("{}-content", doc_id)).await.ok();
                    documents_deleted += 1;
                }
            }
        }
    }

    // 3. Clear graph entities for this workspace (if workspace-scoped)
    // Note: Graph storage may have workspace-scoped tables
    if let Err(e) = state.graph_storage.clear_workspace(&workspace_id).await {
        tracing::warn!(workspace_id = %workspace_id, error = %e, "Failed to clear graph storage");
    }

    // 4. Evict workspace from vector registry cache
    state.vector_registry.evict(&workspace_id).await;

    // 5. Finally delete the workspace record
    state
        .workspace_service
        .delete_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::info!(
        workspace_id = %workspace_id,
        vectors_cleared = vectors_cleared,
        documents_deleted = documents_deleted,
        "Workspace deleted with cascade"
    );

    Ok(StatusCode::NO_CONTENT)
}
```

## Verification Plan

1. Run `cargo clippy` to catch issues
2. Run `cargo test` to verify no regressions
3. Test workspace deletion manually with data
4. Test 50MB file upload
5. Verify workspace limits work correctly

## Commit Strategy

Single commit: `OODA-28: Workspace management updates - 500 limit, 50MB uploads, cascade delete`
