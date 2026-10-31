//! SPEC-180: the one gate in front of every knowledge-graph route.
//!
//! A `normal` workspace has no graph. A request that reaches an entity, relationship,
//! community, neighborhood, graph-image or lineage route for such a workspace gets
//! 409 `profile_has_no_graph` instead of an empty graph that looks like a bug.

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::middleware::TenantContext;
use crate::services::workspace_profile::require_graph_workspace;
use crate::state::AppState;

/// Middleware: pass through unless the request's workspace has no graph.
pub async fn require_graph_profile(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let tenant = request
        .extensions()
        .get::<TenantContext>()
        .cloned()
        .unwrap_or_else(|| TenantContext::from_headers(request.headers()));
    match require_graph_workspace(&state, tenant.workspace_id.as_deref()).await {
        Ok(()) => next.run(request).await,
        Err(err) => err.into_response(),
    }
}
