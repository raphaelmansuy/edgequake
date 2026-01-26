# EdgeQuake Storage Architecture Deep Analysis

**Date**: 2026-01-26 18:00  
**Purpose**: Comprehensive analysis of available storage backends for optimal dashboard stats performance

## Database Investigation Results

### Table Row Counts

| Table | Row Count | Data Type | Purpose |
|-------|-----------|-----------|---------|
| `eq_eq_default_kv` | 8 | JSON documents | Document metadata, chunks |
| `eq_eq_default_graph.Node` | 101 | AGtype (Apache AGE) | Entities (knowledge graph nodes) |
| `eq_eq_default_graph.EDGE` | 4 | AGtype (Apache AGE) | Relationships (knowledge graph edges) |
| `documents` | **0** | SQL rows | ❌ UNUSED (vestigial) |
| `entities` | **0** | SQL rows | ❌ UNUSED (vestigial) |
| `relationships` | **0** | SQL rows | ❌ UNUSED (vestigial) |
| `chunks` | **0** | SQL rows | ❌ UNUSED (vestigial) |
| `workspace_metrics_history` | 0 | SQL rows | ⏳ Empty (metrics recording not yet used) |

### Storage Backend Mapping

```
Document Upload → Pipeline Processing
                 ↓
    ┌────────────┴─────────────┐
    ↓                          ↓
KV Storage              Apache AGE Graph
├─ Documents metadata   ├─ Entities (nodes)
├─ Chunks               └─ Relationships (edges)
├─ entity_count (in metadata)
├─ relationship_count (in metadata)
└─ chunk_count (derived from keys)
```

### Data Storage Reality

**✅ ACTIVE Storage**:
1. **KV Storage (`eq_eq_default_kv`)**: 
   - Document metadata with accurate `entity_count` and `relationship_count`
   - Chunks with embeddings
   - Key format: `{doc_id}-metadata`, `{doc_id}-chunk-{n}`

2. **Apache AGE Graph (`eq_eq_default_graph`)**:
   - Entities as nodes with properties: `{node_id, entity_type, workspace_id, ...}`
   - Relationships as edges
   - Query via: `SELECT COUNT(*) FROM eq_eq_default_graph."Node" WHERE properties::text LIKE '%workspace_id%'`

**❌ VESTIGIAL Tables** (never populated):
- `documents`: Has columns (`entity_count`, `relationship_count`, `chunk_count`) but 0 rows
- `entities`: Has proper schema but 0 rows
- `relationships`: Has proper schema but 0 rows  
- `chunks`: Has proper schema but 0 rows

**⏳ FUTURE Tables**:
- `workspace_metrics_history`: Empty but designed for time-series metrics

## Performance Analysis

### Option 1: PostgreSQL Tables (if populated) ⚡ FASTEST

**Query Performance**: O(1) with indexed workspace_id
```sql
SELECT 
    COUNT(DISTINCT id) as document_count,
    COALESCE(SUM(entity_count), 0) as entity_count,
    COALESCE(SUM(relationship_count), 0) as relationship_count,
    COALESCE(SUM(chunk_count), 0) as chunk_count,
    COALESCE(SUM(file_size_bytes), 0) as storage_bytes
FROM documents 
WHERE workspace_id = $1;

SELECT COUNT(*) FROM chunks WHERE workspace_id = $1 AND embedding IS NOT NULL;
```

**Pros**:
- ⚡ **Fastest**: Single SQL query with indexed lookups
- 📊 **Scalable**: Handles millions of documents efficiently
- 🔍 **Complex queries**: Easy filtering, sorting, pagination
- 💾 **Indexed**: `idx_documents_tenant_workspace` btree index available

**Cons**:
- ❌ **Currently empty**: Not populated by current pipeline
- 🔧 **Requires sync**: Need to modify pipeline to write to both KV + PostgreSQL
- 💥 **Data duplication**: Same data in KV and PostgreSQL

**Performance**: ~1-5ms for typical workspace

---

### Option 2: Apache AGE Graph 🐢 SLOWEST

**Query Performance**: O(n) - full graph scan per query
```sql
SELECT COUNT(*) 
FROM eq_eq_default_graph."Node" 
WHERE properties::text LIKE '%"workspace_id": "23d89fe3-e822-4c06-8f8c-82752436f7f3"%';
```

**Pros**:
- ✅ **Has actual data**: 101 nodes currently stored
- 📈 **Accurate**: Source of truth for entities/relationships
- 🎯 **Query capabilities**: Can traverse relationships

**Cons**:
- 🐢 **Slow**: Full table scan + string matching on JSON
- 🚫 **No indexes**: AGtype properties not indexed for workspace_id
- 📈 **Poor scalability**: Performance degrades with graph size
- 🔄 **JSON casting**: Properties::text conversion adds overhead

**Performance**: ~50-200ms for 101 nodes (grows linearly)

---

### Option 3: KV Storage (Current Implementation) 🟡 MODERATE

**Query Performance**: O(n) where n = total keys
```rust
// Get all keys
let keys = kv_storage.keys().await; // O(n)

// Filter metadata keys
let metadata_keys = keys.filter(|k| k.ends_with("-metadata")); // O(n)

// Aggregate stats
for metadata in metadata_values {
    if metadata.workspace_id == target_workspace {
        entity_count += metadata.entity_count;
    }
}
```

**Pros**:
- ✅ **Has accurate data**: Document metadata includes entity/relationship counts
- 🎯 **Single source**: No sync issues between storage backends
- 💾 **Simple**: No schema migrations needed

**Cons**:
- 🟡 **Moderate speed**: Fetches all metadata, filters in memory
- 📊 **Not indexed**: No workspace_id filtering at storage layer
- 📈 **Poor scalability**: Performance degrades with document count
- 🔄 **Network overhead**: Multiple KV get operations

**Performance**: ~20-100ms for 8 documents (grows linearly)

---

### Option 4: Hybrid Approach (Recommended) ⚡🛡️

**Strategy**: Use fastest available, fallback on failure

```rust
async fn get_workspace_stats(workspace_id: Uuid) -> Result<WorkspaceStats> {
    // 1. Try PostgreSQL documents table (fastest if populated)
    if let Ok(stats) = try_postgres_stats(workspace_id).await {
        return Ok(stats);
    }
    
    // 2. Fallback to KV storage (moderate speed, reliable)
    if let Ok(stats) = try_kv_storage_stats(workspace_id).await {
        return Ok(stats);
    }
    
    // 3. Fallback to AGE graph (slowest, last resort)
    try_age_graph_stats(workspace_id).await
}
```

**Pros**:
- ⚡ **Fast when available**: Uses PostgreSQL if populated
- 🛡️ **Reliable**: Graceful degradation to KV storage
- 🔧 **Forward compatible**: Ready when documents table gets populated
- 📊 **Flexible**: Can add caching layer easily

**Cons**:
- 🔧 **Complex**: More code paths to maintain
- 🐛 **Debugging**: Harder to trace which backend was used
- ⚠️ **Consistency**: Multiple sources of truth can diverge

**Performance**: 1-5ms (PostgreSQL) → 20-100ms (KV) → 50-200ms (AGE)

---

## Recommended Solution

### Phase 1: Immediate (Use Hybrid with KV Primary) ✅

**Status**: Already implemented in commit `ca857b22`

1. **Primary**: KV storage aggregation (current implementation)
2. **Fallback**: AGE graph queries (if KV fails)
3. **Future**: PostgreSQL tables (when populated)

**Rationale**:
- KV storage is the current source of truth for document metadata
- Reliable and works with existing data architecture
- Moderate performance acceptable for current scale

### Phase 2: Performance Optimization (Populate PostgreSQL) 🚀

**Modify document upload pipeline** to write stats to `documents` table:

```rust
// In pipeline.rs after entity extraction
sqlx::query(
    "INSERT INTO documents (id, workspace_id, title, entity_count, relationship_count, chunk_count, file_size_bytes)
     VALUES ($1, $2, $3, $4, $5, $6, $7)
     ON CONFLICT (id) DO UPDATE SET
         entity_count = EXCLUDED.entity_count,
         relationship_count = EXCLUDED.relationship_count,
         chunk_count = EXCLUDED.chunk_count"
)
.bind(doc_id)
.bind(workspace_id)
.bind(title)
.bind(entity_count)
.bind(relationship_count)
.bind(chunk_count)
.bind(file_size_bytes)
.execute(&pool)
.await?;
```

**Modified handler** to use PostgreSQL first:
```rust
// Try PostgreSQL first (fast path)
match get_postgres_stats(workspace_id).await {
    Ok(stats) if stats.document_count > 0 => return Ok(stats),
    _ => {
        // Fallback to KV storage
        get_kv_storage_stats(workspace_id).await
    }
}
```

**Benefits**:
- 10-20x faster queries (1-5ms vs 20-100ms)
- Better scalability for large workspaces
- Enables complex analytics queries

### Phase 3: Metrics History (Time-Series) 📊

**Populate `workspace_metrics_history`** on:
- Document upload completion
- Reprocess completion  
- Rebuild embeddings completion
- Manual snapshot trigger

```sql
INSERT INTO workspace_metrics_history 
(workspace_id, document_count, entity_count, relationship_count, chunk_count, embedding_count, storage_bytes, recorded_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, NOW());
```

**Benefits**:
- Trend analysis (growth over time)
- Debugging (compare before/after processing)
- Auditing (workspace activity tracking)

---

## Implementation Priority

| Phase | Effort | Impact | Priority |
|-------|--------|--------|----------|
| **Phase 1**: Current KV implementation | ✅ Done | Medium | ✅ COMPLETE |
| **Phase 2**: Populate PostgreSQL tables | 4-6 hours | High | 🔴 HIGH |
| **Phase 3**: Metrics history | 2-3 hours | Low | 🟡 MEDIUM |

---

## Performance Comparison Table

| Method | Query Time | Scalability | Data Freshness | Complexity |
|--------|-----------|-------------|----------------|------------|
| PostgreSQL (if populated) | 1-5ms | ⭐⭐⭐⭐⭐ Excellent | Real-time | Low |
| KV Storage (current) | 20-100ms | ⭐⭐⭐ Good | Real-time | Medium |
| AGE Graph | 50-200ms | ⭐⭐ Fair | Real-time | High |
| Cached (future) | <1ms | ⭐⭐⭐⭐⭐ Excellent | Eventual | Medium |

---

## Conclusion

**Current State**: KV storage aggregation works but is suboptimal for performance.

**Recommended Next Step**: Implement Phase 2 to populate PostgreSQL `documents` table during pipeline processing. This provides:
- 10-20x performance improvement
- Better scalability for large workspaces
- Foundation for complex analytics queries
- Maintains backward compatibility (KV as fallback)

**Quick Win**: Add caching layer (Redis/in-memory) with 30-60 second TTL to reduce repeated queries.
