//! Settings-related API handlers.
//!
//! @implements SPEC-032: Ollama/LM Studio provider support - Status API
//! @iteration OODA Loop #5 - Phase 5E.3 + OODA 12

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    error::ApiError,
    provider_types::{AvailableProvidersResponse, ProviderStatusResponse},
    safety_limits::is_model_provider_mismatch,
    state::AppState,
};

// ── Config Explainability Types ───────────────────────────────────────────

/// A single resolved configuration level in the priority chain.
///
/// Levels are returned in ascending priority order so that the UI can walk
/// from the lowest-priority source ("compiled default") to the highest
/// ("workspace DB") and clearly show the user *which* value wins and *why*.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigLevel {
    /// Machine-readable level name (e.g. "compiled_default", "env_var", "workspace_db").
    pub level: String,
    /// Human-readable label shown in the UI.
    pub label: String,
    /// The resolved provider at this level, or `null` if not set at this level.
    pub provider: Option<String>,
    /// The resolved model at this level, or `null` if not set at this level.
    pub model: Option<String>,
    /// Whether this level is the one whose value wins (active level).
    pub active: bool,
    /// Optional explanation/note for the user.
    pub note: Option<String>,
    /// The exact environment variable(s) or DB field that provided this value.
    pub source: Option<String>,
}

/// Config area response for one config domain (llm / embedding / vision).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigAreaResponse {
    /// Ordered resolution chain (lowest → highest priority).
    pub levels: Vec<ConfigLevel>,
    /// Final effective provider (the active level's provider).
    pub effective_provider: String,
    /// Final effective model (the active level's model).
    pub effective_model: String,
    /// True when the effective model is incompatible with the effective provider
    /// (e.g., "gpt-4.1-nano" configured but provider is "ollama").
    pub has_mismatch: bool,
    /// Human-readable mismatch description when `has_mismatch` is true.
    pub mismatch_description: Option<String>,
}

/// Full effective configuration response.
///
/// `GET /api/v1/config/effective`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectiveConfigResponse {
    /// LLM chat/extraction configuration chain.
    pub llm: ConfigAreaResponse,
    /// Embedding configuration chain.
    pub embedding: ConfigAreaResponse,
    /// Vision/PDF configuration chain.
    pub vision: ConfigAreaResponse,
    /// Priority rule explanation shown to the user.
    pub priority_rule: String,
}

// ── Resolution helpers ────────────────────────────────────────────────────

fn non_empty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

/// Build the full LLM config resolution chain, returning the levels list
/// (lowest → highest priority), the winning provider, and the winning model.
fn resolve_llm_chain() -> (Vec<ConfigLevel>, String, String) {
    use edgequake_core::Workspace;

    // ── Level 0: compiled default ──
    let compiled_provider = "ollama".to_string();
    let compiled_model = Workspace::default_model_for_provider(&compiled_provider);

    // ── Level 1: primary canonical env vars ──
    let env_primary_provider = non_empty("EDGEQUAKE_DEFAULT_LLM_PROVIDER");
    let env_primary_model = non_empty("EDGEQUAKE_DEFAULT_LLM_MODEL");

    // ── Level 2: secondary canonical env vars ──
    let env_secondary_provider = non_empty("EDGEQUAKE_LLM_PROVIDER");
    let env_secondary_model = non_empty("EDGEQUAKE_LLM_MODEL");

    // ── Level 3: legacy aliases ──
    let env_alias_provider =
        edgequake_core::env::first_non_empty_env_var(&["MODEL_PROVIDER", "CHAT_PROVIDER"]);
    let env_alias_model =
        edgequake_core::env::first_non_empty_env_var(&["CHAT_MODEL", "LLM_MODEL"]);

    // ── Effective values (mirrors Workspace::default_llm_config) ──
    let effective_provider = env_primary_provider
        .clone()
        .or_else(|| env_secondary_provider.clone())
        .or_else(|| env_alias_provider.clone())
        .unwrap_or_else(|| compiled_provider.clone());

    let effective_model = env_primary_model
        .clone()
        .or_else(|| env_secondary_model.clone())
        .or_else(|| env_alias_model.clone())
        .unwrap_or_else(|| Workspace::default_model_for_provider(&effective_provider));

    // Determine which level is active
    let active_level = if env_primary_provider.is_some() || env_primary_model.is_some() {
        "env_primary"
    } else if env_secondary_provider.is_some() || env_secondary_model.is_some() {
        "env_secondary"
    } else if env_alias_provider.is_some() || env_alias_model.is_some() {
        "env_alias"
    } else {
        "compiled_default"
    };

    let levels = vec![
        ConfigLevel {
            level: "compiled_default".to_string(),
            label: "Compiled Default".to_string(),
            provider: Some(compiled_provider.clone()),
            model: Some(compiled_model.clone()),
            active: active_level == "compiled_default",
            note: Some("Built-in fallback when no env vars are set.".to_string()),
            source: Some("binary constant".to_string()),
        },
        ConfigLevel {
            level: "env_alias".to_string(),
            label: "Env: Legacy Aliases".to_string(),
            provider: env_alias_provider.clone(),
            model: env_alias_model.clone(),
            active: active_level == "env_alias",
            note: Some(
                "Compatibility aliases: MODEL_PROVIDER / CHAT_PROVIDER / CHAT_MODEL / LLM_MODEL"
                    .to_string(),
            ),
            source: Some("MODEL_PROVIDER | CHAT_PROVIDER | CHAT_MODEL | LLM_MODEL".to_string()),
        },
        ConfigLevel {
            level: "env_secondary".to_string(),
            label: "Env: EDGEQUAKE_LLM_*".to_string(),
            provider: env_secondary_provider.clone(),
            model: env_secondary_model.clone(),
            active: active_level == "env_secondary",
            note: Some("Single-environment deployment variables.".to_string()),
            source: Some("EDGEQUAKE_LLM_PROVIDER | EDGEQUAKE_LLM_MODEL".to_string()),
        },
        ConfigLevel {
            level: "env_primary".to_string(),
            label: "Env: EDGEQUAKE_DEFAULT_LLM_*".to_string(),
            provider: env_primary_provider.clone(),
            model: env_primary_model.clone(),
            active: active_level == "env_primary",
            note: Some("Recommended primary variables. Overrides all other env vars.".to_string()),
            source: Some(
                "EDGEQUAKE_DEFAULT_LLM_PROVIDER | EDGEQUAKE_DEFAULT_LLM_MODEL".to_string(),
            ),
        },
    ];

    (levels, effective_provider, effective_model)
}

/// Build the full embedding config resolution chain.
fn resolve_embedding_chain() -> (Vec<ConfigLevel>, String, String) {
    use edgequake_core::Workspace;

    let compiled_provider = "ollama".to_string();
    let compiled_model = Workspace::default_embedding_model_for_provider(&compiled_provider);

    let env_primary_provider = non_empty("EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER");
    let env_primary_model = non_empty("EDGEQUAKE_DEFAULT_EMBEDDING_MODEL");
    let env_secondary_provider = non_empty("EDGEQUAKE_EMBEDDING_PROVIDER");
    let env_secondary_model = non_empty("EDGEQUAKE_EMBEDDING_MODEL");

    let effective_provider = env_primary_provider
        .clone()
        .or_else(|| env_secondary_provider.clone())
        .unwrap_or_else(|| compiled_provider.clone());

    let effective_model = env_primary_model
        .clone()
        .or_else(|| env_secondary_model.clone())
        .unwrap_or_else(|| Workspace::default_embedding_model_for_provider(&effective_provider));

    let active_level = if env_primary_provider.is_some() || env_primary_model.is_some() {
        "env_primary"
    } else if env_secondary_provider.is_some() || env_secondary_model.is_some() {
        "env_secondary"
    } else {
        "compiled_default"
    };

    let levels = vec![
        ConfigLevel {
            level: "compiled_default".to_string(),
            label: "Compiled Default".to_string(),
            provider: Some(compiled_provider),
            model: Some(compiled_model),
            active: active_level == "compiled_default",
            note: Some("Built-in embedding fallback.".to_string()),
            source: Some("binary constant".to_string()),
        },
        ConfigLevel {
            level: "env_secondary".to_string(),
            label: "Env: EDGEQUAKE_EMBEDDING_*".to_string(),
            provider: env_secondary_provider,
            model: env_secondary_model,
            active: active_level == "env_secondary",
            note: None,
            source: Some("EDGEQUAKE_EMBEDDING_PROVIDER | EDGEQUAKE_EMBEDDING_MODEL".to_string()),
        },
        ConfigLevel {
            level: "env_primary".to_string(),
            label: "Env: EDGEQUAKE_DEFAULT_EMBEDDING_*".to_string(),
            provider: env_primary_provider,
            model: env_primary_model,
            active: active_level == "env_primary",
            note: Some("Recommended primary variables. Overrides all other env vars.".to_string()),
            source: Some(
                "EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER | EDGEQUAKE_DEFAULT_EMBEDDING_MODEL"
                    .to_string(),
            ),
        },
    ];

    (levels, effective_provider, effective_model)
}

/// Build the vision LLM config resolution chain.
///
/// Vision falls back to the LLM provider/model when no vision-specific vars
/// are set, which mirrors the logic in `default_vision_model_for_provider`.
fn resolve_vision_chain() -> (Vec<ConfigLevel>, String, String) {
    use edgequake_core::Workspace;

    let (llm_levels, llm_effective_provider, llm_effective_model) = resolve_llm_chain();
    let compiled_provider = llm_effective_provider.clone();
    let compiled_model = Workspace::default_model_for_provider(&compiled_provider);

    // Read ALL vision-related env vars (matching types.rs resolution order)
    let env_vision_provider = non_empty("EDGEQUAKE_VISION_PROVIDER")
        .or_else(|| non_empty("EDGEQUAKE_VISION_LLM_PROVIDER"));
    let env_vision_model =
        non_empty("EDGEQUAKE_VISION_MODEL").or_else(|| non_empty("EDGEQUAKE_VISION_LLM_MODEL"));

    // Track which specific env var was the source for accurate diagnostics
    let vision_provider_source = if non_empty("EDGEQUAKE_VISION_PROVIDER").is_some() {
        "EDGEQUAKE_VISION_PROVIDER"
    } else if non_empty("EDGEQUAKE_VISION_LLM_PROVIDER").is_some() {
        "EDGEQUAKE_VISION_LLM_PROVIDER"
    } else {
        "(inherited from LLM)"
    };
    let vision_model_source = if non_empty("EDGEQUAKE_VISION_MODEL").is_some() {
        "EDGEQUAKE_VISION_MODEL"
    } else if non_empty("EDGEQUAKE_VISION_LLM_MODEL").is_some() {
        "EDGEQUAKE_VISION_LLM_MODEL"
    } else {
        "(inherited from LLM)"
    };

    let effective_provider = env_vision_provider
        .clone()
        .unwrap_or_else(|| llm_effective_provider.clone());

    let effective_model = env_vision_model
        .clone()
        .unwrap_or_else(|| llm_effective_model.clone());

    let active_level: String = if env_vision_provider.is_some() || env_vision_model.is_some() {
        "env_vision".to_string()
    } else {
        llm_levels
            .iter()
            .find(|l| l.active)
            .map(|l| l.level.clone())
            .unwrap_or_else(|| "compiled_default".to_string())
    };

    let llm_fallback_note = format!(
        "Inherited from LLM config (provider={}, model={}). Set EDGEQUAKE_VISION_PROVIDER / EDGEQUAKE_VISION_MODEL to override.",
        llm_effective_provider, llm_effective_model
    );

    let levels = vec![
        ConfigLevel {
            level: "compiled_default".to_string(),
            label: "Compiled Default (via LLM)".to_string(),
            provider: Some(compiled_provider),
            model: Some(compiled_model),
            active: active_level == "compiled_default",
            note: Some(llm_fallback_note.clone()),
            source: Some("binary constant (LLM default)".to_string()),
        },
        ConfigLevel {
            level: "env_llm_inherit".to_string(),
            label: "Env: Inherited from LLM".to_string(),
            provider: Some(llm_effective_provider.clone()),
            model: Some(llm_effective_model.clone()),
            active: active_level != "env_vision" && active_level != "compiled_default",
            note: Some(llm_fallback_note),
            source: Some("EDGEQUAKE_DEFAULT_LLM_* | EDGEQUAKE_LLM_*".to_string()),
        },
        ConfigLevel {
            level: "env_vision".to_string(),
            label: "Env: Vision Override".to_string(),
            provider: env_vision_provider,
            model: env_vision_model,
            active: active_level == "env_vision",
            note: Some(
                "Dedicated vision override. Takes priority over all LLM settings.".to_string(),
            ),
            source: Some(format!(
                "{} | {}",
                vision_provider_source, vision_model_source
            )),
        },
    ];

    (levels, effective_provider, effective_model)
}

fn build_config_area(
    levels: Vec<ConfigLevel>,
    effective_provider: String,
    effective_model: String,
) -> ConfigAreaResponse {
    let has_mismatch = is_model_provider_mismatch(&effective_provider, &effective_model);

    // Find which env var set the mismatched value to give targeted remediation.
    let mismatch_description = if has_mismatch {
        let source_var = levels
            .iter()
            .find(|l| l.active)
            .and_then(|l| l.source.as_deref())
            .unwrap_or("unknown");
        Some(format!(
            "Model '{}' is not compatible with provider '{}'. \
             This causes timeouts or 404 errors. \
             \n\nHow to fix:\n\
             • Option A: Remove or unset the env var that set this model (source: {}).\n\
             • Option B: Set EDGEQUAKE_VISION_PROVIDER to match the model (e.g. 'openai' for gpt-* models).\n\
             • Option C: Set EDGEQUAKE_VISION_MODEL to a model your provider supports (e.g. 'gemma4:latest' for Ollama).\n\
             \nThe backend will auto-correct at runtime, but fixing the source prevents confusion.",
            effective_model, effective_provider, source_var
        ))
    } else {
        None
    };

    ConfigAreaResponse {
        levels,
        effective_provider,
        effective_model,
        has_mismatch,
        mismatch_description,
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────

/// Get current provider status
///
/// Returns detailed information about the currently active LLM provider,
/// embedding provider, and vector storage configuration.
pub async fn get_provider_status(
    State(app_state): State<AppState>,
) -> Result<Json<ProviderStatusResponse>, ApiError> {
    // Create status response from current AppState
    let status = ProviderStatusResponse::from_app_state(&app_state);

    tracing::debug!(
        provider = %status.provider.name,
        embedding_dim = %status.embedding.dimension,
        storage_dim = %status.storage.dimension,
        dimension_mismatch = %status.storage.dimension_mismatch,
        "Provider status requested"
    );

    Ok(Json(status))
}

/// List all available providers
///
/// Returns information about all supported LLM and embedding providers,
/// including their availability status based on environment configuration.
///
/// # Response
///
/// Returns [`AvailableProvidersResponse`] with:
/// - `llm_providers`: List of available LLM providers
/// - `embedding_providers`: List of available embedding providers
/// - `active_llm_provider`: Currently active LLM provider name
/// - `active_embedding_provider`: Currently active embedding provider name
///
/// # Example
///
/// ```json
/// {
///   "llm_providers": [
///     {
///       "id": "openai",
///       "name": "OpenAI",
///       "available": true,
///       "default_models": { "chat_model": "gpt-4o-mini", ... }
///     },
///     ...
///   ],
///   "active_llm_provider": "openai",
///   "active_embedding_provider": "openai"
/// }
/// ```
pub async fn list_available_providers(
    State(app_state): State<AppState>,
) -> Result<Json<AvailableProvidersResponse>, ApiError> {
    let active_llm = app_state.query.llm_provider.name();
    let active_embedding = app_state.query.embedding_provider.name();

    let response = AvailableProvidersResponse::from_models_config(
        app_state.query.models_config.as_ref(),
        active_llm,
        active_embedding,
    );

    tracing::debug!(
        llm_count = response.llm_providers.len(),
        embedding_count = response.embedding_providers.len(),
        active_llm = %active_llm,
        active_embedding = %active_embedding,
        "Available providers listed"
    );

    Ok(Json(response))
}

/// Get the full effective configuration with its resolution chain.
///
/// `GET /api/v1/config/effective`
///
/// Returns the complete priority chain for LLM, Embedding, and Vision config:
/// which level is active, where each value came from, and whether there are
/// any provider/model mismatches that need operator attention.
///
/// This is the "source of truth" endpoint for diagnosing configuration issues.
/// The frontend settings page uses this to render the Config Explainability panel.
pub async fn get_effective_config(
    State(_app_state): State<AppState>,
) -> Result<Json<EffectiveConfigResponse>, ApiError> {
    let (llm_levels, llm_provider, llm_model) = resolve_llm_chain();
    let (emb_levels, emb_provider, emb_model) = resolve_embedding_chain();
    let (vis_levels, vis_provider, vis_model) = resolve_vision_chain();

    tracing::debug!(
        llm_provider = %llm_provider,
        llm_model = %llm_model,
        emb_provider = %emb_provider,
        emb_model = %emb_model,
        vis_provider = %vis_provider,
        vis_model = %vis_model,
        "Effective config requested"
    );

    Ok(Json(EffectiveConfigResponse {
        llm: build_config_area(llm_levels, llm_provider, llm_model),
        embedding: build_config_area(emb_levels, emb_provider, emb_model),
        vision: build_config_area(vis_levels, vis_provider, vis_model),
        priority_rule:
            "Higher-indexed levels override lower. \
             compiled_default < env_alias < env_secondary < env_primary. \
             Vision inherits from LLM when no EDGEQUAKE_VISION_PROVIDER / EDGEQUAKE_VISION_MODEL vars are set. \
             The backend auto-corrects provider/model mismatches at runtime, but fixing the env var source is recommended.".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_provider_status_structure() {
        // Setup: Create AppState with mock provider
        let app_state = AppState::new_memory(None::<String>);

        // Act: Call handler
        let result = get_provider_status(State(app_state)).await;

        // Assert: Success
        assert!(result.is_ok());

        let Json(status) = result.unwrap();

        // Assert: Response structure
        assert!(!status.provider.name.is_empty());
        assert_eq!(status.provider.provider_type, "llm");
        assert!(!status.embedding.model.is_empty());
        assert!(status.embedding.dimension > 0);
    }

    #[tokio::test]
    async fn test_list_available_providers() {
        // Setup: Create AppState with mock provider
        let app_state = AppState::new_memory(None::<String>);

        // Act: Call handler
        let result = list_available_providers(State(app_state)).await;

        // Assert: Success
        assert!(result.is_ok());

        let Json(response) = result.unwrap();

        // Assert: Has all providers
        assert!(response.llm_providers.len() >= 4); // openai, ollama, lmstudio, mock
        assert!(response.embedding_providers.len() >= 4);

        // Assert: Provider IDs
        let ids: Vec<_> = response
            .llm_providers
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        assert!(ids.contains(&"openai"));
        assert!(ids.contains(&"ollama"));
        assert!(ids.contains(&"lmstudio"));
        assert!(ids.contains(&"mock"));
        assert!(
            ids.contains(&"mistral"),
            "mistral provider missing from llm_providers"
        );
        assert!(
            ids.contains(&"vertexai"),
            "vertexai provider missing from llm_providers"
        );

        // Assert: Mock is always available
        let mock = response
            .llm_providers
            .iter()
            .find(|p| p.id == "mock")
            .unwrap();
        assert!(mock.available);
        assert_eq!(mock.default_models.embedding_dimension, 768);

        // Assert: LM Studio defaults (from bundled models.toml)
        let lmstudio = response
            .llm_providers
            .iter()
            .find(|p| p.id == "lmstudio")
            .unwrap();
        assert_eq!(lmstudio.default_models.chat_model, "gemma-3n-e4b-it");
        assert_eq!(
            lmstudio.default_models.embedding_model,
            "text-embedding-nomic-embed-text-v1.5"
        );
        assert_eq!(lmstudio.default_models.embedding_dimension, 768);
    }
}
