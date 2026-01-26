# OODA Iteration 12 - Orient

## Gap Analysis

### GAP-12: WorkspaceStats Returns Zeros

**Root Cause**: The `get_workspace_stats` implementation in `workspace_service_impl.rs` is a stub that returns all zeros.

**Impact**:

- API endpoint `/api/v1/workspaces/{workspace_id}/stats` returns useless data
- No way to monitor workspace health
- Mission requirement for metrics tracking unfulfilled

### Available Building Blocks

| Storage Trait   | Method         | Implementation           |
| --------------- | -------------- | ------------------------ |
| `GraphStorage`  | `node_count()` | Memory ✅, PostgreSQL ✅ |
| `GraphStorage`  | `edge_count()` | Memory ✅, PostgreSQL ✅ |
| `VectorStorage` | `count()`      | Memory ✅, PostgreSQL ✅ |
| `KVStorage`     | `count()`      | Memory ✅, PostgreSQL ✅ |

## Solution Options

### Option A: Real-time Count Queries

**Approach**: Implement `get_workspace_stats` by querying storage adapters directly.

```
WorkspaceStats {
    document_count = kv_storage.count() // documents namespace
    entity_count = graph_storage.node_count()
    relationship_count = graph_storage.edge_count()
    chunk_count = kv_storage.count() // chunks namespace
    embedding_count = vector_storage.count()
}
```

**Pros**:

- Always accurate
- No schema changes needed
- Simple implementation

**Cons**:

- O(n) query for each metric
- Could be slow for large workspaces
- No historical trend data

### Option B: Cached Metrics with Triggers

**Approach**: Add stats columns to workspaces table, update via triggers on document/entity changes.

**Pros**:

- O(1) read performance
- Can add historical snapshots

**Cons**:

- Complex trigger logic
- PostgreSQL-only (memory mode needs different approach)
- Potential drift if triggers fail

### Option C: Hybrid (Real-time first, cache later)

**Approach**: Implement Option A first for correctness, optimize later with caching if needed.

**Pros**:

- Working solution immediately
- Can iterate on performance
- No database migration needed

**Cons**:

- May need refactoring later

## First Principles Analysis

1. **Correctness First**: Real-time counts are always accurate
2. **Simplicity**: Use existing storage methods
3. **Testability**: Easy to verify counts match actual data
4. **Incremental**: Can add caching/history later

## Risk Assessment

| Risk                    | Mitigation                                            |
| ----------------------- | ----------------------------------------------------- |
| Slow counts             | Storage adapters already have efficient COUNT queries |
| Memory mode complexity  | Both providers implement same trait                   |
| Missing embedding_count | Add to WorkspaceStats struct                          |

## Decision Recommendation

**Implement Option C: Hybrid approach**

1. Add `embedding_count` to `WorkspaceStats` struct
2. Implement real-time counting in `get_workspace_stats`
3. Add test coverage to verify correct counts
4. Future iteration can add caching if needed
