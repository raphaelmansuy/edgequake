# OODA Loop 55 - Orient

**Date:** 2026-01-14  
**Focus:** Multi-model support per provider (Focus 7) + Streaming fallback (Focus 8)

---

## 🧭 Analysis

### Architecture Understanding

```
┌──────────────────────────────────────────────────────────────────┐
│                      Model Selection Flow                         │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│  models.toml ──► ModelConfigLoader ──► ModelsConfig               │
│                         │                    │                    │
│                         ▼                    ▼                    │
│              ProviderFactory         WebUI Model Selector         │
│                    │                         │                    │
│         ┌─────────┼─────────┐               │                    │
│         ▼         ▼         ▼               │                    │
│      OpenAI   Ollama   LMStudio ◄───────────┘                    │
│         │         │         │                                     │
│         └─────────┴─────────┘                                     │
│                   │                                               │
│                   ▼                                               │
│           Query/Ingestion                                         │
│                                                                   │
└──────────────────────────────────────────────────────────────────┘
```

### Key Insight: Streaming Fallback Strategy

For LM Studio streaming fallback, we need to:

1. **Try streaming first** - Attempt SSE-based streaming
2. **Detect failure early** - Check response headers/first chunk
3. **Fallback gracefully** - Use non-streaming if streaming fails
4. **Inform caller** - Return indication of fallback

### Configuration Locations

| Component            | Location                                                   |
| -------------------- | ---------------------------------------------------------- |
| Model cards TOML     | `edgequake/models.toml`                                    |
| Model config parser  | `edgequake-llm/src/model_config.rs`                        |
| Provider factory     | `edgequake-llm/src/factory.rs`                             |
| LM Studio provider   | `edgequake-llm/src/providers/lmstudio.rs`                  |
| Query handler        | `edgequake-api/src/handlers/chat.rs`                       |
| WebUI model selector | `edgequake_webui/src/components/models/model-selector.tsx` |

---

## 🎯 Strategic Decisions

### Decision 1: Add OpenAI Future Models

Add gpt-5o-nano and gpt-5o-mini as placeholders with appropriate tags.

### Decision 2: Implement Try-Streaming Pattern

```rust
// Pseudocode for streaming fallback
async fn stream_with_fallback(prompt: &str) -> Result<StreamOrResponse> {
    match self.try_stream(prompt).await {
        Ok(stream) => StreamOrResponse::Stream(stream),
        Err(streaming_error) if is_not_supported(&streaming_error) => {
            // Fallback to non-streaming
            let response = self.complete(prompt).await?;
            StreamOrResponse::SingleResponse(response)
        }
        Err(e) => Err(e),
    }
}
```

### Decision 3: Runtime Capability Detection

Add endpoint check for LM Studio `/v1/models` to detect available models dynamically.

---

## Constraints

1. **Backward Compatibility**: Must not break existing API clients
2. **Performance**: Fallback should be efficient, not doubling latency
3. **Observability**: Log when fallback occurs for debugging
