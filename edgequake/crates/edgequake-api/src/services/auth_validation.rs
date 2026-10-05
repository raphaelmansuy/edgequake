//! Central credential validation — SPEC-027 IMP-002 + SPEC-154 Wave 1 (DRY SSOT).

use axum::http::HeaderMap;
use chrono::Utc;
use edgequake_auth::{Claims, Role};

use crate::handlers::auth::RequestAuthContext;
use crate::mcp::config::McpPublicConfig;
use crate::oauth::types::McpAuthScopes;
use crate::state::AppState;

/// Successful authentication with optional JWT tenant claims.
#[derive(Debug, Clone)]
pub(crate) struct AuthenticatedRequest {
    pub auth: RequestAuthContext,
    pub jwt_tenant_id: Option<String>,
    pub jwt_workspace_id: Option<String>,
    /// Present for API keys — OAuth-normalized scopes (SPEC-154 Wave 3).
    pub api_key_scopes: Option<McpAuthScopes>,
}

/// Token profile / capability surface (SPEC-154 LAW-154-1 / LAW-154-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenProfile {
    /// Web session JWT (must NOT carry MCP resource audience).
    WebSession,
    /// MCP resource-bound JWT (`aud` contains MCP resource URL).
    McpResource,
    /// Opaque API key (master or stored `eq_`).
    ApiKey,
}

/// Single verifier decision shared by REST, MCP, and WebSocket.
#[derive(Debug, Clone)]
pub(crate) struct CredentialDecision {
    #[allow(dead_code)] // Inspected by callers / future profile policy.
    pub profile: TokenProfile,
    pub authenticated: AuthenticatedRequest,
    pub scopes: McpAuthScopes,
}

/// Validate a presented bearer/API key token against all configured sources.
///
/// Profile-neutral (no audience surface gate). Use [`decide`] at resource-server
/// middleware boundaries for SPEC-154 LAW-154-3 audience capability checks.
pub(crate) async fn validate_presented_token(
    state: &AppState,
    token: &str,
) -> Result<Option<AuthenticatedRequest>, crate::error::ApiError> {
    if let Some(auth) = validate_master_or_stored_api_key(state, token).await? {
        return Ok(Some(auth));
    }

    let Ok(claims) = state.auth.jwt.verify_token(token) else {
        return Ok(None);
    };

    // SPEC-154: durable jti denylist applies on every presented JWT path.
    if crate::services::jti_denylist::is_jti_revoked_durable(state, &claims.jti).await {
        return Ok(None);
    }

    refresh_principal(state, authenticated_from_claims(&claims)?).await
}

/// Classify and authorize a credential for the requested surface (SPEC-154 Wave 1).
///
/// Returns `Ok(None)` when the credential is missing/invalid **or** when its
/// profile is incompatible with `want` (audience capability gate).
pub(crate) async fn decide(
    state: &AppState,
    token: &str,
    want: TokenProfile,
    headers: &HeaderMap,
) -> Result<Option<CredentialDecision>, crate::error::ApiError> {
    if let Some(auth) = validate_master_or_stored_api_key(state, token).await? {
        // API keys are accepted on REST and MCP (Wave 3 scopes refine allows()).
        if matches!(want, TokenProfile::WebSession | TokenProfile::McpResource) {
            let scopes = auth
                .api_key_scopes
                .clone()
                .unwrap_or_else(McpAuthScopes::api_key_full);
            return Ok(Some(CredentialDecision {
                profile: TokenProfile::ApiKey,
                authenticated: auth,
                scopes,
            }));
        }
    }

    let Ok(claims) = state.auth.jwt.verify_token(token) else {
        return Ok(None);
    };

    // SPEC-154 LAW-154-8: durable jti denylist (cross-replica).
    if crate::services::jti_denylist::is_jti_revoked_durable(state, &claims.jti).await {
        return Ok(None);
    }

    let resource_url = McpPublicConfig::resolve(headers).resource_url;
    let profile = classify_jwt_profile(&claims, &resource_url);
    if !profiles_compatible(want, profile) {
        tracing::warn!(
            want = ?want,
            profile = ?profile,
            "Credential profile rejected for surface (SPEC-154 LAW-154-3)"
        );
        return Ok(None);
    }

    let scopes = McpAuthScopes::from_scope_claim(claims.scope.as_deref());
    let Some(authenticated) = refresh_principal(state, authenticated_from_claims(&claims)?).await?
    else {
        return Ok(None);
    };
    Ok(Some(CredentialDecision {
        profile,
        authenticated,
        scopes,
    }))
}

fn classify_jwt_profile(claims: &Claims, resource_url: &str) -> TokenProfile {
    let is_mcp = claims
        .aud
        .as_ref()
        .is_some_and(|aud| aud.iter().any(|a| a == resource_url));
    if is_mcp {
        TokenProfile::McpResource
    } else {
        TokenProfile::WebSession
    }
}

fn profiles_compatible(want: TokenProfile, have: TokenProfile) -> bool {
    match want {
        TokenProfile::WebSession => have == TokenProfile::WebSession,
        TokenProfile::McpResource => have == TokenProfile::McpResource,
        TokenProfile::ApiKey => have == TokenProfile::ApiKey,
    }
}

/// Master configured keys + persisted `eq_` API keys (no JWT).
pub(crate) async fn validate_master_or_stored_api_key(
    state: &AppState,
    token: &str,
) -> Result<Option<AuthenticatedRequest>, crate::error::ApiError> {
    // Explicit master key → break-glass (SPEC-154 Wave 3 / gap-close).
    if let Some(ref master) = state.auth.config.master_api_key {
        if crate::services::identity_storage::constant_time_str_eq(master, token) {
            return Ok(Some(AuthenticatedRequest {
                auth: RequestAuthContext {
                    user_id: "master-api-key".to_string(),
                    role: Role::Admin,
                },
                jwt_tenant_id: None,
                jwt_workspace_id: None,
                api_key_scopes: Some(McpAuthScopes::api_key_full()),
            }));
        }
    }

    // Static env API keys (`EDGEQUAKE_API_KEYS`) — default read+query, not break-glass.
    if state.auth.config.api_keys.iter().any(|configured| {
        crate::services::identity_storage::constant_time_str_eq(configured, token)
    }) {
        let scopes =
            McpAuthScopes::from_api_key_scopes(crate::oauth::scopes::default_api_key_scopes());
        return Ok(Some(AuthenticatedRequest {
            auth: RequestAuthContext {
                user_id: "static-api-key".to_string(),
                role: Role::Readonly,
            },
            jwt_tenant_id: None,
            jwt_workspace_id: None,
            api_key_scopes: Some(scopes),
        }));
    }

    validate_stored_api_key(state, token).await
}

/// Build authenticated context from verified JWT claims.
pub(crate) fn authenticated_from_claims(
    claims: &Claims,
) -> Result<AuthenticatedRequest, crate::error::ApiError> {
    Ok(AuthenticatedRequest {
        auth: RequestAuthContext {
            user_id: claims
                .user_id()
                .map_err(|_| crate::error::ApiError::unauthorized())?
                .to_string(),
            role: claims
                .role()
                .map_err(|_| crate::error::ApiError::unauthorized())?,
        },
        jwt_tenant_id: claims.tenant_id.clone(),
        jwt_workspace_id: claims.workspace_id.clone(),
        api_key_scopes: None,
    })
}

/// Lookup Argon2-hashed API keys persisted via `POST /api/v1/api-keys`.
pub(crate) async fn validate_stored_api_key(
    state: &AppState,
    presented_key: &str,
) -> Result<Option<AuthenticatedRequest>, crate::error::ApiError> {
    if !presented_key.starts_with("eq_") || presented_key.len() < 12 {
        return Ok(None);
    }

    let presented_prefix: String = presented_key.chars().take(11).collect();

    #[cfg(feature = "postgres")]
    let pg_holder = state
        .pg_pool
        .clone()
        .map(|pool| crate::state::PostgresRuntime {
            pool: Some(pool),
            capabilities: None,
        });
    #[cfg(feature = "postgres")]
    let pg_runtime = pg_holder.as_ref();
    #[cfg(not(feature = "postgres"))]
    let pg_runtime: Option<&crate::state::PostgresRuntime> = None;

    let candidates = crate::services::session_storage::find_active_api_keys_by_prefix(
        &state.storage,
        pg_runtime,
        &state.security,
        state.operational_stores.sessions.as_deref(),
        &presented_prefix,
    )
    .await?;

    for record in candidates {
        if record
            .expires_at
            .is_some_and(|expires| expires < Utc::now())
        {
            continue;
        }

        let valid = state
            .auth
            .password
            .verify_password(presented_key, &record.key_hash)
            .map_err(|e| crate::error::ApiError::Internal(format!("API key verify failed: {e}")))?;

        if !valid {
            continue;
        }

        let normalized = crate::oauth::scopes::normalize_api_key_scopes(&record.scopes);
        let role = if record.scopes.iter().any(|s| s == "admin" || s == "*")
            || normalized.iter().any(|s| s == "*")
        {
            Role::Admin
        } else if normalized
            .iter()
            .any(|s| s == crate::oauth::scopes::MCP_SCOPE_WRITE)
        {
            Role::User
        } else {
            Role::Readonly
        };

        return refresh_principal(
            state,
            AuthenticatedRequest {
                auth: RequestAuthContext {
                    user_id: record.user_id,
                    role,
                },
                jwt_tenant_id: None,
                jwt_workspace_id: None,
                api_key_scopes: Some(McpAuthScopes::from_api_key_scopes(normalized)),
            },
        )
        .await;
    }

    Ok(None)
}

/// Durable identity is authoritative for account status and role reductions on every surface.
async fn refresh_principal(
    state: &AppState,
    mut authenticated: AuthenticatedRequest,
) -> Result<Option<AuthenticatedRequest>, crate::error::ApiError> {
    // Synthetic identities exist only for explicitly configured service credentials.
    if matches!(
        authenticated.auth.user_id.as_str(),
        "master-api-key" | "static-api-key"
    ) {
        return Ok(Some(authenticated));
    }
    #[cfg(feature = "postgres")]
    let pg = state
        .pg_pool
        .clone()
        .map(|pool| crate::state::PostgresRuntime {
            pool: Some(pool),
            capabilities: None,
        });
    #[cfg(feature = "postgres")]
    let pg = pg.as_ref();
    #[cfg(not(feature = "postgres"))]
    let pg = None;
    let user = crate::handlers::auth::get_record_by_id(
        &state.storage,
        pg,
        &state.security,
        state.operational_stores.identity.as_deref(),
        &authenticated.auth.user_id,
    )
    .await?;
    let Some(user) = user else {
        if state.auth.config.dev_mode {
            return Ok(Some(authenticated));
        }
        return Ok(None);
    };
    if !user.is_active || user.locked_until.is_some_and(|until| until > Utc::now()) {
        return Ok(None);
    }
    let Ok(role) = Role::try_parse(&user.role) else {
        return Ok(None);
    };
    authenticated.auth.role = lesser_role(authenticated.auth.role, role);
    Ok(Some(authenticated))
}

fn lesser_role(granted: Role, current: Role) -> Role {
    if granted == Role::Readonly || current == Role::Readonly {
        Role::Readonly
    } else if granted == Role::User || current == Role::User {
        Role::User
    } else {
        Role::Admin
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_auth::Claims;
    use uuid::Uuid;

    #[test]
    fn classify_mcp_aud_is_mcp_resource() {
        let claims = Claims::new(Uuid::new_v4(), Role::User, 900)
            .with_audience(vec!["http://127.0.0.1:8080/mcp".to_string()]);
        assert_eq!(
            classify_jwt_profile(&claims, "http://127.0.0.1:8080/mcp"),
            TokenProfile::McpResource
        );
    }

    #[test]
    fn classify_no_aud_is_web_session() {
        let claims = Claims::new(Uuid::new_v4(), Role::User, 900);
        assert_eq!(
            classify_jwt_profile(&claims, "http://127.0.0.1:8080/mcp"),
            TokenProfile::WebSession
        );
    }

    #[test]
    fn rest_rejects_mcp_profile() {
        assert!(!profiles_compatible(
            TokenProfile::WebSession,
            TokenProfile::McpResource
        ));
        assert!(profiles_compatible(
            TokenProfile::WebSession,
            TokenProfile::WebSession
        ));
    }

    #[test]
    fn mcp_rejects_web_session_profile() {
        assert!(!profiles_compatible(
            TokenProfile::McpResource,
            TokenProfile::WebSession
        ));
    }
}
