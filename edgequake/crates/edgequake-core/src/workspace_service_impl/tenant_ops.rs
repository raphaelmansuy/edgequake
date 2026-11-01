#[cfg(feature = "postgres")]
use uuid::Uuid;

#[cfg(feature = "postgres")]
use crate::{
    error::{Error, Result},
    types::Tenant,
};

#[cfg(feature = "postgres")]
use super::rows::TenantRow;
#[cfg(feature = "postgres")]
use super::WorkspaceServiceImpl;

#[cfg(feature = "postgres")]
impl WorkspaceServiceImpl {
    // ============ Tenant Operations ============

    pub(super) async fn pg_create_tenant(&self, tenant: Tenant) -> Result<Tenant> {
        let metadata = Self::build_tenant_metadata(&tenant);

        // SPEC-181 LAW-181-8: create is not upsert. The previous `ON CONFLICT (slug) DO UPDATE
        // SET name = tenants.name` made a mistyped slug return *someone else's* organization
        // with a 200, so a caller could believe they had created a tenant they had actually
        // joined. A slug collision is now an identity error, surfaced once at the service layer.
        let row: TenantRow = sqlx::query_as(
            r#"
            INSERT INTO tenants
                (tenant_id, name, slug, description, plan, max_workspaces, max_users,
                 is_active, metadata, settings, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, '{}'::jsonb, $10, $11)
            RETURNING tenant_id, name, slug, is_active, metadata, created_at, updated_at,
                      plan, description, max_workspaces, max_users, archived_at, archived_by
            "#,
        )
        .bind(tenant.tenant_id)
        .bind(&tenant.name)
        .bind(&tenant.slug)
        .bind(&tenant.description)
        .bind(tenant.plan.as_str())
        .bind(tenant.max_workspaces as i32)
        .bind(tenant.max_users as i32)
        .bind(tenant.is_active)
        .bind(metadata)
        .bind(tenant.created_at)
        .bind(tenant.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Self::map_tenant_insert_error(&e, &tenant))?;

        let created = row.into_tenant();
        tracing::info!(
            tenant_id = %created.tenant_id,
            slug = %created.slug,
            plan = %created.plan.as_str(),
            max_workspaces = created.max_workspaces,
            max_users = created.max_users,
            "Created tenant in PostgreSQL"
        );
        Ok(created)
    }

    /// Translate a unique violation on `tenants.slug` into an identity conflict that names the
    /// existing organization, so the caller can offer "open it instead" rather than guessing.
    fn map_tenant_insert_error(e: &sqlx::Error, tenant: &Tenant) -> Error {
        if let sqlx::Error::Database(db) = e {
            if db.code().as_deref() == Some("23505") {
                return Error::conflict(format!(
                    "tenant_slug_taken: slug '{}' already exists",
                    tenant.slug
                ));
            }
            if db.code().as_deref() == Some("23514") {
                // tenants_quota_positive / tenants_plan_fk both land here.
                return Error::validation(format!("Invalid tenant quota or plan: {db}"));
            }
        }
        Error::internal(format!("Failed to create tenant: {}", e))
    }

    pub(super) async fn pg_get_tenant(&self, tenant_id: Uuid) -> Result<Option<Tenant>> {
        let row: Option<TenantRow> = sqlx::query_as(
            r#"
            SELECT tenant_id, name, slug, is_active, metadata, created_at, updated_at, plan, description, max_workspaces, max_users, archived_at, archived_by
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

    pub(super) async fn pg_get_tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>> {
        let row: Option<TenantRow> = sqlx::query_as(
            r#"
            SELECT tenant_id, name, slug, is_active, metadata, created_at, updated_at, plan, description, max_workspaces, max_users, archived_at, archived_by
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

    pub(super) async fn pg_update_tenant(&self, tenant: Tenant) -> Result<Tenant> {
        let metadata = Self::build_tenant_metadata(&tenant);

        // SPEC-181: plan/quota/description/archive are real columns now. `slug` is deliberately
        // absent: it is the stable key for `/w/{slug}` links and SSO organization binding.
        let result = sqlx::query(
            r#"
            UPDATE tenants
            SET name = $2, description = $3, plan = $4, max_workspaces = $5, max_users = $6,
                is_active = $7, metadata = $8, archived_at = $9, archived_by = $10,
                updated_at = NOW()
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant.tenant_id)
        .bind(&tenant.name)
        .bind(&tenant.description)
        .bind(tenant.plan.as_str())
        .bind(tenant.max_workspaces as i32)
        .bind(tenant.max_users as i32)
        .bind(tenant.is_active)
        .bind(metadata)
        .bind(tenant.archived_at)
        .bind(tenant.archived_by)
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

    pub(super) async fn pg_delete_tenant(&self, tenant_id: Uuid) -> Result<()> {
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

    pub(super) async fn pg_list_tenants(&self, limit: usize, offset: usize) -> Result<Vec<Tenant>> {
        let rows: Vec<TenantRow> = sqlx::query_as(
            r#"
            SELECT tenant_id, name, slug, is_active, metadata, created_at, updated_at, plan, description, max_workspaces, max_users, archived_at, archived_by
            FROM tenants
            ORDER BY created_at DESC, tenant_id DESC
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

    pub(super) async fn pg_count_tenants(&self) -> Result<usize> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenants")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("Failed to count tenants: {}", e)))?;
        Ok(count as usize)
    }
}
