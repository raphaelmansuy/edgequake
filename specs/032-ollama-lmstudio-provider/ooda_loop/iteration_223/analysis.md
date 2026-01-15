# OODA-223: CRAG Query Returns "0 Sources" Investigation

## Issue Summary

User reports that querying "What is CRAG?" returns a response with "0 Sources · 20 Topics" but no actual document chunks in the source citations panel.

![Screenshot Description](Screenshot shows query interface with CRAG question, response generated, but "0 Sources" and "No source documents" in Documents tab)

## Investigation Timeline

| Phase | Action | Finding |
|-------|--------|---------|
| OBSERVE | Check PostgreSQL tables | `documents`, `chunks`, `entities` tables have 0 rows |
| OBSERVE | Check vector tables | `eq_eq_default_vectors` has 44 rows with data |
| OBSERVE | Check workspace tables | `eq_eq_default_ws_4e32a055_vectors` has 0 rows |
| ORIENT | Trace data architecture | Data in global table, queries look in workspace-specific table |
| ORIENT | Check creation timestamps | Global data: 2026-01-14, Workspace table: empty |
| DECIDE | Root cause identified | Data migration issue - legacy data not migrated |

## Root Cause

The "0 Sources" issue is caused by a **data location mismatch**:

### Data Storage Architecture

```
eq_eq_default_vectors (GLOBAL - 44 rows)
├── Contains: entities and chunks from OODA64-TestKG workspace
├── Created: 2026-01-14 16:18:49
├── Dimension: vector(1536)
└── Has workspace_id in metadata

eq_eq_default_ws_4e32a055_vectors (WORKSPACE-SPECIFIC - 0 rows)  
├── Created: 2026-01-15 (on first query)
├── Dimension: vector(1536)
└── EMPTY - no data migrated
```

### Timeline Analysis

1. **2026-01-14 16:18:49**: Documents ingested into OODA64-TestKG workspace
   - Data stored in `eq_eq_default_vectors` (global table)
   - 27 vectors with `workspace_id` = `4e32a055-9722-40f9-b03e-ade870b07604` in metadata

2. **Later Implementation**: Per-workspace vector table feature implemented
   - Each workspace now gets dedicated `eq_{ns}_ws_{id}_vectors` table
   - Query handler updated to use workspace-specific tables

3. **2026-01-15 08:56:50**: First query attempt
   - System creates empty `eq_eq_default_ws_4e32a055_vectors` table
   - Query looks in workspace table (empty)
   - Returns 0 chunks, 0 entities

### Why "20 Topics" Shows Up

The "20 Topics" in the UI refers to entities retrieved from the knowledge graph (AGE), not from vector search. The graph storage still contains entity data, but the vector-based semantic search returns no results.

## Evidence

### Vector Tables Content

```sql
-- Global table has data
SELECT COUNT(*) FROM eq_eq_default_vectors;
-- Result: 44

-- Workspace table is empty
SELECT COUNT(*) FROM eq_eq_default_ws_4e32a055_vectors;
-- Result: 0

-- Data belongs to workspace
SELECT metadata->>'workspace_id' as ws, COUNT(*) 
FROM eq_eq_default_vectors 
WHERE metadata->>'tenant_id' = '2898d9de-a380-40bc-91b3-9ce0db8a5798'
GROUP BY metadata->>'workspace_id';
-- Result: 4e32a055-9722-40f9-b03e-ade870b07604 | 27
```

### Query Handler Behavior

```
2026-01-15T08:56:50.382392Z DEBUG query: Getting workspace-specific vector storage 
  workspace_id=4e32a055-9722-40f9-b03e-ade870b07604 
  dimension=1536

2026-01-15T08:56:50.424038Z INFO: Created workspace-specific vector storage
  table=eq_default_ws_4e32a055_vectors
```

## Fix Options

### Option 1: Data Migration Script (Recommended)

Create a one-time migration to copy vectors from global table to workspace-specific tables:

```sql
-- Migrate vectors for OODA64-TestKG workspace
INSERT INTO eq_eq_default_ws_4e32a055_vectors (id, embedding, metadata, created_at)
SELECT id, embedding, metadata, created_at
FROM eq_eq_default_vectors
WHERE metadata->>'workspace_id' = '4e32a055-9722-40f9-b03e-ade870b07604';
```

**Pros**:
- Clean separation of workspace data
- Preserves multi-tenancy isolation
- Query performance improvement (smaller table scans)

**Cons**:
- One-time migration effort
- Need to handle dimension mismatches

### Option 2: Query Fallback

Modify query handler to fallback to global table if workspace table is empty:

```rust
// In query handler
let results = workspace_storage.search(query_embedding, limit).await?;
if results.is_empty() {
    // Fallback: filter global table by workspace_id
    results = global_storage.search_with_filter(
        query_embedding, 
        limit,
        json!({"workspace_id": workspace_id})
    ).await?;
}
```

**Pros**:
- Works immediately without migration
- Backward compatible

**Cons**:
- Performance penalty on global table
- Violates data isolation principle
- Temporary workaround

### Option 3: Re-ingest Documents

User re-uploads documents to workspace after the per-workspace table feature is active.

**Pros**:
- Cleanest solution
- Uses correct embedding provider/dimension

**Cons**:
- User action required
- Loss of document processing history

## Recommended Action

**Option 1 (Migration)** is recommended for this specific case because:

1. Data already exists with correct structure (has workspace_id in metadata)
2. Dimension matches (1536)
3. One-time effort for permanent fix

## Migration Script

```bash
# Execute migration for all affected workspaces
docker exec edgequake-postgres psql -U edgequake -d edgequake << 'EOF'
-- Get list of unique workspace_ids in global table
WITH workspace_ids AS (
    SELECT DISTINCT metadata->>'workspace_id' as ws_id
    FROM eq_eq_default_vectors
    WHERE metadata->>'workspace_id' IS NOT NULL
)
SELECT ws_id, 
       'eq_eq_default_ws_' || substring(ws_id, 1, 8) || '_vectors' as target_table
FROM workspace_ids;
EOF
```

## Migration Executed

### Migration Command

```sql
-- Migrate vectors for OODA64-TestKG workspace (4e32a055)
INSERT INTO eq_eq_default_ws_4e32a055_vectors (id, embedding, metadata, created_at)
SELECT id, embedding, metadata, created_at
FROM eq_eq_default_vectors
WHERE metadata->>'workspace_id' = '4e32a055-9722-40f9-b03e-ade870b07604';
-- Result: INSERT 0 27
```

### Verification Results

Before migration:
```json
{"chunks": 0, "entities": 0, "relationships": 0}
```

After migration - Naive mode (EdgeQuake query):
```json
[{"type": "chunk", "count": 3}]
```

After migration - Hybrid mode (Sarah Chen query):
```json
[
  {"type": "chunk", "count": 2},
  {"type": "entity", "count": 24},
  {"type": "relationship", "count": 19}
]
```

## Status

**Issue**: RESOLVED via data migration  
**Root Cause**: Legacy data stored in global table before per-workspace tables implemented  
**Fix Applied**: Migrated 27 vectors to workspace-specific table  
**Verification**: ✅ Queries now return chunks, entities, and relationships  

## Key Learnings

1. **Feature Flag Migrations**: When adding per-workspace isolation features, a migration strategy is needed for existing data
2. **Metadata-Based Filtering**: Even with global tables, workspace_id in metadata allows targeted migrations
3. **Dimension Compatibility**: The migration worked because source (global) and target (workspace) tables had matching dimensions (1536)

## Future Recommendations

1. Add migration tooling for moving data between storage backends
2. Consider automatic migration on first workspace-specific query if data exists in global table
3. Add monitoring for workspace data isolation integrity

## Related

- OODA-222: Dimension mismatch investigation
- SPEC-033: Per-workspace vector storage isolation
- FEAT0350: Per-workspace vector storage with independent dimensions
