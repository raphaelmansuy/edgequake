//! Models catalog loader — runtime config first, bundled embed last (SPEC-040 #251).
//!
//! Precedence (first found wins):
//! 1. `EDGEQUAKE_MODELS_CONFIG` / `./models.toml` / `~/.edgequake/models.toml` via [`ModelsConfig::load`]
//! 2. Bundled `models.toml` embedded at compile time
//! 3. [`ModelsConfig::builtin_defaults`]

use std::sync::Arc;

use edgequake_llm::ModelsConfig;

const EMBEDDED_MODELS: &str = include_str!("../../../../models.toml");

/// Parse the compile-time embedded `models.toml` (no runtime file lookup).
pub(crate) fn embedded_models_catalog() -> ModelsConfig {
    ModelsConfig::from_toml(EMBEDDED_MODELS).unwrap_or_else(|parse_err| {
        tracing::warn!(error = %parse_err, "Bundled models.toml parse failed; using builtin defaults");
        ModelsConfig::builtin_defaults()
    })
}

/// Load the models catalog, falling back to env/file/builtin defaults.
pub fn bundled_models_config() -> Arc<ModelsConfig> {
    Arc::new(load_bundled_models_config())
}

/// Same as [`bundled_models_config`] but returns an owned value (for tests).
pub fn load_bundled_models_config() -> ModelsConfig {
    match ModelsConfig::load() {
        Ok(config) => {
            tracing::info!(
                "Loaded models catalog from runtime config (EDGEQUAKE_MODELS_CONFIG / cwd / home)"
            );
            config
        }
        Err(err) => {
            tracing::debug!(error = %err, "No runtime models config found; using bundled catalog");
            embedded_models_catalog()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::io::Write;

    #[test]
    fn bundled_embedded_catalog_has_core_providers() {
        let config = embedded_models_catalog();
        let names: Vec<_> = config.providers.iter().map(|p| p.name.as_str()).collect();
        for id in [
            "openai",
            "ollama",
            "lmstudio",
            "mock",
            "mistral",
            "rapid-mlx",
        ] {
            assert!(
                names.contains(&id),
                "missing provider {id} in bundled models.toml"
            );
        }
        let rapid = config
            .providers
            .iter()
            .find(|provider| provider.name == "rapid-mlx")
            .expect("rapid-mlx");
        assert_eq!(
            rapid.base_url.as_deref(),
            Some("http://127.0.0.1:8000"),
            "catalog base_url is the field the deserializer reads"
        );
        assert!(rapid.api_key_env.as_deref().unwrap_or("").is_empty());
    }

    /// Models that a vendor has retired must not reappear in the catalog.
    ///
    /// WHY a denylist rather than a live API check: CI has no vendor keys, so it cannot ask
    /// "is this model still served?". This pins the specific ids that were verified dead, so a
    /// revert or a bad merge cannot quietly put a model the vendor rejects back in the picker.
    /// Each entry was confirmed against the vendor's own model list or HTTP response.
    #[test]
    fn retired_models_are_not_shipped_in_the_catalog() {
        let config = embedded_models_catalog();
        let shipped: Vec<(&str, &str)> = config
            .providers
            .iter()
            .flat_map(|p| {
                p.models
                    .iter()
                    .map(move |m| (p.name.as_str(), m.name.as_str()))
            })
            .collect();

        // (provider, model, why it is dead)
        let retired = [
            (
                "nvidia",
                "meta/llama-3.1-8b-instruct",
                "NVIDIA retired it 2026-08-26 (HTTP 410) and dropped it from /v1/models",
            ),
            (
                "nvidia",
                "cohere/command-r-plus-08-2024",
                "absent from NVIDIA /v1/models",
            ),
            (
                "openrouter",
                "anthropic/claude-sonnet-4-5-20250929",
                "absent from OpenRouter /api/v1/models",
            ),
            (
                "openrouter",
                "meta-llama/llama-3.1-405b-instruct",
                "absent from OpenRouter /api/v1/models",
            ),
        ];

        for (provider, model, why) in retired {
            assert!(
                !shipped.contains(&(provider, model)),
                "{provider} still ships {model}: {why}"
            );
        }
    }

    #[test]
    fn load_models_catalog_falls_back_when_no_runtime_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let original = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("chdir");
        std::env::remove_var("EDGEQUAKE_MODELS_CONFIG");
        let config = load_bundled_models_config();
        std::env::set_current_dir(original).expect("restore cwd");
        assert!(
            config.providers.iter().any(|p| p.name == "openai"),
            "expected bundled fallback catalog"
        );
    }

    #[test]
    #[serial]
    fn runtime_models_config_overrides_bundled() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("custom-models.toml");
        let mut file = std::fs::File::create(&path).expect("create fixture");
        write!(
            file,
            r#"
[defaults]
llm_provider = "mock"
llm_model = "mock-llm"
embedding_provider = "mock"
embedding_model = "mock-embed"

[[providers]]
name = "mock"
display_name = "Mock"
type = "mock"

[[providers.models]]
name = "spec040-custom-model"
display_name = "SPEC-040 Custom"
model_type = "llm"
"#
        )
        .expect("write fixture");

        let key = "EDGEQUAKE_MODELS_CONFIG";
        unsafe { std::env::set_var(key, path.to_str().expect("utf8 path")) };
        let config = load_bundled_models_config();
        unsafe { std::env::remove_var(key) };

        let mock = config
            .providers
            .iter()
            .find(|p| p.name == "mock")
            .expect("mock provider");
        assert!(
            mock.models.iter().any(|m| m.name == "spec040-custom-model"),
            "custom model card must appear when EDGEQUAKE_MODELS_CONFIG is set"
        );
    }
}
