# OODA Iteration 16 - Orient Phase

## Analysis: Root Cause of Workspace Provider Fallback

### The Bug: `strict_workspace_mode` Not Enforced for Pipelines

**Location**: `processor.rs` lines 230-376

**Finding**: The `strict_workspace_mode` flag is checked for vector storage (`get_workspace_vector_storage_strict`) but NOT for pipeline creation (`get_workspace_pipeline`).

When OpenAI provider creation fails (e.g., `OPENAI_API_KEY` not in environment during task processing), the code:

1. Logs `CRITICAL` errors (good for debugging)
2. **Falls back to default pipeline anyway** (bad for production)

This causes documents to be processed with Ollama's 768-dim embeddings even when the workspace is configured for OpenAI's 1536-dim embeddings.

### Code Analysis

```rust
// processor.rs:230 - get_workspace_pipeline does NOT check strict_workspace_mode
async fn get_workspace_pipeline(&self, workspace_id: Option<&str>) -> Arc<Pipeline> {
    // ... provider creation logic ...

    // Falls back on ANY error - ignores strict_workspace_mode
    warn!("Falling back to default pipeline due to provider creation failure");
    Arc::clone(&self.pipeline)  // ← Always returns default
}

// processor.rs:390 - get_workspace_vector_storage_strict DOES check strict_workspace_mode
async fn get_workspace_vector_storage_strict(&self, workspace_id: &str) -> Result<...> {
    let allow_fallback = !self.strict_workspace_mode;  // ← Respects flag
    // ...
}
```

### Why This Matters

| Scenario            | Current Behavior               | Expected (Strict)          |
| ------------------- | ------------------------------ | -------------------------- |
| OpenAI key missing  | Process with Ollama (768 dims) | FAIL task with clear error |
| Ollama not running  | Process with Ollama (fails)    | FAIL task with clear error |
| Workspace not found | Process with default           | FAIL task with clear error |

### Environment Variable Propagation

When the server starts, `OPENAI_API_KEY` is read. But the processor runs in a task queue context where environment variables may not be re-read.

The `ProviderFactory::create_safe_llm_provider` function reads `OPENAI_API_KEY` at the time of provider creation:

```rust
// ProviderFactory likely does:
let api_key = std::env::var("OPENAI_API_KEY")?;  // May fail if not set
```

### Proposed Fix Categories

1. **Strict Pipeline Mode**: Add `get_workspace_pipeline_strict` that returns `Result<Pipeline, Error>` and fails on provider creation errors
2. **Environment Passthrough**: Store API keys in workspace table (encrypted) so they're available at task time
3. **Provider Caching**: Create providers once during server startup and reuse them

### Risk Assessment

| Fix                     | Complexity | Risk   | Benefit                              |
| ----------------------- | ---------- | ------ | ------------------------------------ |
| Strict Pipeline Mode    | Low        | Low    | Immediate - prevents silent fallback |
| Environment Passthrough | Medium     | Medium | Enables per-workspace API keys       |
| Provider Caching        | High       | Medium | Performance + reliability            |

## Recommendation

**Immediate**: Implement strict pipeline mode (OODA-16-FIX-001)
**Next OODA**: Investigate provider caching if tests pass

## Affected Code Paths

1. `processor.rs:get_workspace_pipeline` - needs strict variant
2. `processor.rs:process_document_task` - needs to use strict variant
3. `main.rs:286` - already uses strict constructor (good)
