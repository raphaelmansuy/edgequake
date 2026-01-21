# OODA Loop Iteration 62 - Observe

## Date: January 2025

## Mission Requirements Analyzed

From the spec file `032-ollama-lmstudio-provider.md`, new requirements 22-28 were added:

1. **REQ-22**: Display model name after tokens/second in query responses
2. **REQ-23**: Rebuild dialog should have Close button that doesn't stop rebuild
3. **REQ-24**: Fix rebuild embeddings - documents/chunks not processing
4. **REQ-25**: CRITICAL - Chunk size vs embedding model compatibility validation
5. **REQ-26**: Add stop document extraction capability
6. **REQ-27**: Audit scroll areas on all screens
7. **REQ-28**: OpenAI key propagation in `make dev`

## Current State Observations

### Query Page (REQ-22)

- Located: `edgequake_webui/src/components/query/chat-message.tsx`
- Tokens per second already displayed with Gauge icon
- `llmProvider` and `llmModel` available in message props
- Format needed: `58.5/s • ollama/gemma3:12b`

### Pipeline Status Dialog (REQ-23)

- Located: `edgequake_webui/src/components/documents/pipeline-status-dialog.tsx`
- Currently only has "Cancel Pipeline" button
- Dialog's X button closes without cancelling (correct behavior)
- Need explicit "Close" button for clarity

### Rebuild Embeddings (REQ-24)

- Backend: `edgequake/crates/edgequake-api/src/handlers/workspaces.rs`
- Two-step process:
  1. `rebuild_embeddings` - clears vectors
  2. `reprocess_all_documents` - queues documents
- Need logging to debug why documents aren't being found

### Chunk/Embedding Compatibility (REQ-25)

- Chunker default: 1200 tokens (from `edgequake-pipeline/src/chunker.rs`)
- Embedding models have `context_length` in models.toml:
  - OpenAI text-embedding-3-small: 8191 tokens ✅
  - Ollama embeddinggemma: 2048 tokens (marginal)
  - Ollama mxbai-embed-large: 512 tokens ❌ (too small!)
- Need validation at rebuild time

### Makefile (REQ-28)

- `OPENAI_API_KEY` captured at line 40
- BUT `dev`, `backend-dev`, `backend-db` explicitly set `OPENAI_API_KEY=""`
- Only `backend-bg` correctly forwards the key

## Files to Modify

| File                            | Change                                       |
| ------------------------------- | -------------------------------------------- |
| `chat-message.tsx`              | Add model name after tokens/second           |
| `pipeline-status-dialog.tsx`    | Add Close button                             |
| `workspaces.rs`                 | Add debug logging, chunk compatibility check |
| `workspaces_types.rs`           | Add compatibility_warning to response        |
| `Makefile`                      | Forward OPENAI_API_KEY in dev targets        |
| `edgequake.ts`                  | Update RebuildEmbeddingsResponse type        |
| `rebuild-embeddings-button.tsx` | Show compatibility warning                   |
