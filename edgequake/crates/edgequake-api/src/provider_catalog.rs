//! Build provider listing API responses from [`ModelsConfig`] (models.toml).
//!
//! Replaces the hand-maintained provider matrix in `provider_types.rs` so the
//! settings UI stays aligned with runtime model configuration.

use edgequake_llm::model_config::{ModelType, ProviderConfig};
use edgequake_llm::ModelsConfig;

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
        .filter(|p| p.enabled)
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
    let mut reqs = Vec::new();

    if let Some(env) = provider.api_key_env.as_ref().filter(|s| !s.is_empty()) {
        let satisfied = std::env::var(env).map(|v| !v.is_empty()).unwrap_or(false);
        reqs.push(ConfigRequirement {
            env_var: env.clone(),
            required: true,
            description: format!("{} API key", provider.display_name),
            satisfied,
        });
    }

    if let Some(env) = provider.base_url_env.as_ref().filter(|s| !s.is_empty()) {
        reqs.push(ConfigRequirement {
            env_var: env.clone(),
            required: false,
            description: format!("{} base URL override", provider.display_name),
            satisfied: std::env::var(env).is_ok(),
        });
    }

    match provider.name.as_str() {
        "ollama" => {
            reqs.push(ConfigRequirement {
                env_var: "OLLAMA_HOST".to_string(),
                required: false,
                description: "Ollama server URL (default: http://localhost:11434)".to_string(),
                satisfied: true,
            });
            reqs.push(ConfigRequirement {
                env_var: "OLLAMA_MODEL".to_string(),
                required: false,
                description: "Chat model name".to_string(),
                satisfied: true,
            });
        }
        "lmstudio" => {
            reqs.push(ConfigRequirement {
                env_var: "LMSTUDIO_HOST".to_string(),
                required: false,
                description: "LM Studio server URL (default: http://localhost:1234)".to_string(),
                satisfied: true,
            });
            reqs.push(ConfigRequirement {
                env_var: "LMSTUDIO_MODEL".to_string(),
                required: false,
                description: "Chat model name".to_string(),
                satisfied: true,
            });
        }
        "vertexai" => {
            reqs.push(ConfigRequirement {
                env_var: "GOOGLE_CLOUD_PROJECT".to_string(),
                required: true,
                description: "Google Cloud project ID".to_string(),
                satisfied: std::env::var("GOOGLE_CLOUD_PROJECT").is_ok(),
            });
            reqs.push(ConfigRequirement {
                env_var: "GOOGLE_ACCESS_TOKEN".to_string(),
                required: false,
                description: "Access token (or use Application Default Credentials)".to_string(),
                satisfied: std::env::var("GOOGLE_ACCESS_TOKEN").is_ok()
                    || std::env::var("GOOGLE_APPLICATION_CREDENTIALS").is_ok(),
            });
        }
        "azure" if !reqs.iter().any(|r| r.env_var == "AZURE_OPENAI_ENDPOINT") => {
            reqs.push(ConfigRequirement {
                env_var: "AZURE_OPENAI_ENDPOINT".to_string(),
                required: true,
                description: "Azure OpenAI endpoint URL".to_string(),
                satisfied: std::env::var("AZURE_OPENAI_ENDPOINT").is_ok(),
            });
        }
        _ => {}
    }

    reqs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::bundled_models::load_bundled_models_config;

    #[test]
    fn catalog_includes_vertexai_from_models_toml() {
        let config = load_bundled_models_config();
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
        let config = load_bundled_models_config();
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
    fn mock_provider_always_available() {
        let config = load_bundled_models_config();
        let response = build_available_providers_response(&config, "mock", "mock");
        let mock = response
            .llm_providers
            .iter()
            .find(|p| p.id == "mock")
            .expect("mock");
        assert!(mock.available);
        assert_eq!(mock.default_models.embedding_dimension, 768);
    }

    #[test]
    fn providers_sorted_by_priority() {
        let config = load_bundled_models_config();
        let response = build_available_providers_response(&config, "mock", "mock");
        let openai_idx = response
            .llm_providers
            .iter()
            .position(|p| p.id == "openai")
            .unwrap();
        let mock_idx = response
            .llm_providers
            .iter()
            .position(|p| p.id == "mock")
            .unwrap();
        assert!(
            openai_idx < mock_idx,
            "lower priority number should sort first"
        );
    }
}
