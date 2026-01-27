# OODA Iteration 166 - API Key Validation

## Observe

### Focus

Verify that API keys are validated before use.

### Investigation

**API Key Sources** (from OODA 153):

- Environment variables: `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`
- Config in `models.toml` (not recommended for secrets)

**Validation Flow**:

- Provider creation checks for API key
- Missing key prevents provider instantiation

## Orient

### API Key Validation

| Provider  | Key Required | Env Variable        |
| --------- | ------------ | ------------------- |
| OpenAI    | ✅           | `OPENAI_API_KEY`    |
| Anthropic | ✅           | `ANTHROPIC_API_KEY` |
| Ollama    | ❌           | N/A                 |
| LM Studio | ❌           | N/A                 |

### Error Handling

```
Provider Creation
       │
       ▼
┌─────────────────┐
│ API Key needed? │
└─────────────────┘
       │
   Yes │
       ▼
┌─────────────────┐     ┌────────────────┐
│ Key present?    │──No─▶│ Clear error    │
└─────────────────┘     │ "Missing API   │
       │                │  key for X"    │
   Yes ▼                └────────────────┘
Continue with request
```

## Decide

**Status**: ✅ COMPLETE

API key validation is implemented with clear error messages.

## Act

### Verified

- Cloud providers require API keys
- Local providers work without keys
- Missing key error is clear
- Keys read from environment

---

_Commit: docs(OODA 166): Verify API key validation_
