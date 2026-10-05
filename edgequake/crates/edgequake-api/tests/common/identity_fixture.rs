//! Driver-free persisted identities for authentication contract fixtures.
use async_trait::async_trait;
use edgequake_storage::contracts::{
    AccessResult, AccessScope, IdentityStore, IdentityUser, TenantId,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct FixtureIdentityStore {
    users: RwLock<HashMap<(TenantId, Uuid), IdentityUser>>,
    workspaces: Arc<dyn edgequake_core::WorkspaceService>,
}
impl FixtureIdentityStore {
    pub fn new(workspaces: Arc<dyn edgequake_core::WorkspaceService>) -> Self {
        let now = chrono::Utc::now();
        let user = IdentityUser {
            user_id: edgequake_api::middleware::default_user_uuid(),
            username: "mcp-fixture".into(),
            email: "mcp-fixture@example.test".into(),
            password_hash: "fixture-no-password-login".into(),
            role: "admin".into(),
            is_active: true,
            failed_login_attempts: 0,
            locked_until: None,
            created_at: now,
            updated_at: now,
            last_login_at: None,
            metadata: serde_json::json!({}),
        };
        Self {
            users: RwLock::new(HashMap::from([(
                (
                    TenantId::new(edgequake_api::middleware::default_tenant_uuid()),
                    user.user_id,
                ),
                user,
            )])),
            workspaces,
        }
    }
}
#[async_trait]
impl IdentityStore for FixtureIdentityStore {
    async fn get_user(&self, tenant: TenantId, user: Uuid) -> AccessResult<Option<IdentityUser>> {
        Ok(self.users.read().await.get(&(tenant, user)).cloned())
    }
    async fn upsert_user(&self, tenant: TenantId, user: &IdentityUser) -> AccessResult<()> {
        self.users
            .write()
            .await
            .insert((tenant, user.user_id), user.clone());
        Ok(())
    }
    async fn membership_active(&self, scope: &AccessScope, user: Uuid) -> AccessResult<bool> {
        let memberships = self
            .workspaces
            .get_user_memberships(user)
            .await
            .map_err(|e| edgequake_storage::contracts::AccessError::Unavailable(e.to_string()))?;
        Ok(memberships.iter().any(|m| {
            m.applies_to(
                scope.tenant().into_uuid(),
                Some(scope.workspace().into_uuid()),
            )
        }))
    }
}
