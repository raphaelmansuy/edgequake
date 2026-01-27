# OODA Iteration 12 - Decide

## Selected Solution: Real-time WorkspaceStats Implementation

### Changes to Make

1. **Add `embedding_count` to `WorkspaceStats` struct**
   - File: `edgequake/crates/edgequake-core/src/types/multitenancy.rs`
   - Add new field to match mission requirements

2. **Implement real-time counting in PostgreSQL WorkspaceService**
   - File: `edgequake/crates/edgequake-core/src/workspace_service_impl.rs`
   - Query graph storage for node/edge counts
   - Query vector storage for embedding count
   - Query KV storage for document/chunk counts

3. **Update WorkspaceStatsResponse in API**
   - File: `edgequake/crates/edgequake-api/src/handlers/workspaces.rs`
   - Add `embedding_count` field

4. **Add integration test for workspace stats**
   - Verify counts are accurate after document operations

### Implementation Priority

| Step | Task                          | Risk   | Time Est |
| ---- | ----------------------------- | ------ | -------- |
| 1    | Add embedding_count to struct | Low    | 5 min    |
| 2    | Update API response type      | Low    | 5 min    |
| 3    | Implement real-time counting  | Medium | 20 min   |
| 4    | Add test coverage             | Low    | 15 min   |

### Acceptance Criteria

- [ ] `WorkspaceStats` includes `embedding_count`
- [ ] API returns actual counts (not zeros)
- [ ] Counts match after document upload
- [ ] Counts decrease correctly after deletion
- [ ] Works with Memory provider
- [ ] Works with PostgreSQL provider

### Non-Goals for This Iteration

- Historical time-series tracking (future iteration)
- Tenant-level aggregation (future iteration)
- Cached/materialized stats (optimization iteration)
