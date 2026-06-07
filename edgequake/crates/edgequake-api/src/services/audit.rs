//! Compliance audit helpers (DRY).

use edgequake_audit::{AuditEvent, AuditEventType, AuditResult};
use edgequake_observability::{current_request_id, RequestContext};

use crate::state::AppState;

/// Record a compliance audit event when logger is configured.
pub fn record_audit(state: &AppState, event: AuditEvent) {
    if let Some(logger) = state.audit_logger.as_ref() {
        logger.log(event);
    }
}

/// Attach request correlation from middleware context.
pub fn with_request_context(event: AuditEvent, ctx: &RequestContext) -> AuditEvent {
    event.with_request_context(None, None, Some(ctx.request_id.clone()))
}

/// Record a compliance event using task-local request ID from observability middleware.
#[allow(clippy::too_many_arguments)]
pub fn record_compliance_event(
    state: &AppState,
    tenant_id: impl Into<String>,
    event_type: AuditEventType,
    action: &str,
    result: AuditResult,
    workspace_id: Option<String>,
    user_id: Option<String>,
    resource: Option<(String, String)>,
) {
    let mut event = AuditEvent::new(tenant_id.into(), event_type, action.to_string(), result);
    if let Some(ws) = workspace_id {
        event = event.with_workspace(ws);
    }
    if let Some(uid) = user_id {
        event = event.with_user(uid);
    }
    if let Some((rt, rid)) = resource {
        event = event.with_resource(rt, rid);
    }
    if let Some(rid) = current_request_id() {
        event = event.with_request_context(None, None, Some(rid));
    }
    record_audit(state, event);
}
