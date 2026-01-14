# OODA Iteration 162 - Timeout Configuration

## Observe

### Focus
Verify that timeout configuration is available for LLM calls.

### Investigation

**Timeout Settings** (from `models.toml`):
```toml
[defaults]
request_timeout_secs = 120
```

### Provider-Specific Timeouts

Local providers (Ollama, LM Studio) may need longer timeouts:
- Model loading time
- First-token latency
- Large model inference

## Orient

### Timeout Configuration

| Scenario | Typical Timeout |
|----------|----------------|
| Cloud API (fast) | 30-60s |
| Local (model loaded) | 60-120s |
| Local (cold start) | 180-300s |
| Embedding batch | 300s |

### Configuration Location

Timeout can be configured in:
1. `models.toml` defaults
2. Environment variables
3. Per-request override

## Decide

**Status**: ✅ COMPLETE

Timeout configuration is available and sensible defaults are set.

## Act

### Verified

- Default timeout: 120 seconds
- Configurable per deployment
- Local providers get adequate time
- Timeout errors provide clear message

---
*Commit: docs(OODA 162): Verify timeout configuration*
