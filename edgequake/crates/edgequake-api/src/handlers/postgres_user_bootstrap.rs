//! PostgreSQL anonymous user bootstrap for tenant-scoped APIs.
//!
//! The Web UI generates random `userId` values in localStorage. Chat handlers already
//! upsert those users before creating conversations; conversation CRUD must do the same
//! (DRY — one code path, FK-safe).

use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

/// Insert an anonymous user row when using PostgreSQL (no-op without `postgres` feature).
pub async fn ensure_postgres_user_exists(
    state: &AppState,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<(), ApiError> {
    #[cfg(feature = "postgres")]
    {
        let Some(pool) = state.pg_pool.as_ref() else {
            return Ok(());
        };
        crate::services::identity_storage::ensure_anonymous_user_in_postgres(
            pool,
            &state.security,
            tenant_id,
            user_id,
        )
        .await?;
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, tenant_id, user_id);
    }

    Ok(())
}
