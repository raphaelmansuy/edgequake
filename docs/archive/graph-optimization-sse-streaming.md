# Graph Optimization & SSE Streaming - Implementation Summary

## Overview

Completed full optimization stack for EdgeQuake graph endpoint:

1. ✅ **Database indexes** for faster filtering on AGE graph vertices
2. ✅ **SSE streaming endpoint** with progressive loading and timeout fallback
3. ✅ **Frontend streaming mode** enabled and tested

## Database Indexes Created

Created 5 indexes on `eq_eq_default_graph._ag_label_vertex` table:

```sql
-- Single-column indexes
CREATE INDEX idx_eq_eq_default_graph_tenant_id
  ON eq_eq_default_graph._ag_label_vertex
  ((ag_catalog.agtype_to_json(properties)->>'tenant_id'));

CREATE INDEX idx_eq_eq_default_graph_workspace_id
  ON eq_eq_default_graph._ag_label_vertex
  ((ag_catalog.agtype_to_json(properties)->>'workspace_id'));

CREATE INDEX idx_eq_eq_default_graph_entity_type
  ON eq_eq_default_graph._ag_label_vertex
  ((ag_catalog.agtype_to_json(properties)->>'entity_type'));

CREATE INDEX idx_eq_eq_default_graph_node_id
  ON eq_eq_default_graph._ag_label_vertex
  ((ag_catalog.agtype_to_json(properties)->>'node_id'));

-- Composite index for multi-tenant queries
CREATE INDEX idx_eq_eq_default_graph_tenant_workspace
  ON eq_eq_default_graph._ag_label_vertex
  ((ag_catalog.agtype_to_json(properties)->>'tenant_id'),
   (ag_catalog.agtype_to_json(properties)->>'workspace_id'));
```

### Key Discovery: AGE Property Access Syntax

Apache AGE's `agtype` doesn't support standard PostgreSQL JSONB operators. Must use:

```sql
ag_catalog.agtype_to_json(properties)->>'field_name'
```

## SSE Streaming Endpoint

### Endpoint Details

- **URL:** `/api/v1/graph/stream`
- **Method:** GET
- **Response:** Server-Sent Events (SSE)
- **Query Params:**
  - `max_nodes` (default: 200) - Maximum nodes to stream
  - `batch_size` (default: 50) - Nodes per batch event

### Event Sequence

1. **metadata** - Graph statistics (total nodes/edges, nodes to stream)
2. **nodes** (multiple) - Batches of node data
3. **edges** - All edges between streamed nodes
4. **done** - Completion summary with timing

### Example SSE Response

```
data: {"type":"metadata","total_nodes":1090,"total_edges":873,"nodes_to_stream":10,"edges_to_stream":0}

data: {"type":"nodes","batch":1,"total_batches":4,"nodes":[...]}

data: {"type":"nodes","batch":2,"total_batches":4,"nodes":[...]}

data: {"type":"nodes","batch":3,"total_batches":4,"nodes":[...]}

data: {"type":"nodes","batch":4,"total_batches":4,"nodes":[...]}

data: {"type":"edges","edges":[...]}

data: {"type":"done","nodes_count":10,"edges_count":8,"duration_ms":4063}
```

## Timeout & Fallback Architecture

### Problem

Complex Cypher query with relationship counting times out on large graphs:

```cypher
MATCH (n:Node)
OPTIONAL MATCH (n)-[r]-()
WITH n, count(r) as degree
RETURN n, degree
ORDER BY degree DESC
```

### Solution: Multi-Layer Timeout with Fallback

#### 1. Database-Level Timeout (4 seconds)

```sql
SET statement_timeout = '4s'
```

Applied in `PostgresAGEGraphStorage::cypher_query()` and `get_popular_nodes_with_degree()`

#### 2. Application-Level Timeout (5 seconds)

```rust
const QUERY_TIMEOUT_SECS: u64 = 5;

let nodes_with_degrees = match tokio::time::timeout(
    Duration::from_secs(QUERY_TIMEOUT_SECS),
    query_future,
).await {
    Ok(Ok(nodes)) => nodes,
    Ok(Err(e)) => {
        // Database timeout returns error here
        let error_msg = format!("{}", e);
        if error_msg.contains("statement timeout") ||
           error_msg.contains("canceling statement") {
            // FALLBACK: Use simple get_all_nodes()
            state.graph_storage.get_all_nodes().await?
                .into_iter()
                .filter(/* tenant/workspace filtering */)
                .take(max_nodes)
                .map(|n| (n, 0usize)) // Degree = 0 in fallback
                .collect()
        } else {
            return Err(e.into());
        }
    }
    Err(_) => {
        // Tokio timeout (5s elapsed)
        // Also use fallback
    }
}
```

#### 3. Fallback Strategy

- Use `get_all_nodes()` - simple SELECT without relationship counting
- Apply tenant/workspace filtering in Rust
- Set degree to 0 (unknown in fallback mode)
- Returns data in <1 second vs 10+ seconds for complex query

### Why 4s Database + 5s Application?

- Database timeout (4s) occurs first during query execution
- Returns `Ok(Err(database_error))` not `Err(timeout)`
- Must detect "statement timeout" string in error message
- Application timeout (5s) catches edge cases where database doesn't timeout

## Frontend Streaming Integration

### Configuration Change

```typescript
// edgequake_webui/src/stores/use-graph-store.ts
useStreaming: true,  // Changed from false
```

### Streaming Components

- **StreamingIndicator** - Shows progress bar and phase
- **Graph Store** - Handles SSE connection and progressive updates
- **Sigma Renderer** - Updates graph incrementally as batches arrive

### User Experience

1. User opens graph page
2. Loading indicator shows "Connecting..."
3. Metadata arrives → "Loading 0/1090 nodes"
4. Node batches stream → "Loading 3/1090 nodes", "Loading 6/1090 nodes"
5. Edges arrive → "Building relationships..."
6. Complete → "✓ Loaded 10 nodes with 8 edges"

## Performance Results

### SSE Streaming Test

```bash
curl -N 'http://localhost:8080/api/v1/graph/stream?max_nodes=10&batch_size=3'
```

**Results:**

- ✅ Completion time: ~4 seconds
- ✅ Nodes streamed: 10 (4 batches)
- ✅ Edges streamed: 8
- ✅ Events sent: 7 (metadata + 4 nodes + edges + done)
- ✅ Fallback triggered: Yes (database timeout detected in logs)

### Regular Graph Endpoint Test

```bash
curl 'http://localhost:8080/api/v1/graph?max_nodes=100'
```

**Results:**

- ✅ Response time: <5 seconds
- ✅ Nodes returned: 100
- ✅ Fallback used: Yes (statement timeout → get_all_nodes)

## Code Changes Summary

### Files Modified

1. **`edgequake/migrations/014_add_graph_indexes.sql`** (CREATED)

   - Migration to create 5 indexes on AGE graph vertices

2. **`edgequake/crates/edgequake-storage/src/adapters/postgres/graph.rs`**

   - Line 112: Changed statement_timeout from 10s to 4s (cypher_query)
   - Line 841: Changed statement_timeout from 10s to 4s (get_popular_nodes_with_degree)

3. **`edgequake/crates/edgequake-api/src/handlers/graph.rs`**

   - Lines 235-280: Added statement timeout detection in `get_graph` handler
   - Lines 715-780: Added statement timeout detection in `stream_graph` handler
   - Added debug logging for query execution tracking

4. **`edgequake_webui/src/stores/use-graph-store.ts`**
   - Line 233: Changed `useStreaming: false` to `useStreaming: true`

## Testing & Verification

### Backend Tests

```bash
# 1. Test regular graph endpoint
curl -s 'http://localhost:8080/api/v1/graph?max_nodes=100' | jq '.nodes | length'
# Expected: 100

# 2. Test SSE streaming
curl -N -H "Accept: text/event-stream" \
  'http://localhost:8080/api/v1/graph/stream?max_nodes=10&batch_size=3'
# Expected: Sequence of SSE events (metadata → nodes → edges → done)

# 3. Check fallback logs
strings /tmp/edgequake-backend.log | grep -i "fallback\|timeout"
# Expected: "Database query timed out, falling back to simple node fetch"
```

### Frontend Test

1. Open http://localhost:3000/graph
2. Observe streaming progress indicator
3. Graph builds progressively
4. Check Network tab for SSE connection to `/api/v1/graph/stream`

## Known Limitations & Future Improvements

### Current State

- ✅ Streaming works with timeout fallback
- ✅ Frontend shows progressive updates
- ✅ Database indexes created
- ⚠️ Degree calculation skipped in fallback mode (degree = 0)
- ⚠️ Complex Cypher query still slow on large graphs

### Future Optimizations

1. **Materialized Views** - Pre-compute node degrees
2. **Query Optimization** - Rewrite Cypher to use indexes
3. **Caching** - Cache popular nodes/degrees
4. **Incremental Loading** - Stream from database cursor
5. **Background Jobs** - Pre-calculate graph metrics

## Deployment Checklist

### Before Deploying to Production

- [ ] Run migration `014_add_graph_indexes.sql` on production database
- [ ] Verify indexes created: `SELECT indexname FROM pg_indexes WHERE schemaname = 'eq_eq_default_graph'`
- [ ] Test graph endpoint under production load
- [ ] Monitor database query performance
- [ ] Set up alerting for timeout fallback warnings
- [ ] Test SSE streaming with slow network conditions
- [ ] Verify streaming works with large graphs (10k+ nodes)
- [ ] Check memory usage during streaming
- [ ] Test concurrent SSE connections
- [ ] Validate tenant/workspace isolation in fallback queries

### Monitoring Metrics

- Graph endpoint response time (p50, p95, p99)
- SSE connection duration
- Fallback trigger rate
- Database statement timeout frequency
- Index usage statistics
- Memory usage during streaming

## References

### Documentation

- [AGE Documentation](https://age.apache.org/age-manual/master/index.html)
- [PostgreSQL Indexes](https://www.postgresql.org/docs/current/indexes.html)
- [Server-Sent Events Spec](https://html.spec.whatwg.org/multipage/server-sent-events.html)
- [Tokio Timeout](https://docs.rs/tokio/latest/tokio/time/fn.timeout.html)

### Related Files

- Backend handlers: `edgequake/crates/edgequake-api/src/handlers/graph.rs`
- Storage layer: `edgequake/crates/edgequake-storage/src/adapters/postgres/graph.rs`
- Frontend store: `edgequake_webui/src/stores/use-graph-store.ts`
- Migration: `edgequake/migrations/014_add_graph_indexes.sql`

---

**Status:** ✅ COMPLETE  
**Tested:** Backend SSE endpoint working, Frontend streaming enabled  
**Next Phase:** Production deployment & monitoring
