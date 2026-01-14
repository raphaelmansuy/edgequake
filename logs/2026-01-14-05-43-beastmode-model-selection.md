# Task Log: Full Model Selection Support (SPEC-032)

**Date:** 2026-01-14 05:43 UTC
**Mode:** Beastmode

## Actions

1. Added `model` field to `QuerySettings` type in [types/index.ts](../edgequake_webui/src/types/index.ts)
2. Updated default query settings in [use-settings-store.ts](../edgequake_webui/src/stores/use-settings-store.ts)
3. Rewrote `ProviderModelSelector` component to show ALL models grouped by provider
4. Updated `query-interface.tsx` to parse and pass both provider and model
5. Added `model` field to backend `ChatCompletionRequest` in [chat_types.rs](../edgequake/crates/edgequake-api/src/handlers/chat_types.rs)
6. Updated non-streaming and streaming handlers in [chat.rs](../edgequake/crates/edgequake-api/src/handlers/chat.rs)

## Decisions

- Used `provider/model` format (e.g., "ollama/gemma3:12b") for full model ID
- Backward compatible: legacy format still works if model is embedded in provider field
- Server default option shows when user hasn't selected a model

## Next Steps

- E2E test with actual model selection in query interface
- Verify model preference persists across sessions (localStorage)

## Lessons/Insights

- The `LLMModelSelector` component (workspace creation) already had the correct pattern
- Frontend uses `useLlmModels()` hook to get all 28 models from `/api/v1/models/llm`
- Backend already supported provider/model parsing, just needed separate `model` field
