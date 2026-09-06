//! Tagged principal IDs (LAW-146-19).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Principal identity for ACL / allow-set / audit (not UUID-only FK).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum PrincipalId {
    User(Uuid),
    ApiKey(Uuid),
    Master,
    Worker,
}

impl PrincipalId {
    pub const MASTER_SENTINEL: &'static str = "master";
    pub const WORKER_SENTINEL: &'static str = "worker";

    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::User(_) => "user",
            Self::ApiKey(_) => "api_key",
            Self::Master => "master",
            Self::Worker => "worker",
        }
    }

    pub fn id_str(&self) -> String {
        match self {
            Self::User(u) | Self::ApiKey(u) => u.to_string(),
            Self::Master => Self::MASTER_SENTINEL.to_string(),
            Self::Worker => Self::WORKER_SENTINEL.to_string(),
        }
    }

    /// Parse from storage columns `(principal_kind, principal_id)`.
    pub fn from_storage(kind: &str, id: &str) -> Option<Self> {
        match kind {
            "user" => Uuid::parse_str(id).ok().map(Self::User),
            "api_key" => Uuid::parse_str(id).ok().map(Self::ApiKey),
            "master" => Some(Self::Master),
            "worker" => Some(Self::Worker),
            _ => None,
        }
    }

    /// Map legacy `"master-api-key"` user_id string to [`PrincipalId::Master`].
    pub fn from_auth_user_id(user_id: &str) -> Self {
        if user_id == "master-api-key" || user_id == Self::MASTER_SENTINEL {
            return Self::Master;
        }
        if user_id.starts_with("worker-") || user_id == Self::WORKER_SENTINEL {
            return Self::Worker;
        }
        if let Ok(u) = Uuid::parse_str(user_id) {
            // Heuristic: eq_ API keys are not UUIDs; JWT sub is UUID.
            return Self::User(u);
        }
        // Non-UUID stored API key id — treat as opaque api_key via nil + string in attrs.
        Self::Worker
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_storage() {
        let u = Uuid::nil();
        let p = PrincipalId::User(u);
        assert_eq!(
            PrincipalId::from_storage(p.kind_str(), &p.id_str()),
            Some(p)
        );
        assert_eq!(
            PrincipalId::from_storage("master", "master"),
            Some(PrincipalId::Master)
        );
    }

    #[test]
    fn master_api_key_maps() {
        assert_eq!(
            PrincipalId::from_auth_user_id("master-api-key"),
            PrincipalId::Master
        );
    }

    #[test]
    fn spec146_worker_principal_maps() {
        assert_eq!(
            PrincipalId::from_auth_user_id("worker"),
            PrincipalId::Worker
        );
        assert_eq!(
            PrincipalId::from_auth_user_id("worker-ingest-1"),
            PrincipalId::Worker
        );
        assert_eq!(PrincipalId::Worker.kind_str(), "worker");
        assert_eq!(PrincipalId::Worker.id_str(), "worker");
    }
}
