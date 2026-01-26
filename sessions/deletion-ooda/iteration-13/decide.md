# OODA Iteration 13 - Decide

## Selected Solution: Single SQL Query with Scalar Subqueries

### Implementation Plan

1. **Create StatsRow struct for query result**

   ```rust
   #[derive(sqlx::FromRow)]
   struct StatsRow {
       document_count: i64,
       chunk_count: i64,
       entity_count: i64,
       relationship_count: i64,
       embedding_count: i64,
       storage_bytes: i64,
   }
   ```

2. **Implement SQL query in get_workspace_stats**
   - Use scalar subqueries for each metric
   - Map i64 to usize in response

3. **Add integration test**
   - Test stats after document upload
   - Verify counts match expected values

### Acceptance Criteria

- [ ] PostgreSQL implementation returns real counts
- [ ] document_count matches COUNT(\*) FROM documents
- [ ] entity_count matches COUNT(\*) FROM entities
- [ ] relationship_count matches COUNT(\*) FROM relationships
- [ ] chunk_count matches COUNT(\*) FROM chunks
- [ ] embedding_count matches COUNT(\*) FROM chunks WHERE embedding IS NOT NULL
- [ ] storage_bytes matches SUM(file_size_bytes)
- [ ] All existing tests still pass
- [ ] In-memory implementation unchanged (still returns zeros)

### Non-Goals

- Historical metrics tracking (future iteration)
- Tenant-level aggregation (future iteration)
- Caching (optimization iteration)
- In-memory provider real-time stats (would require storage adapter access)
