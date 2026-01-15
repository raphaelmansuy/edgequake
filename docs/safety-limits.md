# Safety Limits for LLM Providers

This document describes the safety limits feature that enforces hard limits on LLM generation to prevent runaway requests and ensure system stability.

## Overview

EdgeQuake implements safety limits on all LLM provider calls to prevent:

1. **Runaway Token Generation**: LLMs can sometimes generate excessively long responses, consuming resources and causing timeouts
2. **Hung Requests**: Network issues or LLM provider problems can cause requests to hang indefinitely
3. **Resource Exhaustion**: Unbounded API calls can exhaust rate limits and budgets

## Features Implemented

- **FEAT0777**: Safety limits for LLM calls
- **BR0777**: Hard max_tokens limit enforcement
- **BR0778**: Request timeout enforcement

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `EDGEQUAKE_LLM_MAX_TOKENS` | 8192 | Maximum tokens per generation request |
| `EDGEQUAKE_LLM_TIMEOUT_SECS` | 120 | Request timeout in seconds |

### Safety Boundaries

The system enforces the following hard limits regardless of configuration:

| Limit | Value | Rationale |
|-------|-------|-----------|
| Minimum max_tokens | 1 | Prevent accidental 0 token responses |
| Maximum max_tokens | 32768 | Prevent excessive token generation |
| Minimum timeout | 10 seconds | Allow for network latency |
| Maximum timeout | 600 seconds | Prevent indefinite hangs |

## Usage

### Default Configuration

```rust
use edgequake_llm::SafetyLimitsConfig;

// Uses defaults: 8192 tokens, 120 second timeout
let config = SafetyLimitsConfig::default();
```

### Custom Configuration

```rust
use edgequake_llm::SafetyLimitsConfig;

// Custom: 4096 tokens, 60 second timeout
let config = SafetyLimitsConfig::new(4096, 60);
```

### From Environment

```rust
use edgequake_llm::SafetyLimitsConfig;

// Reads from EDGEQUAKE_LLM_MAX_TOKENS and EDGEQUAKE_LLM_TIMEOUT_SECS
let config = SafetyLimitsConfig::from_env();
```

### Preset Configurations

```rust
use edgequake_llm::SafetyLimitsConfig;

// Strict: 1024 tokens, 30 second timeout (for testing)
let strict = SafetyLimitsConfig::strict();

// Permissive: 32768 tokens, 600 second timeout (for complex tasks)
let permissive = SafetyLimitsConfig::permissive();
```

## How It Works

### Workspace Pipeline Creation

When a workspace pipeline is created via `AppState::create_workspace_pipeline()`:

1. Workspace configuration is looked up from the database
2. LLM provider is created with workspace-specific model
3. Provider is wrapped with `SafetyLimitedProviderWrapper`
4. All LLM calls through this provider are protected

### Token Limit Enforcement

When a completion/chat request is made:

1. Request's `max_tokens` is compared to configured limit
2. If request exceeds limit, it's clamped to the configured value
3. A warning is logged if clamping occurs
4. Request proceeds with safe limit

### Timeout Enforcement

When any LLM operation is called:

1. Operation is wrapped in `tokio::time::timeout()`
2. If timeout expires, `LlmError::Timeout` is returned
3. Error is logged with details

## API Integration

### Provider Factory Methods

```rust
use edgequake_llm::ProviderFactory;

// Standard provider (no safety limits)
let provider = ProviderFactory::create_llm_provider("ollama", "gemma3:12b")?;

// Safety-limited provider (recommended for production)
let safe_provider = ProviderFactory::create_safe_llm_provider("ollama", "gemma3:12b")?;
```

### Embedding Provider

```rust
use edgequake_llm::ProviderFactory;

// Safety-limited embedding provider (with timeout)
let provider = ProviderFactory::create_safe_embedding_provider(
    "ollama",
    "nomic-embed-text",
    768,
)?;
```

## Testing

The feature includes comprehensive tests in:

- `edgequake-llm/src/safety_limits.rs` - Unit tests for configuration and limits
- `edgequake-api/tests/e2e_safety_limits.rs` - E2E tests for provider integration

### Running Tests

```bash
# Run safety limits unit tests
cargo test --package edgequake-llm safety_limits

# Run E2E safety limits tests
cargo test --package edgequake-api --test e2e_safety_limits
```

## Logging

Safety limit enforcement is logged at the following levels:

- **INFO**: When creating a safety-limited provider (includes limits used)
- **WARN**: When max_tokens is clamped to configured limit
- **ERROR**: When a request times out

## Best Practices

1. **Production**: Always use `create_safe_llm_provider()` for production deployments
2. **Testing**: Use `SafetyLimitsConfig::strict()` for faster test feedback
3. **Complex Tasks**: Consider using `SafetyLimitsConfig::permissive()` for document processing
4. **Monitoring**: Monitor timeout errors to tune timeout values

## Future Improvements

Potential enhancements:

1. Per-model limits (different limits for different model sizes)
2. Adaptive timeouts based on prompt size
3. Token budget tracking across requests
4. Rate limiting integration
