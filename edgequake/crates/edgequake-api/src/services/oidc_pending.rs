//! OIDC pending session helpers (SPEC-027 phase 55 — in-memory, not KV).

use crate::error::ApiError;
use crate::state::StorageRuntime;

use super::oidc_flow::OidcPendingSession;

/// Store pending OIDC session (short-lived; not identity SSOT).
pub async fn store_oidc_pending(
    storage: &StorageRuntime,
    csrf_token: &str,
    pending: &OidcPendingSession,
) -> Result<(), ApiError> {
    crate::services::auth_memory_store::store_oidc_pending(
        &storage.auth_memory,
        csrf_token,
        pending,
    )
    .await
}

/// Load and remove pending OIDC session.
pub async fn take_oidc_pending(
    storage: &StorageRuntime,
    csrf_token: &str,
) -> Result<OidcPendingSession, ApiError> {
    let pending =
        crate::services::auth_memory_store::take_oidc_pending(&storage.auth_memory, csrf_token)
            .await?
            .ok_or_else(|| ApiError::auth_unauthorized("oidc_callback", "state_expired", None))?;
    Ok(pending)
}
