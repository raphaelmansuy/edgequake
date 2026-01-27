# OODA Iteration 124: Observe

## Date: 2026-01-14

## Mission Checkpoint

Focus on SPEC-032 Item 28:

- Ensure `make dev` propagates OPENAI_API_KEY environment variable
- Allow users to switch to OpenAI models when Ollama is default

## Observations

### 1. Current Makefile Structure

Need to check if OPENAI_API_KEY is passed through to the backend.

### 2. Expected Behavior

When user runs:

```bash
export OPENAI_API_KEY="sk-..."
make dev
```

The backend should:

1. Receive the OPENAI_API_KEY
2. Enable OpenAI provider
3. Allow queries with OpenAI models

### 3. Files to Review

| File           | Purpose                               |
| -------------- | ------------------------------------- |
| `Makefile`     | Check env var propagation             |
| `.env.example` | Check if OPENAI_API_KEY is documented |
| `AGENTS.md`    | Check setup documentation             |

## Next Steps

1. Review Makefile for env var handling
2. Test current behavior
3. Fix if needed
