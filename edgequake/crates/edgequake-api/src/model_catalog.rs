//! Hybrid model catalog — merges models.toml with live provider discovery (SPEC-043).
//!
//! Single Responsibility: aggregate static config + dynamic discovery into API DTOs.
//! Dependency Inversion: depends on `ModelDiscoveryService` trait abstraction from edgequake-llm.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use edgequake_llm::discovery::{
    search_models as rank_search_models, DiscoveredModel, DiscoverySource, ModelDiscoveryService,
    ModelSearchMatch, ModelSearchQuery,
};
use edgequake_llm::model_config::{ModelCapabilities, ModelType, ModelsConfig, ProviderConfig};

use crate::handlers::models_types::{
    EmbeddingModelItem, LlmModelItem, ModelCapabilitiesResponse, ModelCostResponse, ModelResponse,
};

/// Whether live model discovery is enabled (default: true).
pub fn dynamic_discovery_enabled() -> bool {
    !matches!(
        std::env::var("EDGEQUAKE_DYNAMIC_MODELS").ok().as_deref(),
        Some("0") | Some("false") | Some("no") | Some("off")
    )
}

/// Determine which provider names are visible in the current deployment.
pub fn active_provider_names() -> Option<HashSet<String>> {
    let env_val = std::env::var("EDGEQUAKE_ALLOWED_PROVIDERS").unwrap_or_default();
    match env_val.trim() {
        "" | "*" => None,
        list => Some(list.split(',').map(|s| s.trim().to_lowercase()).collect()),
    }
}

/// Filter providers to enabled + optional allowlist (includes mock — use [`crate::provider_visibility::filter_ui_providers`] for UI).
pub fn filter_providers<'a>(
    providers: &'a [ProviderConfig],
    allowed: &Option<HashSet<String>>,
) -> Vec<&'a ProviderConfig> {
    match allowed {
        None => providers.iter().filter(|p| p.enabled).collect(),
        Some(names) => providers
            .iter()
            .filter(|p| p.enabled && names.contains(&p.name.to_lowercase()))
            .collect(),
    }
}

fn provider_visible(
    provider_id: &str,
    config: &ModelsConfig,
    allowed: &Option<HashSet<String>>,
) -> bool {
    let id = provider_id.to_lowercase();
    if !crate::provider_visibility::is_ui_visible_provider_id(&id) {
        return false;
    }
    if let Some(names) = allowed {
        if !names.contains(&id) {
            return false;
        }
    }
    config.get_provider(&id).map(|p| p.enabled).unwrap_or(false)
}

fn provider_display_name(config: &ModelsConfig, provider_id: &str) -> String {
    config
        .get_provider(provider_id)
        .map(|p| p.display_name.clone())
        .unwrap_or_else(|| {
            let mut chars = provider_id.chars();
            chars
                .next()
                .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_else(|| provider_id.to_string())
        })
}

/// Convert a static `ModelCard` to API `ModelResponse`.
pub fn model_card_to_response(card: &edgequake_llm::ModelCard) -> ModelResponse {
    ModelResponse {
        name: card.name.clone(),
        display_name: card.display_name.clone(),
        model_type: card.model_type.to_string(),
        description: card.description.clone(),
        deprecated: card.deprecated,
        replacement: card.replacement.clone(),
        capabilities: ModelCapabilitiesResponse {
            context_length: card.capabilities.context_length,
            max_output_tokens: card.capabilities.max_output_tokens,
            supports_vision: card.capabilities.supports_vision,
            supports_function_calling: card.capabilities.supports_function_calling,
            supports_json_mode: card.capabilities.supports_json_mode,
            supports_streaming: card.capabilities.supports_streaming,
            supports_system_message: card.capabilities.supports_system_message,
            embedding_dimension: card.capabilities.embedding_dimension,
        },
        cost: ModelCostResponse {
            input_per_1k: card.cost.input_per_1k,
            output_per_1k: card.cost.output_per_1k,
            embedding_per_1k: card.cost.embedding_per_1k,
            image_per_unit: card.cost.image_per_unit,
        },
        tags: card.tags.clone(),
        discovery_source: Some("user_config".to_string()),
        available: Some(true),
    }
}

pub fn discovery_source_label(source: &edgequake_llm::discovery::DiscoverySource) -> &'static str {
    match source {
        DiscoverySource::DynamicApi => "dynamic_api",
        DiscoverySource::StaticRegistry => "static_registry",
        DiscoverySource::Hybrid => "hybrid",
        DiscoverySource::UserConfig => "user_config",
        DiscoverySource::Unknown => "unknown",
    }
}

/// Convert a `DiscoveredModel` to API `ModelResponse`.
pub fn discovered_to_response(model: &DiscoveredModel) -> ModelResponse {
    let input_per_1k = model.cost_per_m_input.map(|c| c / 1000.0).unwrap_or(0.0);
    let output_per_1k = model.cost_per_m_output.map(|c| c / 1000.0).unwrap_or(0.0);

    ModelResponse {
        name: model.id.clone(),
        display_name: if model.name.is_empty() {
            model.id.clone()
        } else {
            model.name.clone()
        },
        model_type: model.model_type.to_string(),
        description: String::new(),
        deprecated: model.deprecated,
        replacement: None,
        capabilities: ModelCapabilitiesResponse {
            context_length: model.context_length,
            max_output_tokens: model.max_output_tokens,
            supports_vision: model.capabilities.supports_vision,
            supports_function_calling: model.capabilities.supports_function_calling,
            supports_json_mode: model.capabilities.supports_json_mode,
            supports_streaming: model.capabilities.supports_streaming,
            supports_system_message: model.capabilities.supports_system_message,
            embedding_dimension: model.capabilities.embedding_dimension,
        },
        cost: ModelCostResponse {
            input_per_1k,
            output_per_1k,
            embedding_per_1k: 0.0,
            image_per_unit: 0.0,
        },
        tags: model.tags.clone(),
        discovery_source: Some(discovery_source_label(&model.source).to_string()),
        available: Some(model.available),
    }
}

fn model_key(provider: &str, name: &str) -> (String, String) {
    (provider.to_lowercase(), name.to_string())
}

fn merge_model_response(static_model: &ModelResponse, dynamic: &DiscoveredModel) -> ModelResponse {
    let mut merged = discovered_to_response(dynamic);
    if static_model.description.is_empty() {
        // keep dynamic empty
    } else {
        merged.description = static_model.description.clone();
    }
    if static_model.display_name != static_model.name && merged.display_name == merged.name {
        merged.display_name = static_model.display_name.clone();
    }
    if !static_model.tags.is_empty() && merged.tags.is_empty() {
        merged.tags = static_model.tags.clone();
    }
    merged.discovery_source = Some(if dynamic.source == DiscoverySource::DynamicApi {
        "hybrid".to_string()
    } else {
        discovery_source_label(&dynamic.source).to_string()
    });
    merged
}

fn parse_model_type_label(label: &str) -> ModelType {
    match label.to_lowercase().as_str() {
        "embedding" => ModelType::Embedding,
        "multimodal" => ModelType::Multimodal,
        _ => ModelType::Llm,
    }
}

fn llm_item_to_discovered(item: &LlmModelItem) -> DiscoveredModel {
    let caps = &item.model.capabilities;
    DiscoveredModel {
        id: item.model.name.clone(),
        name: item.model.display_name.clone(),
        provider: item.provider.clone(),
        context_length: caps.context_length,
        max_output_tokens: caps.max_output_tokens,
        capabilities: ModelCapabilities {
            context_length: caps.context_length,
            max_output_tokens: caps.max_output_tokens,
            supports_vision: caps.supports_vision,
            supports_function_calling: caps.supports_function_calling,
            supports_json_mode: caps.supports_json_mode,
            supports_streaming: caps.supports_streaming,
            supports_system_message: caps.supports_system_message,
            embedding_dimension: caps.embedding_dimension,
            ..Default::default()
        },
        source: DiscoverySource::UserConfig,
        available: item.model.available.unwrap_or(true),
        deprecated: item.model.deprecated,
        model_type: parse_model_type_label(&item.model.model_type),
        ..Default::default()
    }
}

fn embedding_item_to_discovered(item: &EmbeddingModelItem) -> DiscoveredModel {
    let caps = &item.model.capabilities;
    DiscoveredModel {
        id: item.model.name.clone(),
        name: item.model.display_name.clone(),
        provider: item.provider.clone(),
        context_length: caps.context_length,
        max_output_tokens: caps.max_output_tokens,
        capabilities: ModelCapabilities {
            context_length: caps.context_length,
            max_output_tokens: caps.max_output_tokens,
            supports_vision: caps.supports_vision,
            supports_function_calling: caps.supports_function_calling,
            supports_json_mode: caps.supports_json_mode,
            supports_streaming: caps.supports_streaming,
            supports_system_message: caps.supports_system_message,
            embedding_dimension: item.dimension,
            ..Default::default()
        },
        source: DiscoverySource::UserConfig,
        available: item.model.available.unwrap_or(true),
        deprecated: item.model.deprecated,
        model_type: ModelType::Embedding,
        ..Default::default()
    }
}

/// Hybrid catalog service wrapping edgequake-llm discovery.
#[derive(Clone)]
pub struct ModelCatalog {
    discovery: Arc<ModelDiscoveryService>,
}

impl Default for ModelCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelCatalog {
    pub fn new() -> Self {
        Self {
            discovery: Arc::new(ModelDiscoveryService::new()),
        }
    }

    pub fn with_discovery(discovery: Arc<ModelDiscoveryService>) -> Self {
        Self { discovery }
    }

    pub fn discovery(&self) -> &Arc<ModelDiscoveryService> {
        &self.discovery
    }

    pub async fn invalidate_all_caches(&self) {
        self.discovery.invalidate_all_caches().await;
    }

    pub async fn invalidate_provider_cache(&self, provider_id: &str) {
        self.discovery.invalidate_cache(provider_id).await;
    }

    /// Collect static LLM/multimodal models from models.toml.
    pub fn static_llm_models(
        config: &ModelsConfig,
        allowed: &Option<HashSet<String>>,
    ) -> Vec<LlmModelItem> {
        crate::provider_visibility::filter_ui_providers(&config.providers, allowed)
            .into_iter()
            .flat_map(|provider| {
                provider
                    .models
                    .iter()
                    .filter(|m| matches!(m.model_type, ModelType::Llm | ModelType::Multimodal))
                    .map(|model| LlmModelItem {
                        provider: provider.name.clone(),
                        provider_display_name: provider.display_name.clone(),
                        model: model_card_to_response(model),
                    })
            })
            .collect()
    }

    /// Collect static embedding models (ModelType::Embedding only).
    pub fn static_embedding_models(
        config: &ModelsConfig,
        allowed: &Option<HashSet<String>>,
    ) -> Vec<EmbeddingModelItem> {
        crate::provider_visibility::filter_ui_providers(&config.providers, allowed)
            .into_iter()
            .flat_map(|provider| {
                provider
                    .models
                    .iter()
                    .filter(|m| matches!(m.model_type, ModelType::Embedding))
                    .map(|model| EmbeddingModelItem {
                        provider: provider.name.clone(),
                        provider_display_name: provider.display_name.clone(),
                        dimension: model.capabilities.embedding_dimension,
                        model: model_card_to_response(model),
                    })
            })
            .collect()
    }

    async fn fetch_discovered(
        &self,
        config: &ModelsConfig,
        allowed: &Option<HashSet<String>>,
    ) -> Vec<DiscoveredModel> {
        if !dynamic_discovery_enabled() {
            return Vec::new();
        }
        match self.discovery.discover_all().await {
            Ok(models) => models
                .into_iter()
                .filter(|m| provider_visible(&m.provider, config, allowed))
                .collect(),
            Err(e) => {
                tracing::warn!(error = %e, "Dynamic model discovery failed; serving static catalog only");
                Vec::new()
            }
        }
    }

    /// models.toml entries as discovered models (for search/find fallback).
    fn toml_discovered_models(
        config: &ModelsConfig,
        allowed: &Option<HashSet<String>>,
    ) -> Vec<DiscoveredModel> {
        let mut out = Vec::new();
        for item in Self::static_llm_models(config, allowed) {
            out.push(llm_item_to_discovered(&item));
        }
        for item in Self::static_embedding_models(config, allowed) {
            out.push(embedding_item_to_discovered(&item));
        }
        out
    }

    fn is_llm_type(model_type: &ModelType) -> bool {
        matches!(model_type, ModelType::Llm | ModelType::Multimodal)
    }

    fn is_embedding_type(model_type: &ModelType) -> bool {
        matches!(model_type, ModelType::Embedding)
    }

    /// Merge static + dynamic LLM models (dynamic wins on capability conflicts).
    pub async fn list_llm_models(
        &self,
        config: &ModelsConfig,
        allowed: &Option<HashSet<String>>,
    ) -> Vec<LlmModelItem> {
        let static_items = Self::static_llm_models(config, allowed);
        let discovered = self.fetch_discovered(config, allowed).await;

        let mut merged: HashMap<(String, String), LlmModelItem> = HashMap::new();
        for item in static_items {
            let key = model_key(&item.provider, &item.model.name);
            merged.insert(key, item);
        }

        for model in discovered
            .into_iter()
            .filter(|m| Self::is_llm_type(&m.model_type))
        {
            let provider = model.provider.to_lowercase();
            let key = model_key(&provider, &model.id);
            if let Some(existing) = merged.get(&key) {
                let updated = merge_model_response(&existing.model, &model);
                merged.insert(
                    key,
                    LlmModelItem {
                        provider: existing.provider.clone(),
                        provider_display_name: existing.provider_display_name.clone(),
                        model: updated,
                    },
                );
            } else {
                merged.insert(
                    key,
                    LlmModelItem {
                        provider: provider.clone(),
                        provider_display_name: provider_display_name(config, &provider),
                        model: discovered_to_response(&model),
                    },
                );
            }
        }

        let mut items: Vec<_> = merged.into_values().collect();
        items.sort_by(|a, b| {
            a.provider
                .cmp(&b.provider)
                .then(a.model.display_name.cmp(&b.model.display_name))
        });
        items
    }

    /// Merge static + dynamic embedding models.
    pub async fn list_embedding_models(
        &self,
        config: &ModelsConfig,
        allowed: &Option<HashSet<String>>,
    ) -> Vec<EmbeddingModelItem> {
        let static_items = Self::static_embedding_models(config, allowed);
        let discovered = self.fetch_discovered(config, allowed).await;

        let mut merged: HashMap<(String, String), EmbeddingModelItem> = HashMap::new();
        for item in static_items {
            let key = model_key(&item.provider, &item.model.name);
            merged.insert(key, item);
        }

        for model in discovered
            .into_iter()
            .filter(|m| Self::is_embedding_type(&m.model_type))
        {
            let provider = model.provider.to_lowercase();
            let key = model_key(&provider, &model.id);
            let dimension = model.capabilities.embedding_dimension;
            if let Some(existing) = merged.get(&key) {
                let updated = merge_model_response(&existing.model, &model);
                merged.insert(
                    key,
                    EmbeddingModelItem {
                        provider: existing.provider.clone(),
                        provider_display_name: existing.provider_display_name.clone(),
                        dimension,
                        model: updated,
                    },
                );
            } else {
                merged.insert(
                    key,
                    EmbeddingModelItem {
                        provider: provider.clone(),
                        provider_display_name: provider_display_name(config, &provider),
                        dimension,
                        model: discovered_to_response(&model),
                    },
                );
            }
        }

        let mut items: Vec<_> = merged.into_values().collect();
        items.sort_by(|a, b| {
            a.provider
                .cmp(&b.provider)
                .then(a.model.display_name.cmp(&b.model.display_name))
        });
        items
    }

    /// Search models via live discovery (falls back to models.toml on failure).
    pub async fn search_models(
        &self,
        config: &ModelsConfig,
        allowed: &Option<HashSet<String>>,
        query: &ModelSearchQuery,
        use_dynamic: bool,
    ) -> Vec<ModelSearchMatch> {
        if use_dynamic && dynamic_discovery_enabled() {
            match self.discovery.search_models(query).await {
                Ok(hits) if !hits.is_empty() => return hits,
                Ok(_) => {}
                Err(e) => {
                    tracing::debug!(error = %e, "Live model search failed; falling back to static catalog");
                }
            }

            let discovered = self.fetch_discovered(config, allowed).await;
            let ranked = rank_search_models(discovered, query);
            if !ranked.is_empty() {
                return ranked;
            }
        }

        let static_hits = edgequake_llm::discovery::search_static_models(query);
        if !static_hits.is_empty() {
            return static_hits;
        }

        rank_search_models(Self::toml_discovered_models(config, allowed), query)
    }

    /// Capability-filtered listing via live discovery (falls back to models.toml).
    pub async fn find_models(
        &self,
        config: &ModelsConfig,
        allowed: &Option<HashSet<String>>,
        filter: &edgequake_llm::discovery::CapabilityFilter,
        use_dynamic: bool,
    ) -> Vec<DiscoveredModel> {
        if use_dynamic && dynamic_discovery_enabled() {
            match self.discovery.find_models(filter).await {
                Ok(models) if !models.is_empty() => return models,
                Ok(_) => {}
                Err(e) => {
                    tracing::debug!(error = %e, "Live model find failed; falling back to static catalog");
                }
            }

            let discovered = self
                .fetch_discovered(config, allowed)
                .await
                .into_iter()
                .filter(|m| filter.matches(m))
                .collect::<Vec<_>>();
            if !discovered.is_empty() {
                return discovered;
            }
        }

        let static_models = edgequake_llm::discovery::find_static_models(filter);
        if !static_models.is_empty() {
            return static_models;
        }

        Self::toml_discovered_models(config, allowed)
            .into_iter()
            .filter(|m| filter.matches(m))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::ModelCapabilities;

    #[test]
    fn dynamic_discovery_env_toggle() {
        std::env::set_var("EDGEQUAKE_DYNAMIC_MODELS", "0");
        assert!(!dynamic_discovery_enabled());
        std::env::remove_var("EDGEQUAKE_DYNAMIC_MODELS");
        assert!(dynamic_discovery_enabled());
    }

    #[test]
    fn upstream_vertexai_static_search_returns_tagged_models() {
        use edgequake_llm::discovery::{find_static_models, CapabilityFilter};

        let filter = CapabilityFilter::default().with_provider("vertexai");
        let models = find_static_models(&filter);
        assert!(
            !models.is_empty(),
            "edgequake-llm 0.10.1+ vertexai static registry"
        );
        assert!(models.iter().all(|m| m.provider == "vertexai"));
    }

    #[test]
    fn merge_model_response_prefers_dynamic_capabilities() {
        let static_model = ModelResponse {
            name: "llama3".into(),
            display_name: "Llama 3".into(),
            model_type: "llm".into(),
            description: "Static desc".into(),
            deprecated: false,
            replacement: None,
            capabilities: ModelCapabilitiesResponse {
                context_length: 8192,
                max_output_tokens: 2048,
                supports_vision: false,
                supports_function_calling: false,
                supports_json_mode: false,
                supports_streaming: true,
                supports_system_message: true,
                embedding_dimension: 0,
            },
            cost: ModelCostResponse {
                input_per_1k: 0.0,
                output_per_1k: 0.0,
                embedding_per_1k: 0.0,
                image_per_unit: 0.0,
            },
            tags: vec!["fast".into()],
            discovery_source: Some("user_config".into()),
            available: Some(true),
        };

        let dynamic = DiscoveredModel {
            id: "llama3".into(),
            name: "llama3:latest".into(),
            provider: "ollama".into(),
            context_length: 128_000,
            max_output_tokens: 4096,
            capabilities: ModelCapabilities {
                supports_vision: true,
                supports_function_calling: true,
                ..Default::default()
            },
            source: DiscoverySource::DynamicApi,
            available: true,
            model_type: ModelType::Llm,
            ..Default::default()
        };

        let merged = merge_model_response(&static_model, &dynamic);
        assert_eq!(merged.capabilities.context_length, 128_000);
        assert!(merged.capabilities.supports_vision);
        assert_eq!(merged.description, "Static desc");
        assert_eq!(merged.discovery_source.as_deref(), Some("hybrid"));
    }

    #[test]
    fn provider_visible_respects_allowlist() {
        let config = ModelsConfig::builtin_defaults();
        let mut allowed = HashSet::new();
        allowed.insert("openai".into());
        assert!(provider_visible("openai", &config, &Some(allowed.clone())));
        assert!(!provider_visible("ollama", &config, &Some(allowed)));
    }
}
