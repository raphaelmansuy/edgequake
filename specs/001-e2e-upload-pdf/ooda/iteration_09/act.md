# OODA Iteration 09 - Act

## Implementation Summary

### Changes Made

1. **Makefile `dev` target** (lines 160-210):
   - Modified to dynamically select provider based on `OPENAI_API_KEY`
   - When `OPENAI_API_KEY` set: Uses `EDGEQUAKE_LLM_PROVIDER=openai`
   - When `OPENAI_API_KEY` not set: Uses Ollama with explicit settings
   - Updated display messages to show which provider is active

2. **Makefile `dev-bg` target** (lines 222-277):
   - Modified to conditionally set provider environment variables
   - When `OPENAI_API_KEY` set: Uses `EDGEQUAKE_LLM_PROVIDER=openai`
   - When `OPENAI_API_KEY` not set: Uses Ollama with host/model vars
   - Prevents conflict between Ollama vars and OpenAI selection

### Root Cause Fix

**Problem**: Factory priority: `EDGEQUAKE_LLM_PROVIDER` > `OLLAMA_HOST` > `OPENAI_API_KEY`

The old Makefile set both `OLLAMA_HOST` and `OPENAI_API_KEY`, but `OLLAMA_HOST`
took priority in factory.rs auto-detection (line 125).

**Fix**: Set `EDGEQUAKE_LLM_PROVIDER=openai` explicitly when OpenAI key available,
which has highest priority in factory selection (line 113).

### Usage

```bash
# Start with Ollama (default - no OPENAI_API_KEY)
make dev

# Start with OpenAI
export OPENAI_API_KEY="sk-your-key"
make dev

# Background mode with OpenAI
export OPENAI_API_KEY="sk-your-key"
make dev-bg
```

### Verification Command

```bash
# Check which provider is active
curl -s http://localhost:8080/health | jq '.llm_provider_name'
# Expected output: "openai" (when OPENAI_API_KEY set)
# Expected output: "ollama" (when OPENAI_API_KEY not set)
```

## Commit

```
OODA-09: Configure OpenAI provider selection in Makefile

- Update `dev` target to use OpenAI when OPENAI_API_KEY is set
- Update `dev-bg` target to use EDGEQUAKE_LLM_PROVIDER=openai
- Prevent OLLAMA_HOST from overriding OpenAI selection
- Display active provider in startup messages

Fixes: Provider auto-detection selecting Ollama even when OpenAI key available
```

## Next Steps

1. OODA-10: Clean tenant setup for E2E tests
2. Test re-indexing with OpenAI provider
3. Verify embedding dimensions match (1536 for OpenAI vs 768 for Ollama)
