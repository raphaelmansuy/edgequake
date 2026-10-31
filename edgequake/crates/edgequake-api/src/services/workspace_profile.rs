//! SPEC-180: read a workspace's profile capabilities once, for every handler (DRY).
//!
//! A workspace that cannot be read is treated as `graph`, the behavior every older
//! caller expects. Handlers never branch on the profile name; they ask the capabilities.

use edgequake_core::rag_profile::ProfileCapabilities;
use edgequake_core::Workspace;

use crate::error::ApiResult;
use crate::state::AppState;

/// The workspace row, or `None` when the id is unknown or unreadable.
pub async fn load_workspace(state: &AppState, workspace_id: &str) -> Option<Workspace> {
    let uuid = crate::middleware::resolve_workspace_uuid(Some(workspace_id))?;
    state
        .workspace_service
        .get_workspace(uuid)
        .await
        .ok()
        .flatten()
}

/// Capabilities of a workspace; `graph` when it cannot be read.
pub async fn workspace_capabilities(
    state: &AppState,
    workspace_id: Option<&str>,
) -> ProfileCapabilities {
    let Some(id) = workspace_id.map(str::trim).filter(|id| !id.is_empty()) else {
        return ProfileCapabilities::default();
    };
    load_workspace(state, id)
        .await
        .map(|ws| ws.capabilities())
        .unwrap_or_default()
}

/// 409 `profile_has_no_graph` unless the workspace has a knowledge graph.
pub async fn require_graph_workspace(
    state: &AppState,
    workspace_id: Option<&str>,
) -> ApiResult<()> {
    workspace_capabilities(state, workspace_id)
        .await
        .require_graph()?;
    Ok(())
}

/// 422 `mode_not_supported_by_profile` when the user-chosen mode word needs a graph the
/// workspace lacks. An omitted mode always passes (the engine serves chunks).
pub fn ensure_mode_allowed(workspace: Option<&Workspace>, mode: Option<&str>) -> ApiResult<()> {
    let caps = workspace.map(Workspace::capabilities).unwrap_or_default();
    caps.check_query_mode(mode)?;
    Ok(())
}

/// [`ensure_mode_allowed`] for callers holding only a workspace id.
pub async fn ensure_mode_allowed_for_id(
    state: &AppState,
    workspace_id: Option<&str>,
    mode: Option<&str>,
) -> ApiResult<()> {
    workspace_capabilities(state, workspace_id)
        .await
        .check_query_mode(mode)?;
    Ok(())
}
