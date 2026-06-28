//! Compliance audit helpers (DRY).

use edgequake_audit::{AuditEvent, AuditEventType, AuditLogger, AuditResult};
use edgequake_observability::{current_request_id, RequestContext};

use crate::state::{AppState, ComplianceRuntime};

/// Record a compliance audit event when logger is configured.
pub fn record_audit(state: &AppState, event: AuditEvent) {
    record_audit_with_logger(state.audit_logger.as_ref(), event);
}

/// Record audit via optional logger (ISP — no full AppState).
pub fn record_audit_with_logger(audit_logger: Option<&AuditLogger>, event: AuditEvent) {
    if let Some(logger) = audit_logger {
        logger.log(event);
    }
}

/// Attach request correlation from middleware context.
pub fn with_request_context(event: AuditEvent, ctx: &RequestContext) -> AuditEvent {
    event.with_request_context(None, None, Some(ctx.request_id.clone()))
}

/// Record a compliance event using task-local request ID (ISP overload).
#[allow(clippy::too_many_arguments)]
pub fn record_compliance_event_with_logger(
    audit_logger: Option<&AuditLogger>,
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
    record_audit_with_logger(audit_logger, event);
}

/// Record a compliance event using [`ComplianceRuntime`] (ISP).
#[allow(clippy::too_many_arguments)]
pub fn record_compliance_event_runtime(
    compliance: &ComplianceRuntime,
    tenant_id: impl Into<String>,
    event_type: AuditEventType,
    action: &str,
    result: AuditResult,
    workspace_id: Option<String>,
    user_id: Option<String>,
    resource: Option<(String, String)>,
) {
    record_compliance_event_with_logger(
        compliance.audit_logger.as_ref(),
        tenant_id,
        event_type,
        action,
        result,
        workspace_id,
        user_id,
        resource,
    );
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
    record_compliance_event_runtime(
        &ComplianceRuntime {
            audit_logger: state.audit_logger.clone(),
        },
        tenant_id,
        event_type,
        action,
        result,
        workspace_id,
        user_id,
        resource,
    );
}
