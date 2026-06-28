//! Shared tenant/workspace scope matching for documents and PDFs.
//!
//! Thin compatibility layer over [`crate::services::isolation_context`] (SPEC-027 IMP-023).

use crate::middleware::TenantContext;

/// Check whether metadata belongs to the requester's workspace (UUID-normalized).
pub fn metadata_matches_workspace_context(
    metadata: &serde_json::Value,
    tenant_ctx: &TenantContext,
) -> bool {
    crate::services::isolation_context::metadata_matches(metadata, tenant_ctx)
}

/// Check whether metadata belongs to the requester's tenant (UUID-normalized).
pub fn metadata_matches_tenant_id_context(
    metadata: &serde_json::Value,
    tenant_ctx: &TenantContext,
) -> bool {
    crate::services::isolation_context::metadata_matches(metadata, tenant_ctx)
}

/// Check whether a metadata payload belongs to the requester's tenant + workspace.
pub fn metadata_matches_tenant_context(
    metadata: &serde_json::Value,
    tenant_ctx: &TenantContext,
) -> bool {
    crate::services::isolation_context::metadata_matches(metadata, tenant_ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::{default_tenant_uuid, default_workspace_uuid};

    fn ctx(tenant: &str, workspace: &str) -> TenantContext {
        TenantContext {
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            user_id: None,
        }
    }

    #[test]
    fn uuid_stored_metadata_visible_from_default_workspace_alias() {
        let metadata = serde_json::json!({
            "workspace_id": default_workspace_uuid().to_string(),
            "tenant_id": default_tenant_uuid().to_string(),
        });
        let tenant_ctx = ctx("default", "default");
        assert!(metadata_matches_tenant_context(&metadata, &tenant_ctx));
    }

    #[test]
    fn uuid_stored_metadata_hidden_from_other_workspace() {
        let metadata = serde_json::json!({
            "workspace_id": default_workspace_uuid().to_string(),
            "tenant_id": default_tenant_uuid().to_string(),
        });
        let other = uuid::Uuid::new_v4().to_string();
        let tenant_ctx = ctx("default", &other);
        assert!(!metadata_matches_tenant_context(&metadata, &tenant_ctx));
    }

    #[test]
    fn legacy_default_metadata_visible_from_default_alias() {
        let metadata = serde_json::json!({
            "workspace_id": "default",
            "tenant_id": "default",
        });
        let tenant_ctx = ctx("default", "default");
        assert!(metadata_matches_tenant_context(&metadata, &tenant_ctx));
    }
}
