//! Central credential validation — SPEC-027 IMP-002 (DRY SSOT for middleware + handlers).

use chrono::Utc;
use edgequake_auth::Role;

use crate::handlers::auth::RequestAuthContext;
use crate::state::AppState;

/// Successful authentication with optional JWT tenant claims.
#[derive(Debug, Clone)]
pub(crate) struct AuthenticatedRequest {
    pub auth: RequestAuthContext,
    pub jwt_tenant_id: Option<String>,
    pub jwt_workspace_id: Option<String>,
}

/// Validate a presented bearer/API key token against all configured sources.
pub(crate) async fn validate_presented_token(
    state: &AppState,
    token: &str,
) -> Result<Option<AuthenticatedRequest>, crate::error::ApiError> {
    if state.auth.config.api_keys.iter().any(|configured| {
        crate::services::identity_storage::constant_time_str_eq(configured, token)
    }) {
        return Ok(Some(AuthenticatedRequest {
            auth: RequestAuthContext {
                user_id: "master-api-key".to_string(),
                role: Role::Admin,
            },
            jwt_tenant_id: None,
            jwt_workspace_id: None,
        }));
    }

    if let Ok(claims) = state.auth.jwt.verify_token(token) {
        return Ok(Some(AuthenticatedRequest {
            auth: RequestAuthContext {
                user_id: claims
                    .user_id()
                    .map_err(|_| crate::error::ApiError::unauthorized())?
                    .to_string(),
                role: claims.role(),
            },
            jwt_tenant_id: claims.tenant_id.clone(),
            jwt_workspace_id: claims.workspace_id.clone(),
        }));
    }

    validate_stored_api_key(state, token).await.map(|auth| {
        auth.map(|auth| AuthenticatedRequest {
            auth,
            jwt_tenant_id: None,
            jwt_workspace_id: None,
        })
    })
}

/// Lookup Argon2-hashed API keys persisted via `POST /api/v1/api-keys`.
pub(crate) async fn validate_stored_api_key(
    state: &AppState,
    presented_key: &str,
) -> Result<Option<RequestAuthContext>, crate::error::ApiError> {
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

        let role = if record.scopes.iter().any(|s| s == "admin") {
            Role::Admin
        } else {
            Role::User
        };

        return Ok(Some(RequestAuthContext {
            user_id: record.user_id,
            role,
        }));
    }

    Ok(None)
}
