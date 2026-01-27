# OODA Loop Iteration 231 - OBSERVE

## Issue Discovered

Security invariant checker flagged potential tenant isolation issue in `query.rs`:

```
/edgequake-api/src/handlers/query.rs:130: with_tenant_id(tenant_id.clone())
/edgequake-api/src/handlers/query.rs:440: with_tenant_id(tenant_id.clone())
```

## Analysis

### Code Flow in execute_query (line 130)

```rust
// Line 130 - Uses header tenant_id
if let Some(ref tenant_id) = tenant_ctx.tenant_id {
    engine_request = engine_request.with_tenant_id(tenant_id.clone());
}

// Line 165-167 - Workspace is fetched LATER
let embedding_result = get_workspace_embedding_provider(&state, workspace_id).await;
```

### Problem

Same as OODA-231 (chat.rs fix): The tenant_id from the header is used for graph filtering, but the data was ingested using the workspace's actual tenant_id.

### Impact

1. Query endpoint may return 0 results if header tenant_id ≠ workspace tenant_id
2. This affects both `execute_query` and `stream_query` handlers

## Files Affected

1. `query.rs:130` - `execute_query` handler
2. `query.rs:440` - `stream_query` handler

## Recommendation

Apply same fix as chat.rs: Use workspace's tenant_id for data queries when workspace is available.
