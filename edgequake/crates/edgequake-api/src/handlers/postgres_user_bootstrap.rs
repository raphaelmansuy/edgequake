//! PostgreSQL user bootstrap for tenant-scoped chat/conversation APIs (SPEC-087).
//!
//! Auth ON: use authenticated `user_id` (JWT/API key) — never mint per-browser anon rows.
//! Auth OFF + `EDGEQUAKE_ALLOW_ANONYMOUS`: ensure one shared per-tenant guest (FK-safe).
//! Auth OFF + allow_anonymous=false: 401, no INSERT.
//!
//! HTTP handlers must not read `X-User-ID` for persistence. The client header is
//! not the ownership principal: anonymous/dev mode maps every caller to one
//! shared guest. Use [`resolve_conversation_identity`] so list and write paths
//! cannot diverge (PR #389: raw-header reads returned 0 conversations).

use uuid::Uuid;

use crate::error::ApiError;
use crate::middleware::TenantContext;
use crate::state::AppState;

/// Effective tenant + persistence user for conversation/chat ownership.
///
/// `user_id` may differ from the client `X-User-ID` when SPEC-087 maps the
/// caller to the shared per-tenant guest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversationIdentity {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}

/// Resolve the persistence principal from raw tenant/user headers.
///
/// Missing/invalid tenant → 400. Missing/invalid user → 401. Then apply
/// [`ensure_postgres_user_exists`] (Auth ON = principal, Auth OFF + anonymous =
/// shared guest).
pub async fn resolve_conversation_identity(
    state: &AppState,
    tenant_ctx: &TenantContext,
) -> Result<ConversationIdentity, ApiError> {
    let tenant_id = tenant_ctx
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Missing X-Tenant-ID header".into()))?;
    let client_user_id = tenant_ctx.user_id_uuid().ok_or(ApiError::unauthorized())?;
    let user_id = ensure_postgres_user_exists(state, tenant_id, client_user_id).await?;
    Ok(ConversationIdentity { tenant_id, user_id })
}

/// Pure policy for SPEC-087 identity bootstrap (unit-testable, no I/O).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityBootstrapPolicy {
    /// Use the authenticated / client-bound user id; do not mint guest.
    UsePrincipal,
    /// Map to shared per-tenant guest (INSERT if PG present).
    UseSharedGuest,
    /// Refuse unauthenticated access.
    DenyAnonymous,
}

/// Resolve bootstrap policy from auth flags (SPEC-087 / Issue #335).
pub fn resolve_identity_bootstrap_policy(
    auth_enabled: bool,
    allow_anonymous: bool,
) -> IdentityBootstrapPolicy {
    if auth_enabled {
        IdentityBootstrapPolicy::UsePrincipal
    } else if allow_anonymous {
        IdentityBootstrapPolicy::UseSharedGuest
    } else {
        IdentityBootstrapPolicy::DenyAnonymous
    }
}

/// Resolve the effective user id for chat/conversation writes and ensure a PG row exists.
///
/// Returns the UUID that must be used for conversation ownership (may differ from the
/// client-supplied `X-User-ID` when mapping to the shared guest).
pub async fn ensure_postgres_user_exists(
    state: &AppState,
    tenant_id: Uuid,
    client_user_id: Uuid,
) -> Result<Uuid, ApiError> {
    let policy = resolve_identity_bootstrap_policy(
        state.auth.config.auth_enabled,
        state.auth.config.allow_anonymous,
    );

    match policy {
        IdentityBootstrapPolicy::DenyAnonymous => Err(ApiError::auth_unauthorized(
            "anonymous_bootstrap",
            "Anonymous access disabled — sign in or set EDGEQUAKE_ALLOW_ANONYMOUS=true",
            None,
        )),
        IdentityBootstrapPolicy::UsePrincipal => Ok(client_user_id),
        IdentityBootstrapPolicy::UseSharedGuest => {
            let guest_id = crate::services::identity_storage::shared_guest_user_id(tenant_id);

            #[cfg(feature = "postgres")]
            if let Some(pool) = state.optional_pg_pool() {
                crate::services::identity_storage::ensure_shared_guest_user_in_postgres(
                    pool,
                    &state.security,
                    tenant_id,
                    guest_id,
                )
                .await?;
            }

            let _ = client_user_id; // intentionally ignored — per-browser mint removed
            Ok(guest_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use axum::http::StatusCode;

    #[test]
    fn policy_auth_on_uses_principal() {
        assert_eq!(
            resolve_identity_bootstrap_policy(true, true),
            IdentityBootstrapPolicy::UsePrincipal
        );
        assert_eq!(
            resolve_identity_bootstrap_policy(true, false),
            IdentityBootstrapPolicy::UsePrincipal
        );
    }

    #[test]
    fn policy_auth_off_guest_or_deny() {
        assert_eq!(
            resolve_identity_bootstrap_policy(false, true),
            IdentityBootstrapPolicy::UseSharedGuest
        );
        assert_eq!(
            resolve_identity_bootstrap_policy(false, false),
            IdentityBootstrapPolicy::DenyAnonymous
        );
    }

    #[tokio::test]
    async fn resolve_identity_missing_tenant_is_400() {
        let state = AppState::test_state();
        let err = resolve_conversation_identity(&state, &TenantContext::default())
            .await
            .unwrap_err();
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn resolve_identity_missing_user_is_401() {
        let state = AppState::test_state();
        let ctx = TenantContext {
            tenant_id: Some(Uuid::new_v4().to_string()),
            ..TenantContext::default()
        };
        let err = resolve_conversation_identity(&state, &ctx)
            .await
            .unwrap_err();
        assert_eq!(err.status_code(), StatusCode::UNAUTHORIZED);
    }
}
