# OODA Iteration 13 - Observe

**Mission Re-read**: specs/033-study-delete-document/003-study-document.md

## Focus Area: Real-time Workspace Stats Implementation

The mission requires:
> "Ensure metric likes number of Entities, Relationships, Embeddings per document, Relations, Entity Types are tracked"

Current state: WorkspaceStats returns all zeros (stub implementation from OODA-12).

## Database Schema Analysis

### Tables for Counting

| Table | Column | Purpose |
|-------|--------|---------|
| `documents` | `workspace_id` | Count documents per workspace |
| `chunks` | `workspace_id` | Count chunks per workspace |
| `entities` | `workspace_id` | Count entities per workspace |
| `relationships` | `workspace_id` | Count relationships per workspace |

### SQL Queries for Stats

```sql
-- Document count
SELECT COUNT(*) FROM documents WHERE workspace_id = $1;

-- Chunk count  
SELECT COUNT(*) FROM chunks WHERE workspace_id = $1;

-- Entity count
SELECT COUNT(*) FROM entities WHERE workspace_id = $1;

-- Relationship count
SELECT COUNT(*) FROM relationships WHERE workspace_id = $1;

-- Embedding count (chunks with non-null embedding)
SELECT COUNT(*) FROM chunks WHERE workspace_id = $1 AND embedding IS NOT NULL;

-- Storage bytes (sum of document file sizes)
SELECT COALESCE(SUM(file_size_bytes), 0) FROM documents WHERE workspace_id = $1;
```

## Current Implementation Location

**File**: `edgequake/crates/edgequake-core/src/workspace_service_impl.rs`
**Lines**: 618-639

```rust
async fn get_workspace_stats(&self, workspace_id: Uuid) -> Result<WorkspaceStats> {
    // Verify workspace exists
    let _ = self.get_workspace(workspace_id).await?...;
    
    // STUB: All zeros
    Ok(WorkspaceStats {
        workspace_id,
        document_count: 0,
        entity_count: 0,
        relationship_count: 0,
        chunk_count: 0,
        embedding_count: 0,
        storage_bytes: 0,
    })
}
```

## Architecture Constraint

The `PostgresWorkspaceService` has access to `self.pool` (sqlx PgPool).
It can run SQL queries directly without needing storage adapters.

## Observation Summary

1. PostgreSQL tables exist with proper `workspace_id` columns
2. Simple COUNT queries can provide real-time stats
3. Implementation only requires adding SQL queries to existing method
4. No schema changes needed
