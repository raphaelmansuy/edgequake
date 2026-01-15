# OODA 183: Observe - Document Ingestion Provider Flow Analysis

**Date**: 2025-01-15
**Focus**: Trace complete document ingestion flow to verify workspace provider switching

## Code Investigation

### Flow Paths Identified

#### Path 1: Async Document Processing

```
documents.rs:upload_document()
  └─> Creates TextInsertData with workspace_id
      └─> Task queued to task_queue
          └─> DocumentTaskProcessor.process()
              └─> process_text_insert()
                  └─> get_workspace_pipeline(workspace_id)
                      └─> ProviderFactory::create_llm_provider()
```

**File**: [processor.rs](../../edgequake/crates/edgequake-api/src/processor.rs#L170)

#### Path 2: Sync Document Processing

```
documents.rs:upload_document()
  └─> state.create_workspace_pipeline()
      └─> ProviderFactory::create_safe_llm_provider()
```

**File**: [state.rs](../../edgequake/crates/edgequake-api/src/state.rs#L955)

### Key Files Analyzed

| File         | Line     | Function                    |
| ------------ | -------- | --------------------------- |
| processor.rs | 170-192  | get_workspace_pipeline()    |
| state.rs     | 933-1005 | create_workspace_pipeline() |
| factory.rs   | 352-397  | create_llm_provider()       |
| documents.rs | 263-290  | async task creation         |
| documents.rs | 319-326  | sync processing             |

## Observations

1. **workspace_id is correctly passed** through both paths
2. **Different factory methods used**: processor.rs uses `create_llm_provider`, state.rs uses `create_safe_llm_provider`
3. **Silent fallback on error**: If provider creation fails, falls back to default pipeline
4. **No lineage tracking**: Extraction results don't record which provider was actually used

## Questions Identified

1. Does `create_llm_provider("openai", "model")` return error when OPENAI_API_KEY missing?
2. What happens when fallback occurs - is it logged prominently?
3. Is there any way to verify which provider was actually used?

## Next Step

OODA 184: Deeper analysis of provider factory behavior and error handling.
