# OODA Iteration 123: Decide

## Date: 2026-01-14

## Decision 1: Verify Current Implementation is Complete

**Assessment**: The rebuild_embeddings flow is complete:

| Component | Status | Evidence |
|-----------|--------|----------|
| API Handler | ✅ Complete | workspaces.rs#L814-L1140 |
| WebUI Button | ✅ Complete | rebuild-embeddings-button.tsx |
| Compatibility Warning | ✅ Complete | toast.warning on line 138 |
| Type Definitions | ✅ Complete | RebuildEmbeddingsResponse |
| Progress Dialog | ✅ Complete | PipelineStatusDialog integration |

## Decision 2: No Code Changes Needed for OODA 123

The chunk-embedding compatibility check is already implemented:
1. ✅ Backend validates chunk_size vs model context_length
2. ✅ Warning included in API response
3. ✅ Frontend displays warning via toast

## Decision 3: Document the Invariant

Add clear documentation about the chunk-embedding compatibility invariant.

## Action Plan

```
Step 1: Verify implementation via code review (DONE)
        - Handler queues documents ✅
        - Warning is displayed ✅
        - Progress tracking works ✅

Step 2: Document the invariant in code comments
        - Add comments explaining the critical invariant
        - Reference models with small context lengths

Step 3: Consider future enhancement
        - Re-chunking when model changes
        - Configurable chunk size per workspace
```

## Future Enhancement (Not This Iteration)

For models with very small context lengths (e.g., mxbai-embed-large: 512 tokens):
1. Add `chunk_size` to workspace settings
2. Provide "Re-chunk Documents" option
3. Adjust chunk size automatically based on selected model

## Files to Update

| File | Action |
|------|--------|
| Documentation | Add note about chunk-model compatibility |
| No code changes needed | Implementation is complete |
