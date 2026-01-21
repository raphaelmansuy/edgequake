# OODA 222: OBSERVE - Document Processing with Workspace Pipeline

## Objective

Test that document upload and processing uses workspace-specific providers correctly. This is a critical user scenario:

1. User creates workspace with Ollama provider
2. User uploads document to that workspace
3. Document is processed with Ollama's entity extraction
4. User switches workspace to OpenAI
5. User uploads another document
6. New document is processed with OpenAI

## Current Flow (from handlers/documents.rs)

```rust
// Line 323-327: Get workspace-specific pipeline
let workspace_pipeline = state
    .create_workspace_pipeline(&workspace_id_for_storage)
    .await;
let result = workspace_pipeline
    .process(&document_id, &request.content)
    .await?;
```

## Key Integration Points

1. **create_workspace_pipeline()** - Reads workspace config and creates pipeline
2. **pipeline.process()** - Uses LLMExtractor with workspace's LLM provider
3. **workspace_pipeline.embedding_provider** - Creates embeddings with workspace config

## What We Need to Test

### Scenario 1: Document with Ollama Pipeline

- Create workspace with Ollama config
- Upload document synchronously
- Verify processing completes (chunks, entities)

### Scenario 2: Document with Mock Pipeline

- Create workspace with mock provider (always works)
- Upload document
- Verify expected entities/relationships from mock

### Scenario 3: Provider Switch Between Documents

- Create workspace with provider A
- Upload doc 1
- Switch to provider B
- Upload doc 2
- Verify doc 2 was processed with provider B

### Scenario 4: Async Processing Uses Workspace Pipeline

- Enable async processing
- Upload document to workspace
- Verify background task uses workspace pipeline

## Files to Create

`edgequake/crates/edgequake-api/tests/e2e_document_processing_pipeline.rs`

## Technical Constraints

- Ollama may not be running in CI → use mock provider
- OpenAI requires API key → skip or use mock
- Async tests need task completion polling
