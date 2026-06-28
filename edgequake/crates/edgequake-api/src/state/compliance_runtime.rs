//! Compliance audit runtime — SPEC-027 ARCH-D-001 / API-SOLID-I-001.

use edgequake_audit::AuditLogger;

/// Optional compliance audit logger (PostgreSQL deployments).
#[derive(Clone)]
pub struct ComplianceRuntime {
    pub audit_logger: Option<AuditLogger>,
}
