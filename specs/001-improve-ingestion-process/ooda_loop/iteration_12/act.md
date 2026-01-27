# Act - Iteration 12: Rebuild KG Flow Verification

## Verification Complete ✅

### Backend Flow Analysis

#### 1. rebuild_knowledge_graph API (workspaces.rs)
Location: Line 1729
- Clears graph storage (nodes, edges) via `graph_storage.clear_workspace()`
- Clears vector storage if `rebuild_embeddings=true` via `vector_storage.clear_workspace()`
- Evicts cached workspace vector storage (for dimension changes)
- Updates workspace LLM config if changed
- Returns info about documents to process

#### 2. reprocess_all_documents API (workspaces.rs)
Location: Line 2048
- Finds all documents for workspace
- Creates `Task::new(TaskType::Insert, ...)` for each document
- Sets `is_reprocess: true` in metadata
- Queues tasks for async processing

#### 3. process_text_insert (processor.rs)
Location: Line 553
Full pipeline execution:
- `chunking` → Split document into chunks
- `extracting` → LLM entity extraction via `pipeline.process()`
- `embedding` → Generate vector embeddings
- `indexing` → Store in graph and vector databases
- `completed` → Final status

### Frontend Flow Analysis

#### RebuildKnowledgeGraphButton (workspace/)
1. Shows confirmation dialog with impact preview
2. Calls `rebuildKnowledgeGraph()` API
3. On success, automatically calls `reprocessMutation.mutate()`
4. Opens `PipelineStatusDialog` to show progress

### Verification Result
✅ **Rebuild KG correctly re-extracts entities AND rebuilds embeddings**

The flow is:
```
User clicks "Rebuild KG"
  → API clears graph (nodes, edges)
  → API clears vectors (if rebuild_embeddings=true)
  → API queues all documents
  → Processor: chunking → extracting → embedding → indexing
  → Graph rebuilt with new entities
  → Vectors rebuilt with new embeddings
```

## No Changes Needed
The implementation is correct. Backend properly:
1. Clears existing data
2. Re-runs full extraction pipeline
3. Regenerates embeddings
4. Stores in databases

## Files Reviewed
- `edgequake-api/src/handlers/workspaces.rs` (rebuild_knowledge_graph, reprocess_all_documents)
- `edgequake-api/src/processor.rs` (process_text_insert)
- `edgequake_webui/src/components/workspace/rebuild-knowledge-graph-button.tsx`

## Next Steps
- Continue with Iteration 13
- Focus on testing rebuild operations
