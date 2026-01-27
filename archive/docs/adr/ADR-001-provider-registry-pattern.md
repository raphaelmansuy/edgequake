# ADR-001: Provider Registry Pattern for Multi-Provider Support

**Status**: Accepted  
**Date**: 2025-01-27  
**Authors**: EdgeQuake Team  
**Implements**: SPEC-032 Ollama/LM Studio Provider Support  
**OODA Loop**: #46

## Context

EdgeQuake needs to support multiple LLM and embedding providers (OpenAI, Ollama, LM Studio) with:

1. Runtime provider selection without code changes
2. Environment-based configuration for CI/CD compatibility
3. Provider-specific optimizations (different dimensions, models)
4. Graceful fallback when providers are unavailable

## Decision

We implemented a **Provider Registry Pattern** with the following components:

### 1. Provider Registry API (`/api/providers`)

```rust
pub struct ProviderInfo {
    pub name: String,           // "openai", "ollama", "lmstudio"
    pub display_name: String,   // "OpenAI", "Ollama (Local)"
    pub available: bool,        // Runtime availability check
    pub embedding_model: String,
    pub embedding_dimension: usize,
}
```

The registry endpoint returns all configured providers with their current status.

### 2. Provider Factory Pattern

```rust
impl ProviderFactory {
    pub fn from_env() -> Box<dyn EmbeddingProvider> {
        if std::env::var("OPENAI_API_KEY").is_ok() {
            return Box::new(OpenAIProvider::from_env()?);
        }
        if std::env::var("OLLAMA_HOST").is_ok() {
            return Box::new(OllamaProvider::from_env()?);
        }
        if std::env::var("LMSTUDIO_HOST").is_ok() {
            return Box::new(LMStudioProvider::from_env()?);
        }
        Box::new(MockProvider::new())
    }
}
```

### 3. Workspace-Level Provider Configuration

Each workspace stores its embedding configuration:

```rust
pub struct WorkspaceEmbeddingConfig {
    pub provider: String,
    pub model: String,
    pub dimension: usize,
}
```

## Consequences

### Positive

- **Zero-code deployment switching**: Change provider by setting environment variables
- **CI/CD friendly**: Tests run with mock provider by default
- **Workspace isolation**: Different workspaces can use different providers
- **Type-safe**: Rust's trait system ensures all providers implement required methods
- **Extensible**: Adding new providers requires only implementing `EmbeddingProvider` trait

### Negative

- **Runtime overhead**: Provider detection happens at startup
- **Configuration complexity**: Multiple environment variables to manage
- **Dimension mismatch risk**: Switching providers may require rebuilding embeddings

### Mitigations

- Provider detection is cached after first call
- PROVIDER_SETUP_GUIDE.md documents all configuration options
- Rebuild embeddings endpoint provides safe migration path

## Related ADRs

- ADR-002: Workspace Embedding Strategy
- ADR-003: Vector Rebuild Safety

## References

- [SPEC-032](../specs/032-ollama-lmstudio-provider/032-ollama-lmstudio-provider.md)
- [Provider Setup Guide](../docs/PROVIDER_SETUP_GUIDE.md)
