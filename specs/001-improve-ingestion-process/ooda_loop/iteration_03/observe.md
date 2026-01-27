# Iteration 03: Observe

**Mission Re-read**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-improve-ingestion-process.md`

---

## Territory Mapping: Rebuild Operations

### 1. Current Rebuild Endpoints

From routes.rs and workspaces.rs analysis:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    REBUILD OPERATIONS INVENTORY                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  POST /api/v1/workspaces/{id}/rebuild-embeddings                         │
│       └── Clears vectors, queues docs for re-embedding                   │
│       └── Updates workspace embedding config                             │
│       └── Workspace-isolated ✅                                          │
│                                                                          │
│  POST /api/v1/workspaces/{id}/rebuild-knowledge-graph                    │
│       └── Clears graph nodes/edges                                       │
│       └── Optionally clears vectors (rebuild_embeddings flag)            │
│       └── Queues all docs for full reprocessing                          │
│       └── Workspace-isolated ✅                                          │
│                                                                          │
│  POST /api/v1/workspaces/{id}/reprocess-documents                        │
│       └── Reprocesses all docs without clearing (SPEC-032)               │
│                                                                          │
│  POST /api/v1/documents/reprocess                                        │
│       └── Reprocesses failed documents only                              │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2. Rebuild Embeddings Handler (workspaces.rs:1343)

**Key Features**:
- Workspace isolation via `clear_workspace(&workspace_id)`
- Auto-detects dimension from model config
- Cache eviction on dimension change (OODA-225)
- Queues documents for re-embedding with track_id
- Returns documents_to_process, chunks_to_process, track_id

**Edge Cases Handled**:
- Dimension mismatch: Auto-updates from model config
- Config unchanged: Requires `force: true`
- Chunk size vs context length validation (REQ-25)

### 3. Rebuild Knowledge Graph Handler (workspaces.rs:1729)

**Key Features**:
- Clears graph via `clear_workspace(&workspace_id)`
- Optional embedding rebuild (`rebuild_embeddings` flag)
- Updates workspace LLM config
- Queues all docs for full reprocessing
- Returns nodes_cleared, edges_cleared, documents_queued

### 4. Frontend UI Components

| Component | Location | Purpose |
|-----------|----------|---------|
| RebuildEmbeddingsButton | workspace/rebuild-embeddings-button.tsx | Card/button to trigger embedding rebuild |
| RebuildKnowledgeGraphButton | workspace/rebuild-knowledge-graph-button.tsx | Card/button to trigger KG rebuild |
| PipelineStatusDialog | documents/pipeline-status-dialog.tsx | Progress tracking |

**UI Flow**:
1. User clicks Rebuild button
2. Confirmation dialog with warning
3. API call to clear + queue
4. PipelineStatusDialog opens with track_id
5. Progress polling until complete

### 5. Workspace Isolation Verification

```rust
// Vector isolation (workspaces.rs:1450)
let vectors_cleared = state
    .vector_storage
    .clear_workspace(&workspace_id)  // ← Scoped to workspace
    .await
    
// Graph isolation (workspaces.rs:1790)
let (nodes_cleared, edges_cleared) = state
    .graph_storage
    .clear_workspace(&workspace_id)  // ← Scoped to workspace
    .await

// Cache eviction (OODA-225)
state.vector_registry.evict(&workspace_id).await;
```

### 6. Status: What Works vs What Needs Improvement

| Feature | Status | Notes |
|---------|--------|-------|
| Rebuild Embeddings API | ✅ | Working with workspace isolation |
| Rebuild KG API | ✅ | Working with workspace isolation |
| Dimension change handling | ✅ | Auto-detects from model config |
| Cache eviction | ✅ | OODA-225 implemented |
| Frontend buttons | ✅ | RebuildEmbeddingsButton, RebuildKnowledgeGraphButton |
| Progress dialog | ✅ | PipelineStatusDialog with polling |
| Processing sub-states | ✅ | Added in iteration 02 |
| E2E tests | ⚠️ | Need Ollama-specific tests |
| Error UX | ⚠️ | Could show more detail |

---

## Areas for Improvement

1. **E2E Tests with Ollama** - Create tests using gemma3/nomic-embed-text
2. **Progress UX** - Show stage (extracting/embedding) during rebuild
3. **Error Details** - Show clearer error context during rebuild failures
4. **Confirmation Dialog** - Add impact preview (X documents, Y chunks)

---

## Next Step

Proceed to **Orient** phase to prioritize improvements.
