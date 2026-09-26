//! Leased projection worker. One iteration is deliberately bounded.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use edgequake_storage_contracts::{
    AccessError, AccessResult, AckDelivery, ClaimDeliveries, DataBindingDescriptor,
    ProjectionEvent, QuarantineDelivery, RenewDelivery,
};
use uuid::Uuid;

use super::ledger::ProjectionWorkLedger;
use super::payload::{
    ProjectionApplyReceipt, ProjectionTarget, ProjectionWorkItem, PROJECTION_SCHEMA_V1,
};
use super::serving_fence_port::ServingFenceOpener;

#[async_trait]
pub trait GraphProjectionApplier: Send + Sync {
    /// One provider entry for a claim-role group. Length of receipts matches `items`.
    async fn apply_batch(
        &self,
        items: &[(&ProjectionEvent, &DataBindingDescriptor)],
    ) -> AccessResult<Vec<ProjectionApplyReceipt>>;

    async fn apply(
        &self,
        event: &ProjectionEvent,
        binding: &DataBindingDescriptor,
    ) -> AccessResult<ProjectionApplyReceipt> {
        let mut receipts = self.apply_batch(&[(event, binding)]).await?;
        receipts
            .pop()
            .ok_or_else(|| AccessError::CorruptData("graph apply_batch returned no receipt".into()))
    }
}

#[async_trait]
pub trait VectorProjectionApplier: Send + Sync {
    /// One provider entry for a claim-role group. Length of receipts matches `items`.
    async fn apply_batch(
        &self,
        items: &[(&ProjectionEvent, &DataBindingDescriptor)],
    ) -> AccessResult<Vec<ProjectionApplyReceipt>>;

    async fn apply(
        &self,
        event: &ProjectionEvent,
        binding: &DataBindingDescriptor,
    ) -> AccessResult<ProjectionApplyReceipt> {
        let mut receipts = self.apply_batch(&[(event, binding)]).await?;
        receipts.pop().ok_or_else(|| {
            AccessError::CorruptData("vector apply_batch returned no receipt".into())
        })
    }
}

/// Test-only stub. Must never be composed into production AppState.
#[cfg(test)]
#[derive(Debug, Default)]
pub struct NoopGraphProjectionApplier;

#[cfg(test)]
#[async_trait]
impl GraphProjectionApplier for NoopGraphProjectionApplier {
    async fn apply_batch(
        &self,
        items: &[(&ProjectionEvent, &DataBindingDescriptor)],
    ) -> AccessResult<Vec<ProjectionApplyReceipt>> {
        Ok(items
            .iter()
            .map(|(event, binding)| noop_receipt("graph", event, binding.binding_id))
            .collect())
    }
}

/// Test-only stub. Must never be composed into production AppState.
#[cfg(test)]
#[derive(Debug, Default)]
pub struct NoopVectorProjectionApplier;

#[cfg(test)]
#[async_trait]
impl VectorProjectionApplier for NoopVectorProjectionApplier {
    async fn apply_batch(
        &self,
        items: &[(&ProjectionEvent, &DataBindingDescriptor)],
    ) -> AccessResult<Vec<ProjectionApplyReceipt>> {
        Ok(items
            .iter()
            .map(|(event, binding)| noop_receipt("vector", event, binding.binding_id))
            .collect())
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
    lease_statements: AtomicU64,
    ack_statements: AtomicU64,
    quarantine_calls: AtomicU64,
}

impl ProjectionWorkerCounters {
    pub fn snapshot(&self) -> ProjectionWorkerCounterSnapshot {
        ProjectionWorkerCounterSnapshot {
            claim_calls: self.claim_calls.load(Ordering::Relaxed),
            graph_apply_calls: self.graph_apply_calls.load(Ordering::Relaxed),
            vector_apply_calls: self.vector_apply_calls.load(Ordering::Relaxed),
            ack_calls: self.ack_calls.load(Ordering::Relaxed),
            lease_statements: self.lease_statements.load(Ordering::Relaxed),
            ack_statements: self.ack_statements.load(Ordering::Relaxed),
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
    pub lease_statements: u64,
    pub ack_statements: u64,
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
    /// When set, open SPEC-091 serving fence after document_batch deliveries settle.
    serving_fence: Option<Arc<dyn ServingFenceOpener>>,
}

impl ProjectionWorker {
    pub fn new(
        owner_token: Uuid,
        ledger: Arc<dyn ProjectionWorkLedger>,
        graph: Arc<dyn GraphProjectionApplier>,
        vector: Arc<dyn VectorProjectionApplier>,
        config: ProjectionWorkerConfig,
    ) -> Self {
        Self::with_serving_fence(owner_token, ledger, graph, vector, config, None)
    }

    pub fn with_serving_fence(
        owner_token: Uuid,
        ledger: Arc<dyn ProjectionWorkLedger>,
        graph: Arc<dyn GraphProjectionApplier>,
        vector: Arc<dyn VectorProjectionApplier>,
        config: ProjectionWorkerConfig,
        serving_fence: Option<Arc<dyn ServingFenceOpener>>,
    ) -> Self {
        Self {
            owner_token,
            ledger,
            graph,
            vector,
            config,
            counters: Arc::new(ProjectionWorkerCounters::default()),
            serving_fence,
        }
    }

    pub fn counters(&self) -> Arc<ProjectionWorkerCounters> {
        Arc::clone(&self.counters)
    }

    /// Claim and process one bounded batch.
    ///
    /// `claim_work` commits its short transaction before returning, so no
    /// provider call below runs while a queue row lock is held.
    ///
    /// Apply counters and `apply_batch` run once per role present in the claim
    /// (budget `c*q + c0` with `c = 1`, `c0 = 0`).
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

        let mut graph_items = Vec::new();
        let mut vector_items = Vec::new();
        let mut other_items = Vec::new();
        for item in items {
            match item.target() {
                Some(ProjectionTarget::Graph) => graph_items.push(item),
                Some(ProjectionTarget::Vector) => vector_items.push(item),
                None => other_items.push(item),
            }
        }

        if !graph_items.is_empty() {
            self.process_role_batch(graph_items, ProjectionTarget::Graph, &mut report)
                .await?;
        }
        if !vector_items.is_empty() {
            self.process_role_batch(vector_items, ProjectionTarget::Vector, &mut report)
                .await?;
        }
        for item in other_items {
            self.quarantine(
                &item,
                format!("unknown projection binding role '{}'", item.binding_role()),
                &mut report,
            )
            .await?;
        }
        Ok(report)
    }

    async fn process_role_batch(
        &self,
        items: Vec<ProjectionWorkItem>,
        target: ProjectionTarget,
        report: &mut ProjectionRunReport,
    ) -> AccessResult<()> {
        let mut candidates = Vec::with_capacity(items.len());
        for item in items {
            if item.event.schema_version != PROJECTION_SCHEMA_V1 {
                self.quarantine(
                    &item,
                    format!(
                        "unknown projection schema version {}",
                        item.event.schema_version
                    ),
                    report,
                )
                .await?;
                continue;
            }
            if let Err(error) = item.require_active_or_draining() {
                self.quarantine(&item, error.to_string(), report).await?;
                continue;
            }
            if let Err(error) = match target {
                ProjectionTarget::Graph => {
                    item.binding.require_graph()?;
                    item.binding.require_provider("age")
                }
                ProjectionTarget::Vector => {
                    item.binding.require_vector()?;
                    item.binding.require_provider("pgvector")
                }
            } {
                self.quarantine(&item, error.to_string(), report).await?;
                continue;
            }
            candidates.push(item);
        }
        if candidates.is_empty() {
            return Ok(());
        }

        let renew_requests: Vec<RenewDelivery> = candidates
            .iter()
            .map(|item| RenewDelivery {
                event_id: item.event.event_id,
                binding_id: item.binding_id(),
                owner_token: self.owner_token,
                epoch: item.delivery.epoch,
                lease_duration_ms: self.config.lease_duration_ms,
            })
            .collect();
        let renewed = self.ledger.renew_work_batch(&renew_requests).await?;
        self.counters
            .lease_statements
            .fetch_add(1, Ordering::Relaxed);
        let renewed_set: HashSet<(Uuid, Uuid)> = renewed.into_iter().collect();
        let mut ready = Vec::with_capacity(candidates.len());
        for item in candidates {
            let key = (item.event.event_id, item.binding_id());
            if renewed_set.contains(&key) {
                ready.push(item);
            } else {
                report.lost_ownership += 1;
                tracing::warn!(
                    event_id = %key.0,
                    binding_id = %key.1,
                    "projection lease renew omitted before apply_batch"
                );
            }
        }
        if ready.is_empty() {
            return Ok(());
        }

        match target {
            ProjectionTarget::Graph => {
                self.counters
                    .graph_apply_calls
                    .fetch_add(1, Ordering::Relaxed);
            }
            ProjectionTarget::Vector => {
                self.counters
                    .vector_apply_calls
                    .fetch_add(1, Ordering::Relaxed);
            }
        }

        let pairs: Vec<(&ProjectionEvent, &DataBindingDescriptor)> = ready
            .iter()
            .map(|item| (&item.event, &item.binding))
            .collect();
        let result = match target {
            ProjectionTarget::Graph => self.graph.apply_batch(&pairs).await,
            ProjectionTarget::Vector => self.vector.apply_batch(&pairs).await,
        };

        let receipts = match result {
            Ok(receipts) => receipts,
            Err(error) if is_poison(&error) => {
                let reason = format!("poison projection payload: {error}");
                for item in ready {
                    self.quarantine(&item, reason.clone(), report).await?;
                }
                return Ok(());
            }
            Err(error) => {
                report.transient_failures += ready.len() as u64;
                for item in &ready {
                    let backoff_ms = self
                        .config
                        .lease_duration_ms
                        .saturating_mul(item.delivery.attempt.max(1) as u64)
                        .min(60_000);
                    if let Err(release_error) = self
                        .ledger
                        .release_for_retry(
                            &RenewDelivery {
                                event_id: item.event.event_id,
                                binding_id: item.binding_id(),
                                owner_token: self.owner_token,
                                epoch: item.delivery.epoch,
                                lease_duration_ms: self.config.lease_duration_ms,
                            },
                            backoff_ms,
                        )
                        .await
                    {
                        tracing::warn!(
                            event_id = %item.event.event_id,
                            error = %release_error,
                            "projection retry release failed"
                        );
                    }
                }
                tracing::warn!(
                    error = %error,
                    count = ready.len(),
                    "projection apply_batch failed; deliveries returned to retry"
                );
                return Ok(());
            }
        };

        if receipts.len() != ready.len() {
            return Err(AccessError::CorruptData(format!(
                "apply_batch returned {} receipts for {} items",
                receipts.len(),
                ready.len()
            )));
        }

        // PROVIDER-ACCESS-E2E04 B3: pause after successful apply, before ack.
        #[cfg(feature = "provider-access-fault")]
        crate::projection::fault::pause_at("b3");

        let mut batch_doc_by_event: HashMap<Uuid, Uuid> = HashMap::new();
        for item in &ready {
            if item.event.object_kind == "document_batch" {
                batch_doc_by_event.insert(item.event.event_id, item.event.object_id);
            }
        }

        let mut ack_requests = Vec::new();
        for (item, receipt) in ready.into_iter().zip(receipts) {
            if receipt.completion_proof.as_slice() != item.expected_completion_proof {
                self.quarantine(
                    &item,
                    "projection completion proof does not match the event manifest".into(),
                    report,
                )
                .await?;
                continue;
            }
            ack_requests.push(AckDelivery {
                event_id: item.event.event_id,
                binding_id: item.binding_id(),
                owner_token: self.owner_token,
                epoch: item.delivery.epoch,
                provider_receipt: receipt.provider_receipt,
                completion_proof: receipt.completion_proof,
            });
        }
        if ack_requests.is_empty() {
            return Ok(());
        }

        let applied = self.ledger.acknowledge_batch(&ack_requests).await?;
        self.counters.ack_statements.fetch_add(1, Ordering::Relaxed);
        let applied_set: HashSet<(Uuid, Uuid)> = applied.into_iter().collect();
        let mut fence_docs: HashSet<Uuid> = HashSet::new();
        for request in &ack_requests {
            let key = (request.event_id, request.binding_id);
            if applied_set.contains(&key) {
                self.counters.ack_calls.fetch_add(1, Ordering::Relaxed);
                report.applied += 1;
                if let Some(doc_id) = batch_doc_by_event.get(&request.event_id) {
                    fence_docs.insert(*doc_id);
                }
            } else {
                report.lost_ownership += 1;
            }
        }

        if let Some(opener) = self.serving_fence.as_ref() {
            for document_id in fence_docs {
                match opener.open_when_settled(document_id).await {
                    Ok(true) => {}
                    Ok(false) => {}
                    Err(error) => {
                        tracing::warn!(
                            document_id = %document_id,
                            error = %error,
                            "SPEC-091: serving fence open after projection ack failed"
                        );
                    }
                }
            }
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

        async fn renew_work_batch(
            &self,
            requests: &[RenewDelivery],
        ) -> AccessResult<Vec<(Uuid, Uuid)>> {
            Ok(requests
                .iter()
                .map(|r| (r.event_id, r.binding_id))
                .collect())
        }

        async fn acknowledge(&self, _request: &AckDelivery) -> AccessResult<ProjectionDelivery> {
            self.ack_result
                .lock()
                .unwrap()
                .take()
                .unwrap_or_else(|| Err(AccessError::Conflict("lost".into())))
        }

        async fn acknowledge_batch(
            &self,
            requests: &[AckDelivery],
        ) -> AccessResult<Vec<(Uuid, Uuid)>> {
            let mut out = Vec::new();
            for request in requests {
                match self.acknowledge(request).await {
                    Ok(_) => out.push((request.event_id, request.binding_id)),
                    Err(AccessError::Conflict(_)) => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(out)
        }

        async fn quarantine_work(
            &self,
            _request: &QuarantineDelivery,
        ) -> AccessResult<ProjectionDelivery> {
            self.quarantines.fetch_add(1, Ordering::Relaxed);
            Ok(delivery(1))
        }

        async fn release_for_retry(
            &self,
            _request: &RenewDelivery,
            _backoff_ms: u64,
        ) -> AccessResult<()> {
            Ok(())
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
            binding: DataBindingDescriptor {
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
            expected_completion_proof: [7; 32],
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

    struct RecordingOpener {
        calls: Mutex<Vec<Uuid>>,
        fail: bool,
    }

    #[async_trait]
    impl ServingFenceOpener for RecordingOpener {
        async fn open_when_settled(&self, document_id: Uuid) -> AccessResult<bool> {
            self.calls.lock().unwrap().push(document_id);
            if self.fail {
                return Err(AccessError::Unavailable("fence boom".into()));
            }
            Ok(true)
        }
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

    #[tokio::test]
    async fn serving_fence_opens_once_per_applied_document_batch() {
        let ledger = Arc::new(FakeLedger {
            item: Mutex::new(Some(work(PROJECTION_SCHEMA_V1))),
            ack_result: Mutex::new(Some(Ok(delivery(1)))),
            quarantines: AtomicU64::new(0),
        });
        let opener = Arc::new(RecordingOpener {
            calls: Mutex::new(Vec::new()),
            fail: false,
        });
        let worker = ProjectionWorker::with_serving_fence(
            Uuid::from_u128(3),
            ledger,
            Arc::new(NoopGraphProjectionApplier),
            Arc::new(NoopVectorProjectionApplier),
            ProjectionWorkerConfig::default(),
            Some(opener.clone() as Arc<dyn ServingFenceOpener>),
        );

        let report = worker.run_once().await.unwrap();

        assert_eq!(report.applied, 1);
        let calls = opener.calls.lock().unwrap().clone();
        assert_eq!(calls, vec![Uuid::from_u128(12)]);
    }

    #[tokio::test]
    async fn serving_fence_skipped_when_ack_lost_ownership() {
        let ledger = Arc::new(FakeLedger {
            item: Mutex::new(Some(work(PROJECTION_SCHEMA_V1))),
            ack_result: Mutex::new(Some(Err(AccessError::Conflict("lost".into())))),
            quarantines: AtomicU64::new(0),
        });
        let opener = Arc::new(RecordingOpener {
            calls: Mutex::new(Vec::new()),
            fail: false,
        });
        let worker = ProjectionWorker::with_serving_fence(
            Uuid::from_u128(3),
            ledger,
            Arc::new(NoopGraphProjectionApplier),
            Arc::new(NoopVectorProjectionApplier),
            ProjectionWorkerConfig::default(),
            Some(opener.clone() as Arc<dyn ServingFenceOpener>),
        );

        let report = worker.run_once().await.unwrap();

        assert_eq!(report.lost_ownership, 1);
        assert!(opener.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn serving_fence_error_is_logged_not_fatal() {
        let ledger = Arc::new(FakeLedger {
            item: Mutex::new(Some(work(PROJECTION_SCHEMA_V1))),
            ack_result: Mutex::new(Some(Ok(delivery(1)))),
            quarantines: AtomicU64::new(0),
        });
        let opener = Arc::new(RecordingOpener {
            calls: Mutex::new(Vec::new()),
            fail: true,
        });
        let worker = ProjectionWorker::with_serving_fence(
            Uuid::from_u128(3),
            ledger,
            Arc::new(NoopGraphProjectionApplier),
            Arc::new(NoopVectorProjectionApplier),
            ProjectionWorkerConfig::default(),
            Some(opener.clone() as Arc<dyn ServingFenceOpener>),
        );

        let report = worker.run_once().await.unwrap();

        assert_eq!(report.applied, 1);
        assert_eq!(opener.calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn worker_source_has_no_pgpool() {
        let src = include_str!("worker.rs");
        // Banned literal built at runtime so this assertion text does not self-match.
        let banned = format!("{}{}", "Pg", "Pool");
        assert!(
            !src.contains(&banned),
            "ProjectionWorker must depend on ServingFenceOpener, not a raw pool type"
        );
    }
}
