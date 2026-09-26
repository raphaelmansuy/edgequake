//! Operational relational ports used by API and orchestration layers.
//!
//! These contracts intentionally contain no SQLx types. Provider adapters own
//! transaction and SQL details while callers depend only on scoped records.

use crate::{AccessResult, AccessScope, TenantId, WorkspaceId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdentityUser {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub is_active: bool,
    pub failed_login_attempts: u32,
    pub locked_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: Value,
}

#[async_trait]
pub trait IdentityStore: Send + Sync {
    async fn get_user(
        &self,
        tenant_id: TenantId,
        user_id: Uuid,
    ) -> AccessResult<Option<IdentityUser>>;

    async fn upsert_user(&self, tenant_id: TenantId, user: &IdentityUser) -> AccessResult<()>;

    async fn membership_active(&self, scope: &AccessScope, user_id: Uuid) -> AccessResult<bool>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshToken {
    pub token_id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKey {
    pub key_id: Uuid,
    pub user_id: Uuid,
    pub key_hash: String,
    pub prefix: String,
    pub name: Option<String>,
    pub scopes: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn put_refresh_token(&self, token: &RefreshToken) -> AccessResult<()>;
    async fn get_refresh_token(&self, token_hash: &str) -> AccessResult<Option<RefreshToken>>;
    async fn revoke_refresh_token(&self, token_hash: &str) -> AccessResult<bool>;

    async fn put_api_key(&self, key: &ApiKey) -> AccessResult<()>;
    async fn list_api_keys(&self, user_id: Uuid) -> AccessResult<Vec<ApiKey>>;
    async fn find_api_keys_by_prefix(&self, prefix: &str) -> AccessResult<Vec<ApiKey>>;
    async fn revoke_api_key(
        &self,
        owner_user_id: Uuid,
        key_id: Uuid,
    ) -> AccessResult<Option<ApiKey>>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceRecord {
    pub workspace_id: WorkspaceId,
    pub tenant_id: TenantId,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub is_active: bool,
    #[serde(default)]
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
pub trait WorkspaceStore: Send + Sync {
    async fn get_workspace(&self, scope: &AccessScope) -> AccessResult<Option<WorkspaceRecord>>;
    async fn list_workspaces(&self, tenant_id: TenantId) -> AccessResult<Vec<WorkspaceRecord>>;
}

#[async_trait]
pub trait CheckpointArtifactStore: Send + Sync {
    async fn put_checkpoint(
        &self,
        document_id: Uuid,
        kind: &str,
        payload: &Value,
    ) -> AccessResult<()>;
    async fn get_checkpoint(&self, document_id: Uuid, kind: &str) -> AccessResult<Option<Value>>;
    async fn delete_checkpoint(&self, document_id: Uuid, kind: &str) -> AccessResult<()>;
    async fn cleanup_stale_checkpoints(&self, max_age_secs: u64) -> AccessResult<u64>;

    async fn put_artifact(
        &self,
        document_id: Uuid,
        kind: &str,
        payload: &Value,
    ) -> AccessResult<()>;
    async fn get_artifact(&self, document_id: Uuid, kind: &str) -> AccessResult<Option<Value>>;
    async fn delete_artifacts(&self, document_id: Uuid) -> AccessResult<()>;
}
