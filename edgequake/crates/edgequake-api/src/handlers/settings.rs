//! Settings-related API handlers.

pub use crate::provider_types::{AvailableProvidersResponse, ProviderStatusResponse};

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{error::ApiError, safety_limits::is_model_provider_mismatch, state::AppState};

// ── Config Explainability Types ───────────────────────────────────────────

/// A single resolved configuration level in the priority chain.
///
/// Levels are returned in ascending priority order so that the UI can walk
/// from the lowest-priority source ("compiled default") to the highest
/// ("workspace DB") and clearly show the user *which* value wins and *why*.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EffectiveConfigResponse {
    /// LLM chat/extraction configuration chain.
    pub llm: ConfigAreaResponse,
    /// Embedding configuration chain.
    pub embedding: ConfigAreaResponse,
    /// Vision/PDF configuration chain.
    pub vision: ConfigAreaResponse,
    /// Priority rule explanation shown to the user.
    pub priority_rule: String,
    /// Active priority mode: `server` or `env`.
    pub priority_mode: String,
    /// Whether PostgreSQL server_config persistence is available.
    pub server_config_available: bool,
}

// ── Resolution helpers ────────────────────────────────────────────────────
// Chain building lives in `config_resolution.rs` (SPEC-043 server_config level).

pub(crate) fn build_config_area(
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
#[utoipa::path(
    get,
    path = "/api/v1/settings/provider/status",
    tag = "Settings",
    responses(
        (status = 200, description = "Provider status", body = ProviderStatusResponse)
    )
)]
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
#[utoipa::path(
    get,
    path = "/api/v1/settings/providers",
    tag = "Settings",
    responses(
        (status = 200, description = "Available providers", body = AvailableProvidersResponse)
    )
)]
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
#[utoipa::path(
    get,
    path = "/api/v1/config/effective",
    tag = "Settings",
    responses(
        (status = 200, description = "Effective configuration chain", body = EffectiveConfigResponse)
    )
)]
pub async fn get_effective_config(
    State(app_state): State<AppState>,
) -> Result<Json<EffectiveConfigResponse>, ApiError> {
    #[cfg(feature = "postgres")]
    let snapshot = if let Some(pool) = app_state.pg_pool.as_ref() {
        app_state
            .server_config
            .snapshot_with_postgres(Some(pool))
            .await
    } else {
        app_state.server_config.snapshot().await
    };

    #[cfg(not(feature = "postgres"))]
    let snapshot = app_state.server_config.snapshot().await;

    let response = crate::config_resolution::build_effective_config(&snapshot);

    tracing::debug!(
        llm_provider = %response.llm.effective_provider,
        llm_model = %response.llm.effective_model,
        priority_mode = %response.priority_mode,
        "Effective config requested"
    );

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

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
    #[serial]
    async fn test_list_available_providers() {
        // Pin catalog to shipped models.toml so tests are stable when ~/.edgequake/models.toml exists.
        let models_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../models.toml");
        let key = "EDGEQUAKE_MODELS_CONFIG";
        unsafe { std::env::set_var(key, models_path) };

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
        assert!(
            !ids.contains(&"mock"),
            "mock must be hidden from UI provider list"
        );
        assert!(
            ids.contains(&"mistral"),
            "mistral provider missing from llm_providers"
        );
        assert!(
            ids.contains(&"vertexai"),
            "vertexai provider missing from llm_providers"
        );

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

        unsafe { std::env::remove_var(key) };
    }
}
