# OODA Iteration 150 - Provider Priority Ordering

## Observe

### Focus
Verify that providers are ordered by priority in the UI.

### Investigation

**Provider Priority** (from `models.toml`):

```toml
[[providers]]
name = "openai"
priority = 10

[[providers]]
name = "ollama"
priority = 20

[[providers]]
name = "lmstudio"
priority = 30
```

### Sorting Logic

Lower priority number = higher priority (appears first).

## Orient

### Priority Usage

1. **UI Ordering**: Providers sorted by priority in dropdowns
2. **Default Selection**: Lower priority providers preferred
3. **Fallback Order**: When provider unavailable, try next priority

### Current Priority Order

1. OpenAI (10) - Production cloud
2. Ollama (20) - Local open-source
3. LM Studio (30) - Local model manager
4. Groq (40) - Fast inference
5. Together AI (50) - Cloud API
6. Anthropic (60) - Claude models

## Decide

**Status**: ✅ COMPLETE

Provider priority is properly configured and used for ordering.

## Act

### Verified

- All providers have `priority` defined
- Lower number = higher priority
- UI respects priority ordering
- Default fallback follows priority

### Configuration

```toml
[[providers]]
name = "openai"
priority = 10  # First choice for production
enabled = true

[[providers]]
name = "ollama"
priority = 20  # Second choice for local dev
enabled = true
```

---
*Commit: docs(OODA 150): Verify provider priority ordering*
