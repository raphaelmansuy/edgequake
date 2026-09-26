//! Canonical projection membership proofs shared by every authority adapter.

use sha2::{Digest as _, Sha256};
use uuid::Uuid;

/// One immutable member of an event's role-specific manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventManifestItem {
    pub role: String,
    pub item_kind: String,
    pub record_id: Uuid,
    pub record_revision: i64,
    pub digest: [u8; 32],
    pub logical_key: String,
}

/// Digest of the ordered role membership. An empty role has a stable proof.
pub fn role_completion_proof(items: &[EventManifestItem]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for item in items {
        hasher.update(item.role.as_bytes());
        hasher.update([0]);
        hasher.update(item.item_kind.as_bytes());
        hasher.update([0]);
        hasher.update(item.record_id.as_bytes());
        hasher.update(item.record_revision.to_be_bytes());
        hasher.update(item.digest);
        hasher.update(item.logical_key.as_bytes());
        hasher.update([0xff]);
    }
    hasher.finalize().into()
}

/// Graph node id shared by the merger and projection appliers.
pub fn canonical_graph_node_id(workspace_id: Uuid, logical_name: &str) -> String {
    crate::entity_id::EntityId::new(logical_name).scoped_graph_node_id(&workspace_id.to_string())
}

pub fn logical_key_for_payload(payload: &[u8], fallback: Uuid) -> String {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(payload) else {
        return fallback.to_string();
    };
    if let Some(node) = value.get("node_id").and_then(|entry| entry.as_str()) {
        if !node.is_empty() {
            return node.to_string();
        }
    }
    if let (Some(source), Some(target)) = (
        value.get("source").and_then(|entry| entry.as_str()),
        value.get("target").and_then(|entry| entry.as_str()),
    ) {
        if !source.is_empty() && !target.is_empty() {
            return format!("{source}->{target}");
        }
    }
    fallback.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_role_proof_is_stable_and_membership_changes_it() {
        let empty = role_completion_proof(&[]);
        assert_eq!(empty, role_completion_proof(&[]));
        let item = EventManifestItem {
            role: "vector".into(),
            item_kind: "embedding".into(),
            record_id: Uuid::nil(),
            record_revision: 1,
            digest: [9; 32],
            logical_key: "subject".into(),
        };
        assert_ne!(empty, role_completion_proof(&[item]));
    }

    #[test]
    fn canonical_graph_identity_is_workspace_scoped() {
        let workspace = Uuid::from_u128(7);
        let id = canonical_graph_node_id(workspace, "Sarah Chen");
        assert!(id.ends_with("::SARAH_CHEN"), "{id}");
        assert!(id.starts_with(&workspace.to_string()));
    }
}
