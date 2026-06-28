//! Central credential validation — SPEC-027 IMP-002 (DRY SSOT for middleware + handlers).

use chrono::Utc;
use edgequake_auth::Role;

use crate::handlers::auth::{ApiKeyRecord, RequestAuthContext, API_KEY_PREFIX};
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
    if state
        .auth
        .config
        .api_keys
        .iter()
        .any(|configured| configured == token)
    {
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

    let storage_keys = state
        .storage
        .kv_storage
        .keys_with_prefix(API_KEY_PREFIX)
        .await
        .map_err(|e| crate::error::ApiError::Internal(format!("API key lookup failed: {e}")))?;

    for storage_key in storage_keys {
        let Some(value) = state
            .storage
            .kv_storage
            .get_by_id(&storage_key)
            .await
            .map_err(|e| crate::error::ApiError::Internal(format!("API key read failed: {e}")))?
        else {
            continue;
        };

        let record: ApiKeyRecord = serde_json::from_value(value)
            .map_err(|e| crate::error::ApiError::Internal(format!("API key parse failed: {e}")))?;

        if !record.is_active {
            continue;
        }
        if record.prefix != presented_prefix {
            continue;
        }
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
