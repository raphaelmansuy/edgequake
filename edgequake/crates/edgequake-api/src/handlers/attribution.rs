//! Application attribution API (SPEC-043).

use axum::Json;

use crate::attribution::{build_attribution_settings_response, AttributionSettingsResponse};
use crate::error::ApiResult;

/// Get application attribution settings and provider header catalog.
///
/// Returns the effective `ApplicationContext` (from env), per-provider upstream
/// header/body field catalog from edgequake-llm, and ingress header names clients
/// may send to override attribution per request.
#[utoipa::path(
    get,
    path = "/api/v1/settings/attribution",
    tag = "Settings",
    responses(
        (status = 200, description = "Attribution settings and provider catalog", body = AttributionSettingsResponse)
    )
)]
pub async fn get_attribution_settings() -> ApiResult<Json<AttributionSettingsResponse>> {
    Ok(Json(build_attribution_settings_response()))
}
