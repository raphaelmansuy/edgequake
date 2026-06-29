//! Session artifact storage SSOT (SPEC-027 phase 39 — refresh tokens + API keys).
//!
//! Reuses [`IdentityPolicy`] from `identity_storage` — PG-primary when pool available;
//! in-memory fallback for E2E/tests without a PG pool.

use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::auth::{ApiKeyRecord, RefreshTokenRecord};
use crate::services::identity_storage::IdentityPolicy;
use crate::state::{ApiSecurityConfig, PostgresRuntime, StorageRuntime};

/// SHA-256 hex digest for refresh-token O(1) PG lookup (indexed in migration 052).
pub(crate) fn refresh_token_lookup_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

async fn persist_refresh_token_kv(
    storage: &StorageRuntime,
    record: &RefreshTokenRecord,
) -> Result<(), ApiError> {
    crate::services::auth_memory_store::persist_refresh_token(&storage.auth_memory, record).await
}

async fn load_refresh_token_kv(
    storage: &StorageRuntime,
    token: &str,
) -> Result<Option<RefreshTokenRecord>, ApiError> {
    crate::services::auth_memory_store::load_refresh_token(&storage.auth_memory, token).await
}

async fn revoke_refresh_token_kv(storage: &StorageRuntime, token: &str) -> Result<bool, ApiError> {
    crate::services::auth_memory_store::revoke_refresh_token(&storage.auth_memory, token).await
}

#[cfg(feature = "postgres")]
async fn persist_refresh_token_pg(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
    record: &RefreshTokenRecord,
) -> Result<(), ApiError> {
    use crate::services::tenant_isolation::{
        acquire_optional_pg_connection, release_optional_pg_connection, PgIsolationScope,
    };

    let user_uuid = Uuid::parse_str(&record.user_id)
        .map_err(|_| ApiError::Internal("invalid user_id for refresh token".into()))?;
    let token_hash = refresh_token_lookup_hash(&record.token);
    let scope = Some(PgIsolationScope::default_identity(Some(user_uuid)));

    let mut conn = acquire_optional_pg_connection(pool, security, scope).await?;

    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (token_id, user_id, token_hash, expires_at, revoked, created_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_uuid)
    .bind(token_hash)
    .bind(record.expires_at)
    .bind(record.revoked)
    .bind(record.created_at)
    .execute(&mut *conn)
    .await
    .map_err(|e| ApiError::Internal(format!("refresh token PG insert: {e}")))?;

    release_optional_pg_connection(&mut conn, security, scope).await;

    Ok(())
}

#[cfg(feature = "postgres")]
async fn load_refresh_token_pg(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
    token: &str,
) -> Result<Option<RefreshTokenRecord>, ApiError> {
    use crate::services::tenant_isolation::{
        acquire_optional_pg_connection, release_optional_pg_connection, PgIsolationScope,
    };

    let token_hash = refresh_token_lookup_hash(token);
    let scope = Some(PgIsolationScope::default_identity(None));

    let mut conn = acquire_optional_pg_connection(pool, security, scope).await?;

    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            bool,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT token_id, user_id, revoked, created_at, expires_at
        FROM refresh_tokens
        WHERE token_hash = $1
        LIMIT 1
        "#,
    )
    .bind(token_hash)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| ApiError::Internal(format!("refresh token PG load: {e}")))?;

    release_optional_pg_connection(&mut conn, security, scope).await;

    Ok(row.map(
        |(_, user_id, revoked, created_at, expires_at)| RefreshTokenRecord {
            token: token.to_string(),
            user_id: user_id.to_string(),
            created_at,
            expires_at,
            revoked,
        },
    ))
}

#[cfg(feature = "postgres")]
async fn revoke_refresh_token_pg(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
    token: &str,
) -> Result<bool, ApiError> {
    use crate::services::tenant_isolation::{
        acquire_optional_pg_connection, release_optional_pg_connection, PgIsolationScope,
    };

    let token_hash = refresh_token_lookup_hash(token);
    let scope = Some(PgIsolationScope::default_identity(None));

    let mut conn = acquire_optional_pg_connection(pool, security, scope).await?;

    let result = sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET revoked = true, revoked_at = NOW()
        WHERE token_hash = $1 AND revoked = false
        "#,
    )
    .bind(token_hash)
    .execute(&mut *conn)
    .await
    .map_err(|e| ApiError::Internal(format!("refresh token PG revoke: {e}")))?;

    release_optional_pg_connection(&mut conn, security, scope).await;

    Ok(result.rows_affected() > 0)
}

/// Persist refresh token — PG SSOT when pool + policy; optional KV mirror.
pub(crate) async fn persist_refresh_token(
    storage: &StorageRuntime,
    pg_runtime: Option<&PostgresRuntime>,
    security: &ApiSecurityConfig,
    record: &RefreshTokenRecord,
) -> Result<(), ApiError> {
    #[cfg(feature = "postgres")]
    {
        let pool = pg_runtime.and_then(|pg| pg.pool.as_ref());
        let policy = IdentityPolicy::resolve(security, pool.is_some());

        if policy.pg_primary {
            if let Some(pool) = pool {
                persist_refresh_token_pg(pool, security, record).await?;
            }
        } else {
            persist_refresh_token_kv(storage, record).await?;
        }

        Ok(())
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (pg_runtime, security);
        persist_refresh_token_kv(storage, record).await
    }
}

/// Load refresh token by presented value.
pub(crate) async fn load_refresh_token(
    storage: &StorageRuntime,
    pg_runtime: Option<&PostgresRuntime>,
    security: &ApiSecurityConfig,
    token: &str,
) -> Result<Option<RefreshTokenRecord>, ApiError> {
    #[cfg(feature = "postgres")]
    {
        let pool = pg_runtime.and_then(|pg| pg.pool.as_ref());
        let policy = IdentityPolicy::resolve(security, pool.is_some());

        if policy.pg_primary {
            if let Some(pool) = pool {
                return load_refresh_token_pg(pool, security, token).await;
            }
            return Ok(None);
        }

        load_refresh_token_kv(storage, token).await
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (pg_runtime, security);
        load_refresh_token_kv(storage, token).await
    }
}

/// Revoke refresh token; returns whether a record was updated.
pub(crate) async fn revoke_refresh_token(
    storage: &StorageRuntime,
    pg_runtime: Option<&PostgresRuntime>,
    security: &ApiSecurityConfig,
    token: &str,
) -> Result<bool, ApiError> {
    #[cfg(feature = "postgres")]
    {
        let pool = pg_runtime.and_then(|pg| pg.pool.as_ref());
        let policy = IdentityPolicy::resolve(security, pool.is_some());

        if policy.pg_primary {
            if let Some(pool) = pool {
                return revoke_refresh_token_pg(pool, security, token).await;
            }
            return Ok(false);
        }

        revoke_refresh_token_kv(storage, token).await
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (pg_runtime, security);
        revoke_refresh_token_kv(storage, token).await
    }
}

async fn persist_api_key_kv(
    storage: &StorageRuntime,
    record: &ApiKeyRecord,
) -> Result<(), ApiError> {
    crate::services::auth_memory_store::persist_api_key(&storage.auth_memory, record).await
}

#[cfg(feature = "postgres")]
async fn persist_api_key_pg(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
    record: &ApiKeyRecord,
) -> Result<(), ApiError> {
    use crate::services::tenant_isolation::{
        acquire_optional_pg_connection, release_optional_pg_connection, PgIsolationScope,
    };

    let key_uuid = Uuid::parse_str(&record.key_id)
        .map_err(|_| ApiError::Internal("invalid key_id for api key".into()))?;
    let user_uuid = Uuid::parse_str(&record.user_id)
        .map_err(|_| ApiError::Internal("invalid user_id for api key".into()))?;
    let scope = Some(PgIsolationScope::default_identity(Some(user_uuid)));

    let mut conn = acquire_optional_pg_connection(pool, security, scope).await?;

    sqlx::query(
        r#"
        INSERT INTO api_keys (
            key_id, user_id, key_hash, key_prefix, name, scopes,
            is_active, created_at, last_used_at, expires_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (key_id) DO UPDATE SET
            key_hash = EXCLUDED.key_hash,
            key_prefix = EXCLUDED.key_prefix,
            name = EXCLUDED.name,
            scopes = EXCLUDED.scopes,
            is_active = EXCLUDED.is_active,
            expires_at = EXCLUDED.expires_at
        "#,
    )
    .bind(key_uuid)
    .bind(user_uuid)
    .bind(&record.key_hash)
    .bind(&record.prefix)
    .bind(&record.name)
    .bind(&record.scopes)
    .bind(record.is_active)
    .bind(record.created_at)
    .bind(record.last_used_at)
    .bind(record.expires_at)
    .execute(&mut *conn)
    .await
    .map_err(|e| ApiError::Internal(format!("api key PG upsert: {e}")))?;

    release_optional_pg_connection(&mut conn, security, scope).await;

    Ok(())
}

#[cfg(feature = "postgres")]
fn api_key_from_row(row: ApiKeyRow) -> ApiKeyRecord {
    let (
        key_id,
        user_id,
        key_hash,
        key_prefix,
        name,
        scopes,
        is_active,
        created_at,
        last_used_at,
        expires_at,
    ) = row;
    ApiKeyRecord {
        key_id: key_id.to_string(),
        user_id: user_id.to_string(),
        key_hash,
        prefix: key_prefix,
        name,
        scopes: scopes.unwrap_or_default(),
        is_active,
        created_at,
        last_used_at,
        expires_at,
    }
}

#[cfg(feature = "postgres")]
type ApiKeyRow = (
    Uuid,
    Uuid,
    String,
    String,
    Option<String>,
    Option<Vec<String>>,
    bool,
    chrono::DateTime<chrono::Utc>,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
);

#[cfg(feature = "postgres")]
async fn list_api_keys_pg(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
    user_id: &str,
) -> Result<Vec<ApiKeyRecord>, ApiError> {
    use crate::services::tenant_isolation::{
        acquire_optional_pg_connection, release_optional_pg_connection, PgIsolationScope,
    };

    let user_uuid = Uuid::parse_str(user_id)
        .map_err(|_| ApiError::Internal("invalid user_id for api key list".into()))?;
    let scope = Some(PgIsolationScope::default_identity(Some(user_uuid)));

    let mut conn = acquire_optional_pg_connection(pool, security, scope).await?;

    let rows = sqlx::query_as::<_, ApiKeyRow>(
        r#"
        SELECT key_id, user_id, key_hash, key_prefix, name, scopes,
               is_active, created_at, last_used_at, expires_at
        FROM api_keys
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_uuid)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| ApiError::Internal(format!("api key PG list: {e}")))?;

    release_optional_pg_connection(&mut conn, security, scope).await;

    Ok(rows.into_iter().map(api_key_from_row).collect())
}

#[cfg(feature = "postgres")]
async fn find_api_keys_by_prefix_pg(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
    prefix: &str,
) -> Result<Vec<ApiKeyRecord>, ApiError> {
    use crate::services::tenant_isolation::{
        acquire_optional_pg_connection, release_optional_pg_connection, PgIsolationScope,
    };

    let scope = Some(PgIsolationScope::default_identity(None));

    let mut conn = acquire_optional_pg_connection(pool, security, scope).await?;

    let rows = sqlx::query_as::<_, ApiKeyRow>(
        r#"
        SELECT key_id, user_id, key_hash, key_prefix, name, scopes,
               is_active, created_at, last_used_at, expires_at
        FROM api_keys
        WHERE key_prefix = $1 AND is_active = true
        "#,
    )
    .bind(prefix)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| ApiError::Internal(format!("api key PG prefix lookup: {e}")))?;

    release_optional_pg_connection(&mut conn, security, scope).await;

    Ok(rows.into_iter().map(api_key_from_row).collect())
}

#[cfg(feature = "postgres")]
async fn revoke_api_key_pg(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
    key_id: &str,
) -> Result<Option<ApiKeyRecord>, ApiError> {
    use crate::services::tenant_isolation::{
        acquire_optional_pg_connection, release_optional_pg_connection, PgIsolationScope,
    };

    let key_uuid = Uuid::parse_str(key_id)
        .map_err(|_| ApiError::Internal("invalid key_id for api key revoke".into()))?;
    let scope = Some(PgIsolationScope::default_identity(None));

    let mut conn = acquire_optional_pg_connection(pool, security, scope).await?;

    let row = sqlx::query_as::<_, ApiKeyRow>(
        r#"
        UPDATE api_keys
        SET is_active = false
        WHERE key_id = $1
        RETURNING key_id, user_id, key_hash, key_prefix, name, scopes,
                  is_active, created_at, last_used_at, expires_at
        "#,
    )
    .bind(key_uuid)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| ApiError::Internal(format!("api key PG revoke: {e}")))?;

    release_optional_pg_connection(&mut conn, security, scope).await;

    Ok(row.map(api_key_from_row))
}

async fn list_api_keys_kv(
    storage: &StorageRuntime,
    user_id: &str,
) -> Result<Vec<ApiKeyRecord>, ApiError> {
    crate::services::auth_memory_store::list_api_keys_for_user(&storage.auth_memory, user_id).await
}

async fn find_api_keys_by_prefix_kv(
    storage: &StorageRuntime,
    prefix: &str,
) -> Result<Vec<ApiKeyRecord>, ApiError> {
    crate::services::auth_memory_store::find_active_api_keys_by_prefix(&storage.auth_memory, prefix)
        .await
}

async fn revoke_api_key_kv(
    storage: &StorageRuntime,
    key_id: &str,
) -> Result<Option<ApiKeyRecord>, ApiError> {
    crate::services::auth_memory_store::revoke_api_key(&storage.auth_memory, key_id).await
}

/// Persist API key record — PG SSOT when pool + policy; optional KV mirror.
pub(crate) async fn persist_api_key(
    storage: &StorageRuntime,
    pg_runtime: Option<&PostgresRuntime>,
    security: &ApiSecurityConfig,
    record: &ApiKeyRecord,
) -> Result<(), ApiError> {
    #[cfg(feature = "postgres")]
    {
        let pool = pg_runtime.and_then(|pg| pg.pool.as_ref());
        let policy = IdentityPolicy::resolve(security, pool.is_some());

        if policy.pg_primary {
            if let Some(pool) = pool {
                persist_api_key_pg(pool, security, record).await?;
            }
        } else {
            persist_api_key_kv(storage, record).await?;
        }

        Ok(())
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (pg_runtime, security);
        persist_api_key_kv(storage, record).await
    }
}

/// List API keys for a user.
pub(crate) async fn list_api_keys_for_user(
    storage: &StorageRuntime,
    pg_runtime: Option<&PostgresRuntime>,
    security: &ApiSecurityConfig,
    user_id: &str,
) -> Result<Vec<ApiKeyRecord>, ApiError> {
    #[cfg(feature = "postgres")]
    {
        let pool = pg_runtime.and_then(|pg| pg.pool.as_ref());
        let policy = IdentityPolicy::resolve(security, pool.is_some());

        if policy.pg_primary {
            if let Some(pool) = pool {
                return list_api_keys_pg(pool, security, user_id).await;
            }
            return Ok(Vec::new());
        }

        list_api_keys_kv(storage, user_id).await
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (pg_runtime, security);
        list_api_keys_kv(storage, user_id).await
    }
}

/// Active API keys matching prefix (for credential validation).
pub(crate) async fn find_active_api_keys_by_prefix(
    storage: &StorageRuntime,
    pg_runtime: Option<&PostgresRuntime>,
    security: &ApiSecurityConfig,
    prefix: &str,
) -> Result<Vec<ApiKeyRecord>, ApiError> {
    #[cfg(feature = "postgres")]
    {
        let pool = pg_runtime.and_then(|pg| pg.pool.as_ref());
        let policy = IdentityPolicy::resolve(security, pool.is_some());

        if policy.pg_primary {
            if let Some(pool) = pool {
                return find_api_keys_by_prefix_pg(pool, security, prefix).await;
            }
            return Ok(Vec::new());
        }

        find_api_keys_by_prefix_kv(storage, prefix).await
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (pg_runtime, security);
        find_api_keys_by_prefix_kv(storage, prefix).await
    }
}

/// Revoke API key by id; returns updated record when found.
pub(crate) async fn revoke_api_key(
    storage: &StorageRuntime,
    pg_runtime: Option<&PostgresRuntime>,
    security: &ApiSecurityConfig,
    key_id: &str,
) -> Result<Option<ApiKeyRecord>, ApiError> {
    #[cfg(feature = "postgres")]
    {
        let pool = pg_runtime.and_then(|pg| pg.pool.as_ref());
        let policy = IdentityPolicy::resolve(security, pool.is_some());

        if policy.pg_primary {
            if let Some(pool) = pool {
                let record = revoke_api_key_pg(pool, security, key_id).await?;
                return Ok(record);
            }
            return Ok(None);
        }

        revoke_api_key_kv(storage, key_id).await
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (pg_runtime, security);
        revoke_api_key_kv(storage, key_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_token_hash_is_deterministic() {
        let a = refresh_token_lookup_hash("abc-123");
        let b = refresh_token_lookup_hash("abc-123");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }
}
