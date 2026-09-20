//! Durable projection delivery ledger contracts.

use crate::error::AccessResult;
use crate::relational::Digest;
use crate::scope::AccessScope;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionOperation {
    Upsert,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryState {
    Pending,
    Retry,
    Leased,
    Applied,
    Quarantined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionEvent {
    pub event_id: Uuid,
    pub schema_version: u32,
    pub scope: AccessScope,
    pub object_kind: String,
    pub object_id: Uuid,
    pub object_revision: u64,
    pub operation: ProjectionOperation,
    pub payload_reference: String,
    pub payload_digest: Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionDelivery {
    pub event_id: Uuid,
    pub binding_id: Uuid,
    pub state: DeliveryState,
    pub epoch: u64,
    pub owner_token: Option<Uuid>,
    pub attempt: u32,
    pub due_at: DateTime<Utc>,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub provider_receipt: Option<String>,
    pub completion_proof: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimDeliveries {
    pub owner_token: Uuid,
    pub limit: u32,
    pub lease_duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenewDelivery {
    pub event_id: Uuid,
    pub binding_id: Uuid,
    pub owner_token: Uuid,
    pub epoch: u64,
    pub lease_duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AckDelivery {
    pub event_id: Uuid,
    pub binding_id: Uuid,
    pub owner_token: Uuid,
    pub epoch: u64,
    pub provider_receipt: String,
    pub completion_proof: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuarantineDelivery {
    pub event_id: Uuid,
    pub binding_id: Uuid,
    pub owner_token: Uuid,
    pub epoch: u64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VisibilityKey {
    pub object_kind: String,
    pub object_id: Uuid,
    pub object_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityLifecycle {
    Staged,
    Active,
    Tombstoned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibilityState {
    pub key: VisibilityKey,
    pub lifecycle: VisibilityLifecycle,
    pub is_current: bool,
    pub serving: bool,
    pub required_bindings: u32,
    pub completed_bindings: u32,
}

#[async_trait]
pub trait ProjectionLedger: Send + Sync {
    async fn claim(&self, request: &ClaimDeliveries) -> AccessResult<Vec<ProjectionDelivery>>;

    async fn renew(&self, request: &RenewDelivery) -> AccessResult<ProjectionDelivery>;

    async fn ack(&self, request: &AckDelivery) -> AccessResult<ProjectionDelivery>;

    async fn quarantine(&self, request: &QuarantineDelivery) -> AccessResult<ProjectionDelivery>;
}

/// Authoritative, positional visibility lookup. Absence is explicitly
/// non-serving and therefore represented by `None`.
#[async_trait]
pub trait VisibilityRepository: Send + Sync {
    async fn batch_current(
        &self,
        scope: &AccessScope,
        keys: &[VisibilityKey],
    ) -> AccessResult<Vec<Option<VisibilityState>>>;
}

/// Aggregates are serving only when every contributing document is present
/// and serving. An empty contribution set cannot authorize content.
pub fn all_contributors_serving(states: &[Option<VisibilityState>]) -> bool {
    !states.is_empty()
        && states
            .iter()
            .all(|state| state.as_ref().is_some_and(|state| state.serving))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_suppresses_absent_or_non_serving_contributor() {
        let key = VisibilityKey {
            object_kind: "document".into(),
            object_id: Uuid::nil(),
            object_revision: 1,
        };
        let serving = VisibilityState {
            key: key.clone(),
            lifecycle: VisibilityLifecycle::Active,
            is_current: true,
            serving: true,
            required_bindings: 1,
            completed_bindings: 1,
        };
        let deleted = VisibilityState {
            lifecycle: VisibilityLifecycle::Tombstoned,
            serving: false,
            ..serving.clone()
        };

        assert!(all_contributors_serving(&[Some(serving.clone())]));
        assert!(!all_contributors_serving(&[Some(serving), Some(deleted)]));
        assert!(!all_contributors_serving(&[None]));
        assert!(!all_contributors_serving(&[]));
    }
}
