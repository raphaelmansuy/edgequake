# OODA Loop 52: Orient - Multi-Model Strategy

**Date:** 2026-01-14

## Analysis

### Why Multiple Models Per Provider Matters

1. **User Choice**: Different use cases need different models

   - Code: deepseek-coder, codellama
   - Vision: gemma3, llama3.2-vision
   - Speed: smaller models (4b, 7b)
   - Quality: larger models (70b, 120b)

2. **Cost Optimization**: Users can pick cost-effective models
3. **Capability Matching**: Vision, function calling, JSON mode vary by model

### Model Discovery Strategy

#### Ollama

- Models are pulled locally with `ollama pull`
- The config should list commonly used models
- `ollama list` shows locally available models
- We should add models that user mentioned: gpt-oss:20b, mistral-nemo:latest

#### LMStudio

- Models are downloaded via LMStudio GUI
- Model names follow HuggingFace format: org/model-name
- We should add the models user mentioned

### Streaming Support Analysis

LMStudio uses OpenAI-compatible API:

- `/v1/chat/completions` with `stream: true`
- SSE (Server-Sent Events) format
- Should work like OpenAI streaming

Current LMStudio implementation:

- ❌ No `stream()` method
- ❌ No `supports_streaming()` method
- Need to implement both

## Priority Order

1. Add missing models to models.toml
2. Implement LMStudio streaming
3. Add streaming fallback logic
4. Test E2E
