//! Provider-independent workspace catalog visibility shared by REST and MCP.

use edgequake_auth::Role;
use edgequake_core::Workspace;
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::auth::RequestAuthContext;
use crate::state::AppState;

pub fn membership_scoped(state: &AppState) -> bool {
    super::request_authorization::binding_required(state) || state.auth.sso_active()
}

/// Filter before pagination so totals and cursors never disclose sibling workspaces.
pub async fn visible_workspace_page(
    state: &AppState,
    auth: Option<&RequestAuthContext>,
    tenant_id: Uuid,
    limit: usize,
    offset: usize,
) -> Result<(usize, Vec<Workspace>), ApiError> {
    let internal = |e: edgequake_core::Error| ApiError::Internal(e.to_string());
    if membership_scoped(state) {
        let auth = auth.ok_or_else(ApiError::unauthorized)?;
        if auth.role != Role::Admin {
            let user = Uuid::parse_str(&auth.user_id).map_err(|_| ApiError::unauthorized())?;
            let memberships = state
                .workspace_service
                .get_user_memberships(user)
                .await
                .map_err(internal)?;
            let allowed: Vec<_> = state
                .workspace_service
                .list_workspaces(tenant_id)
                .await
                .map_err(internal)?
                .into_iter()
                .filter(|ws| {
                    ws.is_active
                        && memberships.iter().any(|m| {
                            m.tenant_id == tenant_id && m.can_access_workspace(&ws.workspace_id)
                        })
                })
                .collect();
            let total = allowed.len();
            return Ok((
                total,
                allowed.into_iter().skip(offset).take(limit).collect(),
            ));
        }
    }
    let total = state
        .workspace_service
        .count_workspaces(tenant_id)
        .await
        .map_err(internal)?;
    // Avoid a redundant page query and signed SQL OFFSET overflow beyond the catalog.
    if offset >= total {
        return Ok((total, Vec::new()));
    }
    let items = state
        .workspace_service
        .list_workspaces_page(tenant_id, limit, offset)
        .await
        .map_err(internal)?;
    Ok((total, items))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn authenticated_catalog_requires_a_principal() {
        let mut state = AppState::test_state();
        state.auth.config.auth_enabled = true;
        state.auth.config.dev_mode = false;
        assert!(visible_workspace_page(&state, None, Uuid::new_v4(), 10, 0)
            .await
            .is_err());
    }
}
