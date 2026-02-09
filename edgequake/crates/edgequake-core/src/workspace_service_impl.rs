//! Production implementation of WorkspaceService.
//!
//! This module provides the production-ready implementation of the WorkspaceService
//! trait, backed by PostgreSQL (the system of record).
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        edgequake-core                           │
//! │  ┌─────────────────────┐    ┌────────────────────────────────┐ │
//! │  │  WorkspaceService   │◄───│ WorkspaceServiceImpl           │ │
//! │  │      (trait)        │    │ (production implementation)    │ │
//! │  └─────────────────────┘    └────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # WHY: Service Layer in Core (not Storage)
//!
//! This service MUST live in `edgequake-core` because:
//! 1. It implements the `WorkspaceService` trait defined in this crate
//! 2. Moving to `edgequake-storage` would create a circular dependency
//! 3. Follows Hexagonal Architecture: adapters live with ports
//!
//! NOTE: Database schema stores plan, max_workspaces, max_users in `metadata` JSONB.

#[cfg(feature = "postgres")]
use async_trait::async_trait;
#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "postgres")]
use std::collections::HashMap;
#[cfg(feature = "postgres")]
use uuid::Uuid;

#[cfg(feature = "postgres")]
use crate::{
    error::{Error, Result},
    types::{
        CreateWorkspaceRequest, Membership, MembershipRole, MetricsSnapshot, MetricsTriggerType,
        Tenant, TenantContext, TenantPlan, UpdateWorkspaceRequest, Workspace, WorkspaceStats,
    },
    workspace_service::WorkspaceService,
};

/// PostgreSQL-backed implementation of WorkspaceService.
///
/// This implementation persists all tenant and workspace data directly
/// to PostgreSQL, ensuring data survives application restarts.
#[cfg(feature = "postgres")]
pub struct WorkspaceServiceImpl {
    pool: PgPool,
}

#[cfg(feature = "postgres")]
impl WorkspaceServiceImpl {
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

#[cfg(feature = "postgres")]
#[async_trait]
impl WorkspaceService for WorkspaceServiceImpl {
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

        // SPEC-032: Apply LLM configuration from request
        // Uses auto-detection for provider if not specified
        if let Some(model) = request.llm_model {
            workspace = workspace.with_llm_model(&model);
            // Explicit provider overrides auto-detection
            if let Some(provider) = request.llm_provider {
                workspace = workspace.with_llm_provider(&provider);
            }
        } else if let Some(provider) = request.llm_provider {
            // Provider specified without model - use default model for provider
            workspace = workspace.with_llm_provider(&provider);
        }

        // SPEC-032: Apply embedding configuration from request
        // Uses auto-detection for provider/dimension if not specified
        if let Some(model) = request.embedding_model {
            workspace = workspace.with_embedding_model(&model);
            // Auto-detect provider if not specified
            if let Some(provider) = request.embedding_provider {
                workspace = workspace.with_embedding_provider(&provider);
            } else {
                let detected = Workspace::detect_provider_from_model(&model);
                workspace = workspace.with_embedding_provider(detected);
            }
            // Auto-detect dimension if not specified
            if let Some(dim) = request.embedding_dimension {
                workspace = workspace.with_embedding_dimension(dim);
            } else {
                let detected = Workspace::detect_dimension_from_model(&model);
                workspace = workspace.with_embedding_dimension(detected);
            }
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
        // SPEC-032: Store LLM and embedding config in metadata until migration adds dedicated columns
        .bind({
            let mut metadata = workspace.metadata.clone();
            // LLM configuration
            metadata.insert("llm_model".to_string(), serde_json::Value::String(workspace.llm_model.clone()));
            metadata.insert("llm_provider".to_string(), serde_json::Value::String(workspace.llm_provider.clone()));
            // Embedding configuration
            metadata.insert("embedding_model".to_string(), serde_json::Value::String(workspace.embedding_model.clone()));
            metadata.insert("embedding_provider".to_string(), serde_json::Value::String(workspace.embedding_provider.clone()));
            metadata.insert("embedding_dimension".to_string(), serde_json::Value::Number(workspace.embedding_dimension.into()));
            serde_json::json!(metadata)
        })
        .bind(workspace.created_at)
        .bind(workspace.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to create workspace: {}", e)))?;

        tracing::info!(
            workspace_id = %workspace.workspace_id,
            tenant_id = %tenant_id,
            slug = %slug,
            llm_model = %workspace.llm_full_id(),
            embedding_model = %workspace.embedding_full_id(),
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
        // SPEC-032: LLM model configuration updates
        // Store in metadata JSON for compatibility with database schema
        if let Some(llm_model) = request.llm_model {
            workspace.llm_model = llm_model.clone();
            workspace
                .metadata
                .insert("llm_model".to_string(), serde_json::json!(llm_model));
        }
        if let Some(llm_provider) = request.llm_provider {
            workspace.llm_provider = llm_provider.clone();
            workspace
                .metadata
                .insert("llm_provider".to_string(), serde_json::json!(llm_provider));
        }
        // SPEC-032: Embedding model configuration updates
        // WARNING: Changing embedding model requires vector rebuild
        if let Some(embedding_model) = request.embedding_model {
            workspace.embedding_model = embedding_model.clone();
            workspace.metadata.insert(
                "embedding_model".to_string(),
                serde_json::json!(embedding_model),
            );
        }
        if let Some(embedding_provider) = request.embedding_provider {
            workspace.embedding_provider = embedding_provider.clone();
            workspace.metadata.insert(
                "embedding_provider".to_string(),
                serde_json::json!(embedding_provider),
            );
        }
        if let Some(embedding_dimension) = request.embedding_dimension {
            workspace.embedding_dimension = embedding_dimension;
            workspace.metadata.insert(
                "embedding_dimension".to_string(),
                serde_json::json!(embedding_dimension),
            );
        }
        workspace.updated_at = chrono::Utc::now();

        // Store all config in metadata JSONB column (database schema uses metadata, not separate columns)
        sqlx::query(
            r#"
            UPDATE workspaces 
            SET name = $2, description = $3, is_active = $4, metadata = $5,
                updated_at = NOW()
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

        // WHY scalar subqueries: Single round-trip to database, efficient counting.
        // Each subquery uses indexed workspace_id for O(log n) performance.
        // OODA-13: Implements real-time metrics per mission requirement.
        #[derive(sqlx::FromRow)]
        struct StatsRow {
            document_count: i64,
            chunk_count: i64,
            entity_count: i64,
            relationship_count: i64,
            embedding_count: i64,
            storage_bytes: i64,
        }

        let stats: StatsRow = sqlx::query_as(
            r#"
            SELECT 
                (SELECT COUNT(*) FROM documents WHERE workspace_id = $1) as document_count,
                (SELECT COUNT(*) FROM chunks WHERE workspace_id = $1) as chunk_count,
                (SELECT COUNT(*) FROM entities WHERE workspace_id = $1) as entity_count,
                (SELECT COUNT(*) FROM relationships WHERE workspace_id = $1) as relationship_count,
                (SELECT COUNT(*) FROM chunks WHERE workspace_id = $1 AND embedding IS NOT NULL) as embedding_count,
                (SELECT COALESCE(SUM(file_size_bytes), 0)::BIGINT FROM documents WHERE workspace_id = $1) as storage_bytes
            "#,
        )
        .bind(workspace_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to get workspace stats: {}", e)))?;

        Ok(WorkspaceStats {
            workspace_id,
            document_count: stats.document_count as usize,
            entity_count: stats.entity_count as usize,
            relationship_count: stats.relationship_count as usize,
            chunk_count: stats.chunk_count as usize,
            embedding_count: stats.embedding_count as usize,
            storage_bytes: stats.storage_bytes as usize,
        })
    }

    // ============ Metrics Operations ============

    async fn record_metrics_snapshot(
        &self,
        workspace_id: Uuid,
        trigger_type: MetricsTriggerType,
    ) -> Result<MetricsSnapshot> {
        // First get current stats
        let stats = self.get_workspace_stats(workspace_id).await?;

        // WHY INSERT ... RETURNING: Single round-trip, atomic operation.
        // OODA-20: Records to workspace_metrics_history from migration 016.
        #[derive(sqlx::FromRow)]
        struct SnapshotRow {
            id: Uuid,
            #[allow(dead_code)]
            workspace_id: String,
            recorded_at: chrono::DateTime<chrono::Utc>,
            trigger_type: String,
            document_count: i64,
            chunk_count: i64,
            entity_count: i64,
            relationship_count: i64,
            embedding_count: i64,
            storage_bytes: i64,
        }

        let row: SnapshotRow = sqlx::query_as(
            r#"
            INSERT INTO workspace_metrics_history (
                workspace_id, trigger_type,
                document_count, chunk_count, entity_count, relationship_count,
                embedding_count, storage_bytes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, workspace_id, recorded_at, trigger_type,
                      document_count, chunk_count, entity_count, relationship_count,
                      embedding_count, storage_bytes
            "#,
        )
        .bind(workspace_id.to_string())
        .bind(trigger_type.as_str())
        .bind(stats.document_count as i64)
        .bind(stats.chunk_count as i64)
        .bind(stats.entity_count as i64)
        .bind(stats.relationship_count as i64)
        .bind(stats.embedding_count as i64)
        .bind(stats.storage_bytes as i64)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to record metrics snapshot: {}", e)))?;

        Ok(MetricsSnapshot {
            id: row.id,
            workspace_id,
            recorded_at: row.recorded_at,
            trigger_type: MetricsTriggerType::parse(&row.trigger_type)
                .unwrap_or(MetricsTriggerType::Event),
            document_count: row.document_count,
            chunk_count: row.chunk_count,
            entity_count: row.entity_count,
            relationship_count: row.relationship_count,
            embedding_count: row.embedding_count,
            storage_bytes: row.storage_bytes,
        })
    }

    async fn get_metrics_history(
        &self,
        workspace_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<MetricsSnapshot>> {
        // WHY ORDER BY DESC: Most recent snapshots first for trend analysis.
        // OODA-22: Query from workspace_metrics_history table.
        #[derive(sqlx::FromRow)]
        struct HistoryRow {
            id: Uuid,
            #[allow(dead_code)]
            workspace_id: String,
            recorded_at: chrono::DateTime<chrono::Utc>,
            trigger_type: String,
            document_count: i64,
            chunk_count: i64,
            entity_count: i64,
            relationship_count: i64,
            embedding_count: i64,
            storage_bytes: i64,
        }

        let rows: Vec<HistoryRow> = sqlx::query_as(
            r#"
            SELECT id, workspace_id, recorded_at, trigger_type,
                   document_count, chunk_count, entity_count, relationship_count,
                   embedding_count, storage_bytes
            FROM workspace_metrics_history
            WHERE workspace_id = $1
            ORDER BY recorded_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(workspace_id.to_string())
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to get metrics history: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|row| MetricsSnapshot {
                id: row.id,
                workspace_id,
                recorded_at: row.recorded_at,
                trigger_type: MetricsTriggerType::parse(&row.trigger_type)
                    .unwrap_or(MetricsTriggerType::Event),
                document_count: row.document_count,
                chunk_count: row.chunk_count,
                entity_count: row.entity_count,
                relationship_count: row.relationship_count,
                embedding_count: row.embedding_count,
                storage_bytes: row.storage_bytes,
            })
            .collect())
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
#[cfg(feature = "postgres")]
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

#[cfg(feature = "postgres")]
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

        // SPEC-032: Extract default LLM config from metadata
        let default_llm_model = self
            .metadata
            .get("default_llm_model")
            .and_then(|v| v.as_str())
            .unwrap_or(crate::types::DEFAULT_LLM_MODEL)
            .to_string();
        let default_llm_provider = self
            .metadata
            .get("default_llm_provider")
            .and_then(|v| v.as_str())
            .unwrap_or(crate::types::DEFAULT_LLM_PROVIDER)
            .to_string();

        // SPEC-032: Extract default embedding config from metadata
        let default_embedding_model = self
            .metadata
            .get("default_embedding_model")
            .and_then(|v| v.as_str())
            .unwrap_or(crate::types::DEFAULT_EMBEDDING_MODEL)
            .to_string();
        let default_embedding_provider = self
            .metadata
            .get("default_embedding_provider")
            .and_then(|v| v.as_str())
            .unwrap_or(crate::types::DEFAULT_EMBEDDING_PROVIDER)
            .to_string();
        let default_embedding_dimension =
            self.metadata
                .get("default_embedding_dimension")
                .and_then(|v| v.as_u64())
                .unwrap_or(crate::types::DEFAULT_EMBEDDING_DIMENSION as u64) as usize;

        Tenant {
            tenant_id: self.tenant_id,
            name: self.name,
            slug: self.slug.unwrap_or_default(),
            description,
            plan: WorkspaceServiceImpl::parse_plan(plan_str),
            is_active: self.is_active,
            max_workspaces,
            max_users,
            created_at: self.created_at,
            updated_at: self.updated_at,
            metadata: HashMap::new(),
            default_llm_model,
            default_llm_provider,
            default_embedding_model,
            default_embedding_provider,
            default_embedding_dimension,
        }
    }
}

/// Workspace row from PostgreSQL.
#[cfg(feature = "postgres")]
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

#[cfg(feature = "postgres")]
impl WorkspaceRow {
    fn into_workspace(self) -> Workspace {
        // Convert metadata from serde_json::Value to HashMap
        let metadata: HashMap<String, serde_json::Value> =
            if let serde_json::Value::Object(map) = self.metadata {
                map.into_iter().collect()
            } else {
                HashMap::new()
            };

        // SPEC-032: Extract LLM config from metadata
        let llm_model = metadata
            .get("llm_model")
            .and_then(|v| v.as_str())
            .unwrap_or(crate::types::DEFAULT_LLM_MODEL)
            .to_string();
        let llm_provider = metadata
            .get("llm_provider")
            .and_then(|v| v.as_str())
            .unwrap_or(crate::types::DEFAULT_LLM_PROVIDER)
            .to_string();

        // SPEC-032: Extract embedding config from metadata
        let embedding_model = metadata
            .get("embedding_model")
            .and_then(|v| v.as_str())
            .unwrap_or(crate::types::DEFAULT_EMBEDDING_MODEL)
            .to_string();
        let embedding_provider = metadata
            .get("embedding_provider")
            .and_then(|v| v.as_str())
            .unwrap_or(crate::types::DEFAULT_EMBEDDING_PROVIDER)
            .to_string();
        let embedding_dimension = metadata
            .get("embedding_dimension")
            .and_then(|v| v.as_u64())
            .unwrap_or(crate::types::DEFAULT_EMBEDDING_DIMENSION as u64)
            as usize;

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
            llm_model,
            llm_provider,
            embedding_model,
            embedding_provider,
            embedding_dimension,
        }
    }
}

/// Membership row from PostgreSQL.
#[cfg(feature = "postgres")]
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

#[cfg(feature = "postgres")]
impl MembershipRow {
    fn into_membership(self) -> Membership {
        Membership {
            membership_id: self.membership_id,
            tenant_id: self.tenant_id,
            workspace_id: self.workspace_id,
            user_id: self.user_id,
            role: WorkspaceServiceImpl::parse_role(&self.role),
            is_active: self.is_active,
            joined_at: self.joined_at,
            metadata: HashMap::new(),
        }
    }
}
