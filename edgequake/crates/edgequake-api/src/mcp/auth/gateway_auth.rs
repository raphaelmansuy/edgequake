//! MCP gateway authentication — OAuth-aware 401 with PRM pointer + audience binding.
//!
//! SPEC-154 Wave 1: credentials classified via [`auth_validation::decide`].

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

use crate::oauth::types::McpAuthScopes;
use crate::services::auth_validation::TokenProfile;

use super::www_authenticate::www_authenticate_bearer;

pub async fn mcp_gateway_auth(
    State(state): State<crate::state::AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Response<Body> {
    if !state.auth.config.auth_enabled {
        request
            .extensions_mut()
            .insert(McpAuthScopes::api_key_full());
        return next.run(request).await;
    }

    let Some(token) = crate::middleware::extract_api_key(&request) else {
        return oauth_unauthorized_response(request.headers());
    };

    match crate::services::auth_validation::decide(
        &state,
        &token,
        TokenProfile::McpResource,
        request.headers(),
    )
    .await
    {
        Ok(Some(decision)) => {
            if let Some(response) = crate::middleware::apply_authenticated_context(
                &state,
                &mut request,
                decision.authenticated,
            ) {
                return response;
            }
            if let Err(error) =
                crate::services::request_authorization::bind_request(&state, &mut request).await
            {
                return error.into_response();
            }
            let mut scopes = decision.scopes;
            if request
                .extensions()
                .get::<crate::handlers::auth::RequestAuthContext>()
                .is_some_and(|auth| auth.role == edgequake_auth::Role::Readonly)
            {
                scopes.break_glass = false;
                scopes.scopes = [
                    crate::oauth::scopes::MCP_SCOPE_READ,
                    crate::oauth::scopes::MCP_SCOPE_QUERY,
                ]
                .into_iter()
                .filter(|scope| scopes.allows(scope))
                .map(str::to_string)
                .collect();
            }
            request.extensions_mut().insert(scopes);
            next.run(request).await
        }
        Ok(None) => oauth_unauthorized_response(request.headers()),
        Err(e) => e.into_response(),
    }
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
