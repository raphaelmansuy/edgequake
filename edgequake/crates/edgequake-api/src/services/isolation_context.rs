//! Unified tenant/workspace isolation predicates — SPEC-027 IMP-023 (SSOT).
//!
//! Graph handlers use [`IsolationMode::Strict`]; document metadata uses
//! [`IsolationMode::LegacyDocumentAlias`] to preserve `"default"` UUID aliasing.

use std::collections::HashMap;

use uuid::Uuid;

use crate::handlers::isolation::properties_match_tenant_context;
use crate::middleware::{resolve_tenant_uuid, resolve_workspace_uuid, TenantContext};

/// Isolation mode for property/metadata matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationMode {
    /// Graph nodes/edges — strict equality, no `"default"` alias.
    Strict,
    /// Document metadata — legacy `"default"` UUID aliasing preserved.
    LegacyDocumentAlias,
}

fn parse_workspace_uuid_or_default(workspace_id: Option<&str>) -> Option<Uuid> {
    resolve_workspace_uuid(workspace_id)
}

fn is_legacy_default_workspace_context(workspace_id: Option<&str>) -> bool {
    match workspace_id.map(str::trim) {
        None | Some("") | Some("default") => true,
        Some(value) => match Uuid::parse_str(value) {
            Ok(uuid) => {
                uuid == crate::middleware::default_tenant_uuid()
                    || uuid == crate::middleware::default_workspace_uuid()
            }
            Err(_) => false,
        },
    }
}

fn is_legacy_default_tenant_context(tenant_id: Option<&str>) -> bool {
    match tenant_id.map(str::trim) {
        None | Some("") | Some("default") => true,
        Some(value) => match Uuid::parse_str(value) {
            Ok(uuid) => uuid == crate::middleware::default_tenant_uuid(),
            Err(_) => false,
        },
    }
}

/// Match a graph property map against tenant context.
pub fn properties_match(
    properties: &HashMap<String, serde_json::Value>,
    ctx: &TenantContext,
    mode: IsolationMode,
) -> bool {
    match mode {
        IsolationMode::Strict => properties_match_tenant_context(properties, ctx),
        IsolationMode::LegacyDocumentAlias => {
            let value = serde_json::json!({
                "tenant_id": properties.get("tenant_id"),
                "workspace_id": properties.get("workspace_id"),
            });
            metadata_matches(&value, ctx)
        }
    }
}

/// Match document metadata JSON against tenant context (legacy alias semantics).
pub fn metadata_matches(metadata: &serde_json::Value, ctx: &TenantContext) -> bool {
    metadata_matches_workspace_context(metadata, ctx)
        && metadata_matches_tenant_id_context(metadata, ctx)
}

fn metadata_matches_workspace_context(
    metadata: &serde_json::Value,
    tenant_ctx: &TenantContext,
) -> bool {
    let stored_workspace_raw = metadata
        .get("workspace_id")
        .and_then(|value| value.as_str())
        .map(str::trim);

    if matches!(stored_workspace_raw, None | Some("") | Some("default")) {
        return is_legacy_default_workspace_context(tenant_ctx.workspace_id.as_deref());
    }

    let Some(ctx_workspace_id) =
        parse_workspace_uuid_or_default(tenant_ctx.workspace_id.as_deref())
    else {
        return true;
    };

    let Some(stored_workspace_id) = parse_workspace_uuid_or_default(stored_workspace_raw) else {
        return false;
    };

    stored_workspace_id == ctx_workspace_id
}

fn metadata_matches_tenant_id_context(
    metadata: &serde_json::Value,
    tenant_ctx: &TenantContext,
) -> bool {
    let stored_tenant_raw = metadata
        .get("tenant_id")
        .and_then(|value| value.as_str())
        .map(str::trim);

    if matches!(stored_tenant_raw, None | Some("") | Some("default")) {
        return is_legacy_default_tenant_context(tenant_ctx.tenant_id.as_deref());
    }

    let ctx_tenant_raw = tenant_ctx.tenant_id.as_deref().map(str::trim);

    match (
        resolve_tenant_uuid(ctx_tenant_raw),
        resolve_tenant_uuid(stored_tenant_raw),
    ) {
        (Some(ctx_id), Some(stored_id)) => ctx_id == stored_id,
        _ => ctx_tenant_raw == stored_tenant_raw,
    }
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
    fn legacy_metadata_visible_from_default_alias() {
        let metadata = serde_json::json!({
            "workspace_id": "default",
            "tenant_id": "default",
        });
        assert!(metadata_matches(&metadata, &ctx("default", "default")));
    }

    #[test]
    fn uuid_metadata_hidden_from_other_workspace() {
        let metadata = serde_json::json!({
            "workspace_id": default_workspace_uuid().to_string(),
            "tenant_id": default_tenant_uuid().to_string(),
        });
        let other = uuid::Uuid::new_v4().to_string();
        assert!(!metadata_matches(&metadata, &ctx("default", &other)));
    }

    #[test]
    fn strict_mode_differs_from_legacy_for_uuid_stored_properties() {
        use std::collections::HashMap;

        let mut properties = HashMap::new();
        properties.insert(
            "tenant_id".to_string(),
            serde_json::json!(default_tenant_uuid().to_string()),
        );
        properties.insert(
            "workspace_id".to_string(),
            serde_json::json!(default_workspace_uuid().to_string()),
        );
        assert!(properties_match(
            &properties,
            &ctx("default", "default"),
            IsolationMode::LegacyDocumentAlias,
        ));
        assert!(!properties_match(
            &properties,
            &ctx("default", "default"),
            IsolationMode::Strict,
        ));
    }

    #[test]
    fn slug_tenant_ids_match_literally_when_not_uuid() {
        let doc_t1 = serde_json::json!({ "tenant_id": "t1", "title": "Alpha" });
        let doc_t2 = serde_json::json!({ "tenant_id": "t2", "title": "Beta" });
        assert!(metadata_matches(&doc_t1, &ctx("t1", "default")));
        assert!(!metadata_matches(&doc_t2, &ctx("t1", "default")));
    }
}
