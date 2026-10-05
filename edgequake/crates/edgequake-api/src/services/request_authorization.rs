//! Provider-independent request authorization. Headers select a scope; they never grant it.

use axum::{extract::Request, http::Method};
use edgequake_auth::Role;
use edgequake_core::MembershipRole;
use uuid::Uuid;

use crate::{
    error::ApiError, handlers::auth::RequestAuthContext, middleware::TenantContext, state::AppState,
};

pub fn binding_required(state: &AppState) -> bool {
    state.security.strict_tenant_bind
        || (state.auth.config.auth_enabled && !state.auth.config.dev_mode)
}

/// Axum nested routers strip prefixes; authorize the caller's original route.
pub(crate) fn request_path(request: &Request) -> &str {
    request
        .extensions()
        .get::<axum::extract::OriginalUri>()
        .map(|uri| uri.0.path())
        .unwrap_or_else(|| request.uri().path())
}

pub fn is_query(method: &Method, path: &str) -> bool {
    method == Method::POST
        && matches!(
            path,
            "/api/v1/query"
                | "/api/v1/query/stream"
                | "/api/v1/query/context"
                | "/api/v1/query/context/search"
                | "/api/v1/graph/degrees/batch"
                | "/api/v1/ingestion/progress"
                | "/api/v1/pipeline/costs/estimate"
                | "/api/chat"
                | "/api/generate"
        )
}

pub fn is_write(method: &Method, path: &str) -> bool {
    !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) && !is_query(method, path)
}

pub fn require_request_permission(
    auth: &RequestAuthContext,
    scopes: Option<&crate::oauth::types::McpAuthScopes>,
    method: &Method,
    path: &str,
) -> Result<(), ApiError> {
    let admin = path.starts_with("/api/v1/admin/");
    if (admin && auth.role != Role::Admin)
        || (is_write(method, path) && auth.role == Role::Readonly)
    {
        return Err(ApiError::forbidden_reason("Insufficient role for request"));
    }
    if let Some(scopes) = scopes {
        let required = if admin {
            "*"
        } else if is_query(method, path) {
            crate::oauth::scopes::MCP_SCOPE_QUERY
        } else if is_write(method, path) {
            crate::oauth::scopes::MCP_SCOPE_WRITE
        } else {
            crate::oauth::scopes::MCP_SCOPE_READ
        };
        if !scopes.allows(required) {
            return Err(ApiError::forbidden_reason(
                "API key scope does not allow request",
            ));
        }
    }
    Ok(())
}

/// Resolve a selected scope and check active tenant, workspace, and membership.
/// Tenant-wide memberships apply to all its workspaces, as specified by Membership.
pub async fn membership_role(
    state: &AppState,
    user_id: &str,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
) -> Result<MembershipRole, ApiError> {
    let denied = || ApiError::forbidden_reason("No active membership for tenant/workspace scope");
    let user = Uuid::parse_str(user_id).map_err(|_| denied())?;
    let tenant = parse_scope(tenant_id, crate::middleware::default_tenant_uuid())?;
    let workspace = parse_scope(workspace_id, crate::middleware::default_workspace_uuid())?;
    let ws = state
        .workspace_service
        .get_workspace(workspace)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(denied)?;
    if !ws.is_active || ws.tenant_id != tenant {
        return Err(denied());
    }
    let active = state
        .workspace_service
        .get_tenant(tenant)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .is_some_and(|t| t.is_active);
    if !active {
        return Err(denied());
    }
    state
        .workspace_service
        .get_user_memberships(user)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .into_iter()
        .filter(|m| m.applies_to(tenant, Some(workspace)))
        .map(|m| m.role)
        .max_by_key(MembershipRole::level)
        .ok_or_else(denied)
}

fn parse_scope(raw: Option<&str>, default: Uuid) -> Result<Uuid, ApiError> {
    match raw.map(str::trim) {
        None | Some("") | Some("default") => Ok(default),
        Some(raw) => Uuid::parse_str(raw)
            .map_err(|_| ApiError::BadRequest("Malformed tenant/workspace scope".into())),
    }
}

/// Global identity/administration endpoints authorize their own target resources.
fn global_resource(path: &str) -> bool {
    [
        "/api/v1/auth/",
        "/api/v1/users",
        "/api/v1/api-keys",
        "/api/v1/tenants",
        "/api/v1/admin/",
        "/api/v1/settings/",
        "/api/v1/models",
        "/api/v1/config/",
        "/api/v1/decision/",
        "/api/v1/setup/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
}

pub async fn bind_request(state: &AppState, request: &mut Request) -> Result<(), ApiError> {
    let mut auth = request
        .extensions()
        .get::<RequestAuthContext>()
        .cloned()
        .ok_or_else(ApiError::unauthorized)?;
    let mut ctx = request
        .extensions()
        .get::<TenantContext>()
        .cloned()
        .unwrap_or_default();
    // Canonicalize aliases, case and missing dimensions once before storage.
    // The selected default workspace must never become a tenant-wide RLS scope.
    let tenant = parse_scope(
        ctx.tenant_id.as_deref(),
        crate::middleware::default_tenant_uuid(),
    )?;
    let workspace = parse_scope(
        ctx.workspace_id.as_deref(),
        crate::middleware::default_workspace_uuid(),
    )?;
    ctx.tenant_id = Some(tenant.to_string());
    ctx.workspace_id = Some(workspace.to_string());
    crate::services::tenant_isolation::attach_pg_isolation_scope(
        request,
        &ctx,
        Some(&auth.user_id),
    );
    request.extensions_mut().insert(ctx.clone());
    if auth.user_id == "master-api-key" {
        crate::middleware::audit_master_api_key_membership_bypass(state);
        return Ok(());
    }
    if !binding_required(state) || global_resource(request_path(request)) {
        return Ok(());
    }
    let role = membership_role(
        state,
        &auth.user_id,
        ctx.tenant_id.as_deref(),
        ctx.workspace_id.as_deref(),
    )
    .await?;
    if let Some(target) = request_path(request)
        .split('/')
        .collect::<Vec<_>>()
        .windows(2)
        .find(|pair| pair[0] == "workspaces")
        .map(|pair| pair[1])
    {
        if Uuid::parse_str(target).ok()
            != Some(parse_scope(
                ctx.workspace_id.as_deref(),
                crate::middleware::default_workspace_uuid(),
            )?)
        {
            return Err(ApiError::forbidden_reason(
                "Target workspace does not match authorized scope",
            ));
        }
    }
    if role == MembershipRole::Readonly {
        auth.role = Role::Readonly;
    }
    let path = request_path(request);
    // Workspace configuration/lifecycle needs a tenant administrator, rather than ordinary content write access.
    if path.contains("/workspaces")
        && is_write(request.method(), path)
        && !matches!(role, MembershipRole::Owner | MembershipRole::Admin)
    {
        return Err(ApiError::forbidden_reason(
            "Workspace administration requires owner/admin membership",
        ));
    }
    request.extensions_mut().insert(auth);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn canonical_scope_is_shared_with_rls_and_never_widens_default_workspace() {
        let mut state = AppState::test_state();
        state.auth.config.auth_enabled = false;
        state.security.strict_tenant_bind = false;
        let tenant = crate::middleware::default_tenant_uuid();
        let workspace = crate::middleware::default_workspace_uuid();
        for context in [
            TenantContext::default(),
            TenantContext {
                tenant_id: Some(tenant.to_string().to_uppercase()),
                workspace_id: Some(workspace.to_string().to_uppercase()),
                ..Default::default()
            },
        ] {
            let mut request = Request::builder()
                .uri("/api/v1/documents")
                .body(axum::body::Body::empty())
                .unwrap();
            request.extensions_mut().insert(context);
            request.extensions_mut().insert(RequestAuthContext {
                user_id: crate::middleware::default_user_uuid().to_string(),
                role: Role::User,
            });
            bind_request(&state, &mut request).await.unwrap();
            let context = request.extensions().get::<TenantContext>().unwrap();
            assert_eq!(
                context.tenant_id.as_deref(),
                Some(tenant.to_string().as_str())
            );
            assert_eq!(
                context.workspace_id.as_deref(),
                Some(workspace.to_string().as_str())
            );
            let rls = request
                .extensions()
                .get::<crate::services::tenant_isolation::PgIsolationScope>()
                .unwrap();
            assert_eq!(rls.tenant_id, tenant);
            assert_eq!(rls.workspace_id, Some(workspace));
        }
    }

    #[test]
    fn nested_router_authorization_retains_original_query_path() {
        let mut request = Request::builder()
            .uri("/query")
            .body(axum::body::Body::empty())
            .unwrap();
        request
            .extensions_mut()
            .insert(axum::extract::OriginalUri("/api/v1/query".parse().unwrap()));
        assert_eq!(request_path(&request), "/api/v1/query");
        assert!(is_query(&Method::POST, request_path(&request)));
        assert!(is_query(&Method::POST, "/api/chat"));
        assert!(!is_write(&Method::POST, "/api/generate"));
    }

    #[test]
    fn readonly_and_scoped_keys_cannot_mutate() {
        let readonly = RequestAuthContext {
            user_id: Uuid::new_v4().to_string(),
            role: Role::Readonly,
        };
        assert!(require_request_permission(
            &readonly,
            None,
            &Method::DELETE,
            "/api/v1/documents/x"
        )
        .is_err());
        assert!(
            require_request_permission(&readonly, None, &Method::POST, "/api/v1/query").is_ok()
        );
        let writer = RequestAuthContext {
            role: Role::User,
            ..readonly
        };
        let scopes = crate::oauth::types::McpAuthScopes::from_api_key_scopes(
            crate::oauth::scopes::default_api_key_scopes(),
        );
        assert!(require_request_permission(
            &writer,
            Some(&scopes),
            &Method::POST,
            "/api/v1/documents"
        )
        .is_err());
        assert!(
            require_request_permission(&writer, Some(&scopes), &Method::POST, "/api/v1/query")
                .is_ok()
        );
        assert!(require_request_permission(
            &writer,
            None,
            &Method::GET,
            "/api/v1/admin/config/defaults"
        )
        .is_err());
    }
}
