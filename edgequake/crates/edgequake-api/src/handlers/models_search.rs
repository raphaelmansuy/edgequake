//! Model search API — capability and name search with live discovery (SPEC-043).

use axum::{
    extract::{Query, State},
    Json,
};
use edgequake_llm::{
    discovery::{CapabilityFilter, ModelSearchQuery},
    ModelCapability,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ApiResult;
use crate::model_catalog::discovery_source_label;
use crate::model_catalog::{active_provider_names, dynamic_discovery_enabled};
use crate::provider_visibility::is_ui_visible_provider_id;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ModelSearchQueryParams {
    pub q: Option<String>,
    pub provider: Option<String>,
    pub requires_vision: Option<bool>,
    pub requires_tools: Option<bool>,
    pub requires_thinking: Option<bool>,
    pub min_context_length: Option<usize>,
    pub max_output_tokens: Option<usize>,
    pub fuzzy: Option<bool>,
    pub limit: Option<usize>,
    /// When false, search static registry only (default: true).
    pub dynamic: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModelSearchHitResponse {
    pub provider: String,
    pub id: String,
    pub name: String,
    pub score: Option<f64>,
    pub context_length: usize,
    pub max_output_tokens: usize,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub supports_thinking: bool,
    pub model_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModelSearchResponse {
    pub hits: Vec<ModelSearchHitResponse>,
    pub total: usize,
    pub dynamic: bool,
}

fn build_capability_filter(params: &ModelSearchQueryParams) -> CapabilityFilter {
    let mut filter = CapabilityFilter::default().excluding_deprecated();
    if params.requires_vision == Some(true) {
        filter = filter.requiring(ModelCapability::Vision);
    }
    if params.requires_tools == Some(true) {
        filter = filter.requiring(ModelCapability::Tools);
    }
    if params.requires_thinking == Some(true) {
        filter = filter.requiring(ModelCapability::Thinking);
    }
    if let Some(min) = params.min_context_length {
        filter = filter.with_min_context_length(min);
    }
    if let Some(max_out) = params.max_output_tokens {
        filter = filter.with_max_output_tokens(max_out);
    }
    if let Some(ref provider) = params.provider {
        filter.provider = Some(provider.clone());
    }
    filter
}

fn discovered_hit(
    model: edgequake_llm::discovery::DiscoveredModel,
    score: Option<f64>,
) -> ModelSearchHitResponse {
    ModelSearchHitResponse {
        provider: model.provider.clone(),
        id: model.id.clone(),
        name: model.name.clone(),
        score,
        context_length: model.context_length,
        max_output_tokens: model.max_output_tokens,
        supports_vision: model.capabilities.supports_vision,
        supports_tools: model.capabilities.supports_function_calling,
        supports_thinking: model.capabilities.supports_thinking,
        model_type: model.model_type.to_string(),
        discovery_source: Some(discovery_source_label(&model.source).to_string()),
        available: Some(model.available),
    }
}

fn dedupe_search_hits(hits: Vec<ModelSearchHitResponse>) -> Vec<ModelSearchHitResponse> {
    let mut seen = std::collections::HashSet::new();
    hits.into_iter()
        .filter(|h| seen.insert((h.provider.to_lowercase(), h.id.clone())))
        .collect()
}

pub async fn search_models(
    State(state): State<AppState>,
    Query(params): Query<ModelSearchQueryParams>,
) -> ApiResult<Json<ModelSearchResponse>> {
    let limit = params.limit.unwrap_or(50).min(200);
    let use_dynamic = params.dynamic.unwrap_or(true) && dynamic_discovery_enabled();
    let catalog = &state.query.model_catalog;
    let models_config = state.query.models_config.as_ref();
    let allowed = active_provider_names();

    let hits: Vec<ModelSearchHitResponse> = if let Some(ref q) = params.q {
        if q.trim().is_empty() {
            catalog
                .find_models(
                    models_config,
                    &allowed,
                    &build_capability_filter(&params),
                    use_dynamic,
                )
                .await
                .into_iter()
                .take(limit)
                .map(|m| discovered_hit(m, None))
                .collect()
        } else {
            let mut query = ModelSearchQuery::new(q.trim());
            if params.fuzzy.unwrap_or(true) {
                query = query.fuzzy(true);
            }
            if let Some(ref provider) = params.provider {
                query = query.with_provider(provider);
            }
            if let Some(min) = params.min_context_length {
                query = query.with_min_context_length(min);
            }
            if let Some(max_out) = params.max_output_tokens {
                query = query.with_max_output_tokens(max_out);
            }
            query = query.with_limit(limit);
            catalog
                .search_models(models_config, &allowed, &query, use_dynamic)
                .await
                .into_iter()
                .map(|hit| discovered_hit(hit.model, Some(hit.score)))
                .collect()
        }
    } else {
        catalog
            .find_models(
                models_config,
                &allowed,
                &build_capability_filter(&params),
                use_dynamic,
            )
            .await
            .into_iter()
            .take(limit)
            .map(|m| discovered_hit(m, None))
            .collect()
    };

    let hits: Vec<_> = dedupe_search_hits(hits)
        .into_iter()
        .filter(|h| is_ui_visible_provider_id(&h.provider))
        .collect();
    let total = hits.len();
    Ok(Json(ModelSearchResponse {
        hits,
        total,
        dynamic: use_dynamic,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_search_hits_keeps_first_provider_id_pair() {
        let hits = vec![
            ModelSearchHitResponse {
                provider: "mistral".into(),
                id: "mistral-large-2512".into(),
                name: "Mistral Large".into(),
                score: None,
                context_length: 128_000,
                max_output_tokens: 8192,
                supports_vision: false,
                supports_tools: true,
                supports_thinking: false,
                model_type: "llm".into(),
                discovery_source: Some("static".into()),
                available: Some(true),
            },
            ModelSearchHitResponse {
                provider: "mistral".into(),
                id: "mistral-large-2512".into(),
                name: "Mistral Large (live)".into(),
                score: None,
                context_length: 131_000,
                max_output_tokens: 8192,
                supports_vision: false,
                supports_tools: true,
                supports_thinking: false,
                model_type: "llm".into(),
                discovery_source: Some("dynamic_api".into()),
                available: Some(true),
            },
        ];
        let deduped = dedupe_search_hits(hits);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].name, "Mistral Large");
    }
}

/// Invalidate discovery caches and force re-fetch on next catalog request.
pub async fn refresh_model_discovery(
    State(state): State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    state.query.model_catalog.invalidate_all_caches().await;
    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "Model discovery cache invalidated"
    })))
}
