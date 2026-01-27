# OODA Iteration 12 - Observe

**Mission Re-read**: specs/033-study-delete-document/003-study-document.md

## Focus Area: Metrics Tracking Gap

The mission explicitly requires:

> "Ensure metric likes number of Entities, Relationships, Embeddings per document, Relations, Entity Types are tracked and logged in specific database table and integrated in edgequake web ui."
> "We want to monitor Documents numbers, Entities numbers, Relationships numbers, Embeddings numbers per workspace and per tenant over time."

## Current State Analysis

### 1. WorkspaceStats Struct Exists

**Location**: `edgequake/crates/edgequake-core/src/types/multitenancy.rs:1069-1081`

```rust
pub struct WorkspaceStats {
    pub workspace_id: Uuid,
    pub document_count: usize,
    pub entity_count: usize,
    pub relationship_count: usize,
    pub chunk_count: usize,
    pub storage_bytes: usize,
}
```

**MISSING**:

- `embedding_count` - not present
- Historical tracking (time series) - not present
- Tenant-level aggregation - not present

### 2. Implementation is Stubbed

**Location**: `edgequake/crates/edgequake-core/src/workspace_service_impl.rs:618-633`

```rust
async fn get_workspace_stats(&self, workspace_id: Uuid) -> Result<WorkspaceStats> {
    // Verify workspace exists
    let _ = self.get_workspace(workspace_id).await?...;

    Ok(WorkspaceStats {
        workspace_id,
        document_count: 0,  // STUB!
        entity_count: 0,    // STUB!
        relationship_count: 0, // STUB!
        chunk_count: 0,     // STUB!
        storage_bytes: 0,   // STUB!
    })
}
```

**GAP-12**: WorkspaceStats returns all zeros - not implemented!

### 3. API Endpoint Exists

**Location**: `edgequake/crates/edgequake-api/src/handlers/workspaces.rs:880-909`

```
GET /api/v1/workspaces/{workspace_id}/stats
```

Response type: `WorkspaceStatsResponse` with same fields as struct.

### 4. Database Schema Has No Metrics Tables

The migration `008_add_multi_tenancy_tables.sql` creates:

- `tenants` - no stats columns
- `workspaces` - no stats columns
- `memberships` - no stats columns

**No historical metrics table exists!**

### 5. Storage Layer Has Count Functions

Let me verify if storage adapters have count methods:

```
GraphStorage trait - needs verification for get_node_count, get_edge_count
VectorStorage trait - needs verification for get_vector_count
KVStorage trait - needs verification for document count
```

## Observations Summary

| Component                | Exists | Implemented | Gap                       |
| ------------------------ | ------ | ----------- | ------------------------- |
| WorkspaceStats struct    | ✅     | ❌          | Missing embedding_count   |
| WorkspaceStats API       | ✅     | ❌          | Returns all zeros         |
| Real-time metrics query  | ❌     | ❌          | No storage layer counting |
| Historical metrics table | ❌     | ❌          | No time-series storage    |
| Tenant aggregation       | ❌     | ❌          | No tenant-level stats     |
| WebUI integration        | ❓     | ❓          | Need to check             |

## Next Step

Orient phase: Analyze options for implementing real-time and historical metrics.
