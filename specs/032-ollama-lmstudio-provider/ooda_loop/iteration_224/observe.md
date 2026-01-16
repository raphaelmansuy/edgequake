# OODA-230 Iteration 224: Observe

**Date**: January 16, 2026
**Issue**: Embedding Retrieval Broken in Hybrid Mode - No document chunks returned

## Problem Statement

User reports that in Hybrid mode:
- ✅ Knowledge Graph entities are retrieved (shows "20 Topics")
- ✅ LLM response is generated with KG context
- ❌ **ZERO document chunks/sources** from embedding retrieval ("0 Sources")

This breaks the fundamental RAG promise: combining vector similarity with graph-based retrieval.

## User Test Protocol

1. Created new tenant
2. Config: LLM = GPT-4.1-nano, Embedding = text-embedding-3-large (3072 dim)
3. Uploaded document → built KG (791 entities, 328 relationships)
4. Queried: "What are the concepts"
5. Follow-up: "Explain RoboFAC in depth"

**Result**: Got KG-based answer with Topics visible, but **0 Sources** shown.

## Screenshot Evidence Analysis

From attached screenshots:
- **Screenshot 1**: Shows "0 Sources · 20 Topics" with strength "Strong (50%)"
- **Screenshot 2**: "RoboFAC" explanation derived from KG context
- **Screenshot 3**: Expands panel showing "Documents" tab is EMPTY, "Knowledge" tab has topics
- **Screenshot 4**: Shows "Key Topics" (Fast-ThinkAct, arXiv, Huang et al., RoboFAC, etc.)

## Hypothesis Matrix

| # | Hypothesis | Test Method | Priority |
|---|------------|-------------|----------|
| 1 | Chunks not stored in vector DB | Check storage after ingestion | HIGH |
| 2 | Embedding dimension mismatch | Compare query vs stored | HIGH |
| 3 | Workspace vector storage not used | Trace query path | HIGH |
| 4 | Filter by type excludes chunks | Check `filter_by_type` logic | MEDIUM |
| 5 | `source_chunk_ids` not populated | Check entity → chunk links | MEDIUM |
| 6 | min_score filtering too aggressive | Check score thresholds | LOW |

## Code Path Analysis

### Query Handler Flow (query.rs)

```
POST /api/v1/query
    ↓
TenantContext extracts workspace_id from header
    ↓
get_workspace_embedding_provider() → creates OpenAI provider
get_workspace_vector_storage() → creates workspace vector storage
    ↓
query_with_workspace_config() → uses BOTH workspace-specific resources
    ↓
query_hybrid() → combines local + global
```

### Hybrid Mode Implementation (sota_engine.rs:2427)

```rust
async fn query_hybrid(...) {
    // Runs local + global in parallel
    let (local_result, global_result) = tokio::join!(
        self.query_local(...),
        self.query_global(...),
    );
    // Merges entities, relationships, AND chunks
}
```

### Local Mode Chunk Retrieval (sota_engine.rs:2055)

```rust
async fn query_local(...) {
    // Step 1: Vector search for ENTITY vectors
    let vector_results = self.vector_storage.query(&embeddings.low_level, ...);
    let entity_vectors = filter_by_type(vector_results, VectorType::Entity);
    
    // Step 7: Retrieve chunks via source_chunk_ids
    for entity in &context.entities {
        for chunk_id in &entity.source_chunk_ids {
            chunk_ids.insert(chunk_id.clone());
        }
    }
    // Query vector storage for specific chunk IDs
    let results = self.vector_storage.query(..., Some(&chunk_ids_vec));
}
```

## Critical Observation: Vector Storage Selection

Looking at `query_with_workspace_config` vs `query_with_embedding_provider`:

1. **query_with_workspace_config** (lines 880-950):
   - Uses workspace embedding ✓
   - Uses `query_*_with_vector_storage` methods → workspace vector storage ✓

2. **query_with_embedding_provider** (lines 701-850):
   - Uses workspace embedding ✓
   - Uses `self.query_*` methods → **DEFAULT vector storage** ✗

**POTENTIAL BUG**: If `get_workspace_vector_storage()` returns an error/None, the code falls 
back to `query_with_embedding_provider()` which uses the WRONG vector storage!

## Evidence from Logs

From document metadata in logs:
```json
{
  "id": "6c2c2ac6-e482-4a84-9dae-215417b677b8",
  "embedding_dimensions": 3072,
  "embedding_model": "text-embedding-3-large",
  "embedding_provider": "openai",
  "entity_count": 791,
  "relationship_count": 328,
  "chunk_count": 25,
  "workspace_id": "5b07e3cf-e6dd-4743-b245-f0e727a25082"
}
```

This confirms:
- Document was processed with 3072-dim OpenAI embeddings
- 25 chunks were created
- Proper workspace association

## Next Steps (Orient Phase)

1. Verify chunks are actually stored in workspace vector storage
2. Check if `filter_by_type(VectorType::Chunk)` is finding anything
3. Add logging to trace the chunk retrieval path
4. Test with explicit chunk-only query (Naive mode)
