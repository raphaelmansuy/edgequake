//! Per-request AuthzContext (LAW-146-4 / LAW-146-22).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::principal::PrincipalId;

/// Stamped once at request start after workspace resolve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzContext {
    pub principal: PrincipalId,
    pub workspace_id: Uuid,
    pub tenant_id: Option<Uuid>,
    /// Monotonic workspace generation (not document policy_etag).
    pub policy_generation: u64,
    pub subject_attrs: HashMap<String, serde_json::Value>,
    /// When true, PEPs must fail-closed (EDGEQUAKE_DOC_ABAC=1).
    pub abac_enabled: bool,
}

impl AuthzContext {
    pub fn new(
        principal: PrincipalId,
        workspace_id: Uuid,
        policy_generation: u64,
        abac_enabled: bool,
    ) -> Self {
        Self {
            principal,
            workspace_id,
            tenant_id: None,
            policy_generation,
            subject_attrs: HashMap::new(),
            abac_enabled,
        }
    }

    pub fn with_tenant(mut self, tenant_id: Uuid) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    pub fn with_attrs(mut self, attrs: HashMap<String, serde_json::Value>) -> Self {
        self.subject_attrs = attrs;
        self
    }

    pub fn attr_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        let mut keys: Vec<_> = self.subject_attrs.keys().collect();
        keys.sort();
        for k in keys {
            hasher.update(k.as_bytes());
            if let Some(v) = self.subject_attrs.get(k) {
                hasher.update(v.to_string().as_bytes());
            }
        }
        hex::encode(hasher.finalize())
    }
}
