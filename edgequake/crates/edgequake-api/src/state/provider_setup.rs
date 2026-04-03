//! Provider setup utilities for application state construction.
//!
//! Provides a single DRY helper to optionally override the embedding provider
//! after [`edgequake_llm::ProviderFactory::from_env()`] with a dedicated host or
//! provider type.  This enables "hybrid mode" where a different service handles
//! LLM inference versus embedding computation.
//!
//! @implements SPEC-140: Separate embedding and chat provider hosts (closes #140)
//!
//! # Environment Variables
//!
//! | Variable | Purpose | Example |
//! |---|---|---|
//! | `EDGEQUAKE_EMBEDDING_PROVIDER` | Override provider type | `ollama`, `openai` |
//! | `OLLAMA_EMBEDDING_HOST` | Dedicated Ollama host for embeddings | `http://gpu-box:11434` |
//! | `OLLAMA_EMBEDDING_MODEL` | Model on the dedicated embedding host | `nomic-embed-text` |
//! | `EDGEQUAKE_EMBEDDING_MODEL` | Alternative embedding model override | `text-embedding-3-small` |
//! | `EDGEQUAKE_EMBEDDING_DIMENSION` | Dimension of the embedding vectors | `768`, `1536` |

use std::sync::Arc;

use edgequake_llm::traits::EmbeddingProvider;
use edgequake_llm::{OllamaProvider, ProviderFactory};

/// Resolve the embedding provider from environment, optionally overriding the
/// `fallback` returned by `ProviderFactory::from_env()`.
///
/// # Priority
///
/// 1. `EDGEQUAKE_EMBEDDING_PROVIDER` + provider-specific vars → explicit override
/// 2. `OLLAMA_EMBEDDING_HOST` → shortcut to route embeddings to a separate Ollama node
/// 3. `fallback` — the provider already created by `ProviderFactory::from_env()`
///
/// Errors during override creation are logged as warnings and the `fallback` is
/// returned, so startup is never blocked by a misconfigured embedding override.
pub fn resolve_embedding_provider(
    fallback: Arc<dyn EmbeddingProvider>,
) -> Arc<dyn EmbeddingProvider> {
    // --- Priority 1: EDGEQUAKE_EMBEDDING_PROVIDER (explicit provider type) ---
    if let Ok(provider_name) = std::env::var("EDGEQUAKE_EMBEDDING_PROVIDER") {
        let model = embedding_model_from_env();
        let dimension = embedding_dimension_from_env();

        match ProviderFactory::create_embedding_provider(&provider_name, &model, dimension) {
            Ok(provider) => {
                tracing::info!(
                    provider = %provider_name,
                    model = %model,
                    dimension,
                    "Embedding provider overridden via EDGEQUAKE_EMBEDDING_PROVIDER"
                );
                return provider;
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    provider = %provider_name,
                    "Failed to create embedding provider from EDGEQUAKE_EMBEDDING_PROVIDER; \
                     using default"
                );
            }
        }
    }

    // --- Priority 2: OLLAMA_EMBEDDING_HOST (dedicated Ollama embedding node) ---
    if let Ok(embedding_host) = std::env::var("OLLAMA_EMBEDDING_HOST") {
        let model = std::env::var("OLLAMA_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "nomic-embed-text".to_string());

        match OllamaProvider::builder()
            .host(&embedding_host)
            .embedding_model(&model)
            .build()
        {
            Ok(provider) => {
                tracing::info!(
                    host = %embedding_host,
                    model = %model,
                    "Embedding provider overridden via OLLAMA_EMBEDDING_HOST"
                );
                return Arc::new(provider);
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    host = %embedding_host,
                    "Failed to create Ollama embedding provider from OLLAMA_EMBEDDING_HOST; \
                     using default"
                );
            }
        }
    }

    // --- Priority 3: use whatever from_env() already gave us ---
    fallback
}

/// Read the embedding model name from environment variables.
///
/// Checks `OLLAMA_EMBEDDING_MODEL` then `EDGEQUAKE_EMBEDDING_MODEL`, falling
/// back to `"nomic-embed-text"` if neither is set.
fn embedding_model_from_env() -> String {
    std::env::var("OLLAMA_EMBEDDING_MODEL")
        .or_else(|_| std::env::var("EDGEQUAKE_EMBEDDING_MODEL"))
        .unwrap_or_else(|_| "nomic-embed-text".to_string())
}

/// Read the embedding dimension from `EDGEQUAKE_EMBEDDING_DIMENSION`, defaulting
/// to 768 (compatible with most Ollama embedding models).
fn embedding_dimension_from_env() -> usize {
    std::env::var("EDGEQUAKE_EMBEDDING_DIMENSION")
        .ok()
        .and_then(|d| d.parse::<usize>().ok())
        .unwrap_or(768)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::MockProvider;
    use serial_test::serial;

    fn mock_embedding() -> Arc<dyn EmbeddingProvider> {
        Arc::new(MockProvider::new())
    }

    #[test]
    #[serial]
    fn returns_fallback_when_no_env_vars() {
        std::env::remove_var("EDGEQUAKE_EMBEDDING_PROVIDER");
        std::env::remove_var("OLLAMA_EMBEDDING_HOST");

        let fallback = mock_embedding();
        let result = resolve_embedding_provider(fallback.clone());
        assert_eq!(result.name(), "mock");
    }

    #[test]
    #[serial]
    fn returns_fallback_on_unknown_provider() {
        std::env::remove_var("OLLAMA_EMBEDDING_HOST");
        std::env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "totally_unknown_provider");

        let fallback = mock_embedding();
        let result = resolve_embedding_provider(fallback);
        assert_eq!(result.name(), "mock");

        std::env::remove_var("EDGEQUAKE_EMBEDDING_PROVIDER");
    }

    #[test]
    #[serial]
    fn ollama_embedding_host_overrides_provider() {
        std::env::remove_var("EDGEQUAKE_EMBEDDING_PROVIDER");
        std::env::set_var("OLLAMA_EMBEDDING_HOST", "http://localhost:11434");
        std::env::set_var("OLLAMA_EMBEDDING_MODEL", "nomic-embed-text");

        let result = resolve_embedding_provider(mock_embedding());
        assert_eq!(result.name(), "ollama");

        std::env::remove_var("OLLAMA_EMBEDDING_HOST");
        std::env::remove_var("OLLAMA_EMBEDDING_MODEL");
    }

    #[test]
    #[serial]
    fn embedding_model_from_env_reads_ollama_first() {
        std::env::remove_var("EDGEQUAKE_EMBEDDING_MODEL");
        std::env::set_var("OLLAMA_EMBEDDING_MODEL", "my-model");
        assert_eq!(embedding_model_from_env(), "my-model");
        std::env::remove_var("OLLAMA_EMBEDDING_MODEL");
    }

    #[test]
    #[serial]
    fn embedding_model_from_env_reads_edgequake_fallback() {
        std::env::remove_var("OLLAMA_EMBEDDING_MODEL");
        std::env::set_var("EDGEQUAKE_EMBEDDING_MODEL", "other-model");
        assert_eq!(embedding_model_from_env(), "other-model");
        std::env::remove_var("EDGEQUAKE_EMBEDDING_MODEL");
    }

    #[test]
    #[serial]
    fn embedding_model_from_env_default() {
        std::env::remove_var("OLLAMA_EMBEDDING_MODEL");
        std::env::remove_var("EDGEQUAKE_EMBEDDING_MODEL");
        assert_eq!(embedding_model_from_env(), "nomic-embed-text");
    }

    #[test]
    #[serial]
    fn embedding_dimension_from_env_parses_value() {
        std::env::remove_var("EDGEQUAKE_EMBEDDING_DIMENSION");
        std::env::set_var("EDGEQUAKE_EMBEDDING_DIMENSION", "1536");
        assert_eq!(embedding_dimension_from_env(), 1536);
        std::env::remove_var("EDGEQUAKE_EMBEDDING_DIMENSION");
    }

    #[test]
    #[serial]
    fn embedding_dimension_from_env_default() {
        std::env::remove_var("EDGEQUAKE_EMBEDDING_DIMENSION");
        assert_eq!(embedding_dimension_from_env(), 768);
    }

    #[test]
    #[serial]
    fn embedding_dimension_from_env_invalid_falls_back() {
        std::env::set_var("EDGEQUAKE_EMBEDDING_DIMENSION", "not_a_number");
        assert_eq!(embedding_dimension_from_env(), 768);
        std::env::remove_var("EDGEQUAKE_EMBEDDING_DIMENSION");
    }
}
