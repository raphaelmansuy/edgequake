//! GET/PATCH server-wide LLM defaults (SPEC-043).

use axum::{extract::State, Json};
use edgequake_core::{ConfigPriorityMode, ServerLlmDefaults};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

use crate::config_resolution::{build_effective_config, resolve_field_sources};
use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::ApiRequireAdmin;
use crate::server_config_store::ServerConfigSnapshot;
use crate::state::AppState;

#[cfg(feature = "postgres")]
use crate::server_config_store::{save_llm_defaults, save_priority_mode};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LlmDefaultsEffective {
    pub llm_provider: String,
    pub llm_model: String,
    pub embedding_provider: String,
    pub embedding_model: String,
    pub vision_provider: String,
    pub vision_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SavedLlmDefaults {
    pub llm_provider: Option<String>,
    pub llm_model: Option<String>,
    pub embedding_provider: Option<String>,
    pub embedding_model: Option<String>,
    pub vision_provider: Option<String>,
    pub vision_model: Option<String>,
}

impl From<ServerLlmDefaults> for SavedLlmDefaults {
    fn from(v: ServerLlmDefaults) -> Self {
        Self {
            llm_provider: v.llm_provider,
            llm_model: v.llm_model,
            embedding_provider: v.embedding_provider,
            embedding_model: v.embedding_model,
            vision_provider: v.vision_provider,
            vision_model: v.vision_model,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LlmDefaultsResponse {
    pub effective: LlmDefaultsEffective,
    pub sources: HashMap<String, String>,
    pub saved: SavedLlmDefaults,
    pub priority_mode: String,
    pub editable: bool,
    pub requires_restart: bool,
    pub note: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateLlmDefaultsRequest {
    pub llm_provider: Option<String>,
    pub llm_model: Option<String>,
    pub embedding_provider: Option<String>,
    pub embedding_model: Option<String>,
    pub vision_provider: Option<String>,
    pub vision_model: Option<String>,
    /// `server` (DB wins) or `env` (env wins). Optional — keeps current when omitted.
    pub priority_mode: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateLlmDefaultsResponse {
    pub saved: bool,
    pub note: String,
}

async fn snapshot_from_state(app_state: &AppState) -> ServerConfigSnapshot {
    #[cfg(feature = "postgres")]
    {
        if let Some(pool) = app_state.pg_pool.as_ref() {
            return app_state
                .server_config
                .snapshot_with_postgres(Some(pool))
                .await;
        }
    }
    app_state.server_config.snapshot().await
}

fn empty_to_none(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.trim().is_empty())
}

/// GET /api/v1/settings/llm-defaults
pub async fn get_llm_defaults(
    State(app_state): State<AppState>,
) -> ApiResult<Json<LlmDefaultsResponse>> {
    let snapshot = snapshot_from_state(&app_state).await;
    let effective_cfg = build_effective_config(&snapshot);
    let sources = resolve_field_sources(&snapshot);

    #[cfg(feature = "postgres")]
    let editable = app_state.pg_pool.is_some();
    #[cfg(not(feature = "postgres"))]
    let editable = false;

    Ok(Json(LlmDefaultsResponse {
        effective: LlmDefaultsEffective {
            llm_provider: effective_cfg.llm.effective_provider,
            llm_model: effective_cfg.llm.effective_model,
            embedding_provider: effective_cfg.embedding.effective_provider,
            embedding_model: effective_cfg.embedding.effective_model,
            vision_provider: effective_cfg.vision.effective_provider,
            vision_model: effective_cfg.vision.effective_model,
        },
        sources,
        saved: snapshot.llm_defaults.clone().into(),
        priority_mode: snapshot.priority_mode.as_str().to_string(),
        editable,
        requires_restart: false,
        note: "Server defaults apply immediately to new workspace resets and explainability. \
              Running provider instances may keep startup env until restart."
            .into(),
    }))
}

/// PATCH /api/v1/settings/llm-defaults (admin)
pub async fn update_llm_defaults(
    State(app_state): State<AppState>,
    _admin: ApiRequireAdmin,
    Json(request): Json<UpdateLlmDefaultsRequest>,
) -> ApiResult<Json<UpdateLlmDefaultsResponse>> {
    #[cfg(feature = "postgres")]
    if let Some(pool) = app_state.pg_pool.as_ref() {
        let current = snapshot_from_state(&app_state).await;
        let mut saved = current.llm_defaults.clone();

        if let Some(v) = request.llm_provider {
            saved.llm_provider = empty_to_none(Some(v));
        }
        if let Some(v) = request.llm_model {
            saved.llm_model = empty_to_none(Some(v));
        }
        if let Some(v) = request.embedding_provider {
            saved.embedding_provider = empty_to_none(Some(v));
        }
        if let Some(v) = request.embedding_model {
            saved.embedding_model = empty_to_none(Some(v));
        }
        if let Some(v) = request.vision_provider {
            saved.vision_provider = empty_to_none(Some(v));
        }
        if let Some(v) = request.vision_model {
            saved.vision_model = empty_to_none(Some(v));
        }

        let priority = request
            .priority_mode
            .as_deref()
            .map(ConfigPriorityMode::parse)
            .unwrap_or(current.priority_mode);

        save_llm_defaults(pool, &saved)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to save llm_defaults: {e}")))?;
        save_priority_mode(pool, priority)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to save config_priority: {e}")))?;

        app_state.server_config.apply(saved, priority).await;

        return Ok(Json(UpdateLlmDefaultsResponse {
            saved: true,
            note: format!(
                "Saved to server_config. Priority mode: {}. \
                 Refresh Configuration Explainability to see the updated chain.",
                priority.as_str()
            ),
        }));
    }

    #[cfg(not(feature = "postgres"))]
    let _ = (&app_state, request);

    Err(ApiError::BadRequest(
        "Server LLM defaults require PostgreSQL storage.".into(),
    ))
}
