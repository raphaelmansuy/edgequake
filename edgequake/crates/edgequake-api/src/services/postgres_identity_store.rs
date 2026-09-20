//! PostgreSQL implementation of the driver-free identity port.

use async_trait::async_trait;
use edgequake_storage::contracts::{
    AccessError, AccessResult, AccessScope, IdentityStore, IdentityUser, TenantId,
};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresIdentityStore {
    pool: PgPool,
}

impl PostgresIdentityStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IdentityStore for PostgresIdentityStore {
    async fn get_user(
        &self,
        tenant_id: TenantId,
        user_id: Uuid,
    ) -> AccessResult<Option<IdentityUser>> {
        let row = sqlx::query_as::<_, IdentityRow>(
            "SELECT user_id, username, email, password_hash, role, is_active, \
                    COALESCE(failed_login_attempts, 0) AS failed_login_attempts, \
                    locked_until, created_at, updated_at, last_login_at \
             FROM users WHERE user_id = $1 AND tenant_id = $2",
        )
        .bind(user_id)
        .bind(tenant_id.into_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)?;
        row.map(IdentityRow::try_into_identity).transpose()
    }

    async fn upsert_user(&self, tenant_id: TenantId, user: &IdentityUser) -> AccessResult<()> {
        let failed_login_attempts = i32::try_from(user.failed_login_attempts)
            .map_err(|_| AccessError::InvalidInput("failed login attempts exceed i32".into()))?;
        sqlx::query(
            "INSERT INTO users (\
                 user_id, tenant_id, username, email, password_hash, role, is_active, \
                 failed_login_attempts, locked_until, created_at, updated_at, last_login_at\
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) \
             ON CONFLICT (user_id) DO UPDATE SET \
                 username=EXCLUDED.username, email=EXCLUDED.email, \
                 password_hash=EXCLUDED.password_hash, role=EXCLUDED.role, \
                 is_active=EXCLUDED.is_active, \
                 failed_login_attempts=EXCLUDED.failed_login_attempts, \
                 locked_until=EXCLUDED.locked_until, updated_at=EXCLUDED.updated_at, \
                 last_login_at=EXCLUDED.last_login_at",
        )
        .bind(user.user_id)
        .bind(tenant_id.into_uuid())
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.role)
        .bind(user.is_active)
        .bind(failed_login_attempts)
        .bind(user.locked_until)
        .bind(user.created_at)
        .bind(user.updated_at)
        .bind(user.last_login_at)
        .execute(&self.pool)
        .await
        .map_err(database_error)?;
        Ok(())
    }

    async fn membership_active(&self, scope: &AccessScope, user_id: Uuid) -> AccessResult<bool> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM memberships \
             WHERE user_id=$1 AND tenant_id=$2 AND workspace_id=$3 AND is_active=TRUE)",
        )
        .bind(user_id)
        .bind(scope.tenant().into_uuid())
        .bind(scope.workspace().into_uuid())
        .fetch_one(&self.pool)
        .await
        .map_err(database_error)
    }
}

#[derive(sqlx::FromRow)]
struct IdentityRow {
    user_id: Uuid,
    username: String,
    email: String,
    password_hash: String,
    role: String,
    is_active: bool,
    failed_login_attempts: i32,
    locked_until: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    last_login_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl IdentityRow {
    fn try_into_identity(self) -> AccessResult<IdentityUser> {
        Ok(IdentityUser {
            user_id: self.user_id,
            username: self.username,
            email: self.email,
            password_hash: self.password_hash,
            role: self.role,
            is_active: self.is_active,
            failed_login_attempts: u32::try_from(self.failed_login_attempts)
                .map_err(|_| AccessError::CorruptData("negative failed login attempts".into()))?,
            locked_until: self.locked_until,
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_login_at: self.last_login_at,
            metadata: serde_json::Value::Object(Default::default()),
        })
    }
}

fn database_error(error: sqlx::Error) -> AccessError {
    AccessError::Unavailable(format!("identity store: {error}"))
}
