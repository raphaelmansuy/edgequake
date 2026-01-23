# OODA Loop Iteration 01 - Observe

## Mission Re-read

- Ensure 500 workspaces by tenant by default
- Ensure up to 50MB by document uploaded - Ensure it works
- Ensure I can delete a workspace
- Ensure when a document is deleted from a workspace, all associated embeddings and knowledge graph data are also removed

## Territory Mapping

### 1. Workspace Limits per Tenant Plan

**Location**: `edgequake/crates/edgequake-core/src/types/multitenancy.rs:192-199`

```rust
impl TenantPlan {
    pub fn default_max_workspaces(&self) -> usize {
        match self {
            TenantPlan::Free => 2,      // TOO LOW
            TenantPlan::Basic => 5,     // TOO LOW
            TenantPlan::Pro => 20,      // TOO LOW
            TenantPlan::Enterprise => 100,  // NEEDS TO BE 500
        }
    }
}
```

**Observation**: Current max_workspaces limits are far below the 500 target.

**Also found in**: `edgequake/crates/edgequake-auth/src/tenant.rs:64-71` (duplicated TenantPlan)

### 2. Document Upload Size Limits

**Locations**:

1. `edgequake/crates/edgequake-api/src/state.rs:252,262`

```rust
pub struct AppConfig {
    pub max_document_size: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            max_document_size: 10 * 1024 * 1024, // 10 MB <- NEEDS 50MB
        }
    }
}
```

2. `edgequake/crates/edgequake-core/src/config.rs:225,239`

```rust
pub struct ApiConfig {
    pub body_limit: usize,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            body_limit: 10 * 1024 * 1024, // 10MB <- NEEDS 50MB
        }
    }
}
```

**Observation**: Two places need updating for 50MB support:

- `max_document_size` for content validation
- `body_limit` for HTTP request body limit

### 3. Workspace Deletion

**Location**: `edgequake/crates/edgequake-core/src/workspace_service_impl.rs:590-598`

```rust
async fn delete_workspace(&self, workspace_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM workspaces WHERE workspace_id = $1")
        .bind(workspace_id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to delete workspace: {}", e)))?;

    tracing::info!(workspace_id = %workspace_id, "Deleted workspace from PostgreSQL");
    Ok(())
}
```

**CRITICAL ISSUE**: Workspace deletion ONLY deletes the workspace row.
It does NOT cascade to:

- Documents in KV storage
- Embeddings in vector storage
- Entities/relationships in graph storage
- Tasks associated with the workspace

**API Handler**: `edgequake/crates/edgequake-api/src/handlers/workspaces.rs:726-737`

- Just calls `workspace_service.delete_workspace()` without any cleanup

### 4. Document Deletion

**Location**: `edgequake/crates/edgequake-core/src/orchestrator.rs:795-880`

```rust
pub async fn delete_document(&self, document_id: &str) -> Result<DocumentDeletionResult>
```

**Observation**: Document deletion properly cascades to:

- ✅ Chunks (KV storage)
- ✅ Entities (graph storage + vector storage)
- ✅ Relationships (graph storage)
- ✅ Embeddings (vector storage)

**The implementation exists and is well-documented.**

### 5. Vector Storage Workspace Clearing

**Location**: `edgequake/crates/edgequake-api/src/handlers/workspaces.rs:921`

```rust
state.vector_storage.clear_workspace(&workspace_id)
```

This is used in rebuild_embeddings but NOT in delete_workspace.

## Summary of Gaps

| Requirement              | Current State        | Gap                                   |
| ------------------------ | -------------------- | ------------------------------------- |
| 500 workspaces/tenant    | Max 100 (Enterprise) | Update default_max_workspaces()       |
| 50MB document upload     | 10MB limit           | Update max_document_size + body_limit |
| Delete workspace cascade | Only deletes row     | Need cascade to KV/Vector/Graph       |
| Delete document cascade  | ✅ Implemented       | Working correctly                     |

## Files to Modify

1. `edgequake/crates/edgequake-core/src/types/multitenancy.rs` - TenantPlan::default_max_workspaces()
2. `edgequake/crates/edgequake-auth/src/tenant.rs` - TenantPlan::default_max_workspaces() (duplicate)
3. `edgequake/crates/edgequake-api/src/state.rs` - AppConfig::max_document_size
4. `edgequake/crates/edgequake-core/src/config.rs` - ApiConfig::body_limit
5. `edgequake/crates/edgequake-core/src/workspace_service_impl.rs` - delete_workspace()
6. `edgequake/crates/edgequake-api/src/handlers/workspaces.rs` - delete_workspace handler
