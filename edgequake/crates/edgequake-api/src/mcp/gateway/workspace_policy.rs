//! Workspace / tenant claim enforcement for MCP tools (FP-MCP-04, EC-MCP-30).

use edgequake_auth::Role;
use serde_json::Value;

use crate::error::ApiError;
use crate::middleware::TenantContext;

use super::json_rpc::GatewayError;

/// Auth claims beat tool-supplied workspace (EC-MCP-30).
///
/// SPEC-146: when `doc_abac` is on, missing workspace claim must not all-pass
/// (fail-closed).
pub fn enforce_workspace_claim(
    tenant_ctx: &TenantContext,
    arguments: &Value,
    auth_role: Option<Role>,
    doc_abac: bool,
) -> Result<(), GatewayError> {
    if auth_role.is_none() && !doc_abac {
        return Ok(());
    }

    let ctx_ws = tenant_ctx
        .workspace_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    if doc_abac && ctx_ws.is_none() {
        return Err(GatewayError::Api(ApiError::forbidden_reason(
            "Workspace required when document ABAC is enabled",
        )));
    }

    let Some(ctx_ws) = ctx_ws else {
        return Ok(());
    };

    let Some(arg_ws) = arguments
        .get("workspace_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        // No tool workspace arg — claim alone is fine.
        return Ok(());
    };

    if ctx_ws != arg_ws {
        return Err(GatewayError::Api(ApiError::forbidden_reason(format!(
            "workspace_id '{arg_ws}' does not match authenticated workspace claim '{ctx_ws}'"
        ))));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_auth::Role;
    use serde_json::json;

    #[test]
    fn allows_matching_workspace() {
        let ctx = TenantContext {
            workspace_id: Some("ws-a".into()),
            ..Default::default()
        };
        enforce_workspace_claim(
            &ctx,
            &json!({ "workspace_id": "ws-a" }),
            Some(Role::User),
            false,
        )
        .expect("match");
    }

    #[test]
    fn rejects_mismatch() {
        let ctx = TenantContext {
            workspace_id: Some("ws-a".into()),
            ..Default::default()
        };
        assert!(enforce_workspace_claim(
            &ctx,
            &json!({ "workspace_id": "ws-b" }),
            Some(Role::User),
            false,
        )
        .is_err());
    }

    #[test]
    fn abac_missing_workspace_fail_closed() {
        let ctx = TenantContext {
            workspace_id: None,
            ..Default::default()
        };
        assert!(enforce_workspace_claim(&ctx, &json!({}), Some(Role::User), true).is_err());
    }
}
