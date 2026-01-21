# OODA-230 Iteration 224: Orient

**Date**: January 16, 2026
**Issue**: Embedding Retrieval Broken in Hybrid Mode - ROOT CAUSE IDENTIFIED

## Root Cause Analysis

### The Bug: Missing `source_chunk_ids` Logic in Workspace Vector Methods

When the `_with_vector_storage` variants were created for workspace isolation (SPEC-033),
the **source_chunk_id retrieval logic was accidentally omitted**.

### Evidence Comparison

#### OLD `query_local` (lines 2055-2190) - CORRECT

```rust
// Step 7: Retrieve chunks from source_chunk_ids
let mut chunk_ids = std::collections::HashSet::new();

// Collect chunk IDs from entities
for entity in &context.entities {
    for chunk_id in &entity.source_chunk_ids {
        chunk_ids.insert(chunk_id.clone());
    }
}

// Collect chunk IDs from relationships
for rel in &context.relationships {
    if let Some(chunk_id) = &rel.source_chunk_id {
        chunk_ids.insert(chunk_id.clone());
    }
}

// Query with SPECIFIC chunk IDs
let results = self.vector_storage.query(
    &embeddings.low_level,
    chunk_ids_vec.len(),
    Some(&chunk_ids_vec),  // <-- FILTERS BY CHUNK ID
).await?;
```

#### NEW `query_local_with_vector_storage` (lines 2797-2894) - BUGGY

```rust
// Step 7: Get source chunks using workspace vector storage
let chunk_results = vector_storage.query(
    &embeddings.query,            // <-- USES QUERY EMBEDDING
    self.config.max_chunks * 2,
    None,                         // <-- NO CHUNK ID FILTER!
).await?;

let chunks = filter_by_type(chunk_results, VectorType::Chunk);  // <-- MAY RETURN EMPTY!
```

### Why This Causes "0 Sources"

1. Vector storage contains: entities, relationships, AND chunks
2. Query embedding returns TOP-K results by semantic similarity
3. Entities and relationships often score HIGHER than chunks for concept queries
4. `filter_by_type(VectorType::Chunk)` filters to only chunks
5. If all TOP-K results are entities/relationships → **0 chunks returned**

### Affected Methods

All `_with_vector_storage` variants have this bug:

| Method                             | Status                              |
| ---------------------------------- | ----------------------------------- |
| `query_local_with_vector_storage`  | ❌ Missing source_chunk_ids logic   |
| `query_global_with_vector_storage` | ❌ Missing source_chunk_ids logic   |
| `query_hybrid_with_vector_storage` | ❌ Calls buggy local/global methods |
| `query_naive_with_vector_storage`  | ⚠️ May work (direct chunk search)   |

### Confirmation Test

We can confirm this by:

1. Using Naive mode (direct chunk search) - should return chunks
2. Using Hybrid mode - returns 0 chunks

The user sees "20 Topics" (entities work) but "0 Sources" (chunks don't).

## Impact Assessment

| Severity        | Impact                                                                                   |
| --------------- | ---------------------------------------------------------------------------------------- |
| **CRITICAL**    | All workspace-specific queries in Local/Global/Hybrid/Mix modes return 0 document chunks |
| **Scope**       | Affects ALL workspaces using non-default providers (OpenAI, Ollama, LM Studio)           |
| **User Impact** | Answers lack document context; RAG is broken                                             |
| **Data Impact** | No data loss - chunks ARE stored correctly, just not retrieved                           |

## Fix Strategy

### Option A: Copy source_chunk_ids Logic (Simple, Verbose)

Copy the chunk collection logic from `query_local` to `query_local_with_vector_storage`.
Repeat for `query_global_with_vector_storage`.

**Pros**: Simple, low risk
**Cons**: Code duplication

### Option B: Extract Shared Method (DRY)

Create a shared helper function for chunk retrieval:

```rust
async fn collect_and_retrieve_chunks(
    &self,
    context: &QueryContext,
    embeddings: &QueryEmbeddings,
    vector_storage: &Arc<dyn VectorStorage>,
    tenant_id: &Option<String>,
    workspace_id: &Option<String>,
) -> Result<Vec<RetrievedChunk>>
```

Call from both `query_local` and `query_local_with_vector_storage`.

**Pros**: DRY, easier maintenance
**Cons**: More refactoring

### Recommendation: Option A with Logging

Fix the immediate bug with explicit chunk retrieval logic.
Add detailed logging to trace chunk collection.
Follow up with refactoring in a separate OODA loop.

## Files to Modify

| File                                                  | Changes                                                                   |
| ----------------------------------------------------- | ------------------------------------------------------------------------- |
| `edgequake/crates/edgequake-query/src/sota_engine.rs` | Fix `query_local_with_vector_storage`, `query_global_with_vector_storage` |

## Test Strategy

1. **Unit Test**: Mock vector storage with chunks, verify retrieval
2. **Integration Test**: E2E query with workspace providers, verify chunks returned
3. **Regression Test**: Ensure existing tests still pass

## Next Steps (Decide Phase)

1. Implement fix for `query_local_with_vector_storage`
2. Implement fix for `query_global_with_vector_storage`
3. Add comprehensive tests
4. Verify user scenario works
