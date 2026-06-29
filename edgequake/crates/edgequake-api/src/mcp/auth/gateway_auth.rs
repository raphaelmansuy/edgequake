//! MCP gateway authentication — OAuth-aware 401 with PRM pointer.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

use super::www_authenticate::www_authenticate_bearer;

pub async fn mcp_gateway_auth(
    State(state): State<crate::state::AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Response<Body> {
    if !state.auth.config.auth_enabled {
        return next.run(request).await;
    }

    if let Some(token) = crate::middleware::extract_api_key(&request) {
        match crate::services::auth_validation::validate_presented_token(&state, &token).await {
            Ok(Some(authenticated)) => {
                if let Some(response) = crate::middleware::apply_authenticated_context(
                    &state,
                    &mut request,
                    authenticated,
                ) {
                    return response;
                }
                return next.run(request).await;
            }
            Ok(None) => {}
            Err(e) => return e.into_response(),
        }
    }

    oauth_unauthorized_response(request.headers())
}

fn oauth_unauthorized_response(headers: &axum::http::HeaderMap) -> Response<Body> {
    let www = www_authenticate_bearer(headers);
    let mut response = (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "error": "unauthorized",
            "message": "Authentication required — use OAuth 2.1 Bearer token or API key"
        })),
    )
        .into_response();
    if let Ok(val) = www.parse() {
        response.headers_mut().insert("WWW-Authenticate", val);
    }
    response
}
