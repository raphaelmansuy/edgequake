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
        sqlx::query(
            r#"
            INSERT INTO users (user_id, tenant_id, username, email, password_hash, role, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, 'anonymous', 'user', TRUE, NOW(), NOW())
            ON CONFLICT (user_id) DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .bind(format!("anon_{}", &user_id.to_string()[..8]))
        .bind(format!("{}@anonymous.local", &user_id.to_string()[..8]))
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to ensure user exists: {}", e)))?;
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, tenant_id, user_id);
    }

    Ok(())
}
