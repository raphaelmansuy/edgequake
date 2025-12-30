//! PostgreSQL-backed workspace service implementation.
//!
//! This service provides persistent tenant and workspace management
//! using PostgreSQL as the backend storage.
//!
//! NOTE: The actual database schema stores plan, max_workspaces, max_users
//! in the `metadata` JSONB column rather than as separate columns.

use async_trait::async_trait;
use edgequake_core::{
    CreateWorkspaceRequest, Error, Membership, MembershipRole, Result, Tenant, TenantContext,
    TenantPlan, UpdateWorkspaceRequest, Workspace, WorkspaceService, WorkspaceStats,
};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

/// PostgreSQL-backed implementation of WorkspaceService.
///
/// This implementation persists all tenant and workspace data directly
/// to PostgreSQL, ensuring data survives application restarts.
pub struct PostgresWorkspaceService {
    pool: PgPool,
}

impl PostgresWorkspaceService {
    /// Create a new PostgreSQL workspace service.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Generate a URL-friendly slug from a name.
    fn generate_slug(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }

    /// Ensure default tenant and workspace exist.
    /// Returns the default tenant ID and workspace ID.
    pub async fn ensure_defaults(&self) -> Result<(Uuid, Uuid)> {
        let default_tenant_id = Uuid::parse_str("00000000-0000-0000-0000-000000000002")
            .expect("Invalid default tenant UUID");
        let default_workspace_id = Uuid::parse_str("00000000-0000-0000-0000-000000000003")
            .expect("Invalid default workspace UUID");

        // Ensure default tenant exists
        // Schema: tenant_id, name, slug, settings, metadata, is_active, created_at, updated_at
        // Note: plan, max_workspaces, max_users stored in metadata JSONB
        sqlx::query(
            r#"
            INSERT INTO tenants (tenant_id, name, slug, is_active, metadata, settings, created_at, updated_at)
            VALUES ($1, 'Default', 'default', TRUE, 
                    '{"plan": "pro", "max_workspaces": 100, "max_users": 100, "description": "Default tenant"}'::jsonb,
                    '{}'::jsonb, NOW(), NOW())
            ON CONFLICT (tenant_id) DO NOTHING
            "#,
        )
        .bind(default_tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to ensure default tenant: {}", e)))?;

        // Ensure default workspace exists
        // Schema: workspace_id, tenant_id, name, slug, description, settings, metadata, is_active, created_at, updated_at
        sqlx::query(
            r#"
            INSERT INTO workspaces (workspace_id, tenant_id, name, slug, description, is_active, metadata, settings, created_at, updated_at)
            VALUES ($1, $2, 'Default Workspace', 'default', 'Default knowledge base', TRUE,
                    '{}'::jsonb, '{}'::jsonb, NOW(), NOW())
            ON CONFLICT (workspace_id) DO NOTHING
            "#,
        )
        .bind(default_workspace_id)
        .bind(default_tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to ensure default workspace: {}", e)))?;

        tracing::info!(
            tenant_id = %default_tenant_id,
            workspace_id = %default_workspace_id,
            "Ensured default tenant and workspace exist"
        );

        Ok((default_tenant_id, default_workspace_id))
    }

    /// Parse TenantPlan from string
    fn parse_plan(s: &str) -> TenantPlan {
        match s.to_lowercase().as_str() {
            "basic" => TenantPlan::Basic,
            "pro" => TenantPlan::Pro,
            "enterprise" => TenantPlan::Enterprise,
            _ => TenantPlan::Free,
        }
    }

    /// Parse MembershipRole from string
    fn parse_role(s: &str) -> MembershipRole {
        match s.to_lowercase().as_str() {
            "readonly" => MembershipRole::Readonly,
            "admin" => MembershipRole::Admin,
            "owner" => MembershipRole::Owner,
            _ => MembershipRole::Member,
        }
    }

    /// Build metadata JSON with tenant plan info
    fn build_tenant_metadata(tenant: &Tenant) -> serde_json::Value {
        serde_json::json!({
            "plan": tenant.plan.to_string(),
            "max_workspaces": tenant.max_workspaces,
            "max_users": tenant.max_users,
            "description": tenant.description,
        })
    }
}

#[async_trait]
impl WorkspaceService for PostgresWorkspaceService {
    // ============ Tenant Operations ============

    async fn create_tenant(&self, tenant: Tenant) -> Result<Tenant> {
        let metadata = Self::build_tenant_metadata(&tenant);

        sqlx::query(
            r#"
            INSERT INTO tenants (tenant_id, name, slug, is_active, metadata, settings, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, '{}'::jsonb, $6, $7)
            "#,
        )
        .bind(tenant.tenant_id)
        .bind(&tenant.name)
        .bind(&tenant.slug)
        .bind(tenant.is_active)
        .bind(metadata)
        .bind(tenant.created_at)
        .bind(tenant.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") || e.to_string().contains("unique constraint") {
                Error::validation(format!("Tenant with slug '{}' already exists", tenant.slug))
            } else {
                Error::internal(format!("Failed to create tenant: {}", e))
            }
        })?;

        tracing::info!(tenant_id = %tenant.tenant_id, slug = %tenant.slug, "Created tenant in PostgreSQL");
        Ok(tenant)
    }

    async fn get_tenant(&self, tenant_id: Uuid) -> Result<Option<Tenant>> {
        let row: Option<TenantRow> = sqlx::query_as(
            r#"
            SELECT tenant_id, name, slug, is_active, metadata, created_at, updated_at
            FROM tenants
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to get tenant: {}", e)))?;

        Ok(row.map(|r| r.into_tenant()))
    }

    async fn get_tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>> {
        let row: Option<TenantRow> = sqlx::query_as(
            r#"
            SELECT tenant_id, name, slug, is_active, metadata, created_at, updated_at
            FROM tenants
            WHERE slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to get tenant by slug: {}", e)))?;

        Ok(row.map(|r| r.into_tenant()))
    }

    async fn update_tenant(&self, tenant: Tenant) -> Result<Tenant> {
        let metadata = Self::build_tenant_metadata(&tenant);

        let result = sqlx::query(
            r#"
            UPDATE tenants 
            SET name = $2, is_active = $3, metadata = $4, updated_at = NOW()
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant.tenant_id)
        .bind(&tenant.name)
        .bind(tenant.is_active)
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to update tenant: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(Error::not_found(format!(
                "Tenant {} not found",
                tenant.tenant_id
            )));
        }

        Ok(tenant)
    }

    async fn delete_tenant(&self, tenant_id: Uuid) -> Result<()> {
        // Delete workspaces first (cascade would handle this but being explicit)
        sqlx::query("DELETE FROM workspaces WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("Failed to delete tenant workspaces: {}", e)))?;

        // Delete memberships
        sqlx::query("DELETE FROM memberships WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("Failed to delete tenant memberships: {}", e)))?;

        // Delete tenant
        sqlx::query("DELETE FROM tenants WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("Failed to delete tenant: {}", e)))?;

        tracing::info!(tenant_id = %tenant_id, "Deleted tenant and all workspaces from PostgreSQL");
        Ok(())
    }

    async fn list_tenants(&self, limit: usize, offset: usize) -> Result<Vec<Tenant>> {
        let rows: Vec<TenantRow> = sqlx::query_as(
            r#"
            SELECT tenant_id, name, slug, is_active, metadata, created_at, updated_at
            FROM tenants
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to list tenants: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into_tenant()).collect())
    }

    // ============ Workspace Operations ============

    async fn create_workspace(
        &self,
        tenant_id: Uuid,
        request: CreateWorkspaceRequest,
    ) -> Result<Workspace> {
        // Check tenant exists and get max workspaces from metadata
        let tenant = self
            .get_tenant(tenant_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("Tenant {} not found", tenant_id)))?;

        // Check workspace limit
        let current_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM workspaces WHERE tenant_id = $1")
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| Error::internal(format!("Failed to count workspaces: {}", e)))?;

        if current_count as usize >= tenant.max_workspaces {
            return Err(Error::validation(format!(
                "Tenant has reached maximum workspace limit ({})",
                tenant.max_workspaces
            )));
        }

        let slug = request
            .slug
            .unwrap_or_else(|| Self::generate_slug(&request.name));

        // Check slug uniqueness within tenant
        let existing: Option<(Uuid,)> = sqlx::query_as(
            "SELECT workspace_id FROM workspaces WHERE tenant_id = $1 AND slug = $2",
        )
        .bind(tenant_id)
        .bind(&slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to check workspace slug: {}", e)))?;

        if existing.is_some() {
            return Err(Error::validation(format!(
                "Workspace with slug '{}' already exists in this tenant",
                slug
            )));
        }

        let mut workspace = Workspace::new(tenant_id, &request.name, &slug);
        if let Some(desc) = request.description {
            workspace = workspace.with_description(desc);
        }
        if let Some(max_docs) = request.max_documents {
            workspace = workspace.with_max_documents(max_docs);
        }

        sqlx::query(
            r#"
            INSERT INTO workspaces (workspace_id, tenant_id, name, slug, description, is_active, metadata, settings, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, '{}'::jsonb, $8, $9)
            "#,
        )
        .bind(workspace.workspace_id)
        .bind(workspace.tenant_id)
        .bind(&workspace.name)
        .bind(&workspace.slug)
        .bind(&workspace.description)
        .bind(workspace.is_active)
        .bind(serde_json::json!(workspace.metadata))
        .bind(workspace.created_at)
        .bind(workspace.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to create workspace: {}", e)))?;

        tracing::info!(
            workspace_id = %workspace.workspace_id,
            tenant_id = %tenant_id,
            slug = %slug,
            "Created workspace in PostgreSQL"
        );

        Ok(workspace)
    }

    async fn insert_workspace(&self, workspace: Workspace) -> Result<Workspace> {
        // Validate tenant exists
        if self.get_tenant(workspace.tenant_id).await?.is_none() {
            return Err(Error::not_found(format!(
                "Tenant {} not found",
                workspace.tenant_id
            )));
        }

        sqlx::query(
            r#"
            INSERT INTO workspaces (workspace_id, tenant_id, name, slug, description, is_active, metadata, settings, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, '{}'::jsonb, $8, $9)
            ON CONFLICT (workspace_id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                is_active = EXCLUDED.is_active,
                metadata = EXCLUDED.metadata,
                updated_at = NOW()
            "#,
        )
        .bind(workspace.workspace_id)
        .bind(workspace.tenant_id)
        .bind(&workspace.name)
        .bind(&workspace.slug)
        .bind(&workspace.description)
        .bind(workspace.is_active)
        .bind(serde_json::json!(workspace.metadata))
        .bind(workspace.created_at)
        .bind(workspace.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to insert workspace: {}", e)))?;

        tracing::info!(
            workspace_id = %workspace.workspace_id,
            tenant_id = %workspace.tenant_id,
            "Inserted workspace in PostgreSQL"
        );

        Ok(workspace)
    }

    async fn get_workspace(&self, workspace_id: Uuid) -> Result<Option<Workspace>> {
        let row: Option<WorkspaceRow> = sqlx::query_as(
            r#"
            SELECT workspace_id, tenant_id, name, slug, description, is_active, metadata, created_at, updated_at
            FROM workspaces
            WHERE workspace_id = $1
            "#,
        )
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to get workspace: {}", e)))?;

        Ok(row.map(|r| r.into_workspace()))
    }

    async fn get_workspace_by_slug(
        &self,
        tenant_id: Uuid,
        slug: &str,
    ) -> Result<Option<Workspace>> {
        let row: Option<WorkspaceRow> = sqlx::query_as(
            r#"
            SELECT workspace_id, tenant_id, name, slug, description, is_active, metadata, created_at, updated_at
            FROM workspaces
            WHERE tenant_id = $1 AND slug = $2
            "#,
        )
        .bind(tenant_id)
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to get workspace by slug: {}", e)))?;

        Ok(row.map(|r| r.into_workspace()))
    }

    async fn update_workspace(
        &self,
        workspace_id: Uuid,
        request: UpdateWorkspaceRequest,
    ) -> Result<Workspace> {
        // First get the existing workspace
        let mut workspace = self
            .get_workspace(workspace_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("Workspace {} not found", workspace_id)))?;

        // Apply updates
        if let Some(name) = request.name {
            workspace.name = name;
        }
        if let Some(desc) = request.description {
            workspace.description = Some(desc);
        }
        if let Some(is_active) = request.is_active {
            workspace.is_active = is_active;
        }
        if let Some(max_docs) = request.max_documents {
            workspace
                .metadata
                .insert("max_documents".to_string(), serde_json::json!(max_docs));
        }
        workspace.updated_at = chrono::Utc::now();

        sqlx::query(
            r#"
            UPDATE workspaces 
            SET name = $2, description = $3, is_active = $4, metadata = $5, updated_at = NOW()
            WHERE workspace_id = $1
            "#,
        )
        .bind(workspace.workspace_id)
        .bind(&workspace.name)
        .bind(&workspace.description)
        .bind(workspace.is_active)
        .bind(serde_json::json!(workspace.metadata))
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to update workspace: {}", e)))?;

        Ok(workspace)
    }

    async fn delete_workspace(&self, workspace_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM workspaces WHERE workspace_id = $1")
            .bind(workspace_id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("Failed to delete workspace: {}", e)))?;

        tracing::info!(workspace_id = %workspace_id, "Deleted workspace from PostgreSQL");
        Ok(())
    }

    async fn list_workspaces(&self, tenant_id: Uuid) -> Result<Vec<Workspace>> {
        let rows: Vec<WorkspaceRow> = sqlx::query_as(
            r#"
            SELECT workspace_id, tenant_id, name, slug, description, is_active, metadata, created_at, updated_at
            FROM workspaces
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to list workspaces: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into_workspace()).collect())
    }

    async fn get_workspace_stats(&self, workspace_id: Uuid) -> Result<WorkspaceStats> {
        // Verify workspace exists
        let _ = self
            .get_workspace(workspace_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("Workspace {} not found", workspace_id)))?;

        Ok(WorkspaceStats {
            workspace_id,
            document_count: 0,
            entity_count: 0,
            relationship_count: 0,
            chunk_count: 0,
            storage_bytes: 0,
        })
    }

    // ============ Membership Operations ============

    async fn add_membership(&self, membership: Membership) -> Result<Membership> {
        sqlx::query(
            r#"
            INSERT INTO memberships (membership_id, tenant_id, workspace_id, user_id, role, is_active, joined_at, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, '{}'::jsonb)
            "#,
        )
        .bind(membership.membership_id)
        .bind(membership.tenant_id)
        .bind(membership.workspace_id)
        .bind(membership.user_id)
        .bind(membership.role.to_string())
        .bind(membership.is_active)
        .bind(membership.joined_at)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to add membership: {}", e)))?;

        Ok(membership)
    }

    async fn get_user_memberships(&self, user_id: Uuid) -> Result<Vec<Membership>> {
        let rows: Vec<MembershipRow> = sqlx::query_as(
            r#"
            SELECT membership_id, tenant_id, workspace_id, user_id, role, is_active, joined_at
            FROM memberships
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to get user memberships: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into_membership()).collect())
    }

    async fn get_tenant_memberships(&self, tenant_id: Uuid) -> Result<Vec<Membership>> {
        let rows: Vec<MembershipRow> = sqlx::query_as(
            r#"
            SELECT membership_id, tenant_id, workspace_id, user_id, role, is_active, joined_at
            FROM memberships
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to get tenant memberships: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into_membership()).collect())
    }

    async fn update_membership_role(
        &self,
        membership_id: Uuid,
        role: MembershipRole,
    ) -> Result<Membership> {
        let result = sqlx::query("UPDATE memberships SET role = $2 WHERE membership_id = $1")
            .bind(membership_id)
            .bind(role.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("Failed to update membership: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(Error::not_found(format!(
                "Membership {} not found",
                membership_id
            )));
        }

        // Fetch and return updated membership
        let row: MembershipRow = sqlx::query_as(
            "SELECT membership_id, tenant_id, workspace_id, user_id, role, is_active, joined_at FROM memberships WHERE membership_id = $1",
        )
        .bind(membership_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to fetch updated membership: {}", e)))?;

        Ok(row.into_membership())
    }

    async fn remove_membership(&self, membership_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM memberships WHERE membership_id = $1")
            .bind(membership_id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("Failed to remove membership: {}", e)))?;

        Ok(())
    }

    async fn check_tenant_access(&self, user_id: Uuid, tenant_id: Uuid) -> Result<bool> {
        let exists: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM memberships WHERE user_id = $1 AND tenant_id = $2 LIMIT 1",
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to check tenant access: {}", e)))?;

        Ok(exists.is_some())
    }

    async fn check_workspace_access(&self, user_id: Uuid, workspace_id: Uuid) -> Result<bool> {
        let exists: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM memberships WHERE user_id = $1 AND workspace_id = $2 LIMIT 1",
        )
        .bind(user_id)
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to check workspace access: {}", e)))?;

        Ok(exists.is_some())
    }

    async fn get_user_role(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Option<MembershipRole>> {
        let role: Option<(String,)> = sqlx::query_as(
            "SELECT role FROM memberships WHERE user_id = $1 AND tenant_id = $2 LIMIT 1",
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to get user role: {}", e)))?;

        Ok(role.map(|(r,)| Self::parse_role(&r)))
    }

    // ============ Context Operations ============

    async fn build_context(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        workspace_id: Option<Uuid>,
    ) -> Result<TenantContext> {
        // Verify tenant exists
        let _tenant = self
            .get_tenant(tenant_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("Tenant {} not found", tenant_id)))?;

        // Verify workspace if provided
        if let Some(ws_id) = workspace_id {
            let workspace = self
                .get_workspace(ws_id)
                .await?
                .ok_or_else(|| Error::not_found(format!("Workspace {} not found", ws_id)))?;

            if workspace.tenant_id != tenant_id {
                return Err(Error::validation(
                    "Workspace does not belong to the specified tenant",
                ));
            }
        }

        // Get user's role in this tenant
        let role = self.get_user_role(user_id, tenant_id).await?;

        Ok(TenantContext {
            tenant_id: Some(tenant_id),
            workspace_id,
            user_id: Some(user_id),
            role,
        })
    }
}

// ============ Database Row Types ============

/// Tenant row from PostgreSQL.
/// The actual schema uses metadata JSONB for plan, max_workspaces, max_users, description.
#[derive(sqlx::FromRow)]
struct TenantRow {
    tenant_id: Uuid,
    name: String,
    slug: Option<String>,
    is_active: bool,
    metadata: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl TenantRow {
    fn into_tenant(self) -> Tenant {
        // Extract values from metadata JSONB
        let plan_str = self
            .metadata
            .get("plan")
            .and_then(|v| v.as_str())
            .unwrap_or("free");
        let max_workspaces = self
            .metadata
            .get("max_workspaces")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as usize;
        let max_users = self
            .metadata
            .get("max_users")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;
        let description = self
            .metadata
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Tenant {
            tenant_id: self.tenant_id,
            name: self.name,
            slug: self.slug.unwrap_or_default(),
            description,
            plan: PostgresWorkspaceService::parse_plan(plan_str),
            is_active: self.is_active,
            max_workspaces,
            max_users,
            created_at: self.created_at,
            updated_at: self.updated_at,
            metadata: HashMap::new(),
        }
    }
}

/// Workspace row from PostgreSQL.
#[derive(sqlx::FromRow)]
struct WorkspaceRow {
    workspace_id: Uuid,
    tenant_id: Uuid,
    name: String,
    slug: Option<String>,
    description: Option<String>,
    is_active: bool,
    metadata: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl WorkspaceRow {
    fn into_workspace(self) -> Workspace {
        // Convert metadata from serde_json::Value to HashMap
        let metadata: HashMap<String, serde_json::Value> =
            if let serde_json::Value::Object(map) = self.metadata {
                map.into_iter().collect()
            } else {
                HashMap::new()
            };

        Workspace {
            workspace_id: self.workspace_id,
            tenant_id: self.tenant_id,
            name: self.name,
            slug: self.slug.unwrap_or_default(),
            description: self.description,
            is_active: self.is_active,
            created_at: self.created_at,
            updated_at: self.updated_at,
            metadata,
        }
    }
}

/// Membership row from PostgreSQL.
#[derive(sqlx::FromRow)]
struct MembershipRow {
    membership_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    user_id: Uuid,
    role: String,
    is_active: bool,
    joined_at: chrono::DateTime<chrono::Utc>,
}

impl MembershipRow {
    fn into_membership(self) -> Membership {
        Membership {
            membership_id: self.membership_id,
            tenant_id: self.tenant_id,
            workspace_id: self.workspace_id,
            user_id: self.user_id,
            role: PostgresWorkspaceService::parse_role(&self.role),
            is_active: self.is_active,
            joined_at: self.joined_at,
            metadata: HashMap::new(),
        }
    }
}
