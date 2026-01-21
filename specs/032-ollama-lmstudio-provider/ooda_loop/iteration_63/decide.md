# OODA Loop Iteration 63 - Decide

**Date**: 2026-01-14
**Focus**: Ollama Stop Token Handling & KG Rebuild Verification

## Decision

### 1. Add `stream_with_options()` to LLMProvider Trait

**Rationale**: Minimal trait change with backward compatibility.

**Implementation**:

```rust
/// Generate a streaming completion with options.
async fn stream_with_options(
    &self,
    prompt: &str,
    options: &CompletionOptions
) -> Result<BoxStream<'static, Result<String>>> {
    // Default implementation delegates to stream() for backward compatibility
    self.stream(prompt).await
}
```

### 2. Implement in Ollama Provider

**Changes to `ChatOptions`**:

- Already has `stop: Option<Vec<String>>` ✓

**Changes to `stream()` method**:

- Add new `stream_with_options()` that passes stop tokens

### 3. Implement in OpenAI Provider

The OpenAI SDK already supports stop tokens in streaming requests.
Need to pass them through.

### 4. Update Streaming Chat Flow

**File**: `chat.rs`
**Change**: When calling `query_stream_with_context_and_llm()`, pass options with stop tokens.

### 5. Test Plan

1. Start Ollama with gemma3:12b
2. Make streaming query with stop token test
3. Verify response stops at configured sequence

## Files to Modify

| File                                      | Change                                   |
| ----------------------------------------- | ---------------------------------------- |
| `edgequake-llm/src/traits.rs`             | Add `stream_with_options()` default impl |
| `edgequake-llm/src/providers/ollama.rs`   | Implement `stream_with_options()`        |
| `edgequake-llm/src/providers/openai.rs`   | Implement `stream_with_options()`        |
| `edgequake-llm/src/providers/lmstudio.rs` | Implement `stream_with_options()`        |
| `edgequake-llm/src/providers/gemini.rs`   | Implement `stream_with_options()`        |
| `edgequake-llm/src/providers/mock.rs`     | Add default impl                         |
| `edgequake-query/src/sota_engine.rs`      | Use `stream_with_options()`              |

## Risk Assessment

| Risk                        | Mitigation                              |
| --------------------------- | --------------------------------------- |
| Breaking existing providers | Default impl delegates to `stream()`    |
| Performance impact          | None - same code path if no stop tokens |
| Missing implementations     | Compile-time errors will catch          |

## Go/No-Go: **GO**
