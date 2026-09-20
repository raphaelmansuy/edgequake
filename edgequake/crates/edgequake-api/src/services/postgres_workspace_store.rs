//! PostgreSQL implementation of scoped workspace reads.

use async_trait::async_trait;
use edgequake_storage::contracts::{
    AccessError, AccessResult, AccessScope, TenantId, WorkspaceRecord, WorkspaceStore,
};
use sqlx::PgPool;

#[derive(Clone)]
pub struct PostgresWorkspaceStore {
    pool: PgPool,
}

impl PostgresWorkspaceStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WorkspaceStore for PostgresWorkspaceStore {
    async fn get_workspace(&self, scope: &AccessScope) -> AccessResult<Option<WorkspaceRecord>> {
        sqlx::query_as::<_, WorkspaceRow>(
            "SELECT workspace_id,tenant_id,name,slug,description,is_active,metadata,created_at,updated_at \
             FROM workspaces WHERE tenant_id=$1 AND workspace_id=$2",
        )
        .bind(scope.tenant().into_uuid())
        .bind(scope.workspace().into_uuid())
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(Into::into))
        .map_err(database_error)
    }

    async fn list_workspaces(&self, tenant_id: TenantId) -> AccessResult<Vec<WorkspaceRecord>> {
        sqlx::query_as::<_, WorkspaceRow>(
            "SELECT workspace_id,tenant_id,name,slug,description,is_active,metadata,created_at,updated_at \
             FROM workspaces WHERE tenant_id=$1 ORDER BY created_at DESC",
        )
        .bind(tenant_id.into_uuid())
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(database_error)
    }
}

#[derive(sqlx::FromRow)]
struct WorkspaceRow {
    workspace_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    name: String,
    slug: String,
    description: Option<String>,
    is_active: bool,
    metadata: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<WorkspaceRow> for WorkspaceRecord {
    fn from(row: WorkspaceRow) -> Self {
        Self {
            workspace_id: edgequake_storage::contracts::WorkspaceId::new(row.workspace_id),
            tenant_id: TenantId::new(row.tenant_id),
            name: row.name,
            slug: row.slug,
            description: row.description,
            is_active: row.is_active,
            metadata: row.metadata,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

fn database_error(error: sqlx::Error) -> AccessError {
    AccessError::Unavailable(format!("workspace store: {error}"))
}
