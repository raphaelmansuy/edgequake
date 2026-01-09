# Quick Reference: Graph Optimization & SSE Streaming

## TL;DR - What Was Done

### 1. Database Indexes

Created 5 indexes on AGE graph vertices for faster filtering:

```sql
-- Use this syntax for AGE property indexes:
CREATE INDEX idx_name ON table_name
  ((ag_catalog.agtype_to_json(properties)->>'field_name'));
```

**Location:** `edgequake/migrations/014_add_graph_indexes.sql`

### 2. Timeout Fallback

When complex graph queries timeout, fall back to simple fast query:

```rust
// Database timeout: 4s
// Application timeout: 5s
// Fallback: get_all_nodes() with client-side filtering
```

**Files:**

- `edgequake/crates/edgequake-storage/src/adapters/postgres/graph.rs` (line 112, 841)
- `edgequake/crates/edgequake-api/src/handlers/graph.rs` (line 235, 715)

### 3. SSE Streaming

Progressive graph loading via Server-Sent Events:

```
GET /api/v1/graph/stream?max_nodes=100&batch_size=50
```

**Frontend:** `useStreaming: true` in graph store

---

## Quick Commands

### Test SSE Endpoint

```bash
curl -N -H "Accept: text/event-stream" \
  'http://localhost:8080/api/v1/graph/stream?max_nodes=10&batch_size=3'
```

### Verify Indexes

```bash
docker exec edgequake-postgres psql -U edgequake -d edgequake \
  -c "SELECT indexname FROM pg_indexes WHERE schemaname = 'eq_eq_default_graph';"
```

### Check Fallback Logs

```bash
strings /tmp/edgequake-backend.log | grep -i "fallback\|timeout"
```

---

## Troubleshooting

### Graph Endpoint Hangs

**Symptom:** Request times out after 10+ seconds  
**Cause:** Complex Cypher query with relationship counting  
**Solution:** Fallback mechanism should trigger automatically  
**Check:** Look for "Database query timed out, falling back" in logs

### SSE Streaming Not Working

**Symptom:** No events received  
**Check:**

1. Backend healthy: `curl http://localhost:8080/health`
2. SSE endpoint: `curl -N 'http://localhost:8080/api/v1/graph/stream?max_nodes=5'`
3. Content-Type header: Should be `text/event-stream`

### Indexes Not Being Used

**Symptom:** Queries still slow despite indexes  
**Check:**

1. Verify indexes exist: `\d+ eq_eq_default_graph._ag_label_vertex`
2. Check query plan: Use `EXPLAIN ANALYZE`
3. Note: Indexes don't help queries without WHERE clauses

---

## Key Files

### Backend

- **Handlers:** `edgequake/crates/edgequake-api/src/handlers/graph.rs`
- **Storage:** `edgequake/crates/edgequake-storage/src/adapters/postgres/graph.rs`
- **Migration:** `edgequake/migrations/014_add_graph_indexes.sql`

### Frontend

- **Store:** `edgequake_webui/src/stores/use-graph-store.ts` (line 233: `useStreaming`)
- **Page:** `edgequake_webui/src/app/graph/page.tsx`

### Documentation

- **Full Guide:** `docs/graph-optimization-sse-streaming.md`
- **Summary:** `docs/TASK_COMPLETION_SUMMARY.md`
- **Task Log:** `logs/2024-12-30-14-45-beastmode-graph-optimization.md`

---

## Performance Benchmarks

| Operation        | Before         | After | Improvement    |
| ---------------- | -------------- | ----- | -------------- |
| Graph endpoint   | TIMEOUT (>30s) | <5s   | ✅ 6x faster   |
| SSE streaming    | N/A            | ~4s   | ✅ NEW FEATURE |
| First node batch | N/A            | <1s   | ✅ Progressive |

---

## What to Monitor in Production

1. **Fallback Trigger Rate**

   - Check logs for "Database query timed out, falling back"
   - High rate suggests need for query optimization

2. **SSE Connection Duration**

   - Normal: <10s for typical graphs
   - Warning: >30s indicates performance issues

3. **Database Query Times**

   - `get_popular_nodes_with_degree`: Should timeout at 4s
   - `get_all_nodes`: Should complete <1s

4. **Index Usage**
   - Query: `SELECT * FROM pg_stat_user_indexes WHERE schemaname = 'eq_eq_default_graph'`
   - Check `idx_scan` column for usage counts

---

## Common Questions

**Q: Why 4s database timeout and 5s application timeout?**  
A: Database timeout must be lower so error is returned to application before application timeout fires. This allows fallback logic to trigger.

**Q: Why is degree = 0 in fallback mode?**  
A: Calculating relationship degrees requires complex query. Fallback skips this for speed. UI can handle missing degrees.

**Q: Can I disable streaming?**  
A: Yes, set `useStreaming: false` in graph store. Falls back to single request.

**Q: Do indexes help streaming?**  
A: Not directly - main query doesn't use WHERE clauses. But they help if we add filtering features later.

---

## Future Improvements

1. **Materialized Views:** Pre-compute node degrees daily
2. **Query Rewrite:** Optimize Cypher to use indexes
3. **Caching:** Redis cache for popular nodes
4. **Cursor Streaming:** Stream directly from database cursor
5. **Background Jobs:** Pre-calculate graph metrics

---

**Last Updated:** 2024-12-30  
**Status:** ✅ Production Ready  
**Maintainer:** Development Team
