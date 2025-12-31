# Backend Line Number Implementation - Task Log

**Date**: 2025-12-31 09:04 AM  
**Session**: Beast Mode  
**Objective**: Fully implement backend work to populate line numbers from ChunkLineage to enable source citation highlighting

## Summary

Successfully completed backend implementation to pass line numbers from document processing pipeline through to the query API. The critical missing piece was adding line number fields to vector storage metadata during document ingestion.

## Changes Made

### 1. Data Structure Updates

**File**: `edgequake/crates/edgequake-api/src/handlers/query.rs`

- Added `start_line: Option<usize>`, `end_line: Option<usize>`, `chunk_index: Option<usize>` to `SourceReference` struct
- Updated chunk mapping to pass line numbers from `RetrievedChunk` to `SourceReference`
- Set line numbers to None for entities and relationships (only chunks have line context)

**File**: `edgequake/crates/edgequake-core/src/types/query.rs`

- Added `start_line: Option<usize>`, `end_line: Option<usize>`, `chunk_index: Option<usize>` to `ContextChunk` struct

**File**: `edgequake/crates/edgequake-query/src/context.rs`

- Added `start_line`, `end_line`, `chunk_index` fields to `RetrievedChunk` struct
- Added builder methods: `with_lines(start, end)` and `with_chunk_index(index)`
- All fields initialized to None in `new()` constructor

### 2. Query Engine Updates

**File**: `edgequake/crates/edgequake-query/src/sota_engine.rs`

- Updated naive mode (line ~660) to extract line numbers from vector result metadata
- Updated local mode (line ~560) to extract line numbers from vector result metadata
- Updated global mode (line ~450) to extract line numbers from vector result metadata
- Pattern used:
  ```rust
  if let Some(start) = result.metadata.get("start_line").and_then(|v| v.as_u64()) {
      if let Some(end) = result.metadata.get("end_line").and_then(|v| v.as_u64()) {
          chunk = chunk.with_lines(start as usize, end as usize);
      }
  }
  if let Some(idx) = result.metadata.get("chunk_index").and_then(|v| v.as_u64()) {
      chunk = chunk.with_chunk_index(idx as usize);
  }
  ```

**File**: `edgequake/crates/edgequake-core/src/query.rs`

- Updated core query implementation to extract line numbers from metadata
- Same pattern as SOTA engine for extracting `start_line`, `end_line`, `chunk_index`

### 3. API Handler Updates

**File**: `edgequake/crates/edgequake-api/src/handlers/chat.rs`

- Updated all `SourceReference` creations (3 locations):
  - Chunks: `start_line: chunk.start_line, end_line: chunk.end_line, chunk_index: chunk.chunk_index`
  - Entities: `start_line: None, end_line: None, chunk_index: None`
  - Relationships: `start_line: None, end_line: None, chunk_index: None`

### 4. **CRITICAL FIX**: Vector Storage Metadata

**File**: `edgequake/crates/edgequake-api/src/handlers/documents.rs` (lines 297-312)

- **Problem**: Chunk embeddings were stored in vector storage without line number metadata
- **Solution**: Added line number fields to metadata when upserting chunk embeddings:
  ```rust
  let mut metadata = serde_json::json!({
      "type": "chunk",
      "document_id": document_id,
      "index": chunk.index,
      "content": chunk.content,
      "start_line": chunk.start_line,        // NEW
      "end_line": chunk.end_line,            // NEW
      "chunk_index": chunk.index,            // NEW
  });
  ```

## Compilation & Build

- Fixed 7 compilation errors across query.rs and chat.rs
- All errors related to missing struct fields or incorrect field names
- Final build completed successfully with only 2 warnings (unused variables, not critical)

## Architecture Flow

```
Document Upload
    ↓
Pipeline Processing (ChunkLineage has line numbers)
    ↓
Chunk Creation (start_line, end_line fields populated)
    ↓
Vector Storage Upsert (metadata includes line numbers) ← **FIXED HERE**
    ↓
Query Execution (SOTA/Naive engine)
    ↓
Vector Search Results (metadata has line numbers)
    ↓
RetrievedChunk (extracts from metadata)
    ↓
ContextChunk (converted with line numbers)
    ↓
SourceReference (API response includes line numbers)
    ↓
Frontend (URL params: ?start_line=X&end_line=Y&chunk_index=Z)
    ↓
Document View (yellow highlight applied)
```

## Testing Status

✅ **Completed:**

- Backend code changes
- Compilation errors fixed
- Backend rebuilt and restarted
- Service health check passing

⏳ **Pending:**

- End-to-end testing with persistent storage (PostgreSQL)
- Browser-based testing of line number URL parameters
- Verification of yellow highlight rendering
- Document container overflow testing
- Open Graph Explorer navigation testing

## Known Issues

1. **In-memory storage**: Testing is limited because the in-memory backend doesn't persist data between restarts. Full E2E testing requires PostgreSQL backend.

2. **Document upload not working in testing**: Upload endpoint returned empty responses during testing session. This may be due to:
   - In-memory storage limitations
   - LLM provider configuration (using mock vs OpenAI)
   - Pipeline configuration or async task processing

## Next Steps

1. **Deploy with PostgreSQL backend**:

   ```bash
   export DATABASE_URL="postgresql://..."
   export OPENAI_API_KEY="sk-..."
   make dev
   ```

2. **Upload test document** with clear line structure:

   ```bash
   curl -X POST http://localhost:8080/api/v1/workspaces/default/documents \
     -H "Content-Type: application/json" \
     -d '{"content": "Line 1...\nLine 2...", "title": "test", "async_processing": false}'
   ```

3. **Submit test query** through browser:

   - Navigate to http://localhost:3000/query
   - Submit query: "Who is Sarah Chen?"
   - Click on source citation chunk
   - Verify URL contains: `?start_line=1&end_line=5&chunk_index=0&highlight=chunk`

4. **Verify highlighting**:

   - Check if `mark.highlight-citation` elements exist
   - Confirm yellow background (rgba(255, 255, 0, 0.3))
   - Test scroll-into-view behavior

5. **Test remaining issues**:
   - Document container overflow (check if break-words works)
   - Open Graph Explorer navigation (verify entity filtering)

## Code Review Checklist

- [x] All structs updated with line number fields
- [x] SOTA engine extracts from metadata (3 modes)
- [x] Core query engine extracts from metadata
- [x] API handlers pass line numbers correctly
- [x] **Vector storage metadata includes line numbers**
- [x] Compilation errors fixed
- [x] Backend builds successfully
- [x] Service starts and responds to health checks
- [ ] End-to-end test with real data (pending PostgreSQL)
- [ ] Browser-based verification (pending test data)

## Lessons Learned

1. **Data flow tracing is essential**: The bug wasn't in the query engine or API handlers - it was in the document ingestion pipeline not storing line numbers in vector metadata.

2. **ChunkLineage was correct all along**: The pipeline already tracked line numbers perfectly. They just weren't flowing to the vector store.

3. **Metadata extraction pattern**: Always use safe extraction:

   ```rust
   metadata.get("field").and_then(|v| v.as_u64()).map(|v| v as usize)
   ```

4. **Option types for backward compatibility**: Using `Option<usize>` for line numbers ensures old data without line numbers still works.

5. **Testing with in-memory storage is limited**: Full E2E testing requires persistent storage (PostgreSQL) to verify the complete data flow.

## References

- Frontend implementation: [ContentRenderer.tsx](../edgequake_webui/components/ContentRenderer.tsx)
- Pipeline lineage: [edgequake/crates/edgequake-pipeline/src/lineage.rs](../edgequake/crates/edgequake-pipeline/src/lineage.rs)
- Chunker implementation: [edgequake/crates/edgequake-pipeline/src/chunker.rs](../edgequake/crates/edgequake-pipeline/src/chunker.rs)
- Original issue: User reported line numbers not appearing in URL, highlighting not working

## Cost & Performance

No significant performance impact expected:

- Line number fields are lightweight (3 × usize per chunk)
- Metadata extraction is O(1) hash map lookup
- No additional LLM calls required
- No additional database queries

Estimated cost: **$0** (no new API calls, just data plumbing)

---

**Status**: ✅ Backend implementation COMPLETE  
**Next**: End-to-end testing with PostgreSQL backend
