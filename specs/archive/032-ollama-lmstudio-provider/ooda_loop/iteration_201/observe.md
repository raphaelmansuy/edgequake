# OODA 201 - Observe: Rebuild Operations Provider Switching

## Observation

Analyzed how `rebuild_embeddings` and `rebuild_knowledge_graph` handlers work with provider switching.

## Key Findings

### Flow Analysis

1. **Rebuild handlers update workspace config BEFORE queueing**:

   - `rebuild_embeddings`: Updates `embedding_model`, `embedding_provider`, `embedding_dimension`
   - `rebuild_knowledge_graph`: Updates `llm_model`, `llm_provider`

2. **Documents are queued with workspace_id**:

   - `TextInsertData` contains `workspace_id`
   - Task metadata includes `is_reprocess: true` and `is_embedding_rebuild: true` / `is_kg_rebuild: true`

3. **Processor reads config at processing time**:
   - `get_workspace_pipeline()` fetches workspace from service
   - Creates providers using `ProviderFactory::create_safe_llm_provider()`
   - Providers use the **current** workspace config (which was updated)

### Critical Path

```
rebuild_embeddings/rebuild_knowledge_graph
    ↓
update_workspace() - updates provider config
    ↓
queue TextInsertData tasks with workspace_id
    ↓
processor.process_text_insert()
    ↓
get_workspace_pipeline(workspace_id)  ← reads UPDATED config
    ↓
ProviderFactory::create_safe_llm_provider() ← uses NEW provider
    ↓
process with new provider ✓
```

### Evidence in Code

1. [workspaces.rs#L933-L950](../../../edgequake/crates/edgequake-api/src/handlers/workspaces.rs#L933-L950) - Config update in rebuild_embeddings
2. [workspaces.rs#L1273-L1294](../../../edgequake/crates/edgequake-api/src/handlers/workspaces.rs#L1273-L1294) - Config update in rebuild_knowledge_graph
3. [processor.rs#L139-L280](../../../edgequake/crates/edgequake-api/src/processor.rs#L139-L280) - Pipeline creation with workspace config

## Hypothesis

Provider switching during rebuild operations **should work correctly** because:

- Config is updated BEFORE documents are queued
- Processor reads config at processing time, not at queue time

## Next Step

Create E2E test to verify the complete flow.
