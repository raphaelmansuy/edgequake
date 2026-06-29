//! API-layer auth extractors (ARCH-D-001 / SPEC-027).
//!
//! Bridges [`edgequake_auth`] JWT extractors with EdgeQuake KV/API-key validation SSOT.
//! When auth is disabled, ascending-compat grants synthetic Admin (same as legacy handlers).

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap},
};

use crate::error::ApiError;
use crate::handlers::auth::{
    authenticate_request_async, require_admin_request, require_authenticated_request,
    RequestAuthContext,
};
use crate::state::AppState;

/// Authenticated request context (JWT, stored API key, or master key when auth enabled).
#[derive(Debug, Clone)]
pub struct ApiAuthenticated(pub RequestAuthContext);

impl ApiAuthenticated {
    pub fn context(&self) -> &RequestAuthContext {
        &self.0
    }
}

impl FromRequestParts<AppState> for ApiAuthenticated {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        require_authenticated_request(&parts.headers, state)
            .await
            .map(ApiAuthenticated)
    }
}

/// Admin-only gate — uses auth_validation SSOT (supports API keys, not JWT-only).
#[derive(Debug, Clone)]
pub struct ApiRequireAdmin(pub RequestAuthContext);

impl ApiRequireAdmin {
    pub fn context(&self) -> &RequestAuthContext {
        &self.0
    }
}

impl FromRequestParts<AppState> for ApiRequireAdmin {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        require_admin_request(&parts.headers, state)
            .await
            .map(ApiRequireAdmin)
    }
}

/// Optional auth for registration-style endpoints (create user when allow_registration).
#[derive(Debug, Clone)]
pub struct ApiOptionalAuth(pub Option<RequestAuthContext>);

impl ApiOptionalAuth {
    pub fn context(&self) -> Option<&RequestAuthContext> {
        self.0.as_ref()
    }
}

impl FromRequestParts<AppState> for ApiOptionalAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if !state.auth.config.auth_enabled {
            return Ok(ApiOptionalAuth(None));
        }
        authenticate_request_async(&parts.headers, state)
            .await
            .map(ApiOptionalAuth)
    }
}

/// Re-export native JWT [`AuthState`] for handlers that only need JWT (login/session).
pub use edgequake_auth::extractors::{AuthState, AuthUser, OptionalAuth, RequireAdmin};

/// Extract admin from headers + state without Axum extractor (legacy call sites).
pub async fn admin_from_headers(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<ApiRequireAdmin, ApiError> {
    require_admin_request(headers, state)
        .await
        .map(ApiRequireAdmin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, Request};
    use edgequake_auth::Role;

    #[tokio::test]
    async fn api_require_admin_synthetic_when_auth_disabled() {
        let state = AppState::test_state();
        assert!(!state.auth.config.auth_enabled);
        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();
        let (mut parts, _) = request.into_parts();
        let admin = ApiRequireAdmin::from_request_parts(&mut parts, &state)
            .await
            .expect("synthetic admin");
        assert_eq!(admin.0.role, Role::Admin);
    }
}
