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
use sqlx::PgPool;
#[cfg(feature = "postgres")]
use uuid::Uuid;

#[cfg(feature = "postgres")]
use crate::{
    error::{Error, Result},
    types::{MembershipRole, Tenant, TenantPlan},
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

        // Ensure default tenant exists.
        // SPEC-181: plan/quota/description are real columns (migration 226). The bootstrap
        // tenant is a Pro with unlimited seats; `ON CONFLICT DO NOTHING` keeps an existing row
        // and its operator-raised quota intact.
        sqlx::query(
            r#"
            INSERT INTO tenants
                (tenant_id, name, slug, is_active, plan, max_workspaces, max_users,
                 description, metadata, settings, created_at, updated_at)
            VALUES ($1, 'Default', 'default', TRUE, 'pro', 100, 0,
                    'Default tenant',
                    '{"plan": "pro", "max_workspaces": 100, "max_users": 0}'::jsonb,
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
    pub(crate) fn parse_plan(s: &str) -> TenantPlan {
        match s.to_lowercase().as_str() {
            "basic" => TenantPlan::Basic,
            "pro" => TenantPlan::Pro,
            "enterprise" => TenantPlan::Enterprise,
            _ => TenantPlan::Free,
        }
    }

    /// Parse MembershipRole from string
    pub(crate) fn parse_role(s: &str) -> MembershipRole {
        match s.to_lowercase().as_str() {
            "readonly" => MembershipRole::Readonly,
            "admin" => MembershipRole::Admin,
            "owner" => MembershipRole::Owner,
            _ => MembershipRole::Member,
        }
    }

    /// Build metadata JSON with tenant configuration.
    ///
    /// SPEC-181: `plan`, `max_workspaces`, `max_users` and `description` are real columns since
    /// migration 226 and are no longer duplicated here. They are still written for one release
    /// so a rollback to the previous binary keeps reading the values it expects; the read path
    /// prefers the column.
    fn build_tenant_metadata(tenant: &Tenant) -> serde_json::Value {
        let mut map = serde_json::json!({
            "plan": tenant.plan.to_string(),
            "max_workspaces": tenant.max_workspaces,
            "max_users": tenant.max_users,
            "description": tenant.description,
            // SPEC-032: Persist default LLM configuration
            "default_llm_model": tenant.default_llm_model,
            "default_llm_provider": tenant.default_llm_provider,
            // SPEC-032: Persist default embedding configuration
            "default_embedding_model": tenant.default_embedding_model,
            "default_embedding_provider": tenant.default_embedding_provider,
            "default_embedding_dimension": tenant.default_embedding_dimension,
        });
        // SPEC-041: Persist default vision LLM configuration (optional, only if set)
        if let Some(ref vision_provider) = tenant.default_vision_llm_provider {
            map["default_vision_llm_provider"] = serde_json::json!(vision_provider);
        }
        if let Some(ref vision_model) = tenant.default_vision_llm_model {
            map["default_vision_llm_model"] = serde_json::json!(vision_model);
        }
        // SPEC-109: Persist default reasoning effort seed
        if let Some(ref effort) = tenant.default_reasoning_effort {
            map["default_reasoning_effort"] = serde_json::json!(effort);
        }
        // SPEC-123: Persist tenant PDF parser default
        if let Some(backend) = tenant.pdf_parser_backend {
            map["pdf_parser_backend"] = serde_json::json!(backend.as_str());
        }
        // Tenant brand color: omitted when unset, so an update that clears it drops the key.
        if let Some(ref color) = tenant.primary_color {
            map["primary_color"] = serde_json::json!(color);
        }
        map
    }
}

#[cfg(feature = "postgres")]
mod helpers;
#[cfg(feature = "postgres")]
mod membership_ops;
#[cfg(feature = "postgres")]
mod metrics_ops;
#[cfg(feature = "postgres")]
mod quota_ops;
#[cfg(feature = "postgres")]
mod rows;
#[cfg(feature = "postgres")]
mod service_trait_impl;
#[cfg(feature = "postgres")]
mod tenant_ops;
#[cfg(feature = "postgres")]
mod workspace_ops;

#[cfg(test)]
mod helpers_tests;
