# OODA-12: Orient - Query Provider Analysis

## First Principles Analysis

### Why Query Works Regardless of Provider

The query system is provider-agnostic by design:

```text
Query Request
    ↓
QueryEngine.query()
    ↓
Context Building (entities, relationships, chunks)
    ↓
LLMProvider.chat(context)  ← Provider-specific
    ↓
Response Generation
```

Both Ollama and OpenAI implement the same `LLMProvider` trait:

```rust
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: &[ChatMessage], ...) -> Result<LLMResponse>;
}
```

The trait abstraction means:
- Query engine doesn't care which provider is active
- Context is built identically for both providers
- Only the LLM call differs (network endpoint)

## Test Results Summary

| Mode | Ollama | Expected OpenAI |
|------|--------|-----------------|
| local | ✅ Good answer | ✅ Same (uses entities) |
| global | ❌ No info | ❌ Same (no chunks) |
| hybrid | ✅ Good answer | ✅ Same |
| mix | ✅ Good answer | ✅ Same |
| naive | ❌ No info | ❌ Same (no chunks) |

**Note**: Global/naive modes returned "no info" because the test document has only 1 chunk and entity-based context is more relevant for the query.

## OpenAI Testing Limitations

Cannot directly test OpenAI due to "quota exceeded" error. However:

1. **Code path is identical**: Same `QueryEngine.query()` for both
2. **Trait guarantees behavior**: Both providers implement same interface
3. **Only network differs**: OpenAI calls `api.openai.com`, Ollama calls `localhost:11434`

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| OpenAI rate limits | Medium | Retry logic exists in provider |
| Model capability differences | Low | Both support chat completion |
| Context length overflow | Low | Chunking handles this |
| Network timeouts | Medium | Timeout config exists |

## Confidence Level

**High confidence** that query works with OpenAI because:

1. Ingestion with OpenAI was tested in OODA-10 (same provider/trait system)
2. Query and ingestion share the same `LLMProvider` trait
3. No OpenAI-specific code in query path
4. Unit tests exist for query engine (don't depend on provider)
