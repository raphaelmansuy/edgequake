//! Build provider listing API responses from [`ModelsConfig`] (models.toml).
//!
//! Replaces the hand-maintained provider matrix in `provider_types.rs` so the
//! settings UI stays aligned with runtime model configuration.

use edgequake_llm::model_config::{ModelType, ProviderConfig};
use edgequake_llm::ModelsConfig;

use crate::provider_visibility::is_ui_visible_provider_id;

use crate::provider_types::{
    AvailableProvidersResponse, ConfigRequirement, DefaultModels, ProviderInfo,
};
use crate::providers::credentials::provider_credentials_configured;

/// Build the available-providers response from configuration.
pub fn build_available_providers_response(
    config: &ModelsConfig,
    active_llm: &str,
    active_embedding: &str,
) -> AvailableProvidersResponse {
    let mut providers: Vec<ProviderConfig> = config
        .providers
        .iter()
        .filter(|p| p.enabled && is_ui_visible_provider_id(&p.name))
        .cloned()
        .collect();

    providers.sort_by_key(|p| p.priority);

    let provider_infos: Vec<ProviderInfo> =
        providers.iter().map(provider_info_from_config).collect();

    AvailableProvidersResponse {
        llm_providers: provider_infos.clone(),
        embedding_providers: provider_infos,
        active_llm_provider: active_llm.to_string(),
        active_embedding_provider: active_embedding.to_string(),
    }
}

fn provider_info_from_config(provider: &ProviderConfig) -> ProviderInfo {
    let embedding_model = resolve_default_embedding_model(provider);
    let chat_model = resolve_default_chat_model(provider);
    let embedding_dimension = resolve_embedding_dimension(provider, &embedding_model);

    ProviderInfo {
        id: provider.name.clone(),
        name: provider.display_name.clone(),
        description: provider.description.clone(),
        available: provider_is_available(provider),
        config_requirements: build_config_requirements(provider),
        default_models: DefaultModels {
            chat_model,
            embedding_model,
            embedding_dimension,
        },
    }
}

fn resolve_default_chat_model(provider: &ProviderConfig) -> String {
    if let Some(model) = &provider.default_llm_model {
        return model.clone();
    }
    pick_model_by_type(provider, &[ModelType::Llm, ModelType::Multimodal])
}

fn resolve_default_embedding_model(provider: &ProviderConfig) -> String {
    if let Some(model) = &provider.default_embedding_model {
        return model.clone();
    }
    pick_model_by_type(provider, &[ModelType::Embedding])
}

fn pick_model_by_type(provider: &ProviderConfig, types: &[ModelType]) -> String {
    provider
        .models
        .iter()
        .find(|m| m.tags.iter().any(|t| t == "default"))
        .filter(|m| types.contains(&m.model_type))
        .or_else(|| {
            provider.models.iter().find(|m| {
                m.tags.iter().any(|t| t == "recommended") && types.contains(&m.model_type)
            })
        })
        .or_else(|| {
            provider
                .models
                .iter()
                .find(|m| types.contains(&m.model_type))
        })
        .map(|m| m.name.clone())
        .unwrap_or_default()
}

fn resolve_embedding_dimension(provider: &ProviderConfig, embedding_model: &str) -> usize {
    if embedding_model.is_empty() {
        return 0;
    }
    provider
        .models
        .iter()
        .find(|m| m.name == embedding_model)
        .map(|m| m.capabilities.embedding_dimension)
        .filter(|d| *d > 0)
        .unwrap_or_else(|| {
            provider
                .models
                .iter()
                .find(|m| m.model_type == ModelType::Embedding)
                .map(|m| m.capabilities.embedding_dimension)
                .unwrap_or(0)
        })
}

fn provider_is_available(provider: &ProviderConfig) -> bool {
    provider_credentials_configured(provider)
}

fn build_config_requirements(provider: &ProviderConfig) -> Vec<ConfigRequirement> {
    crate::providers::credentials::provider_config_requirements(provider)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::bundled_models::embedded_models_catalog;

    #[test]
    fn catalog_includes_vertexai_from_models_toml() {
        let config = embedded_models_catalog();
        let response = build_available_providers_response(&config, "mock", "mock");
        let ids: Vec<_> = response
            .llm_providers
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        assert!(ids.contains(&"vertexai"));
    }

    #[test]
    fn catalog_uses_models_toml_defaults_for_lmstudio() {
        let config = embedded_models_catalog();
        let response = build_available_providers_response(&config, "mock", "mock");
        let lmstudio = response
            .llm_providers
            .iter()
            .find(|p| p.id == "lmstudio")
            .expect("lmstudio");
        assert_eq!(lmstudio.default_models.chat_model, "gemma-3n-e4b-it");
        assert_eq!(
            lmstudio.default_models.embedding_model,
            "text-embedding-nomic-embed-text-v1.5"
        );
        assert_eq!(lmstudio.default_models.embedding_dimension, 768);
    }

    #[test]
    fn mock_provider_excluded_from_ui_catalog() {
        let config = embedded_models_catalog();
        let response = build_available_providers_response(&config, "mock", "mock");
        assert!(
            !response.llm_providers.iter().any(|p| p.id == "mock"),
            "mock must not appear in UI provider catalog"
        );
    }

    #[test]
    fn providers_sorted_by_priority() {
        let config = embedded_models_catalog();
        let response = build_available_providers_response(&config, "mock", "mock");
        let openai_idx = response
            .llm_providers
            .iter()
            .position(|p| p.id == "openai")
            .unwrap();
        let ollama_idx = response
            .llm_providers
            .iter()
            .position(|p| p.id == "ollama")
            .unwrap();
        assert!(
            openai_idx < ollama_idx,
            "lower priority number should sort first"
        );
    }

    #[test]
    fn bundled_catalog_covers_edgequake_llm_chat_providers() {
        use crate::provider_visibility::expected_chat_provider_ids;
        let config = embedded_models_catalog();
        let configured: std::collections::HashSet<_> = config
            .providers
            .iter()
            .filter(|p| p.enabled && is_ui_visible_provider_id(&p.name))
            .map(|p| p.name.as_str())
            .collect();

        // Optional: disabled until credentials/deployment configured, or env-driven generic compat.
        let optional_missing = ["openai-compatible", "bedrock", "azure"];
        for id in expected_chat_provider_ids() {
            if optional_missing.contains(&id) {
                continue;
            }
            assert!(
                configured.contains(id),
                "models.toml missing enabled chat provider '{id}' from edgequake-llm catalog"
            );
        }
    }
}
