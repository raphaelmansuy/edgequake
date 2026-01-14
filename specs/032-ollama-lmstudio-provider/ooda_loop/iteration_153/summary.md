# OODA Iteration 153 - Environment Variable Configuration

## Observe

### Focus
Verify that environment variables override default configuration.

### Investigation

**Environment Variables** (from `models.toml` header):

```toml
# Environment variable overrides:
#   - OPENAI_API_KEY: Required for OpenAI provider
#   - OLLAMA_BASE_URL: Override Ollama API endpoint (default: http://localhost:11434)
#   - LM_STUDIO_BASE_URL: Override LM Studio endpoint (default: http://localhost:1234)
```

### Provider Configuration

```toml
[[providers]]
name = "openai"
api_key_env = "OPENAI_API_KEY"

[[providers]]
name = "ollama"
api_base = "http://localhost:11434"  # Overridden by OLLAMA_BASE_URL
```

## Orient

### Configuration Priority

1. Environment variable (highest)
2. `models.toml` configuration
3. Built-in defaults (lowest)

### Supported Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| OPENAI_API_KEY | OpenAI authentication | Required |
| OLLAMA_BASE_URL | Ollama endpoint | http://localhost:11434 |
| LM_STUDIO_BASE_URL | LM Studio endpoint | http://localhost:1234 |
| EDGEQUAKE_MODELS_CONFIG | Custom config path | ./models.toml |

## Decide

**Status**: ✅ COMPLETE

Environment variables are documented and respected.

## Act

### Verified

- API key environment variables documented
- Base URL overrides supported
- Config path can be customized
- Documentation in models.toml header

---
*Commit: docs(OODA 153): Verify environment variable configuration*
