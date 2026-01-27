# OODA Iteration 13 - Orient

## Solution Analysis

### Option A: Single Query with JOINs

```sql
SELECT
    (SELECT COUNT(*) FROM documents WHERE workspace_id = $1) as document_count,
    (SELECT COUNT(*) FROM chunks WHERE workspace_id = $1) as chunk_count,
    (SELECT COUNT(*) FROM entities WHERE workspace_id = $1) as entity_count,
    (SELECT COUNT(*) FROM relationships WHERE workspace_id = $1) as relationship_count,
    (SELECT COUNT(*) FROM chunks WHERE workspace_id = $1 AND embedding IS NOT NULL) as embedding_count,
    (SELECT COALESCE(SUM(file_size_bytes), 0) FROM documents WHERE workspace_id = $1) as storage_bytes;
```

**Pros**: Single round-trip to database
**Cons**: More complex query, harder to debug

### Option B: Separate Queries (Parallel)

Run 6 separate COUNT queries in parallel using tokio::join!

**Pros**: Simple queries, easier to debug, can parallelize
**Cons**: Multiple round-trips (though parallel)

### Option C: Subquery Approach

```sql
SELECT
    d.document_count,
    c.chunk_count,
    e.entity_count,
    r.relationship_count,
    em.embedding_count,
    s.storage_bytes
FROM
    (SELECT COUNT(*) as document_count FROM documents WHERE workspace_id = $1) d,
    (SELECT COUNT(*) as chunk_count FROM chunks WHERE workspace_id = $1) c,
    (SELECT COUNT(*) as entity_count FROM entities WHERE workspace_id = $1) e,
    (SELECT COUNT(*) as relationship_count FROM relationships WHERE workspace_id = $1) r,
    (SELECT COUNT(*) as embedding_count FROM chunks WHERE workspace_id = $1 AND embedding IS NOT NULL) em,
    (SELECT COALESCE(SUM(file_size_bytes), 0) as storage_bytes FROM documents WHERE workspace_id = $1) s;
```

**Pros**: Single round-trip, readable
**Cons**: PostgreSQL specific syntax

## First Principles Decision

1. **Correctness**: All options are equally correct
2. **Simplicity**: Option A (scalar subqueries) is most readable in Rust
3. **Performance**: Single query is best
4. **Testability**: Easy to verify with integration tests

## Risk Assessment

| Risk              | Mitigation                                    |
| ----------------- | --------------------------------------------- |
| Query error       | Use proper error handling                     |
| Large tables      | COUNT(\*) on indexed workspace_id is O(log n) |
| Missing workspace | Already verified in existing code             |

## Recommendation

**Use Option A** - Single query with scalar subqueries.

- Simplest Rust code (single sqlx::query_as call)
- Single database round-trip
- Clear and readable SQL
