//! Bundled `models.toml` loader shared across AppState constructors and API catalog.
//!
//! Single source of truth for provider/model metadata — avoids duplicating
//! `include_str!(...)` + parse fallback in memory and postgres paths.

use std::sync::Arc;

use edgequake_llm::ModelsConfig;

/// Load the workspace-bundled `models.toml`, falling back to env/file/builtin defaults.
pub fn bundled_models_config() -> Arc<ModelsConfig> {
    Arc::new(load_bundled_models_config())
}

/// Same as [`bundled_models_config`] but returns an owned value (for tests).
pub fn load_bundled_models_config() -> ModelsConfig {
    const BUNDLED_MODELS: &str = include_str!("../../../../models.toml");
    ModelsConfig::from_toml(BUNDLED_MODELS)
        .or_else(|_| ModelsConfig::load())
        .unwrap_or_else(|_| ModelsConfig::builtin_defaults())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_models_config_parses_and_has_core_providers() {
        let config = load_bundled_models_config();
        let names: Vec<_> = config.providers.iter().map(|p| p.name.as_str()).collect();
        for id in ["openai", "ollama", "lmstudio", "mock", "mistral"] {
            assert!(
                names.contains(&id),
                "missing provider {id} in bundled models.toml"
            );
        }
    }
}
