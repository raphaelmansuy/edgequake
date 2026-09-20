//! PostgreSQL implementation of refresh-token and API-key persistence.

use async_trait::async_trait;
use edgequake_storage::contracts::{AccessError, AccessResult, ApiKey, RefreshToken, SessionStore};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresSessionStore {
    pool: PgPool,
}

impl PostgresSessionStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionStore for PostgresSessionStore {
    async fn put_refresh_token(&self, token: &RefreshToken) -> AccessResult<()> {
        sqlx::query(
            "INSERT INTO refresh_tokens \
                 (token_id,user_id,token_hash,expires_at,revoked,created_at) \
             VALUES ($1,$2,$3,$4,$5,$6) \
             ON CONFLICT (token_id) DO UPDATE SET \
                 token_hash=EXCLUDED.token_hash, expires_at=EXCLUDED.expires_at, \
                 revoked=EXCLUDED.revoked",
        )
        .bind(token.token_id)
        .bind(token.user_id)
        .bind(&token.token_hash)
        .bind(token.expires_at)
        .bind(token.revoked)
        .bind(token.created_at)
        .execute(&self.pool)
        .await
        .map_err(database_error)?;
        Ok(())
    }

    async fn get_refresh_token(&self, token_hash: &str) -> AccessResult<Option<RefreshToken>> {
        sqlx::query_as::<_, RefreshTokenRow>(
            "SELECT token_id,user_id,token_hash,expires_at,revoked,created_at \
             FROM refresh_tokens WHERE token_hash=$1 LIMIT 1",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(Into::into))
        .map_err(database_error)
    }

    async fn revoke_refresh_token(&self, token_hash: &str) -> AccessResult<bool> {
        sqlx::query(
            "UPDATE refresh_tokens SET revoked=TRUE, revoked_at=NOW() \
             WHERE token_hash=$1 AND revoked=FALSE",
        )
        .bind(token_hash)
        .execute(&self.pool)
        .await
        .map(|result| result.rows_affected() > 0)
        .map_err(database_error)
    }

    async fn put_api_key(&self, key: &ApiKey) -> AccessResult<()> {
        sqlx::query(
            "INSERT INTO api_keys \
                 (key_id,user_id,key_hash,key_prefix,name,scopes,is_active,created_at,last_used_at,expires_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) \
             ON CONFLICT (key_id) DO UPDATE SET \
                 key_hash=EXCLUDED.key_hash,key_prefix=EXCLUDED.key_prefix, \
                 name=EXCLUDED.name,scopes=EXCLUDED.scopes, \
                 is_active=EXCLUDED.is_active,expires_at=EXCLUDED.expires_at",
        )
        .bind(key.key_id)
        .bind(key.user_id)
        .bind(&key.key_hash)
        .bind(&key.prefix)
        .bind(&key.name)
        .bind(&key.scopes)
        .bind(key.is_active)
        .bind(key.created_at)
        .bind(key.last_used_at)
        .bind(key.expires_at)
        .execute(&self.pool)
        .await
        .map_err(database_error)?;
        Ok(())
    }

    async fn list_api_keys(&self, user_id: Uuid) -> AccessResult<Vec<ApiKey>> {
        load_api_keys(
            &self.pool,
            "SELECT key_id,user_id,key_hash,key_prefix,name,scopes,is_active,created_at,last_used_at,expires_at \
             FROM api_keys WHERE user_id=$1 ORDER BY created_at DESC",
            user_id,
        )
        .await
    }

    async fn find_api_keys_by_prefix(&self, prefix: &str) -> AccessResult<Vec<ApiKey>> {
        sqlx::query_as::<_, ApiKeyRow>(
            "SELECT key_id,user_id,key_hash,key_prefix,name,scopes,is_active,created_at,last_used_at,expires_at \
             FROM api_keys WHERE key_prefix=$1 AND is_active=TRUE",
        )
        .bind(prefix)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(database_error)
    }

    async fn revoke_api_key(&self, key_id: Uuid) -> AccessResult<Option<ApiKey>> {
        sqlx::query_as::<_, ApiKeyRow>(
            "UPDATE api_keys SET is_active=FALSE WHERE key_id=$1 \
             RETURNING key_id,user_id,key_hash,key_prefix,name,scopes,is_active,created_at,last_used_at,expires_at",
        )
        .bind(key_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(Into::into))
        .map_err(database_error)
    }
}

async fn load_api_keys(pool: &PgPool, query: &str, user_id: Uuid) -> AccessResult<Vec<ApiKey>> {
    sqlx::query_as::<_, ApiKeyRow>(query)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(database_error)
}

#[derive(sqlx::FromRow)]
struct RefreshTokenRow {
    token_id: Uuid,
    user_id: Uuid,
    token_hash: String,
    expires_at: chrono::DateTime<chrono::Utc>,
    revoked: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<RefreshTokenRow> for RefreshToken {
    fn from(row: RefreshTokenRow) -> Self {
        Self {
            token_id: row.token_id,
            user_id: row.user_id,
            token_hash: row.token_hash,
            expires_at: row.expires_at,
            revoked: row.revoked,
            created_at: row.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ApiKeyRow {
    key_id: Uuid,
    user_id: Uuid,
    key_hash: String,
    key_prefix: String,
    name: Option<String>,
    scopes: Option<Vec<String>>,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<ApiKeyRow> for ApiKey {
    fn from(row: ApiKeyRow) -> Self {
        Self {
            key_id: row.key_id,
            user_id: row.user_id,
            key_hash: row.key_hash,
            prefix: row.key_prefix,
            name: row.name,
            scopes: row.scopes.unwrap_or_default(),
            is_active: row.is_active,
            created_at: row.created_at,
            last_used_at: row.last_used_at,
            expires_at: row.expires_at,
        }
    }
}

fn database_error(error: sqlx::Error) -> AccessError {
    AccessError::Unavailable(format!("session store: {error}"))
}
