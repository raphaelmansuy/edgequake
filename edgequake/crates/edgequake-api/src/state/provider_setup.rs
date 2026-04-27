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
//! | `EDGEQUAKE_EMBEDDING_PROVIDER` | Override provider type | `ollama`, `openai`, `azure`, `mistral` |
//! | `OLLAMA_EMBEDDING_HOST` | Dedicated Ollama host for embeddings | `http://gpu-box:11434` |
//! | `OLLAMA_EMBEDDING_MODEL` | Model for Ollama embedding (Ollama-provider path only) | `nomic-embed-text` |
//! | `MISTRAL_EMBEDDING_MODEL` | Model for Mistral embedding | `mistral-embed` |
//! | `OPENAI_EMBEDDING_MODEL` | Model for OpenAI embedding | `text-embedding-3-small` |
//! | `EDGEQUAKE_EMBEDDING_MODEL` | Generic embedding model override (all providers) | `text-embedding-3-small` |
//! | `EDGEQUAKE_EMBEDDING_DIMENSION` | Dimension of the embedding vectors (overrides well-known table) | `768`, `1536` |
//! | `EDGEQUAKE_EMBEDDING_BASE_URL` | Override base URL for embedding provider | `https://embed.example.com/v1` |
//! | `EDGEQUAKE_EMBEDDING_API_KEY` | Override API key for embedding provider | `sk-embed-...` |
//! | `AZURE_OPENAI_API_KEY` | Azure embedding (auto-detected when `EDGEQUAKE_EMBEDDING_PROVIDER=azure`) | `sk-...` |
//! | `AZURE_OPENAI_ENDPOINT` | Azure endpoint for embedding | `https://my-resource.openai.azure.com` |
//! | `MISTRAL_API_KEY` | Mistral embedding (auto-detected when `EDGEQUAKE_EMBEDDING_PROVIDER=mistral`) | `...` |

use std::sync::Arc;

use edgequake_core::Workspace;
use edgequake_llm::traits::EmbeddingProvider;
use edgequake_llm::{OllamaProvider, OpenAIProvider, ProviderFactory};

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
    // WHY: docker-compose may pass an empty string when the host env var is unset
    // (e.g. `EDGEQUAKE_EMBEDDING_PROVIDER: ${EDGEQUAKE_EMBEDDING_PROVIDER:-}`).
    // Treat empty string as "not set" to avoid a spurious warning and fall through
    // to the auto-detection logic below.
    if let Ok(provider_name) = std::env::var("EDGEQUAKE_EMBEDDING_PROVIDER") {
        if !provider_name.is_empty() {
            // WHY: Use provider-aware model resolution so that e.g. MISTRAL_EMBEDDING_MODEL
            // is not shadowed by OLLAMA_EMBEDDING_MODEL from an unrelated .env entry.
            let model = embedding_model_for_provider(&provider_name);
            let dimension = embedding_dimension_for_model_and_env(&model);

            // FIX #163: Check for embedding-specific base URL and API key.
            // WHY: In split-provider deployments, chat and embedding traffic go to
            // different servers with different API keys. Without these overrides,
            // both providers share OPENAI_BASE_URL / OPENAI_API_KEY.
            let embed_base_url = std::env::var("EDGEQUAKE_EMBEDDING_BASE_URL").ok();
            let embed_api_key = std::env::var("EDGEQUAKE_EMBEDDING_API_KEY").ok();

            let has_custom_base_url = embed_base_url.is_some();
            let has_custom_api_key = embed_api_key.is_some();
            let is_openai_compatible = matches!(
                provider_name.to_ascii_lowercase().as_str(),
                "openai" | "openai-compatible" | "openai_compatible"
            );

            if is_openai_compatible && (has_custom_base_url || has_custom_api_key) {
                // Use dedicated credentials only for OpenAI-compatible embedding providers.
                let api_key = embed_api_key
                    .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                    .unwrap_or_default();
                let base_url = embed_base_url.or_else(|| std::env::var("OPENAI_BASE_URL").ok());

                let provider: Arc<dyn EmbeddingProvider> = if let Some(base_url) = base_url {
                    Arc::new(
                        OpenAIProvider::compatible(api_key, base_url).with_embedding_model(&model),
                    )
                } else {
                    Arc::new(OpenAIProvider::new(api_key).with_embedding_model(&model))
                };

                tracing::info!(
                    provider = %provider_name,
                    model = %model,
                    dimension,
                    has_custom_base_url,
                    has_custom_api_key,
                    "Embedding provider overridden with dedicated base URL/API key (FIX #163)"
                );
                return provider;
            }

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

/// Apply `EDGEQUAKE_CHAT_*` → standard LLM env var aliases.
///
/// FIX #166: Users expect symmetry with `EDGEQUAKE_EMBEDDING_*` naming.
/// This function maps chat-specific env vars to the standard ones used by
/// `ProviderFactory::from_env()`:
///
/// - `EDGEQUAKE_CHAT_BASE_URL` → `OPENAI_BASE_URL` (if not already set)
/// - `EDGEQUAKE_CHAT_API_KEY`  → `OPENAI_API_KEY`  (if not already set)
/// - `EDGEQUAKE_CHAT_MODEL`    → `EDGEQUAKE_LLM_MODEL` (if not already set)
///
/// Must be called BEFORE `ProviderFactory::from_env()`.
pub fn apply_chat_env_aliases() {
    if let Ok(chat_base_url) = std::env::var("EDGEQUAKE_CHAT_BASE_URL") {
        if std::env::var("OPENAI_BASE_URL").is_err() {
            std::env::set_var("OPENAI_BASE_URL", chat_base_url);
        }
    }
    if let Ok(chat_api_key) = std::env::var("EDGEQUAKE_CHAT_API_KEY") {
        if std::env::var("OPENAI_API_KEY").is_err() {
            std::env::set_var("OPENAI_API_KEY", chat_api_key);
        }
    }
    if let Ok(chat_model) = std::env::var("EDGEQUAKE_CHAT_MODEL") {
        if std::env::var("EDGEQUAKE_LLM_MODEL").is_err() {
            std::env::set_var("EDGEQUAKE_LLM_MODEL", chat_model);
        }
    }
}

/// Resolve the embedding model name for a named provider.
///
/// # Priority (First Principle: most-specific wins)
///
/// 1. Provider-specific env var (e.g. `MISTRAL_EMBEDDING_MODEL`, `OPENAI_EMBEDDING_MODEL`)
/// 2. Generic `EDGEQUAKE_EMBEDDING_MODEL`
/// 3. `Workspace::default_embedding_model_for_provider(provider_name)`
///
/// WHY NOT `OLLAMA_EMBEDDING_MODEL` here: that var is Ollama-specific and would
/// erroneously override Mistral / OpenAI embedding models when both are present
/// in the environment (e.g. from a `.env` that also configures Ollama).
fn embedding_model_for_provider(provider_name: &str) -> String {
    // 1. Provider-specific env var
    if let Some(key) = provider_specific_embedding_env_key(provider_name) {
        if let Ok(model) = std::env::var(key) {
            if !model.is_empty() {
                return model;
            }
        }
    }
    // 2. Generic override
    if let Ok(model) = std::env::var("EDGEQUAKE_EMBEDDING_MODEL") {
        if !model.is_empty() {
            return model;
        }
    }
    // 3. Provider default
    Workspace::default_embedding_model_for_provider(provider_name)
}

/// Return the well-known env-var key for a provider's embedding model, or `None`
/// if no provider-specific key is defined.
fn provider_specific_embedding_env_key(provider_name: &str) -> Option<&'static str> {
    match provider_name.to_ascii_lowercase().as_str() {
        "mistral" => Some("MISTRAL_EMBEDDING_MODEL"),
        "openai" | "openai-compatible" | "openai_compatible" => Some("OPENAI_EMBEDDING_MODEL"),
        "ollama" => Some("OLLAMA_EMBEDDING_MODEL"),
        "lmstudio" | "lm-studio" | "lm_studio" => Some("LMSTUDIO_EMBEDDING_MODEL"),
        "anthropic" => Some("ANTHROPIC_EMBEDDING_MODEL"),
        "gemini" => Some("GEMINI_EMBEDDING_MODEL"),
        _ => None,
    }
}

/// Resolve the embedding dimension.
///
/// # Priority
///
/// 1. `EDGEQUAKE_EMBEDDING_DIMENSION` — explicit user override
/// 2. `Workspace::known_embedding_dimension(model)` — compile-time well-known table
/// 3. 768 — conservative default (safe for most Ollama models)
fn embedding_dimension_for_model_and_env(model: &str) -> usize {
    if let Some(d) = std::env::var("EDGEQUAKE_EMBEDDING_DIMENSION")
        .ok()
        .and_then(|d| d.parse::<usize>().ok())
    {
        return d;
    }
    Workspace::known_embedding_dimension(model).unwrap_or(768)
}

/// Read the embedding model name from environment variables (Ollama-specific path).
///
/// Used exclusively by the `OLLAMA_EMBEDDING_HOST` code path where we know the
/// provider is Ollama.  Checks `OLLAMA_EMBEDDING_MODEL` first, then the generic
/// `EDGEQUAKE_EMBEDDING_MODEL`, then falls back to `"nomic-embed-text"`.
#[allow(dead_code)] // kept for backward compatibility / tests
fn embedding_model_from_env() -> String {
    std::env::var("OLLAMA_EMBEDDING_MODEL")
        .or_else(|_| std::env::var("EDGEQUAKE_EMBEDDING_MODEL"))
        .unwrap_or_else(|_| "nomic-embed-text".to_string())
}

/// Read the embedding dimension from `EDGEQUAKE_EMBEDDING_DIMENSION`, defaulting
/// to 768 (compatible with most Ollama embedding models).
#[allow(dead_code)] // kept for tests that call it directly
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

    #[test]
    #[serial]
    fn apply_chat_env_aliases_populates_missing_standard_vars() {
        std::env::remove_var("EDGEQUAKE_CHAT_BASE_URL");
        std::env::remove_var("EDGEQUAKE_CHAT_API_KEY");
        std::env::remove_var("EDGEQUAKE_CHAT_MODEL");
        std::env::remove_var("OPENAI_BASE_URL");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("EDGEQUAKE_LLM_MODEL");

        std::env::set_var("EDGEQUAKE_CHAT_BASE_URL", "https://chat.example.test/v1");
        std::env::set_var("EDGEQUAKE_CHAT_API_KEY", "chat-key");
        std::env::set_var("EDGEQUAKE_CHAT_MODEL", "gpt-test");

        apply_chat_env_aliases();

        assert_eq!(
            std::env::var("OPENAI_BASE_URL").as_deref(),
            Ok("https://chat.example.test/v1")
        );
        assert_eq!(std::env::var("OPENAI_API_KEY").as_deref(), Ok("chat-key"));
        assert_eq!(
            std::env::var("EDGEQUAKE_LLM_MODEL").as_deref(),
            Ok("gpt-test")
        );

        std::env::remove_var("EDGEQUAKE_CHAT_BASE_URL");
        std::env::remove_var("EDGEQUAKE_CHAT_API_KEY");
        std::env::remove_var("EDGEQUAKE_CHAT_MODEL");
        std::env::remove_var("OPENAI_BASE_URL");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("EDGEQUAKE_LLM_MODEL");
    }

    #[test]
    #[serial]
    fn apply_chat_env_aliases_preserves_explicit_standard_vars() {
        std::env::set_var("EDGEQUAKE_CHAT_BASE_URL", "https://chat.example.test/v1");
        std::env::set_var("EDGEQUAKE_CHAT_API_KEY", "chat-key");
        std::env::set_var("EDGEQUAKE_CHAT_MODEL", "gpt-chat");
        std::env::set_var("OPENAI_BASE_URL", "https://explicit.example.test/v1");
        std::env::set_var("OPENAI_API_KEY", "explicit-key");
        std::env::set_var("EDGEQUAKE_LLM_MODEL", "gpt-explicit");

        apply_chat_env_aliases();

        assert_eq!(
            std::env::var("OPENAI_BASE_URL").as_deref(),
            Ok("https://explicit.example.test/v1")
        );
        assert_eq!(
            std::env::var("OPENAI_API_KEY").as_deref(),
            Ok("explicit-key")
        );
        assert_eq!(
            std::env::var("EDGEQUAKE_LLM_MODEL").as_deref(),
            Ok("gpt-explicit")
        );

        std::env::remove_var("EDGEQUAKE_CHAT_BASE_URL");
        std::env::remove_var("EDGEQUAKE_CHAT_API_KEY");
        std::env::remove_var("EDGEQUAKE_CHAT_MODEL");
        std::env::remove_var("OPENAI_BASE_URL");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("EDGEQUAKE_LLM_MODEL");
    }

    // -------------------------------------------------------------------------
    // Tests for the new provider-aware embedding model resolution (First Principle)
    // -------------------------------------------------------------------------

    /// Mistral-specific env var wins over OLLAMA_EMBEDDING_MODEL.
    /// WHY: This is the exact scenario that caused the "embeddinggemma" leak
    /// when both MISTRAL_EMBEDDING_MODEL and OLLAMA_EMBEDDING_MODEL were set.
    #[test]
    #[serial]
    fn embedding_model_for_provider_mistral_specific_key_wins() {
        std::env::set_var("OLLAMA_EMBEDDING_MODEL", "embeddinggemma:latest");
        std::env::set_var("MISTRAL_EMBEDDING_MODEL", "mistral-embed");
        std::env::remove_var("EDGEQUAKE_EMBEDDING_MODEL");

        assert_eq!(embedding_model_for_provider("mistral"), "mistral-embed");

        std::env::remove_var("OLLAMA_EMBEDDING_MODEL");
        std::env::remove_var("MISTRAL_EMBEDDING_MODEL");
    }

    /// Generic EDGEQUAKE_EMBEDDING_MODEL is used when no provider-specific key is set.
    #[test]
    #[serial]
    fn embedding_model_for_provider_generic_override() {
        std::env::remove_var("MISTRAL_EMBEDDING_MODEL");
        std::env::remove_var("OLLAMA_EMBEDDING_MODEL");
        std::env::set_var("EDGEQUAKE_EMBEDDING_MODEL", "custom-embed");

        assert_eq!(embedding_model_for_provider("mistral"), "custom-embed");

        std::env::remove_var("EDGEQUAKE_EMBEDDING_MODEL");
    }

    /// Provider default returned when no env vars set.
    #[test]
    #[serial]
    fn embedding_model_for_provider_falls_back_to_workspace_default() {
        std::env::remove_var("MISTRAL_EMBEDDING_MODEL");
        std::env::remove_var("OLLAMA_EMBEDDING_MODEL");
        std::env::remove_var("EDGEQUAKE_EMBEDDING_MODEL");

        assert_eq!(
            embedding_model_for_provider("mistral"),
            "mistral-embed",
            "Should return Workspace default for mistral"
        );
        assert_eq!(
            embedding_model_for_provider("openai"),
            "text-embedding-3-small",
            "Should return Workspace default for openai"
        );
    }

    /// OLLAMA_EMBEDDING_MODEL is NOT picked up for the Mistral provider.
    /// This test codifies the fix for the .env bleed-through bug.
    #[test]
    #[serial]
    fn ollama_embedding_model_does_not_bleed_into_mistral() {
        std::env::set_var("OLLAMA_EMBEDDING_MODEL", "embeddinggemma:latest");
        std::env::remove_var("MISTRAL_EMBEDDING_MODEL");
        std::env::remove_var("EDGEQUAKE_EMBEDDING_MODEL");

        let model = embedding_model_for_provider("mistral");
        assert_ne!(
            model, "embeddinggemma:latest",
            "OLLAMA_EMBEDDING_MODEL must not bleed into Mistral provider"
        );
        assert_eq!(model, "mistral-embed");

        std::env::remove_var("OLLAMA_EMBEDDING_MODEL");
    }

    /// Dimension resolves from model name for well-known models.
    #[test]
    #[serial]
    fn embedding_dimension_for_model_uses_known_table() {
        std::env::remove_var("EDGEQUAKE_EMBEDDING_DIMENSION");
        assert_eq!(embedding_dimension_for_model_and_env("mistral-embed"), 1024);
        assert_eq!(
            embedding_dimension_for_model_and_env("codestral-embed"),
            1024
        );
        assert_eq!(
            embedding_dimension_for_model_and_env("text-embedding-3-small"),
            1536
        );
        assert_eq!(
            embedding_dimension_for_model_and_env("unknown-model"),
            768,
            "Unknown model should fall back to 768"
        );
    }

    /// Explicit EDGEQUAKE_EMBEDDING_DIMENSION overrides the well-known table.
    #[test]
    #[serial]
    fn embedding_dimension_env_override_wins_over_table() {
        std::env::set_var("EDGEQUAKE_EMBEDDING_DIMENSION", "2048");
        assert_eq!(
            embedding_dimension_for_model_and_env("mistral-embed"),
            2048,
            "Env override must win over well-known table"
        );
        std::env::remove_var("EDGEQUAKE_EMBEDDING_DIMENSION");
    }
}
