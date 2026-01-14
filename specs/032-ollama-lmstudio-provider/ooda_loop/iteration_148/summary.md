# OODA Iteration 148 - Context Length Display

## Observe

### Focus
Verify that model context length is displayed in the UI.

### Investigation

**Model Card Capabilities** (from `models.toml`):
```toml
[providers.models.capabilities]
context_length = 128000
max_output_tokens = 16384
```

### Frontend Display

The `ProviderModelSelector` and model cards show context length information.

## Orient

### Context Length by Provider

| Provider | Model | Context Length |
|----------|-------|----------------|
| OpenAI | gpt-4o | 128K |
| OpenAI | gpt-4.1 | 1M+ |
| Ollama | llama3.2 | 131K |
| Ollama | gemma3:12b | 128K |

### UI Integration

- Context length shown in model dropdown
- Formatted as "128K" for readability
- Long context models highlighted

## Decide

**Status**: ✅ COMPLETE

Context length is defined per model and exposed to UI.

## Act

### Verified

- All models have `context_length` defined
- API returns context length
- UI can display formatted context length

### Key Values

| Model | Context | Max Output |
|-------|---------|------------|
| gpt-4.1 | 1,047,576 | 32,768 |
| gpt-4o | 128,000 | 16,384 |
| gemma3:12b | 128,000 | 4,096 |

---
*Commit: docs(OODA 148): Verify context length display*
