//! SPEC-175 W2: adapts the deployment catalog (`/models` source) into the pure facts
//! that `edgequake_core::model_policy` decides on.
//!
//! The probe signal is `ModelResponse.available` (live discovery). Models with
//! `discovery_source = dynamic_api` have no static card and are never eligible.

use edgequake_core::model_policy::{CatalogEntry, ModelFacts};

use crate::error::ApiResult;
use crate::handlers::models_types::{EmbeddingModelItem, LlmModelItem, ModelResponse};
use crate::model_catalog::active_provider_names;
use crate::state::AppState;

/// One catalog model plus the display fields the member picker needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogItem {
    pub entry: CatalogEntry,
    pub display_name: String,
    pub provider_display_name: String,
    pub supports_vision: bool,
    pub supports_streaming: bool,
    /// Set for embedding cards. Chat cards leave this empty.
    pub embedding_dimension: Option<i32>,
}

/// Discovery-only models have no static card, so they are not eligible (EC-175-11).
fn registered(model: &ModelResponse) -> bool {
    model.discovery_source.as_deref() != Some("dynamic_api")
}

pub fn from_llm(item: &LlmModelItem) -> CatalogItem {
    let model = &item.model;
    let vision = model.capabilities.supports_vision;
    CatalogItem {
        entry: CatalogEntry {
            provider: item.provider.to_lowercase(),
            model: model.name.clone(),
            facts: ModelFacts {
                chat: true,
                vision,
                embedding: false,
                registered: registered(model),
                probe: model.available,
            },
        },
        display_name: model.display_name.clone(),
        provider_display_name: item.provider_display_name.clone(),
        supports_vision: vision,
        supports_streaming: model.capabilities.supports_streaming,
        embedding_dimension: None,
    }
}

pub fn from_embedding(item: &EmbeddingModelItem) -> CatalogItem {
    let model = &item.model;
    let dim = crate::model_catalog::resolved_embedding_dimension(
        &model.name,
        model.capabilities.embedding_dimension,
        item.dimension,
    );
    CatalogItem {
        entry: CatalogEntry {
            provider: item.provider.to_lowercase(),
            model: model.name.clone(),
            facts: ModelFacts {
                chat: false,
                vision: model.capabilities.supports_vision,
                embedding: true,
                registered: registered(model),
                probe: model.available,
            },
        },
        display_name: model.display_name.clone(),
        provider_display_name: item.provider_display_name.clone(),
        // For an embedding card this flag means "embeds images" (SPEC-180 08).
        supports_vision: model.capabilities.supports_vision,
        supports_streaming: false,
        embedding_dimension: (dim > 0).then_some(dim as i32),
    }
}

/// Every LLM and embedding model the deployment exposes, in the same order as `/models`.
pub async fn load_catalog(state: &AppState) -> Vec<CatalogItem> {
    let config = &*state.query.models_config;
    let allowed = active_provider_names();
    let llm = state
        .query
        .model_catalog
        .list_llm_models(config, &allowed)
        .await;
    let embedding = state
        .query
        .model_catalog
        .list_embedding_models(config, &allowed)
        .await;
    let mut items: Vec<CatalogItem> = llm
        .iter()
        .map(from_llm)
        .chain(embedding.iter().map(from_embedding))
        .collect();
    // The mock pair is the test double, not a user-selectable model. It joins
    // the catalog only while the test escape hatch is set, so a context-only
    // query that names mock-model can pass the same gate production uses.
    if crate::provider_visibility::mock_provider_allowed() {
        for card in mock_test_cards() {
            let present = items.iter().any(|item| {
                item.entry.provider == card.entry.provider && item.entry.model == card.entry.model
            });
            if !present {
                items.push(card);
            }
        }
    }
    items
}

fn mock_test_cards() -> Vec<CatalogItem> {
    let chat = |model: &str, embedding: bool, dimension: Option<i32>| CatalogItem {
        entry: CatalogEntry {
            provider: "mock".into(),
            model: model.into(),
            facts: ModelFacts {
                chat: !embedding,
                vision: false,
                embedding,
                registered: true,
                probe: None,
            },
        },
        display_name: model.into(),
        provider_display_name: "Mock".into(),
        supports_vision: false,
        supports_streaming: false,
        embedding_dimension: dimension,
    };
    vec![
        chat("mock-model", false, None),
        chat("mock-embedding", true, Some(1536)),
    ]
}

/// Models saved on a fleet connection join the catalog under that connection's slug.
/// A custom provider does not inherit another provider's cards.
///
/// A model the provider reports but an admin has not registered joins as **discovered**:
/// present in the catalog so the UI can show it, but `registered = false` so it is never
/// eligible for a job until someone confirms it (SPEC-179 LAW-179-6). Without this, a
/// refresh stored ids that nothing ever read, and a newly pulled model stayed invisible.
#[cfg(feature = "postgres")]
pub async fn append_connection_models(
    state: &AppState,
    catalog: &mut Vec<CatalogItem>,
) -> ApiResult<()> {
    let Some(pool) = state.optional_pg_pool() else {
        return Ok(());
    };
    let rows: Vec<(String, String, bool, bool, bool, Option<i32>)> = sqlx::query_as(
        "SELECT c.slug, m.model, m.supports_chat, m.supports_vision, m.supports_embedding, \
                m.embedding_dimension \
         FROM connection_models m \
         JOIN provider_connections c ON c.id = m.connection_id \
         WHERE c.tenant_id IS NULL \
         ORDER BY c.slug, m.model",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| crate::error::ApiError::Internal(err.to_string()))?;
    for (slug, model, chat, vision, embedding, dimension) in rows {
        let slug = slug.to_lowercase();
        let dimension = dimension.filter(|value| *value > 0).or_else(|| {
            embedding
                .then(|| edgequake_core::Workspace::known_embedding_dimension(&model))
                .flatten()
                .map(|value| value as i32)
        });
        // A confirmation must UPGRADE an entry the live discovery already added as
        // unregistered. Skipping it here was a real bug: `load_catalog` runs live discovery
        // first, so every refreshed id was already present with `registered = false`, the
        // registration row was dropped, and confirming a model changed nothing.
        if let Some(existing) = catalog
            .iter_mut()
            .find(|item| item.entry.provider == slug && item.entry.model == model)
        {
            existing.entry.facts.registered = true;
            existing.entry.facts.chat = chat;
            existing.entry.facts.vision = vision;
            existing.entry.facts.embedding = embedding;
            existing.entry.facts.probe = Some(true);
            existing.supports_vision = vision;
            existing.supports_streaming = chat;
            if dimension.is_some() {
                existing.embedding_dimension = dimension;
            }
            continue;
        }
        catalog.push(CatalogItem {
            display_name: model.clone(),
            provider_display_name: slug.clone(),
            supports_vision: vision,
            supports_streaming: chat,
            embedding_dimension: dimension,
            entry: CatalogEntry {
                provider: slug,
                model,
                facts: ModelFacts {
                    chat,
                    vision,
                    embedding,
                    registered: true,
                    probe: Some(true),
                },
            },
        });
    }
    append_discovered_models(state, catalog).await?;
    Ok(())
}

/// Models a provider reported on its last refresh that no admin has registered yet.
///
/// These join the catalog with `registered = false`. Two consequences, both intended:
/// 1. `models_for` (the provider card) shows them, so "I pulled a model, why can't I see it"
///    has an answer on screen.
/// 2. `eligible_models` still excludes them (EC-175-11), so extraction never silently starts
///    using a model whose capabilities nobody confirmed.
///
/// A row already in the catalog from a static card or a registration keeps its `registered`
/// flag: the live list cannot downgrade a model an admin already confirmed.
#[cfg(feature = "postgres")]
async fn append_discovered_models(
    state: &AppState,
    catalog: &mut Vec<CatalogItem>,
) -> ApiResult<()> {
    let Some(pool) = state.optional_pg_pool() else {
        return Ok(());
    };
    let rows: Vec<(String, String, bool, bool, bool, Option<i32>)> = sqlx::query_as(
        "SELECT provider_slug, model, supports_chat, supports_vision, \
                supports_embedding, embedding_dimension \
         FROM provider_model_cache \
         ORDER BY provider_slug, model",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| crate::error::ApiError::Internal(err.to_string()))?;
    for (slug, model, chat, vision, embedding, dimension) in rows {
        let slug = slug.to_lowercase();
        // A static card or a registration already speaks for this id.
        if catalog
            .iter()
            .any(|item| item.entry.provider == slug && item.entry.model == model)
        {
            continue;
        }
        let dimension = dimension.filter(|value| *value > 0).or_else(|| {
            embedding
                .then(|| edgequake_core::Workspace::known_embedding_dimension(&model))
                .flatten()
                .map(|value| value as i32)
        });
        catalog.push(CatalogItem {
            display_name: model.clone(),
            provider_display_name: slug.clone(),
            supports_vision: vision,
            supports_streaming: chat,
            embedding_dimension: dimension,
            entry: CatalogEntry {
                provider: slug,
                model,
                facts: ModelFacts {
                    // The cache stores chat/vision/embedding; a refreshed Ollama tag is chat
                    // unless it reported the embedding capability.
                    chat,
                    vision,
                    embedding,
                    registered: false,
                    probe: Some(true),
                },
            },
        });
    }
    Ok(())
}

#[cfg(test)]
mod registration_eligibility_tests {
    use std::collections::BTreeSet;

    use edgequake_core::model_policy::{eligible_models, CatalogEntry, ModelFacts};
    use edgequake_core::usage_resolution::Usage;

    fn entry(provider: &str, model: &str, registered: bool) -> CatalogEntry {
        CatalogEntry {
            provider: provider.to_string(),
            model: model.to_string(),
            facts: ModelFacts {
                chat: true,
                vision: false,
                embedding: false,
                registered,
                probe: Some(true),
            },
        }
    }

    /// Confirmed `connection_models` rows are registered and eligible.
    /// A discovery-only id (`registered = false`) stays off the choice list.
    #[test]
    fn confirmed_models_are_eligible_for_each_refreshable_provider() {
        let catalog = vec![
            entry("ollama", "gemma4:latest", true),
            entry("openai", "gpt-5-nano", true),
            entry("openrouter", "live-only", false),
        ];
        let offered = BTreeSet::from([
            "ollama".to_string(),
            "openai".to_string(),
            "openrouter".to_string(),
        ]);
        let eligible = eligible_models(Usage::Query, &catalog, &[], &[], Some(&offered));
        let pairs: Vec<_> = eligible
            .iter()
            .map(|item| (item.provider.as_str(), item.model.as_str()))
            .collect();
        assert!(pairs.contains(&("ollama", "gemma4:latest")));
        assert!(pairs.contains(&("openai", "gpt-5-nano")));
        assert!(!pairs.iter().any(|(_, model)| *model == "live-only"));
    }
}

#[cfg(not(feature = "postgres"))]
pub async fn append_connection_models(
    _state: &AppState,
    _catalog: &mut Vec<CatalogItem>,
) -> ApiResult<()> {
    Ok(())
}
