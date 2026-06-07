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

    pub(super) async fn pg_get_tenant(&self, tenant_id: Uuid) -> Result<Option<Tenant>> {
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

    pub(super) async fn pg_get_tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>> {
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

    pub(super) async fn pg_update_tenant(&self, tenant: Tenant) -> Result<Tenant> {
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
}
