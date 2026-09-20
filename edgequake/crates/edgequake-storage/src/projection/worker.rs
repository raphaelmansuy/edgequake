//! Leased projection worker. One iteration is deliberately bounded.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use edgequake_storage_contracts::{
    AccessError, AccessResult, AckDelivery, ClaimDeliveries, ProjectionEvent, QuarantineDelivery,
    RenewDelivery,
};
use uuid::Uuid;

use super::ledger::ProjectionWorkLedger;
use super::payload::{
    ProjectionApplyReceipt, ProjectionTarget, ProjectionWorkItem, PROJECTION_SCHEMA_V1,
};

#[async_trait]
pub trait GraphProjectionApplier: Send + Sync {
    async fn apply(
        &self,
        event: &ProjectionEvent,
        binding: &edgequake_storage_contracts::DataBindingDescriptor,
    ) -> AccessResult<ProjectionApplyReceipt>;
}

#[async_trait]
pub trait VectorProjectionApplier: Send + Sync {
    async fn apply(
        &self,
        event: &ProjectionEvent,
        binding: &edgequake_storage_contracts::DataBindingDescriptor,
    ) -> AccessResult<ProjectionApplyReceipt>;
}

/// Test-only stub. Must never be composed into production AppState.
#[cfg(test)]
#[derive(Debug, Default)]
pub struct NoopGraphProjectionApplier;

#[cfg(test)]
#[async_trait]
impl GraphProjectionApplier for NoopGraphProjectionApplier {
    async fn apply(
        &self,
        event: &ProjectionEvent,
        binding: &edgequake_storage_contracts::DataBindingDescriptor,
    ) -> AccessResult<ProjectionApplyReceipt> {
        Ok(noop_receipt("graph", event, binding.binding_id))
    }
}

/// Test-only stub. Must never be composed into production AppState.
#[cfg(test)]
#[derive(Debug, Default)]
pub struct NoopVectorProjectionApplier;

#[cfg(test)]
#[async_trait]
impl VectorProjectionApplier for NoopVectorProjectionApplier {
    async fn apply(
        &self,
        event: &ProjectionEvent,
        binding: &edgequake_storage_contracts::DataBindingDescriptor,
    ) -> AccessResult<ProjectionApplyReceipt> {
        Ok(noop_receipt("vector", event, binding.binding_id))
    }
}

#[cfg(test)]
fn noop_receipt(axis: &str, event: &ProjectionEvent, binding_id: Uuid) -> ProjectionApplyReceipt {
    ProjectionApplyReceipt {
        provider_receipt: format!("p0-noop:{axis}:{}:{binding_id}", event.event_id),
        completion_proof: event.payload_digest.to_vec(),
    }
}

/// Physical call counters for operation-count gates and diagnostics.
#[derive(Debug, Default)]
pub struct ProjectionWorkerCounters {
    claim_calls: AtomicU64,
    graph_apply_calls: AtomicU64,
    vector_apply_calls: AtomicU64,
    ack_calls: AtomicU64,
    quarantine_calls: AtomicU64,
}

impl ProjectionWorkerCounters {
    pub fn snapshot(&self) -> ProjectionWorkerCounterSnapshot {
        ProjectionWorkerCounterSnapshot {
            claim_calls: self.claim_calls.load(Ordering::Relaxed),
            graph_apply_calls: self.graph_apply_calls.load(Ordering::Relaxed),
            vector_apply_calls: self.vector_apply_calls.load(Ordering::Relaxed),
            ack_calls: self.ack_calls.load(Ordering::Relaxed),
            quarantine_calls: self.quarantine_calls.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectionWorkerCounterSnapshot {
    pub claim_calls: u64,
    pub graph_apply_calls: u64,
    pub vector_apply_calls: u64,
    pub ack_calls: u64,
    pub quarantine_calls: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ProjectionWorkerConfig {
    pub batch_size: u32,
    pub lease_duration_ms: u64,
}

impl Default for ProjectionWorkerConfig {
    fn default() -> Self {
        Self {
            batch_size: 64,
            lease_duration_ms: 30_000,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectionRunReport {
    pub claimed: u64,
    pub applied: u64,
    pub quarantined: u64,
    pub lost_ownership: u64,
    pub transient_failures: u64,
}

pub struct ProjectionWorker {
    owner_token: Uuid,
    ledger: Arc<dyn ProjectionWorkLedger>,
    graph: Arc<dyn GraphProjectionApplier>,
    vector: Arc<dyn VectorProjectionApplier>,
    config: ProjectionWorkerConfig,
    counters: Arc<ProjectionWorkerCounters>,
}

impl ProjectionWorker {
    pub fn new(
        owner_token: Uuid,
        ledger: Arc<dyn ProjectionWorkLedger>,
        graph: Arc<dyn GraphProjectionApplier>,
        vector: Arc<dyn VectorProjectionApplier>,
        config: ProjectionWorkerConfig,
    ) -> Self {
        Self {
            owner_token,
            ledger,
            graph,
            vector,
            config,
            counters: Arc::new(ProjectionWorkerCounters::default()),
        }
    }

    pub fn counters(&self) -> Arc<ProjectionWorkerCounters> {
        Arc::clone(&self.counters)
    }

    /// Claim and process one bounded batch.
    ///
    /// `claim_work` commits its short transaction before returning, so no
    /// provider call below runs while a queue row lock is held.
    pub async fn run_once(&self) -> AccessResult<ProjectionRunReport> {
        self.counters.claim_calls.fetch_add(1, Ordering::Relaxed);
        let items = self
            .ledger
            .claim_work(&ClaimDeliveries {
                owner_token: self.owner_token,
                limit: self.config.batch_size,
                lease_duration_ms: self.config.lease_duration_ms,
            })
            .await?;

        let mut report = ProjectionRunReport {
            claimed: items.len() as u64,
            ..ProjectionRunReport::default()
        };
        for item in items {
            self.process_item(item, &mut report).await?;
        }
        Ok(report)
    }

    async fn process_item(
        &self,
        item: ProjectionWorkItem,
        report: &mut ProjectionRunReport,
    ) -> AccessResult<()> {
        if item.event.schema_version != PROJECTION_SCHEMA_V1 {
            return self
                .quarantine(
                    &item,
                    format!(
                        "unknown projection schema version {}",
                        item.event.schema_version
                    ),
                    report,
                )
                .await;
        }
        if let Err(error) = item.require_active_or_draining() {
            return self.quarantine(&item, error.to_string(), report).await;
        }

        // Renew before provider I/O so long applies cannot be stolen mid-write.
        if let Err(error) = self
            .ledger
            .renew_work(&RenewDelivery {
                event_id: item.event.event_id,
                binding_id: item.binding_id(),
                owner_token: self.owner_token,
                epoch: item.delivery.epoch,
                lease_duration_ms: self.config.lease_duration_ms,
            })
            .await
        {
            report.lost_ownership += 1;
            tracing::warn!(
                event_id = %item.event.event_id,
                binding_id = %item.binding_id(),
                error = %error,
                "projection lease renew failed before apply"
            );
            return Ok(());
        }

        let result = match item.target() {
            Some(ProjectionTarget::Graph) => {
                item.binding.require_graph()?;
                item.binding.require_provider("age")?;
                self.counters
                    .graph_apply_calls
                    .fetch_add(1, Ordering::Relaxed);
                self.graph.apply(&item.event, &item.binding).await
            }
            Some(ProjectionTarget::Vector) => {
                item.binding.require_vector()?;
                item.binding.require_provider("pgvector")?;
                self.counters
                    .vector_apply_calls
                    .fetch_add(1, Ordering::Relaxed);
                self.vector.apply(&item.event, &item.binding).await
            }
            None => {
                return self
                    .quarantine(
                        &item,
                        format!("unknown projection binding role '{}'", item.binding_role()),
                        report,
                    )
                    .await;
            }
        };

        let receipt = match result {
            Ok(receipt) => receipt,
            Err(error) if is_poison(&error) => {
                return self
                    .quarantine(&item, format!("poison projection payload: {error}"), report)
                    .await;
            }
            Err(error) => {
                report.transient_failures += 1;
                tracing::warn!(
                    event_id = %item.event.event_id,
                    binding_id = %item.binding_id(),
                    error = %error,
                    "projection provider call failed; lease will be retried after expiry"
                );
                return Ok(());
            }
        };

        // Digest proof must match the claimed event — refuse empty/mismatched proofs.
        if receipt.completion_proof != item.event.payload_digest {
            return self
                .quarantine(
                    &item,
                    "projection completion proof does not match event digest".into(),
                    report,
                )
                .await;
        }

        self.counters.ack_calls.fetch_add(1, Ordering::Relaxed);
        match self
            .ledger
            .acknowledge(&AckDelivery {
                event_id: item.event.event_id,
                binding_id: item.binding_id(),
                owner_token: self.owner_token,
                epoch: item.delivery.epoch,
                provider_receipt: receipt.provider_receipt,
                completion_proof: receipt.completion_proof,
            })
            .await
        {
            Ok(_) => report.applied += 1,
            Err(AccessError::Conflict(_)) => report.lost_ownership += 1,
            Err(error) => return Err(error),
        }
        Ok(())
    }

    async fn quarantine(
        &self,
        item: &ProjectionWorkItem,
        reason: String,
        report: &mut ProjectionRunReport,
    ) -> AccessResult<()> {
        self.counters
            .quarantine_calls
            .fetch_add(1, Ordering::Relaxed);
        match self
            .ledger
            .quarantine_work(&QuarantineDelivery {
                event_id: item.event.event_id,
                binding_id: item.binding_id(),
                owner_token: self.owner_token,
                epoch: item.delivery.epoch,
                reason,
            })
            .await
        {
            Ok(_) => report.quarantined += 1,
            Err(AccessError::Conflict(_)) => report.lost_ownership += 1,
            Err(error) => return Err(error),
        }
        Ok(())
    }
}

fn is_poison(error: &AccessError) -> bool {
    matches!(
        error,
        AccessError::InvalidInput(_)
            | AccessError::CorruptData(_)
            | AccessError::UnsupportedCapability(_)
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use chrono::Utc;
    use edgequake_storage_contracts::{
        AccessScope, DeliveryState, ProjectionDelivery, ProjectionOperation, TenantId, WorkspaceId,
    };

    use super::*;

    struct FakeLedger {
        item: Mutex<Option<ProjectionWorkItem>>,
        ack_result: Mutex<Option<AccessResult<ProjectionDelivery>>>,
        quarantines: AtomicU64,
    }

    #[async_trait]
    impl ProjectionWorkLedger for FakeLedger {
        async fn claim_work(
            &self,
            _request: &ClaimDeliveries,
        ) -> AccessResult<Vec<ProjectionWorkItem>> {
            Ok(self.item.lock().unwrap().take().into_iter().collect())
        }

        async fn renew_work(&self, _request: &RenewDelivery) -> AccessResult<()> {
            Ok(())
        }

        async fn acknowledge(&self, _request: &AckDelivery) -> AccessResult<ProjectionDelivery> {
            self.ack_result
                .lock()
                .unwrap()
                .take()
                .unwrap_or_else(|| Err(AccessError::Conflict("lost".into())))
        }

        async fn quarantine_work(
            &self,
            _request: &QuarantineDelivery,
        ) -> AccessResult<ProjectionDelivery> {
            self.quarantines.fetch_add(1, Ordering::Relaxed);
            Ok(delivery(1))
        }
    }

    fn delivery(epoch: u64) -> ProjectionDelivery {
        ProjectionDelivery {
            event_id: Uuid::from_u128(1),
            binding_id: Uuid::from_u128(2),
            state: DeliveryState::Leased,
            epoch,
            owner_token: Some(Uuid::from_u128(3)),
            attempt: 1,
            due_at: Utc::now(),
            lease_expires_at: Some(Utc::now()),
            provider_receipt: None,
            completion_proof: None,
        }
    }

    fn work(schema_version: u32) -> ProjectionWorkItem {
        let scope = AccessScope::new(
            TenantId::new(Uuid::from_u128(10)),
            WorkspaceId::new(Uuid::from_u128(11)),
        );
        ProjectionWorkItem {
            event: ProjectionEvent {
                event_id: Uuid::from_u128(1),
                schema_version,
                scope,
                object_kind: "document_batch".into(),
                object_id: Uuid::from_u128(12),
                object_revision: 1,
                operation: ProjectionOperation::Upsert,
                payload_reference: "manifest://one".into(),
                payload_digest: [7; 32],
            },
            delivery: delivery(1),
            binding: edgequake_storage_contracts::DataBindingDescriptor {
                binding_id: Uuid::from_u128(2),
                scope,
                role: edgequake_storage_contracts::BindingRole::Graph,
                provider: "age".into(),
                config_ref: "test".into(),
                layout: "colocated".into(),
                physical_index: "age".into(),
                model_descriptor: None,
                generation: 1,
                state: edgequake_storage_contracts::BindingState::Active,
            },
        }
    }

    fn worker(ledger: Arc<FakeLedger>) -> ProjectionWorker {
        ProjectionWorker::new(
            Uuid::from_u128(3),
            ledger,
            Arc::new(NoopGraphProjectionApplier),
            Arc::new(NoopVectorProjectionApplier),
            ProjectionWorkerConfig::default(),
        )
    }

    #[tokio::test]
    async fn unknown_schema_is_quarantined_before_provider_io() {
        let ledger = Arc::new(FakeLedger {
            item: Mutex::new(Some(work(99))),
            ack_result: Mutex::new(None),
            quarantines: AtomicU64::new(0),
        });
        let worker = worker(Arc::clone(&ledger));

        let report = worker.run_once().await.unwrap();

        assert_eq!(report.quarantined, 1);
        assert_eq!(ledger.quarantines.load(Ordering::Relaxed), 1);
        assert_eq!(worker.counters().snapshot().graph_apply_calls, 0);
    }

    #[tokio::test]
    async fn lost_lease_zero_row_ack_is_rejected() {
        let ledger = Arc::new(FakeLedger {
            item: Mutex::new(Some(work(PROJECTION_SCHEMA_V1))),
            ack_result: Mutex::new(Some(Err(AccessError::Conflict("lost".into())))),
            quarantines: AtomicU64::new(0),
        });
        let worker = worker(ledger);

        let report = worker.run_once().await.unwrap();

        assert_eq!(report.applied, 0);
        assert_eq!(report.lost_ownership, 1);
    }
}
