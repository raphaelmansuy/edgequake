# OODA Iteration 09 - Decide

## Decision

Update the `dev-bg` Makefile target to use OpenAI provider when `OPENAI_API_KEY` is set.

## Implementation

### Changes to Makefile

**Current** (lines 223-260):

```makefile
dev-bg:
    # Always sets OLLAMA_* vars regardless of OPENAI_API_KEY
    OLLAMA_HOST="http://localhost:11434" \
    OLLAMA_MODEL="gemma3:latest" \
    ...
```

**New**:

```makefile
dev-bg:
    # Use EDGEQUAKE_LLM_PROVIDER for explicit selection
    # If OPENAI_API_KEY is set → openai
    # Otherwise → ollama (with host/model vars)
```

### Specific Changes

1. Add conditional logic to select provider based on `OPENAI_API_KEY`
2. When OpenAI: Set `EDGEQUAKE_LLM_PROVIDER=openai`
3. When Ollama: Set Ollama host/model vars (existing behavior)

### Test Plan

1. Stop current services: `make stop`
2. Start with OpenAI: `OPENAI_API_KEY=sk-xxx make dev-bg`
3. Check health: `curl localhost:8080/health | jq '.llm_provider_name'`
4. Verify output is `"openai"`

### Success Criteria

1. ✅ When `OPENAI_API_KEY` set → provider is `openai`
2. ✅ When `OPENAI_API_KEY` not set → provider is `ollama`
3. ✅ All existing functionality preserved
4. ✅ Clear logging indicates which provider is active
