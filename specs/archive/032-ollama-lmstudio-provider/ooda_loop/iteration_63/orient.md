# OODA Loop Iteration 63 - Orient

**Date**: 2026-01-14
**Focus**: Ollama Stop Token Handling & KG Rebuild Verification

## Analysis

### 1. Stop Token Architecture Gap

The current architecture has a fundamental gap:

```
LLMProvider trait:
├── complete(prompt)           → Returns LLMResponse
├── complete_with_options()    → Accepts CompletionOptions (has stop tokens!)
├── chat()                     → Accepts CompletionOptions (has stop tokens!)
└── stream(prompt)             → NO OPTIONS! Stop tokens ignored!
```

**Root Cause**: The `stream()` method was designed with a simple interface that doesn't accept `CompletionOptions`. This means stop sequences configured for extraction prompts are silently ignored during streaming.

### 2. Impact Analysis

| Feature                 | Stop Tokens Impact                      |
| ----------------------- | --------------------------------------- |
| Entity Extraction       | ✅ Uses `complete_with_options` - WORKS |
| Relationship Extraction | ✅ Uses `chat` - WORKS                  |
| Query (non-streaming)   | ✅ Uses `chat` - WORKS                  |
| Query (streaming)       | ❌ Uses `stream` - **BROKEN**           |
| KG Rebuild              | ✅ Uses extraction - WORKS              |

**Critical**: Streaming responses may not respect stop tokens like `"\n\n---"` used to prevent runaway generation.

### 3. Provider Implementation Audit

| Provider | `stream()` accepts options | Stop token support |
| -------- | -------------------------- | ------------------ |
| OpenAI   | ❌ No                      | ❌ Broken          |
| Ollama   | ❌ No                      | ❌ Broken          |
| LMStudio | ❌ No                      | ❌ Broken          |
| Gemini   | ❌ No                      | ❌ Broken          |
| Mock     | N/A                        | N/A                |

### 4. Recommended Solution

Add new trait method with backward compatibility:

```rust
// New method with options
async fn stream_with_options(
    &self,
    prompt: &str,
    options: &CompletionOptions
) -> Result<BoxStream<'static, Result<String>>> {
    // Default: delegates to stream() for backward compatibility
    self.stream(prompt).await
}
```

Then update each provider to properly pass stop tokens.

### 5. KG Rebuild Flow Analysis

Current flow requires two API calls:

1. `POST /workspaces/{id}/rebuild-knowledge-graph` - Clears data
2. `POST /workspaces/{id}/reprocess-documents` - Queues docs

**Recommendation**: Combine into single operation OR ensure UI handles both calls automatically.

## Priority Matrix

| Task                                 | Priority | Effort | Impact |
| ------------------------------------ | -------- | ------ | ------ |
| Add `stream_with_options()` to trait | HIGH     | Medium | High   |
| Implement in OllamaProvider          | HIGH     | Low    | High   |
| Implement in OpenAIProvider          | HIGH     | Low    | High   |
| Wire through to chat handlers        | HIGH     | Medium | High   |
| Fix KG rebuild UX                    | MEDIUM   | Low    | Medium |
| Add E2E tests                        | MEDIUM   | Medium | High   |

## Next Steps (Decide)

1. Add `stream_with_options()` method to `LLMProvider` trait
2. Implement in OllamaProvider with stop token support
3. Implement in OpenAIProvider
4. Update sota_engine to use new method
5. Test streaming stop tokens work correctly
