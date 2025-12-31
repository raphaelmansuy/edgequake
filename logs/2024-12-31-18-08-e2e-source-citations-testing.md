# Task Log: E2E Testing and Source Citations Fix

**Date:** 2024-12-31
**Mode:** beastmode (E2E testing with Playwright)
**Duration:** Extended session

## Session Summary

Testing EdgeQuake WebUI source citations with E2E browser automation and fixing critical backend chunk retrieval bug.

## Actions Performed

1. **E2E Testing Setup**
   - Started backend and frontend services
   - Uploaded test document via Playwright browser automation
   - Tested query modes: Hybrid and Naive (Simple)

2. **Issue #1 Diagnosed**
   - Hybrid mode shows "0 Sources" in Documents tab
   - Backend only returns entities and relationships, NO chunks
   - Root cause: Local and Global modes don't retrieve chunks from source_chunk_ids

3. **Backend Fix Implemented**
   - Added chunk retrieval logic to `query_local()` in `sota_engine.rs` (lines 777-840)
   - Added chunk retrieval logic to `query_global()` in `sota_engine.rs` (lines 1145-1200)
   - Fixed dummy embedding bug (was zero vector, changed to keyword embeddings)
   - Local mode uses `embeddings.low_level` for chunk queries
   - Global mode uses `embeddings.high_level` for chunk queries

4. **Issue #2 Validated**
   - Naive mode DOES show chunks with navigation links
   - Links work: navigate to document detail page
   - URL uses `highlight` parameter instead of `start_line`/`end_line`
   - Line numbers not visible in document view (expected from backend work)

5. **Issue #3 Inspected**
   - Document detail right panel has `overflow-y: visible`
   - Should be `overflow-y: auto` for scrollability
   - Currently content fits (scrollHeight == clientHeight)

## Key Findings

### Backend Architecture Issue
```rust
// BEFORE: Local and Global modes
// ❌ Only retrieved entities and relationships
// ❌ Ignored source_chunk_ids from entities/relationships
context.add_entity(entity);
context.add_relationship(rel);
return Ok(context); // chunks array empty!

// AFTER: Now retrieves chunks
// ✅ Collects source_chunk_ids from entities and relationships  
// ✅ Queries vector storage with filter_ids
// ✅ Adds chunks with line numbers to context
context.add_chunk(chunk);
```

### Vector Storage Filter Pattern
```rust
// ❌ WRONG: Zero vector has zero cosine similarity
let dummy = vec![0.0; 1536];
vector_storage.query(&dummy, top_k, Some(&chunk_ids))

// ✅ CORRECT: Use query embedding for similarity
vector_storage.query(&embeddings.low_level, top_k, Some(&chunk_ids))
```

## Files Modified

1. **`/Users/raphaelmansuy/Github/03-working/edgequake/edgequake/crates/edgequake-query/src/sota_engine.rs`**
   - `query_local()`: Added Step 7 for chunk retrieval (62 lines)
   - `query_global()`: Added Step 7 for source chunk retrieval (72 lines)

## Test Results

| Mode   | Sources | Chunks | Status                       |
|--------|---------|--------|------------------------------|
| Naive  | 1       | 1      | ✅ Working (baseline)         |
| Hybrid | 8/8     | 0      | ❌ No chunks before fix       |
| Local  | N/A     | N/A    | ⏳ Not tested yet             |
| Global | N/A     | N/A    | ⏳ Not tested yet             |

## Issues Status

| # | Issue                                                    | Status   |
|---|----------------------------------------------------------|----------|
| 1 | No source documents/chunks in Hybrid mode                | 🔧 Fixed |
| 2 | Links use `highlight` not `start_line`/`end_line`        | 📝 TODO  |
| 3 | Document detail right panel not scrollable               | 📝 TODO  |
| 4 | All documents in sources accessible/visible              | ⏳ TODO  |

## Next Steps

1. ✅ Restart backend with fixed code
2. ⏳ Re-upload test document to memory storage
3. ⏳ Test Hybrid mode query to verify chunks appear
4. ⏳ Fix Issue #2: Update frontend to use start_line/end_line params
5. ⏳ Fix Issue #3: Add overflow-y-auto CSS to right panel
6. ⏳ Complete E2E validation of all fixes
7. ⏳ Update `audit_lightrag_vs_edgequake/` documentation

## Technical Decisions

### Why use keyword embeddings for chunk retrieval?
- Local mode: Uses low-level keywords (specific terms) → chunks are specific
- Global mode: Uses high-level keywords (broader concepts) → chunks are general
- Better semantic matching than zero vector or random embedding

### Why not use KV storage for chunks?
- Chunks are stored in vector storage with metadata
- Existing `chunk_retrieval.rs` module expects KV storage (different design)
- Direct vector query with filter_ids is simpler and more efficient

### Why collect source_chunk_ids?
- Entities and relationships track which chunks they came from
- LightRAG pattern: retrieve original text for entity/relationship context
- Provides provenance and enables line number highlighting

## Lessons Learned

1. **E2E Testing Workflow**
   - Playwright MCP tools are powerful for automated UI testing
   - File upload requires programmatic click workaround (intercepted events)
   - Browser state inspection reveals backend data flow issues

2. **Backend Data Flow**
   - Query context flows: SOTA engine → API handler → SSE stream → Frontend
   - Each mode (Local/Global/Hybrid/Naive) has different retrieval strategy
   - Hybrid mode merges Local + Global but wasn't merging chunks

3. **Vector Storage Design**
   - filter_ids parameter works well for targeted retrieval
   - Cosine similarity requires non-zero query vector
   - Metadata includes line numbers from backend fix (previous session)

## Code Quality

- ✅ Cargo build successful
- ✅ No compile errors
- ⚠️ Two warnings (unused variables) - not critical
- ⏳ Tests not run yet (need functional backend)

## Cost Tracking

- Test document processing: $0.00037 (mock provider)
- E2E queries: 2-3 queries × ~100 tokens = minimal cost
- Total session cost: < $0.01

## Session Metrics

- Files read: 20+
- Files modified: 1
- Code added: ~140 lines (chunk retrieval logic)
- Terminal commands: 30+
- Browser operations: 15+ (navigate, click, type, evaluate)
- Token usage: ~104,000 (approaching limit)

