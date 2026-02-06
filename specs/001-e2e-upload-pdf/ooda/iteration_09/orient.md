# OODA Iteration 09 - Orient

## Analysis

### Root Cause

The Makefile `dev-bg` target always sets `OLLAMA_HOST` and `OLLAMA_MODEL`, which triggers Ollama provider selection even when `OPENAI_API_KEY` is available.

**Factory Logic** (factory.rs:111-145):
1. `EDGEQUAKE_LLM_PROVIDER` → explicit selection (HIGHEST priority)
2. `OLLAMA_HOST` or `OLLAMA_MODEL` → selects Ollama
3. `LMSTUDIO_HOST` or `LMSTUDIO_MODEL` → selects LM Studio
4. `OPENAI_API_KEY` → selects OpenAI
5. Fallback to Mock

**Problem**: Step 2 always triggers because Makefile sets Ollama vars.

### Solution

Modify `dev-bg` target to dynamically select provider based on `OPENAI_API_KEY`:

```makefile
dev-bg:
    # If OPENAI_API_KEY is set, use OpenAI provider
    # Otherwise, use Ollama
    if [ -n "$(OPENAI_API_KEY)" ]; then
        EDGEQUAKE_LLM_PROVIDER=openai
    else
        OLLAMA_HOST=http://localhost:11434
        OLLAMA_MODEL=gemma3:latest
    fi
```

### Implementation Plan

1. **Modify `dev-bg` target** to conditionally set provider vars
2. **Verify** with `make dev-bg` and check health endpoint
3. **Test** re-indexing with OpenAI provider

### Provider Configuration Matrix

| Environment | Provider | Model | Embedding |
|-------------|----------|-------|-----------|
| `OPENAI_API_KEY` set | OpenAI | gpt-4o-mini | text-embedding-3-small (1536d) |
| `OPENAI_API_KEY` unset | Ollama | gemma3:latest | nomic-embed-text (768d) |

### Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing dev flow | Low | Medium | Keep Ollama as default |
| OpenAI cost | Medium | Low | Use gpt-4o-mini (cheapest) |
| API rate limits | Low | Medium | Backend handles retries |

## Decision

Update `dev-bg` Makefile target to use OpenAI when `OPENAI_API_KEY` is set, otherwise fallback to Ollama.
