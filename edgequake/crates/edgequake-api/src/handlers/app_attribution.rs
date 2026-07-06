//! PATCH application attribution into server_config (SPEC-043).

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::attribution::build_attribution_settings_response;
use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::ApiRequireAdmin;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAppAttributionRequest {
    pub app_id: Option<String>,
    pub app_name: Option<String>,
    pub app_url: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateAppAttributionResponse {
    pub saved: bool,
    pub note: String,
}

/// PATCH /api/v1/settings/app-attribution
pub async fn update_app_attribution(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Json(request): Json<UpdateAppAttributionRequest>,
) -> ApiResult<Json<UpdateAppAttributionResponse>> {
    let value = serde_json::json!({
        "app_id": request.app_id.filter(|s| !s.trim().is_empty()),
        "app_name": request.app_name.filter(|s| !s.trim().is_empty()),
        "app_url": request.app_url.filter(|s| !s.trim().is_empty()),
    });

    #[cfg(feature = "postgres")]
    if let Some(pool) = state.pg_pool.as_ref() {
        sqlx::query(
            r#"
            INSERT INTO server_config (key, value, updated_at)
            VALUES ('app_attribution', $1::jsonb, NOW())
            ON CONFLICT (key) DO UPDATE
              SET value = EXCLUDED.value,
                  updated_at = NOW()
            "#,
        )
        .bind(value)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to save app_attribution: {e}")))?;

        return Ok(Json(UpdateAppAttributionResponse {
            saved: true,
            note:
                "Saved to server_config. Env vars (EDGEQUAKE_APP_*) still apply at process start."
                    .into(),
        }));
    }

    #[cfg(not(feature = "postgres"))]
    let _ = (&state, value);

    Err(ApiError::BadRequest(
        "Application attribution persistence requires PostgreSQL storage.".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_request_accepts_optional_fields() {
        let req = UpdateAppAttributionRequest {
            app_id: Some("eq".into()),
            app_name: None,
            app_url: None,
        };
        assert_eq!(req.app_id.as_deref(), Some("eq"));
    }
}

/// GET wrapper re-export for OpenAPI symmetry.
pub async fn get_app_attribution_settings(
) -> ApiResult<Json<crate::attribution::AttributionSettingsResponse>> {
    Ok(Json(build_attribution_settings_response()))
}
