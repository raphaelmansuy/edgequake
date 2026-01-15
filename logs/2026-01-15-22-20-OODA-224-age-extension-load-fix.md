# OODA-224: AGE Extension Load Fix for Rebuild Knowledge Graph

**Date:** 2026-01-15  
**Status:** ✅ RESOLVED  
**Severity:** HIGH - Blocked workspace model changes

## Problem

When attempting to change the embedding or LLM model for a workspace and clicking "Rebuild Knowledge Graph" in the WebUI, the operation failed with the error:

```
Failed to rebuild knowledge graph: Internal error: Failed to clear graph:
Database error: Failed to clear workspace: error returned from database:
type "agtype" does not exist
```

## Root Cause Analysis

Apache AGE (A Graph Extension) for PostgreSQL requires the `LOAD 'age'` command to be executed in each PostgreSQL session before using AGE-specific types and functions like:

- `agtype` - AGE's custom type for graph data
- `ag_catalog.cypher()` - Function to execute Cypher queries

The `clear_workspace` function in `PostgresAGEGraphStorage` was missing this session initialization, causing the error when it tried to execute Cypher queries to delete workspace nodes and edges.

### Affected Code Location

File: `edgequake/crates/edgequake-storage/src/adapters/postgres/graph.rs`  
Function: `clear_workspace` (line 1545)

### Before Fix (BUGGY)

```rust
async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> Result<(usize, usize)> {
    let pool = self.pool.get().await?;
    // ... directly executed ag_catalog.cypher() without LOAD 'age'
    let cypher_query = format!(
        "SELECT * FROM cypher('{}', $$ {} $$) AS (result agtype)",
        self.graph_name, delete_cypher
    );
    sqlx::query(&cypher_query).execute(&pool).await  // FAILS - agtype not loaded
}
```

### After Fix

```rust
async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> Result<(usize, usize)> {
    let pool = self.pool.get().await?;
    let mut conn = pool.acquire().await?;  // Dedicated connection

    // OODA-224: CRITICAL - Must load AGE extension first
    sqlx::query("LOAD 'age'").execute(&mut *conn).await?;
    sqlx::query("SET search_path = ag_catalog, \"$user\", public").execute(&mut *conn).await?;

    // ... rest of function now uses &mut *conn instead of &pool
}
```

## Fix Applied

1. **Acquire dedicated connection** - Ensures AGE session state persists across queries
2. **Load AGE extension** - `LOAD 'age'` makes agtype and cypher() available
3. **Set search path** - Ensures ag_catalog functions are accessible
4. **Use connection for all queries** - All subsequent queries use the AGE-enabled connection

## Verification

### Manual Testing

```bash
# Rebuild knowledge graph endpoint
curl -X POST http://localhost:8080/api/v1/workspaces/{id}/rebuild-knowledge-graph \
  -H "Content-Type: application/json" \
  -d '{"force": true, "rebuild_embeddings": false}'

# Response: 200 OK
{
  "status": "graph_cleared",
  "nodes_cleared": 10,
  "edges_cleared": 6,
  ...
}
```

### Automated Test Added

File: `edgequake/crates/edgequake-api/tests/e2e_postgres_rebuild.rs`

```rust
/// Test that rebuild-knowledge-graph correctly loads AGE extension before clearing.
/// @implements OODA-224: AGE Extension Load Fix
#[tokio::test]
async fn test_postgres_rebuild_kg_loads_age_extension() {
    // Creates workspace, calls rebuild-knowledge-graph
    // Verifies 200 response (not 500 with "agtype does not exist")
}
```

## Files Changed

1. **`edgequake/crates/edgequake-storage/src/adapters/postgres/graph.rs`**

   - Added `LOAD 'age'` and `SET search_path` to `clear_workspace` function
   - Changed from pool.execute to conn.execute for session persistence

2. **`edgequake/crates/edgequake-api/tests/e2e_postgres_rebuild.rs`**
   - Added `test_postgres_rebuild_kg_loads_age_extension` test

## Other AGE Functions Reviewed

Checked all other functions using AGE in the codebase - they already have proper session initialization:

| Function             | Line | Status              |
| -------------------- | ---- | ------------------- |
| `cypher_query`       | 133  | ✅ Has LOAD 'age'   |
| `cypher_execute`     | 197  | ✅ Has LOAD 'age'   |
| `cypher_query_count` | 234  | ✅ Has LOAD 'age'   |
| `create_graph`       | 394  | ✅ Has LOAD 'age'   |
| `ensure_indexes`     | 452  | ✅ Has LOAD 'age'   |
| `batch_sql_query`    | 609  | ✅ Has LOAD 'age'   |
| `get_popular_labels` | 1337 | ✅ Has LOAD 'age'   |
| `clear_workspace`    | 1545 | 🔧 FIXED in this PR |

## Lessons Learned

1. **AGE requires per-session initialization** - The `LOAD 'age'` command must be called in every PostgreSQL session that uses AGE types/functions
2. **Connection pooling complicates AGE usage** - When using connection pools, each acquired connection needs AGE to be loaded
3. **Pattern established** - All functions using AGE cypher() should:
   - Acquire a dedicated connection
   - Call `LOAD 'age'`
   - Call `SET search_path = ag_catalog, "$user", public`
   - Execute all AGE queries on that connection

## Impact

- **Before:** Workspace model changes blocked - rebuild operations failed
- **After:** Full support for changing LLM/embedding providers and rebuilding

## Related Issues

- Follows OODA-223 (workspace isolation fix)
- Enables dynamic provider switching feature
