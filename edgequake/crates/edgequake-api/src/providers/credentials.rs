//! First-principles LLM credential prerequisites.
//!
//! **Goal:** only select providers that can authenticate in this runtime.
//!
//! **Ladder (query-time):**
//! 1. Request override — if credentials for that provider are configured.
//! 2. Workspace override — same gate; creation/auth failures fall through.
//! 3. Server default — startup `from_env()` provider (no override).
//! 4. Runtime auth rejection on override — retry without override (see `query_execution`).

use edgequake_llm::model_config::ProviderConfig;
use edgequake_llm::ModelsConfig;

/// True when the provider's runtime prerequisites are satisfied (key env non-empty, etc.).
///
/// This does **not** prove the key is valid — only that configuration exists.
/// Invalid keys are handled at execution time via auth fallback.
pub fn provider_credentials_configured(provider: &ProviderConfig) -> bool {
    match provider.name.as_str() {
        "mock" | "ollama" | "lmstudio" => true,
        "vertexai" => {
            std::env::var("GOOGLE_CLOUD_PROJECT").is_ok()
                && (std::env::var("GOOGLE_ACCESS_TOKEN").is_ok()
                    || std::env::var("GOOGLE_APPLICATION_CREDENTIALS").is_ok())
        }
        _ => {
            if let Some(env) = provider.api_key_env.as_ref().filter(|s| !s.is_empty()) {
                std::env::var(env)
                    .map(|v| !v.trim().is_empty())
                    .unwrap_or(false)
            } else {
                true
            }
        }
    }
}

/// Lookup provider in [`ModelsConfig`] and apply the credential gate.
pub fn llm_provider_credentials_configured(config: &ModelsConfig, provider_name: &str) -> bool {
    config
        .providers
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(provider_name))
        .map(provider_credentials_configured)
        .unwrap_or_else(|| llm_provider_credentials_configured_by_name(provider_name))
}

/// Fallback when models.toml has no entry (tests, unknown ids).
pub fn llm_provider_credentials_configured_by_name(provider_name: &str) -> bool {
    match provider_name.to_ascii_lowercase().as_str() {
        "mock" | "ollama" | "lmstudio" => true,
        "openai" => env_non_empty("OPENAI_API_KEY"),
        "mistral" => env_non_empty("MISTRAL_API_KEY"),
        "anthropic" => env_non_empty("ANTHROPIC_API_KEY"),
        "google" | "gemini" => env_non_empty("GOOGLE_API_KEY"),
        "vertexai" => {
            std::env::var("GOOGLE_CLOUD_PROJECT").is_ok()
                && (std::env::var("GOOGLE_ACCESS_TOKEN").is_ok()
                    || std::env::var("GOOGLE_APPLICATION_CREDENTIALS").is_ok())
        }
        _ => true,
    }
}

fn env_non_empty(name: &str) -> bool {
    std::env::var(name)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::ModelsConfig;

    #[test]
    fn openai_requires_non_empty_key_env() {
        let prev = std::env::var("OPENAI_API_KEY").ok();
        std::env::set_var("OPENAI_API_KEY", "");
        assert!(!llm_provider_credentials_configured_by_name("openai"));
        if let Some(v) = prev {
            std::env::set_var("OPENAI_API_KEY", v);
        } else {
            std::env::remove_var("OPENAI_API_KEY");
        }
    }

    #[test]
    fn mock_always_configured() {
        assert!(llm_provider_credentials_configured_by_name("mock"));
    }

    #[test]
    fn uses_models_config_when_present() {
        let config = ModelsConfig::builtin_defaults();
        let openai = config.providers.iter().find(|p| p.name == "openai");
        if let Some(p) = openai {
            let prev = std::env::var("OPENAI_API_KEY").ok();
            std::env::remove_var("OPENAI_API_KEY");
            assert!(!provider_credentials_configured(p));
            if let Some(v) = prev {
                std::env::set_var("OPENAI_API_KEY", v);
            }
        }
    }
}
