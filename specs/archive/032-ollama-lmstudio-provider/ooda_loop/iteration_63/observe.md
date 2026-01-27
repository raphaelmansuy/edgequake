# OODA Loop Iteration 63 - Observe

**Date**: 2026-01-14
**Focus**: Ollama Stop Token Handling & KG Rebuild Verification

## Mission Re-read

Per spec requirement, re-reading key focus areas:

- **Ensure build KG knowledge graph, reprocess works very well**
- **Ensure Ollama stop token is correctly handled**
- **At least 50 OODA loops** (currently at iteration 63)

## Current State Observations

### 1. Codebase Compilation

```bash
cargo check --all-features
# Result: Finished `dev` profile in 10.52s (SUCCESS)
```

### 2. Ollama Provider Stop Token Support

**File**: [ollama.rs](../../edgequake/crates/edgequake-llm/src/providers/ollama.rs)

Current implementation:

- `ChatOptions.stop` field exists (line 202)
- Used in `chat()` method via `CompletionOptions.stop`
- **ISSUE FOUND**: `stream()` method does NOT pass stop tokens

```rust
// stream() method - line 358
async fn stream(&self, prompt: &str) -> Result<BoxStream<'static, Result<String>>> {
    let chat_options = ChatOptions {
        temperature: None,
        num_predict: None,
        stop: None,  // ❌ Always None - stop tokens ignored in streaming
    };
```

### 3. KG Rebuild Flow Analysis

**Endpoints**:

1. `/workspaces/{id}/rebuild-knowledge-graph` - Clears graph, optionally clears vectors
2. `/workspaces/{id}/reprocess-documents` - Queues documents for reprocessing

**Current Flow**:

```
User clicks "Rebuild KG"
  → API clears graph storage (nodes/edges)
  → Optionally clears vectors
  → Returns immediately with track_id
  → User must SEPARATELY call /reprocess-documents
```

**ISSUE FOUND**: Two-step process is confusing. Should be single operation.

### 4. Streaming Stop Token Handling

The OpenAI provider handles stop tokens in streaming:

```rust
// openai.rs - properly passes stop tokens
let mut body = json!({
    "model": &self.model,
    "messages": messages,
    "stream": true,
});
if let Some(stop) = &options.stop {
    body["stop"] = json!(stop);
}
```

Ollama should do the same.

### 5. WebUI State

Need to verify:

- [ ] Query page model selector works
- [ ] Workspace settings page accessible
- [ ] Rebuild KG button triggers correct flow
- [ ] Pipeline status shows progress

## Critical Issues Identified

| Issue                                 | Severity | Component     |
| ------------------------------------- | -------- | ------------- |
| Ollama stream() ignores stop tokens   | HIGH     | edgequake-llm |
| KG rebuild is two-step (confusing UX) | MEDIUM   | edgequake-api |
| Need streaming stop token support     | HIGH     | edgequake-llm |

## Next Steps (Orient)

1. Fix Ollama stop token handling in streaming
2. Add `stream_with_options()` method to LLMProvider trait
3. Unify KG rebuild + reprocess into single operation
4. Add E2E tests for stop token behavior
