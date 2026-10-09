//! SPEC-163 `POST /api/v1/providers/test`.

use axum::{extract::State, Json};
use utoipa::ToSchema;

use crate::error::ApiResult;
use crate::handlers::auth::ApiRequireAdmin;
use crate::providers::probe::{probe_provider, ProbeRequest, ProbeResponse};
use crate::state::AppState;

#[derive(Debug, Clone, serde::Deserialize, ToSchema)]
pub struct ProviderTestBody {
    pub shape: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub embedding_model: Option<String>,
    pub api_key: Option<String>,
    pub auth_scheme: Option<String>,
    pub allow_private_network: Option<bool>,
    pub expected_dimension: Option<usize>,
}

#[utoipa::path(
    post,
    path = "/api/v1/providers/test",
    request_body = ProviderTestBody,
    responses((status = 200, description = "Probe result", body = ProbeResponse)),
    tag = "Providers"
)]
pub async fn test_provider(
    State(_state): State<AppState>,
    _admin: ApiRequireAdmin,
    Json(body): Json<ProviderTestBody>,
) -> ApiResult<Json<ProbeResponse>> {
    let req = ProbeRequest {
        shape: body.shape,
        base_url: body.base_url,
        model: body.model,
        embedding_model: body.embedding_model,
        api_key: body.api_key,
        auth_scheme: body.auth_scheme,
        allow_private_network: body.allow_private_network,
        expected_dimension: body.expected_dimension,
    };
    Ok(Json(probe_provider(req).await))
}
