#[cfg(feature = "postgres")]
use uuid::Uuid;

#[cfg(feature = "postgres")]
use crate::{
    error::{Error, Result},
    workspace_service::UpdateTenantQuotaResult,
};

#[cfg(feature = "postgres")]
use super::rows::TenantRow;
#[cfg(feature = "postgres")]
use super::WorkspaceServiceImpl;

#[cfg(feature = "postgres")]
impl WorkspaceServiceImpl {
    // ============ Quota Operations (SPEC-0001) ============

    pub(super) async fn pg_update_tenant_quota(
        &self,
        tenant_id: Uuid,
        new_max_workspaces: usize,
    ) -> Result<UpdateTenantQuotaResult> {
        // Validation V1: must be positive
        if new_max_workspaces == 0 {
            return Err(Error::validation("max_workspaces must be positive"));
        }
        // Validation V3: sanity limit
        if new_max_workspaces > 10_000 {
            return Err(Error::validation(
                "max_workspaces exceeds sanity limit (10000)",
            ));
        }

        // Use a transaction with SELECT FOR UPDATE to avoid TOCTOU race (SPEC-0001)
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::internal(format!("Failed to begin transaction: {}", e)))?;

        // Fetch tenant with row lock
        let row: Option<TenantRow> = sqlx::query_as(
            r#"
            SELECT tenant_id, name, slug, is_active, metadata, created_at, updated_at
            FROM tenants
            WHERE tenant_id = $1
            FOR UPDATE
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| Error::internal(format!("Failed to lock tenant: {}", e)))?;

        let tenant_row =
            row.ok_or_else(|| Error::not_found(format!("Tenant {} not found", tenant_id)))?;
        let previous_max = tenant_row
            .metadata
            .get("max_workspaces")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        // Count current workspaces within the transaction
        let workspace_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM workspaces WHERE tenant_id = $1")
                .bind(tenant_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| Error::internal(format!("Failed to count workspaces: {}", e)))?;

        let current_count = workspace_count as usize;

        // Validation V2: cannot reduce below current usage
        if new_max_workspaces < current_count {
            tx.rollback().await.ok();
            return Err(Error::validation(format!(
                "Cannot reduce below current workspace count ({})",
                current_count
            )));
        }

        // Update max_workspaces in the metadata JSONB directly
        sqlx::query(
            r#"
            UPDATE tenants
            SET metadata = jsonb_set(metadata, '{max_workspaces}', $2::text::jsonb),
                updated_at = NOW()
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .bind(new_max_workspaces.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::internal(format!("Failed to update tenant quota: {}", e)))?;

        tx.commit()
            .await
            .map_err(|e| Error::internal(format!("Failed to commit quota update: {}", e)))?;

        tracing::info!(
            tenant_id = %tenant_id,
            previous = previous_max,
            new = new_max_workspaces,
            current_count = current_count,
            "SPEC-0001: Updated tenant quota in PostgreSQL"
        );

        Ok(UpdateTenantQuotaResult {
            tenant_id,
            max_workspaces: new_max_workspaces,
            previous_max_workspaces: previous_max,
            current_workspace_count: current_count,
        })
    }

    pub(super) async fn pg_get_server_default_max_workspaces(&self) -> Result<usize> {
        // Try server_config table first
        let row: Option<(serde_json::Value,)> =
            sqlx::query_as("SELECT value FROM server_config WHERE key = 'default_max_workspaces'")
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| Error::internal(format!("Failed to query server_config: {}", e)))?;

        if let Some((val,)) = row {
            if let Some(n) = val.as_u64() {
                return Ok(n as usize);
            }
        }

        // Fallback to env var
        if let Ok(val) = std::env::var("EDGEQUAKE_DEFAULT_MAX_WORKSPACES") {
            if let Ok(n) = val.parse::<usize>() {
                return Ok(n);
            }
        }

        // Compile-time fallback
        Ok(100)
    }

    pub(super) async fn pg_set_server_default_max_workspaces(&self, value: usize) -> Result<usize> {
        if value == 0 {
            return Err(Error::validation("default_max_workspaces must be positive"));
        }
        if value > 10_000 {
            return Err(Error::validation(
                "default_max_workspaces exceeds sanity limit (10000)",
            ));
        }

        sqlx::query(
            r#"
            INSERT INTO server_config (key, value, updated_at)
            VALUES ('default_max_workspaces', $1::text::jsonb, NOW())
            ON CONFLICT (key) DO UPDATE
              SET value = EXCLUDED.value,
                  updated_at = NOW()
            "#,
        )
        .bind(value.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to update server_config: {}", e)))?;

        tracing::info!(
            value = value,
            "SPEC-0001: Updated server default max_workspaces in PostgreSQL"
        );
        Ok(value)
    }
}
