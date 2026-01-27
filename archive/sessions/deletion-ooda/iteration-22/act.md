# OODA-22 Act: Metrics History API Endpoint

## Completed Actions

### 1. Added Trait Method

Added `get_metrics_history` to WorkspaceService trait:

```rust
async fn get_metrics_history(
    &self,
    workspace_id: Uuid,
    limit: usize,
    offset: usize,
) -> Result<Vec<MetricsSnapshot>>;
```

### 2. Implemented PostgreSQL Version

Added query in workspace_service_impl.rs:

- Queries workspace_metrics_history table
- Returns snapshots in reverse chronological order
- Supports pagination with limit/offset

### 3. Added In-Memory Stub

Returns empty vector for testing compatibility.

### 4. Added Response DTOs

In workspaces_types.rs:

- `MetricsSnapshotDTO` - Individual snapshot
- `MetricsHistoryResponse` - Paginated list response

### 5. Added API Handler

In workspaces.rs:

- `get_metrics_history` handler
- `MetricsHistoryParams` query struct
- Defaults: limit=100, max=1000

### 6. Added Route

In routes.rs:

```
GET /api/v1/workspaces/{workspace_id}/metrics-history
```

## Files Modified

1. `crates/edgequake-core/src/workspace_service.rs` - Trait method
2. `crates/edgequake-core/src/workspace_service_impl.rs` - PostgreSQL impl
3. `crates/edgequake-api/src/handlers/workspaces_types.rs` - DTOs
4. `crates/edgequake-api/src/handlers/workspaces.rs` - Handler
5. `crates/edgequake-api/src/routes.rs` - Route

## Test Results

- 27/27 deletion tests pass
- Build succeeds

## Commit

Pending: "feat(api): add metrics history endpoint (OODA-22)"
