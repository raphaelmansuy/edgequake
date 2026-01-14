# OODA Iteration 170 - Provider Priority Resolution

## Observe

### Focus

Verify that provider priority is used for default selection.

### Investigation

**Priority Configuration** (from `models.toml`):

```toml
[[providers]]
id = "openai"
priority = 10

[[providers]]
id = "ollama"
priority = 20

[[providers]]
id = "lmstudio"
priority = 30
```

### Priority Order

1. OpenAI (priority 10) - First choice
2. Ollama (priority 20) - Second choice
3. LM Studio (priority 30) - Third choice

## Orient

### Priority Resolution Flow

```
Get Default Provider
         │
         ▼
Sort by priority (ascending)
         │
         ▼
Filter available providers
         │
         ▼
Select first available
         │
         ▼
Return provider
```

### Use Cases

1. **All available**: OpenAI selected (lowest priority number)
2. **OpenAI unavailable**: Ollama selected
3. **Only LM Studio**: LM Studio selected

## Decide

**Status**: ✅ COMPLETE

Provider priority resolution is implemented correctly.

## Act

### Verified

- Priority numbers in provider config
- Lower number = higher priority
- Automatic fallback selection
- Consistent with OODA 150

---

_Commit: docs(OODA 170): Verify provider priority resolution_
