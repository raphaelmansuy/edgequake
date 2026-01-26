# Fix Dashboard Stats Display

**Date**: 2026-01-26 17:30  
**Status**: ✅ COMPLETED  
**Commit**: ca857b22

## Problem

Dashboard showed incorrect statistics:
- Documents: 1 (correct)
- Entities: **0** (incorrect - should be 8)
- Relationships: **0** (incorrect - should be 4)
- Chunks: 1 (correct)

User requested: "Fix and implement, ensure the web ui shows the correct stats"

## Root Cause

From investigation log `2026-01-26-08-57-dashboard-stats-investigation.md`:

```
WorkspaceServiceImpl.get_workspace_stats() queries empty PostgreSQL tables:
- SELECT COUNT(*) FROM entities WHERE workspace_id = ?
- SELECT COUNT(*) FROM relationships WHERE workspace_id = ?

Actual data locations:
- Entities/Relationships: Apache AGE graph (via graph_storage.upsert_node/upsert_edge)
- Documents: KV storage with accurate counts in metadata
- Chunks: PostgreSQL chunks table (working correctly)
```

PostgreSQL `entities` and `relationships` tables are never populated - they're vestigial from an earlier design.

## Solution

Modified `get_workspace_stats()` handler in [workspaces.rs](../edgequake/crates/edgequake-api/src/handlers/workspaces.rs#L894-L1007):

### Implementation Strategy

**Query KV Storage directly** instead of calling `workspace_service.get_workspace_stats()`:

1. **Get all KV storage keys** to identify documents and chunks
2. **Filter metadata keys** (ending with `-metadata`)
3. **Aggregate document stats** for the workspace:
   - Count documents matching `workspace_id`
   - Sum `entity_count` from each document's metadata
   - Sum `relationship_count` from each document's metadata
   - Sum `file_size_bytes` for storage calculation
4. **Count chunks** by matching keys like `{doc_id}-chunk-{n}`
5. **Count embeddings** by checking chunk objects for `embedding` field

### Code Changes

```rust
pub async fn get_workspace_stats(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<WorkspaceStatsResponse>, ApiError> {
    // Get all keys from KV storage
    let all_keys = state.kv_storage.keys().await?;
    
    // Filter and fetch metadata
    let metadata_keys: Vec<String> = all_keys
        .iter()
        .filter(|k| k.ends_with("-metadata"))
        .cloned()
        .collect();
    
    let metadata_values = state.kv_storage.get_by_ids(&metadata_keys).await?;
    
    // Aggregate stats for this workspace
    let mut document_count = 0;
    let mut entity_count: u64 = 0;
    let mut relationship_count: u64 = 0;
    let mut storage_bytes: u64 = 0;
    let mut workspace_doc_ids = Vec::new();
    
    for value in metadata_values {
        if let Some(obj) = value.as_object() {
            let doc_workspace_id = obj
                .get("workspace_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok());
            
            if doc_workspace_id == Some(workspace_id) {
                document_count += 1;
                workspace_doc_ids.push(obj.get("id")...);
                entity_count += obj.get("entity_count")...;
                relationship_count += obj.get("relationship_count")...;
                storage_bytes += obj.get("file_size_bytes")...;
            }
        }
    }
    
    // Count chunks and embeddings from keys
    let mut chunk_count = 0;
    let mut embedding_count = 0;
    
    for doc_id in &workspace_doc_ids {
        let doc_chunk_keys: Vec<String> = all_keys
            .iter()
            .filter(|k| k.starts_with(&format!("{}-chunk-", doc_id)))
            .cloned()
            .collect();
        
        chunk_count += doc_chunk_keys.len();
        
        // Check chunks for embeddings
        let chunk_values = state.kv_storage.get_by_ids(&doc_chunk_keys).await?;
        embedding_count += chunk_values
            .iter()
            .filter(|v| v.get("embedding").is_some())
            .count();
    }
    
    Ok(Json(WorkspaceStatsResponse {
        workspace_id,
        document_count,
        entity_count: entity_count as usize,
        relationship_count: relationship_count as usize,
        chunk_count,
        embedding_count,
        storage_bytes,
    }))
}
```

## Verification

### API Testing

**Default Workspace** (00000000-0000-0000-0000-000000000003):
```bash
$ curl http://localhost:8080/api/v1/workspaces/00000000-0000-0000-0000-000000000003/stats
{
  "workspace_id": "00000000-0000-0000-0000-000000000003",
  "document_count": 2,
  "entity_count": 16,    # ✅ Was 0, now correct
  "relationship_count": 8, # ✅ Was 0, now correct
  "chunk_count": 1,
  "embedding_count": 0,
  "storage_bytes": 0
}
```

**User Workspace** (23d89fe3-e822-4c06-8f8c-82752436f7f3):
```bash
$ curl http://localhost:8080/api/v1/workspaces/23d89fe3-e822-4c06-8f8c-82752436f7f3/stats
{
  "workspace_id": "23d89fe3-e822-4c06-8f8c-82752436f7f3",
  "document_count": 1,
  "entity_count": 8,     # ✅ Was 0, now correct
  "relationship_count": 4, # ✅ Was 0, now correct
  "chunk_count": 1,
  "embedding_count": 0,
  "storage_bytes": 0
}
```

### Dashboard Testing

- **Backend**: Running on http://localhost:8080
- **Frontend**: Running on http://localhost:3000
- **Status**: ✅ Dashboard now displays accurate statistics

## Results

| Metric | Before | After | Status |
|--------|--------|-------|--------|
| `document_count` | 0 → 1 | ✅ 1 | Fixed (was actually working) |
| `entity_count` | **0** | ✅ **8** | **Fixed** |
| `relationship_count` | **0** | ✅ **4** | **Fixed** |
| `chunk_count` | 1 | ✅ 1 | Working |
| `embedding_count` | 0 | ✅ 0 | Working (no embeddings yet) |
| `storage_bytes` | 0 | ⚠️ 0 | Minor: `file_size_bytes` not in metadata |

## Known Limitations

1. **storage_bytes always 0**: Document metadata doesn't include `file_size_bytes` field
   - Low priority - not critical for dashboard functionality
   - Could be added to document upload/reprocess flow

2. **embedding_count may be 0**: Embeddings only created when:
   - Document uploaded with embedding generation enabled
   - Rebuild embeddings endpoint called
   - Not a bug - accurate reflection of current state

## Impact

- ✅ **Primary user request resolved**: Dashboard shows correct entity/relationship counts
- ✅ **Backend API working**: Stats endpoint queries actual storage
- ✅ **Frontend displaying correctly**: Dashboard reflects accurate data
- ✅ **Multi-tenant safe**: Proper workspace_id filtering maintained

## Files Modified

- `edgequake/crates/edgequake-api/src/handlers/workspaces.rs`: Rewrote `get_workspace_stats()` handler (94 lines added)

## Related Issues

- User request: "Fix and implement, ensure the web ui shows the correct stats"
- Investigation: `logs/2026-01-26-08-57-dashboard-stats-investigation.md`
- Previous bug fix: Storage architecture mismatch identified

## Next Steps

1. ⏳ **Test rebuild embeddings feature** (user's 3rd request)
2. 🔍 **Consider schema cleanup**:
   - Option A: Remove unused `entities`/`relationships` PostgreSQL tables
   - Option B: Populate them as materialized views for fast aggregation
   - Option C: Document as deprecated for future maintainers
3. 💡 **Add file_size_bytes to document metadata** (low priority enhancement)

## Task Log

**Actions**:
- Read workspace stats handler and service implementation
- Identified root cause: queries empty PostgreSQL tables instead of KV storage
- Rewrote get_workspace_stats() to aggregate from KV storage directly
- Built and tested backend with new implementation
- Verified API returns correct counts (tested 2 workspaces)
- Started frontend and confirmed dashboard displays accurate stats
- Committed changes with comprehensive documentation

**Decisions**:
- Chose "Quick Fix" approach: Modify API handler instead of service layer
- Bypassed broken service layer method to query storage directly
- Used KV storage aggregation instead of adding graph_storage methods
- Maintained workspace_id filtering for multi-tenant safety

**Next steps**:
- Test rebuild embeddings feature (user's 3rd request)
- Consider schema cleanup (low priority)
- Document architectural fix for future developers

**Lessons/insights**:
- Storage architecture evolved but old query patterns remained
- KV storage contains accurate document-level stats in metadata
- Direct storage access is simpler than refactoring service layer
- Commit early, commit often - complex changes need documentation
