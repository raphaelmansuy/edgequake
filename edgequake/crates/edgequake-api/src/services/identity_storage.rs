//! Identity & rights storage SSOT (SPEC-027 phase 33–38 — First Principles).
//!
//! ## Storage layers (code is law — phase 38)
//!
//! | Concern | SSOT (postgres deploy) | Fallback (no PG pool) |
//! |---------|------------------------|------------------------|
//! | Login credentials + lockout | PostgreSQL `users` | in-memory `AuthMemoryStore` |
//! | Global RBAC role | JWT claim + PG `users.role` | memory user `role` |
//! | Tenant/workspace membership | PostgreSQL `memberships` | — |
//! | Refresh tokens + API keys | PostgreSQL `refresh_tokens` / `api_keys` | in-memory store |
//! | Data isolation (documents/graph) | PG RLS + handler filters | KV metadata filters |
//!
//! **First Principles (phase 47):** PostgreSQL is the sole auth SSOT when a pool is available.
//! KV is used only when no PG pool (in-memory tests). `EDGEQUAKE_KV_IDENTITY_MIRROR` is ignored when pool exists.

use edgequake_auth::{Claims, Role};
use uuid::Uuid;

use crate::middleware::default_tenant_uuid;
use crate::state::ApiSecurityConfig;

#[cfg(feature = "postgres")]
use crate::error::ApiError;
#[cfg(feature = "postgres")]
use crate::handlers::auth::UserRecord;
#[cfg(feature = "postgres")]
use crate::state::{PostgresRuntime, StorageRuntime};
#[cfg(feature = "postgres")]
use chrono::{DateTime, Utc};

/// Resolved identity persistence policy (SPEC-027 phase 38).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentityPolicy {
    /// PostgreSQL is authoritative for reads/writes.
    pub pg_primary: bool,
    /// Mirror writes to KV — effective only when `!pg_primary` (test harness).
    pub kv_mirror: bool,
}

impl IdentityPolicy {
    /// Resolve from security flags and pool availability.
    ///
    /// Phase 47: when PostgreSQL pool + `pg_identity_ssot`, KV mirror env is **ignored**
    /// (ascending-compat: env still parsed but never applied).
    pub fn resolve(security: &ApiSecurityConfig, has_pg: bool) -> Self {
        if has_pg && security.pg_identity_ssot {
            Self {
                pg_primary: true,
                kv_mirror: false,
            }
        } else {
            Self {
                pg_primary: false,
                kv_mirror: true,
            }
        }
    }

    /// KV is authoritative for auth **reads** only when PostgreSQL pool is unavailable (tests).
    pub fn kv_auth_reads_enabled(self) -> bool {
        !self.pg_primary
    }

    /// KV receives auth **writes** only in test-harness mode (no PG pool).
    pub fn kv_auth_writes_enabled(self) -> bool {
        !self.pg_primary
    }

    /// Operator-facing label for `/health` capabilities (SPEC-027 phase 45).
    pub fn identity_backend_label(self) -> &'static str {
        if self.pg_primary {
            "postgresql"
        } else {
            "in-memory"
        }
    }
}

/// Constant-time equality for env-configured API keys (SPEC-027 SEC-010).
pub fn constant_time_str_eq(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.bytes().zip(right.bytes()) {
        diff |= a ^ b;
    }
    diff == 0
}

/// Default tenant + workspace UUIDs (aligned with `WorkspaceServiceImpl::ensure_defaults`).
pub fn default_identity_scope() -> (Uuid, Uuid) {
    (
        default_tenant_uuid(),
        edgequake_core::default_workspace_uuid(),
    )
}

/// JWT claims with default tenant/workspace scope (SPEC-027 phase 34).
pub fn access_token_claims(user_id: Uuid, role: Role, expiry_seconds: i64) -> Claims {
    let (tenant_id, workspace_id) = default_identity_scope();
    Claims::new(user_id, role, expiry_seconds)
        .with_tenant_id(tenant_id.to_string())
        .with_workspace_id(workspace_id.to_string())
}

/// Map global auth role to PostgreSQL membership role string.
pub fn membership_role_from_global_role(role: &str) -> &'static str {
    match role.to_ascii_lowercase().as_str() {
        "admin" => "admin",
        "readonly" => "readonly",
        _ => "member",
    }
}

/// Upsert auth user into PostgreSQL (identity SSOT write path).
#[cfg(feature = "postgres")]
pub(crate) async fn sync_auth_user_to_postgres(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
    record: &UserRecord,
) -> Result<(), ApiError> {
    use crate::services::tenant_isolation::{with_optional_pg_rls, PgIsolationScope};
    use edgequake_storage::StorageError;

    let user_uuid = Uuid::parse_str(&record.user_id)
        .map_err(|_| ApiError::Internal("Invalid user_id for PG sync".into()))?;
    let (tenant_id, _) = default_identity_scope();
    let scope = Some(PgIsolationScope::default_identity(Some(user_uuid)));
    let username = record.username.clone();
    let email = record.email.clone();
    let password_hash = record.password_hash.clone();
    let role = record.role.clone();
    let is_active = record.is_active;
    let failed_login_attempts = record.failed_login_attempts as i32;
    let locked_until = record.locked_until;
    let created_at = record.created_at;
    let updated_at = record.updated_at;
    let last_login_at = record.last_login_at;

    with_optional_pg_rls(pool, security, scope, move |conn| {
        Box::pin(async move {
            sqlx::query(
                r#"
                INSERT INTO users (
                    user_id, tenant_id, username, email, password_hash, role, is_active,
                    failed_login_attempts, locked_until, created_at, updated_at, last_login_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                ON CONFLICT (user_id) DO UPDATE SET
                    username = EXCLUDED.username,
                    email = EXCLUDED.email,
                    password_hash = EXCLUDED.password_hash,
                    role = EXCLUDED.role,
                    is_active = EXCLUDED.is_active,
                    failed_login_attempts = EXCLUDED.failed_login_attempts,
                    locked_until = EXCLUDED.locked_until,
                    updated_at = EXCLUDED.updated_at,
                    last_login_at = EXCLUDED.last_login_at
                "#,
            )
            .bind(user_uuid)
            .bind(tenant_id)
            .bind(username)
            .bind(email)
            .bind(password_hash)
            .bind(role)
            .bind(is_active)
            .bind(failed_login_attempts)
            .bind(locked_until)
            .bind(created_at)
            .bind(updated_at)
            .bind(last_login_at)
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("PG user sync failed: {e}")))?;
            Ok(())
        })
    })
    .await?;

    sync_default_membership_to_postgres(pool, security, user_uuid, &record.role).await?;

    Ok(())
}

/// Ensure default tenant and workspace rows exist (idempotent).
#[cfg(feature = "postgres")]
pub async fn ensure_default_tenant_workspace(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
) -> Result<(), ApiError> {
    use crate::services::tenant_isolation::{with_optional_pg_rls, PgIsolationScope};
    use edgequake_storage::StorageError;

    let (tenant_id, workspace_id) = default_identity_scope();
    let scope = Some(PgIsolationScope::default_identity(None));

    with_optional_pg_rls(pool, security, scope, move |conn| {
        Box::pin(async move {
            sqlx::query(
                r#"
                INSERT INTO tenants (tenant_id, name, slug, is_active, metadata, settings, created_at, updated_at)
                VALUES ($1, 'Default', 'default', TRUE,
                        '{"plan": "pro", "max_workspaces": 100, "max_users": 100, "description": "Default tenant"}'::jsonb,
                        '{}'::jsonb, NOW(), NOW())
                ON CONFLICT (tenant_id) DO NOTHING
                "#,
            )
            .bind(tenant_id)
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("PG default tenant ensure failed: {e}")))?;

            sqlx::query(
                r#"
                INSERT INTO workspaces (workspace_id, tenant_id, name, slug, description, is_active, metadata, settings, created_at, updated_at)
                VALUES ($1, $2, 'Default Workspace', 'default', 'Default knowledge base', TRUE,
                        '{}'::jsonb, '{}'::jsonb, NOW(), NOW())
                ON CONFLICT (workspace_id) DO NOTHING
                "#,
            )
            .bind(workspace_id)
            .bind(tenant_id)
            .execute(&mut *conn)
            .await
            .map_err(|e| {
                StorageError::Database(format!("PG default workspace ensure failed: {e}"))
            })?;

            Ok(())
        })
    })
    .await
}

/// Upsert default tenant/workspace membership for a user (SPEC-027 phase 34).
#[cfg(feature = "postgres")]
pub async fn sync_default_membership_to_postgres(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
    user_id: Uuid,
    global_role: &str,
) -> Result<(), ApiError> {
    use crate::services::tenant_isolation::{with_optional_pg_rls, PgIsolationScope};
    use edgequake_storage::StorageError;

    ensure_default_tenant_workspace(pool, security).await?;
    let (tenant_id, workspace_id) = default_identity_scope();
    let membership_role = membership_role_from_global_role(global_role);
    let scope = Some(PgIsolationScope::default_identity(Some(user_id)));

    with_optional_pg_rls(pool, security, scope, move |conn| {
        Box::pin(async move {
            sqlx::query(
                r#"
                INSERT INTO memberships (tenant_id, workspace_id, user_id, role, is_active)
                VALUES ($1, $2, $3, $4, TRUE)
                ON CONFLICT (user_id, tenant_id, workspace_id) DO UPDATE SET
                    role = EXCLUDED.role,
                    is_active = TRUE
                "#,
            )
            .bind(tenant_id)
            .bind(workspace_id)
            .bind(user_id)
            .bind(membership_role)
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("PG membership sync failed: {e}")))?;
            Ok(())
        })
    })
    .await
}

/// Verify active membership when strict tenant bind + PostgreSQL are available.
#[cfg(feature = "postgres")]
pub async fn verify_membership_active(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
    user_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Uuid,
) -> Result<bool, ApiError> {
    use crate::services::tenant_isolation::{with_optional_pg_rls, PgIsolationScope};
    use edgequake_storage::StorageError;

    let scope = Some(PgIsolationScope::for_membership(
        tenant_id,
        workspace_id,
        user_id,
    ));

    with_optional_pg_rls(pool, security, scope, move |conn| {
        Box::pin(async move {
            let exists = sqlx::query_scalar::<_, bool>(
                r#"
                SELECT EXISTS(
                    SELECT 1 FROM memberships
                    WHERE user_id = $1 AND tenant_id = $2 AND workspace_id = $3 AND is_active = TRUE
                )
                "#,
            )
            .bind(user_id)
            .bind(tenant_id)
            .bind(workspace_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Membership lookup failed: {e}")))?;

            Ok(exists)
        })
    })
    .await
}

/// DNS namespace UUID for deterministic per-tenant guest ids (SPEC-087 / Issue #335).
///
/// Fixed bytes — do not change; existing guest rows depend on this constant.
pub const EDGEQUAKE_GUEST_NAMESPACE: Uuid = Uuid::from_bytes([
    0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8,
]);

/// Stable shared guest markers (admin filter + login rejection).
pub const SHARED_GUEST_USERNAME: &str = "guest";
pub const SHARED_GUEST_EMAIL: &str = "guest@anonymous.local";
pub const ANONYMOUS_PASSWORD_HASH: &str = "anonymous";

/// Deterministic per-tenant guest user id (one row per tenant when auth is off).
pub fn shared_guest_user_id(tenant_id: Uuid) -> Uuid {
    Uuid::new_v5(&EDGEQUAKE_GUEST_NAMESPACE, tenant_id.as_bytes())
}

/// True when a stored user is an anonymous/guest system account (SPEC-087).
pub fn is_anonymous_identity(username: &str, email: &str, password_hash: &str) -> bool {
    let hash = password_hash.trim();
    hash == ANONYMOUS_PASSWORD_HASH
        || hash == "not_a_real_hash"
        || email.ends_with("@anonymous.local")
        || username == SHARED_GUEST_USERNAME
        || username.starts_with("anon_")
}

/// Ensure the shared per-tenant guest user exists (SPEC-087 / Issue #335).
///
/// Replaces per-browser `anon_*` minting. FK-safe: conversations reference this single row.
#[cfg(feature = "postgres")]
pub async fn ensure_shared_guest_user_in_postgres(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
    tenant_id: Uuid,
    guest_user_id: Uuid,
) -> Result<(), ApiError> {
    use crate::services::tenant_isolation::{with_optional_pg_rls, PgIsolationScope};
    use edgequake_storage::StorageError;

    let workspace_id = edgequake_core::default_workspace_uuid();
    let scope = Some(PgIsolationScope::for_membership(
        tenant_id,
        workspace_id,
        guest_user_id,
    ));
    let username = SHARED_GUEST_USERNAME.to_string();
    let email = SHARED_GUEST_EMAIL.to_string();

    with_optional_pg_rls(pool, security, scope, move |conn| {
        Box::pin(async move {
            sqlx::query(
                r#"
                INSERT INTO users (user_id, tenant_id, username, email, password_hash, role, is_active, created_at, updated_at)
                VALUES ($1, $2, $3, $4, 'anonymous', 'user', TRUE, NOW(), NOW())
                ON CONFLICT (user_id) DO NOTHING
                "#,
            )
            .bind(guest_user_id)
            .bind(tenant_id)
            .bind(username)
            .bind(email)
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("shared guest user ensure failed: {e}")))?;
            Ok(())
        })
    })
    .await
}

/// Legacy name retained for grep/docs; delegates to [`ensure_shared_guest_user_in_postgres`].
///
/// Prefer calling the shared-guest helper directly. The `user_id` argument is ignored in
/// favor of [`shared_guest_user_id`].
#[cfg(feature = "postgres")]
#[deprecated(note = "SPEC-087: use ensure_shared_guest_user_in_postgres + shared_guest_user_id")]
pub async fn ensure_anonymous_user_in_postgres(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
    tenant_id: Uuid,
    _user_id: Uuid,
) -> Result<(), ApiError> {
    let guest_id = shared_guest_user_id(tenant_id);
    ensure_shared_guest_user_in_postgres(pool, security, tenant_id, guest_id).await
}

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
struct PgUserRow {
    user_id: Uuid,
    username: String,
    email: String,
    password_hash: String,
    role: String,
    is_active: bool,
    failed_login_attempts: i32,
    locked_until: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_login_at: Option<DateTime<Utc>>,
}

#[cfg(feature = "postgres")]
fn pg_row_to_user_record(row: PgUserRow) -> UserRecord {
    UserRecord {
        user_id: row.user_id.to_string(),
        username: row.username,
        email: row.email,
        password_hash: row.password_hash,
        role: row.role,
        is_active: row.is_active,
        created_at: row.created_at,
        updated_at: row.updated_at,
        last_login_at: row.last_login_at,
        failed_login_attempts: row.failed_login_attempts.max(0) as u32,
        locked_until: row.locked_until,
        metadata: std::collections::HashMap::new(),
    }
}

#[cfg(feature = "postgres")]
async fn load_user_record_pg(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
    user_id: &str,
) -> Result<Option<UserRecord>, ApiError> {
    use crate::services::tenant_isolation::{with_optional_pg_rls, PgIsolationScope};
    use edgequake_storage::StorageError;

    let user_uuid = Uuid::parse_str(user_id)
        .map_err(|_| ApiError::Internal("Invalid user_id for PG load".into()))?;
    let (tenant_id, _) = default_identity_scope();
    let scope = Some(PgIsolationScope::default_identity(Some(user_uuid)));

    with_optional_pg_rls(pool, security, scope, move |conn| {
        Box::pin(async move {
            let row = sqlx::query_as::<_, PgUserRow>(
                r#"
                SELECT user_id, username, email, password_hash, role, is_active,
                       COALESCE(failed_login_attempts, 0) AS failed_login_attempts,
                       locked_until, created_at, updated_at, last_login_at
                FROM users
                WHERE user_id = $1 AND tenant_id = $2
                "#,
            )
            .bind(user_uuid)
            .bind(tenant_id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("PG user load failed: {e}")))?;

            Ok(row.map(pg_row_to_user_record))
        })
    })
    .await
}

#[cfg(feature = "postgres")]
async fn find_user_record_by_login_pg(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
    login: &str,
) -> Result<Option<UserRecord>, ApiError> {
    use crate::services::tenant_isolation::{with_optional_pg_rls, PgIsolationScope};
    use edgequake_storage::StorageError;

    let (tenant_id, _) = default_identity_scope();
    let login_lower = login.to_ascii_lowercase();
    let scope = Some(PgIsolationScope::default_identity(None));

    with_optional_pg_rls(pool, security, scope, move |conn| {
        Box::pin(async move {
            let row = sqlx::query_as::<_, PgUserRow>(
                r#"
                SELECT user_id, username, email, password_hash, role, is_active,
                       COALESCE(failed_login_attempts, 0) AS failed_login_attempts,
                       locked_until, created_at, updated_at, last_login_at
                FROM users
                WHERE tenant_id = $1
                  AND (lower(username) = $2 OR lower(email) = $2)
                LIMIT 1
                "#,
            )
            .bind(tenant_id)
            .bind(&login_lower)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("PG user login lookup failed: {e}")))?;

            Ok(row.map(pg_row_to_user_record))
        })
    })
    .await
}

/// List all users for default tenant from PostgreSQL.
#[cfg(feature = "postgres")]
pub(crate) async fn list_user_records_pg(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
) -> Result<Vec<UserRecord>, ApiError> {
    use crate::services::tenant_isolation::{with_optional_pg_rls, PgIsolationScope};
    use edgequake_storage::StorageError;

    let (tenant_id, _) = default_identity_scope();
    let scope = Some(PgIsolationScope::default_identity(None));

    with_optional_pg_rls(pool, security, scope, move |conn| {
        Box::pin(async move {
            let rows = sqlx::query_as::<_, PgUserRow>(
                r#"
                SELECT user_id, username, email, password_hash, role, is_active,
                       COALESCE(failed_login_attempts, 0) AS failed_login_attempts,
                       locked_until, created_at, updated_at, last_login_at
                FROM users
                WHERE tenant_id = $1
                ORDER BY username
                "#,
            )
            .bind(tenant_id)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("PG user list failed: {e}")))?;

            Ok(rows.into_iter().map(pg_row_to_user_record).collect())
        })
    })
    .await
}

/// Delete user from PostgreSQL (identity SSOT).
#[cfg(feature = "postgres")]
pub async fn delete_user_pg(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
    user_id: &str,
) -> Result<(), ApiError> {
    use crate::services::tenant_isolation::{with_optional_pg_rls, PgIsolationScope};
    use edgequake_storage::StorageError;

    let user_uuid = Uuid::parse_str(user_id)
        .map_err(|_| ApiError::Internal("Invalid user_id for PG delete".into()))?;
    let (tenant_id, _) = default_identity_scope();
    let scope = Some(PgIsolationScope::default_identity(Some(user_uuid)));

    with_optional_pg_rls(pool, security, scope, move |conn| {
        Box::pin(async move {
            sqlx::query("DELETE FROM users WHERE user_id = $1 AND tenant_id = $2")
                .bind(user_uuid)
                .bind(tenant_id)
                .execute(&mut *conn)
                .await
                .map_err(|e| StorageError::Database(format!("PG user delete failed: {e}")))?;
            Ok(())
        })
    })
    .await
}

/// Load user record — PG SSOT when pool + policy; KV only when no pool (tests).
#[cfg(feature = "postgres")]
pub(crate) async fn load_user_record(
    storage: &StorageRuntime,
    pg_runtime: Option<&PostgresRuntime>,
    security: &ApiSecurityConfig,
    user_id: &str,
) -> Result<Option<UserRecord>, ApiError> {
    let pool = pg_runtime.and_then(|pg| pg.pool.as_ref());
    let policy = IdentityPolicy::resolve(security, pool.is_some());

    if policy.pg_primary {
        if let Some(pool) = pool {
            return load_user_record_pg(pool, security, user_id).await;
        }
        return Ok(None);
    }

    crate::services::auth_memory_store::get_user_record_by_id(&storage.auth_memory, user_id).await
}

/// Find user by username or email — PG SSOT when available.
#[cfg(feature = "postgres")]
pub(crate) async fn find_user_record_by_login(
    storage: &StorageRuntime,
    pg_runtime: Option<&PostgresRuntime>,
    security: &ApiSecurityConfig,
    login: &str,
) -> Result<Option<UserRecord>, ApiError> {
    let pool = pg_runtime.and_then(|pg| pg.pool.as_ref());
    let policy = IdentityPolicy::resolve(security, pool.is_some());

    if policy.pg_primary {
        if let Some(pool) = pool {
            return find_user_record_by_login_pg(pool, security, login).await;
        }
        return Ok(None);
    }

    crate::services::auth_memory_store::find_user_record_by_login(&storage.auth_memory, login).await
}

/// Persist user record — PostgreSQL SSOT when pool; KV test harness otherwise.
#[cfg(feature = "postgres")]
pub(crate) async fn persist_user_record(
    storage: &StorageRuntime,
    pg_runtime: Option<&PostgresRuntime>,
    security: &ApiSecurityConfig,
    record: &UserRecord,
) -> Result<(), ApiError> {
    let pool = pg_runtime.and_then(|pg| pg.pool.as_ref());
    let policy = IdentityPolicy::resolve(security, pool.is_some());

    if policy.pg_primary {
        if let Some(pool) = pool {
            sync_auth_user_to_postgres(pool, security, record).await?;
        }
    } else {
        crate::services::auth_memory_store::persist_user_record(&storage.auth_memory, record)
            .await?;
    }

    Ok(())
}

/// Count users that can authenticate with username/password (GitHub #288).
#[cfg(feature = "postgres")]
pub(crate) async fn count_login_capable_users_pg(
    pool: &sqlx::PgPool,
    security: &ApiSecurityConfig,
) -> Result<u32, ApiError> {
    let users = list_user_records_pg(pool, security).await?;
    Ok(users
        .iter()
        .filter(|user| {
            crate::services::auth_bootstrap::is_login_capable_password_hash(&user.password_hash)
        })
        .count() as u32)
}

/// List all user records — PG SSOT when pool; KV test harness otherwise.
#[cfg(feature = "postgres")]
pub(crate) async fn list_user_records(
    storage: &StorageRuntime,
    pg_runtime: Option<&PostgresRuntime>,
    security: &ApiSecurityConfig,
) -> Result<Vec<UserRecord>, ApiError> {
    let pool = pg_runtime.and_then(|pg| pg.pool.as_ref());
    let policy = IdentityPolicy::resolve(security, pool.is_some());

    if policy.pg_primary {
        if let Some(pool) = pool {
            return list_user_records_pg(pool, security).await;
        }
        return Ok(Vec::new());
    }

    crate::services::auth_memory_store::list_user_records(&storage.auth_memory).await
}

/// Delete user — PG SSOT when pool; in-memory test harness otherwise.
#[cfg(feature = "postgres")]
pub(crate) async fn delete_user_record(
    storage: &StorageRuntime,
    pg_runtime: Option<&PostgresRuntime>,
    security: &ApiSecurityConfig,
    record: &UserRecord,
) -> Result<(), ApiError> {
    let pool = pg_runtime.and_then(|pg| pg.pool.as_ref());
    let policy = IdentityPolicy::resolve(security, pool.is_some());

    if policy.pg_primary {
        if let Some(pool) = pool {
            delete_user_pg(pool, security, &record.user_id).await?;
        }
    } else {
        crate::services::auth_memory_store::delete_user_record(&storage.auth_memory, record)
            .await?;
    }

    Ok(())
}

/// Update KV email index when email changes (test harness only).
#[cfg(feature = "postgres")]
pub(crate) async fn reindex_user_email_kv(
    storage: &StorageRuntime,
    user_id: &str,
    old_email: &str,
    new_email: &str,
) -> Result<(), ApiError> {
    crate::services::auth_memory_store::update_email_index(
        &storage.auth_memory,
        user_id,
        old_email,
        new_email,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_policy_ignores_kv_mirror_when_pool_phase47() {
        let security = ApiSecurityConfig {
            kv_identity_mirror: true,
            ..Default::default()
        };
        let policy = IdentityPolicy::resolve(&security, true);
        assert!(policy.pg_primary);
        assert!(!policy.kv_mirror);
        assert!(!policy.kv_auth_writes_enabled());
        assert!(!policy.kv_auth_reads_enabled());
    }

    #[test]
    fn identity_policy_pg_primary_when_pool_and_flag() {
        let security = ApiSecurityConfig {
            pg_identity_ssot: true,
            kv_identity_mirror: false,
            ..Default::default()
        };
        let policy = IdentityPolicy::resolve(&security, true);
        assert!(policy.pg_primary);
        assert!(!policy.kv_mirror);
    }

    #[test]
    fn identity_policy_kv_reads_only_without_pool() {
        let security = ApiSecurityConfig::default();
        let policy = IdentityPolicy::resolve(&security, false);
        assert!(!policy.pg_primary);
        assert!(policy.kv_auth_reads_enabled());
        assert!(policy.kv_auth_writes_enabled());
    }

    #[test]
    fn identity_policy_pg_reads_no_kv_when_pool() {
        let security = ApiSecurityConfig::default();
        let policy = IdentityPolicy::resolve(&security, true);
        assert!(policy.pg_primary);
        assert!(!policy.kv_auth_reads_enabled());
        assert!(!policy.kv_auth_writes_enabled());
    }

    #[test]
    fn constant_time_eq_matches_and_rejects() {
        assert!(constant_time_str_eq("sk_live_abc", "sk_live_abc"));
        assert!(!constant_time_str_eq("sk_live_abc", "sk_live_abd"));
        assert!(!constant_time_str_eq("short", "longer"));
    }

    #[test]
    fn membership_role_mapping() {
        assert_eq!(membership_role_from_global_role("admin"), "admin");
        assert_eq!(membership_role_from_global_role("Admin"), "admin");
        assert_eq!(membership_role_from_global_role("readonly"), "readonly");
        assert_eq!(membership_role_from_global_role("user"), "member");
    }

    #[test]
    fn access_token_claims_include_default_scope() {
        let user_id = Uuid::new_v4();
        let claims = access_token_claims(user_id, Role::User, 3600);
        let (tenant_id, workspace_id) = default_identity_scope();
        assert_eq!(
            claims.tenant_id.as_deref(),
            Some(tenant_id.to_string().as_str())
        );
        assert_eq!(
            claims.workspace_id.as_deref(),
            Some(workspace_id.to_string().as_str())
        );
    }
}
