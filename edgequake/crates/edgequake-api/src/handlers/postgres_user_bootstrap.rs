//! PostgreSQL user bootstrap for tenant-scoped chat/conversation APIs (SPEC-087).
//!
//! Auth ON: use authenticated `user_id` (JWT/API key) — never mint per-browser anon rows.
//! Auth OFF + `EDGEQUAKE_ALLOW_ANONYMOUS`: ensure one shared per-tenant guest (FK-safe).
//! Auth OFF + allow_anonymous=false: 401, no INSERT.

use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

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
            if let Some(pool) = state.pg_pool.as_ref() {
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
}
