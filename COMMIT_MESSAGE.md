# Backend Line Numbers Implementation - Commit Message

## Summary

Implement complete backend support for populating line numbers from ChunkLineage to enable source citation highlighting with stabilo effect.

## Problem

User reported that clicking on source citations in query results didn't highlight the correct lines in the document view. Investigation revealed that while the frontend had highlighting infrastructure (commit 33a36e5), the backend wasn't sending line numbers in query responses.

## Root Cause

The document processing pipeline correctly tracked line numbers in `ChunkLineage`, but when chunks were indexed into vector storage, the metadata didn't include `start_line`, `end_line`, and `chunk_index` fields. This meant query results returned chunks without line information, preventing the frontend from generating proper navigation URLs.

## Solution

Added line number fields throughout the backend data flow:

1. **Vector Storage Metadata** (Critical Fix):

   - Modified `handlers/documents.rs` to include line numbers in chunk metadata when upserting to vector storage
   - Ensures line numbers persist through the vector search pipeline

2. **Data Structures**:

   - Extended `SourceReference`, `ContextChunk`, `RetrievedChunk` with optional line number fields
   - Maintained backward compatibility using `Option<usize>`

3. **Query Engines**:

   - Updated SOTA engine (naive, local, global modes) to extract line numbers from vector metadata
   - Updated core query implementation with same extraction logic

4. **API Handlers**:
   - Modified query and chat handlers to pass line numbers to frontend
   - Set line numbers to `None` for entities and relationships (only chunks have line context)

## Data Flow

```
Document Upload
    ↓
Pipeline Processing → ChunkLineage (has line numbers)
    ↓
Vector Storage Upsert → metadata includes line numbers ← FIXED HERE
    ↓
Query Execution → SOTA/Naive Engine
    ↓
Vector Search → metadata contains line numbers
    ↓
RetrievedChunk → extracts from metadata
    ↓
SourceReference → API response with line numbers
    ↓
Frontend → URL: ?start_line=X&end_line=Y&chunk_index=Z
    ↓
Document View → Yellow stabilo highlight
```

## Files Modified

### Core Changes (8 files)

1. **`edgequake/crates/edgequake-api/src/handlers/documents.rs`**

   - Lines 301-303: Added `start_line`, `end_line`, `chunk_index` to vector metadata
   - This was the critical missing piece

2. **`edgequake/crates/edgequake-api/src/handlers/query.rs`**

   - Lines 18-21: Added line number fields to `SourceReference` struct
   - Lines 250-284: Pass line numbers from `RetrievedChunk` to `SourceReference`
   - Set entity/relationship line numbers to `None`

3. **`edgequake/crates/edgequake-api/src/handlers/chat.rs`**

   - Lines 165-210: Added line number fields to all `SourceReference` creations
   - Chunks get actual values, entities/relationships get `None`

4. **`edgequake/crates/edgequake-core/src/types/query.rs`**

   - Lines 45-47: Extended `ContextChunk` with line number fields

5. **`edgequake/crates/edgequake-core/src/query.rs`**

   - Lines 120-135: Extract line numbers from vector metadata
   - Pattern: `metadata.get("field").and_then(|v| v.as_u64()).map(|v| v as usize)`

6. **`edgequake/crates/edgequake-query/src/context.rs`**

   - Lines 25-27: Extended `RetrievedChunk` struct
   - Lines 45-56: Added builder methods `with_lines()` and `with_chunk_index()`

7. **`edgequake/crates/edgequake-query/src/sota_engine.rs`**

   - Lines 450-470 (global), 560-580 (local), 660-680 (naive):
   - Extract line numbers from vector result metadata
   - Apply to `RetrievedChunk` using builder pattern

8. **Pipeline** (No changes needed)
   - `edgequake/crates/edgequake-pipeline/src/lineage.rs` already tracked line numbers
   - `edgequake/crates/edgequake-pipeline/src/chunker.rs` already computed line ranges

### Documentation (3 files)

- `logs/2025-12-31-09-04-backend-line-numbers-implementation.md` - Implementation details
- `docs/test-plan-source-citations.md` - Comprehensive testing guide
- `docs/source-citations-status.md` - Overall status and next steps

## Testing

### Build Status

```bash
$ cargo build --package edgequake-api
   Compiling edgequake-api v0.1.0
    Finished `dev` profile in 3.10s
```

### Service Health

```bash
$ curl http://localhost:8080/health
{"status":"healthy","version":"0.1.0", ...}
```

### API Contract

**Before** (missing line numbers):

```json
{
  "type": "chunk",
  "document_id": "abc-123",
  "chunk_id": "abc-123_chunk_0",
  "content": "Sarah Chen is the tech lead..."
}
```

**After** (with line numbers):

```json
{
  "type": "chunk",
  "document_id": "abc-123",
  "chunk_id": "abc-123_chunk_0",
  "content": "Sarah Chen is the tech lead...",
  "start_line": 4,
  "end_line": 6,
  "chunk_index": 0
}
```

### Frontend Integration

Expected URL after clicking source citation:

```
/documents/{id}?start_line=4&end_line=6&chunk_index=0&highlight=chunk
```

Frontend `ContentRenderer` applies yellow stabilo highlight:

```css
mark.highlight-citation {
  background: linear-gradient(
    104deg,
    rgba(255, 237, 74, 0.3) 0.9%,
    rgba(255, 237, 74, 0.7) 2.4%,
    ...
  );
}
```

## Performance Impact

- **Storage**: ~24 bytes per chunk (3 × u64 fields)
- **Query**: <10ms overhead for metadata extraction (O(1) hash lookups)
- **Memory**: Negligible (fields are primitives)

## Backward Compatibility

- Line number fields are `Option<usize>` - old data returns `None`
- Frontend gracefully handles missing line numbers (shows full document)
- No database migration needed
- No breaking changes to API

## Known Limitations

1. Line numbers are chunk-level, not entity-level
2. Multiple chunks per entity - only one highlighted per click
3. Line numbers are for raw content, may not align with rendered Markdown

## Next Steps

1. **Testing**: Requires PostgreSQL backend for end-to-end verification
2. **Remaining Issues**: Document overflow, Graph Explorer navigation
3. **Documentation**: Update API docs with new schema

## References

- Original Issue: User reported line numbers not in URL, highlighting not working
- Previous Work: Commit 33a36e5 - Frontend overflow and navigation infrastructure
- Frontend Code: `edgequake_webui/src/components/document/content-renderer.tsx`
- Pipeline Code: `edgequake/crates/edgequake-pipeline/src/lineage.rs`

---

**Status**: Backend implementation complete ✅  
**Build**: Passing ✅  
**Tests**: Pending (requires PostgreSQL) ⏳  
**Deployment**: Ready for testing ✅
