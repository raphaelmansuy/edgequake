//! Application attribution API (SPEC-043).

use axum::Json;

use crate::attribution::{build_attribution_settings_response, AttributionSettingsResponse};
use crate::error::ApiResult;

/// GET /api/v1/settings/attribution
pub async fn get_attribution_settings() -> ApiResult<Json<AttributionSettingsResponse>> {
    Ok(Json(build_attribution_settings_response()))
}
