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
    fn bundled_catalog_marks_models_shut_down_on_2026_10_23_as_deprecated() {
        let config = embedded_models_catalog();
        let openai = config
            .providers
            .iter()
            .find(|p| p.name == "openai")
            .expect("openai provider");
        for (name, replacement) in [
            ("gpt-3.5-turbo", "gpt-5.6-terra"),
            ("gpt-4-turbo", "gpt-5.6-sol"),
            ("o4-mini", "gpt-5.6-terra"),
            ("gpt-4.1-nano", "gpt-5.6-luna"),
        ] {
            let card = openai
                .models
                .iter()
                .find(|m| m.name == name)
                .unwrap_or_else(|| panic!("{name} missing from bundled models.toml"));
            assert!(card.deprecated, "{name} is shut down on 2026-10-23");
            assert_eq!(card.replacement.as_deref(), Some(replacement), "{name}");
        }
    }

    #[test]
    fn bundled_embedded_catalog_has_core_providers() {
        let config = embedded_models_catalog();
        let names: Vec<_> = config.providers.iter().map(|p| p.name.as_str()).collect();
        for id in ["openai", "ollama", "lmstudio", "mock", "mistral"] {
            assert!(
                names.contains(&id),
                "missing provider {id} in bundled models.toml"
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
