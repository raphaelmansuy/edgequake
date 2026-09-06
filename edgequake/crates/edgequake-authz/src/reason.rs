//! Stable deny reason codes for audit (LAW-146-23) — never return to client.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DenyReasonCode {
    NotInAllowSet,
    CapabilityDenied,
    Quarantined,
    CedarForbid,
    BreakGlassExpired,
    WorkerDenied,
    AuthRequired,
    EmptyAllowSet,
}

impl DenyReasonCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotInAllowSet => "not_in_allow_set",
            Self::CapabilityDenied => "capability_denied",
            Self::Quarantined => "quarantined",
            Self::CedarForbid => "cedar_forbid",
            Self::BreakGlassExpired => "break_glass_expired",
            Self::WorkerDenied => "worker_denied",
            Self::AuthRequired => "auth_required",
            Self::EmptyAllowSet => "empty_allow_set",
        }
    }
}
