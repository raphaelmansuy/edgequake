//! Authz decision types.

use crate::reason::DenyReasonCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthzDecision {
    Allow,
    Deny(DenyReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenyReason {
    pub code: DenyReasonCode,
    pub resource_kind: &'static str,
}

impl DenyReason {
    pub fn new(code: DenyReasonCode, resource_kind: &'static str) -> Self {
        Self {
            code,
            resource_kind,
        }
    }
}
