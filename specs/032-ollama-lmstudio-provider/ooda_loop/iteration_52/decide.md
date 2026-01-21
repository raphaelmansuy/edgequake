# OODA Loop 52: Decide - Implementation Plan

**Date:** 2026-01-14

## Decisions

### 1. Add Missing Ollama Models

Add to `models.toml` under Ollama provider:

**LLM Models:**

- `gpt-oss:20b` - Open-source GPT-style model
- `mistral-nemo:latest` - Mistral AI's Nemo model (12B params)

**Embedding Models:**
Already covered - embeddinggemma:latest exists

### 2. Add Missing LMStudio Models

Add to `models.toml` under LMStudio provider:

**LLM Models:**

- `lfm2.5-1.2b-instruct-mlx` - LFM 2.5 1.2B instruction model
- `granite-4.0-h-tiny-dwq` - IBM Granite 4.0 tiny
- `zai-org/glm-4.6v-flash` - GLM 4.6 vision flash
- `mlx-community/GLM-4.7-REAP-50-mxfp4` - GLM 4.7 REAP quantized

### 3. Implement LMStudio Streaming

Add to `lmstudio.rs`:

- `async fn stream()` - SSE streaming implementation
- `fn supports_streaming()` - Returns true

### 4. Add Streaming Fallback

In chat handlers:

- Check `supports_streaming()` before streaming
- Fall back to non-streaming if not supported
- Log when fallback occurs

## Files to Modify

1. `/edgequake/models.toml` - Add models
2. `/edgequake/crates/edgequake-llm/src/providers/lmstudio.rs` - Add streaming
3. `/edgequake/crates/edgequake-api/src/handlers/chat.rs` - Add fallback logic

## Success Criteria

- [ ] All specified models appear in models.toml
- [ ] LMStudio stream() method implemented
- [ ] LMStudio supports_streaming() returns true
- [ ] Streaming fallback works when provider doesn't support it
- [ ] All tests pass
