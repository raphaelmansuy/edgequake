//! PATCH application attribution into server_config (SPEC-043).

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::attribution::build_attribution_settings_response;
use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::ApiRequireAdmin;
#[cfg(feature = "postgres")]
use crate::server_config_store::{save_app_attribution, ServerAppAttribution};
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

/// Save application attribution to server_config (admin, PostgreSQL).
#[utoipa::path(
    patch,
    path = "/api/v1/settings/app-attribution",
    tag = "Settings",
    request_body = UpdateAppAttributionRequest,
    responses(
        (status = 200, description = "Attribution saved", body = UpdateAppAttributionResponse),
        (status = 400, description = "PostgreSQL storage required for persistence"),
        (status = 403, description = "Admin role required"),
    )
)]
pub async fn update_app_attribution(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Json(request): Json<UpdateAppAttributionRequest>,
) -> ApiResult<Json<UpdateAppAttributionResponse>> {
    #[cfg(feature = "postgres")]
    let saved = ServerAppAttribution {
        app_id: request.app_id.filter(|s| !s.trim().is_empty()),
        app_name: request.app_name.filter(|s| !s.trim().is_empty()),
        app_url: request.app_url.filter(|s| !s.trim().is_empty()),
    };

    #[cfg(feature = "postgres")]
    if let Some(pool) = state.pg_pool.as_ref() {
        save_app_attribution(pool, &saved)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to save app_attribution: {e}")))?;

        state.server_config.apply_app_attribution(saved).await;

        return Ok(Json(UpdateAppAttributionResponse {
            saved: true,
            note: "Saved to server_config and applied immediately. \
                   Env vars (EDGEQUAKE_APP_*) still override on conflict."
                .into(),
        }));
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (&state, request);
        return Err(ApiError::BadRequest(
            "Application attribution persistence requires PostgreSQL storage.".into(),
        ));
    }

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

/// Get application attribution (alias of `/settings/attribution` for settings save UI).
#[utoipa::path(
    get,
    path = "/api/v1/settings/app-attribution",
    tag = "Settings",
    responses(
        (status = 200, description = "Attribution settings and provider catalog", body = crate::attribution::AttributionSettingsResponse)
    )
)]
pub async fn get_app_attribution_settings(
) -> ApiResult<Json<crate::attribution::AttributionSettingsResponse>> {
    Ok(Json(build_attribution_settings_response()))
}
