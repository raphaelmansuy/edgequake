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
    /// Binding identity accepted by this applier. Alternate providers override it.
    fn provider_name(&self) -> &str {
        "age"
    }

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
    /// Binding identity accepted by this applier. Alternate providers override it.
    fn provider_name(&self) -> &str {
        "pgvector"
    }

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

/// Operator override for [`ProjectionWorkerConfig::max_attempts`].
pub const PROJECTION_MAX_ATTEMPTS_ENV: &str = "EDGEQUAKE_PROJECTION_MAX_ATTEMPTS";

/// Product default: a delivery that fails this many claimed attempts is quarantined.
pub const DEFAULT_PROJECTION_MAX_ATTEMPTS: u32 = 10;

#[derive(Debug, Clone, Copy)]
pub struct ProjectionWorkerConfig {
    pub batch_size: u32,
    pub lease_duration_ms: u64,
    /// Maximum number of claimed apply attempts before a delivery is quarantined.
    ///
    /// The counter comes from the ledger (`projection_deliveries.attempts`, one
    /// per claim) and survives process restarts, so this also bounds a delivery
    /// that keeps dying mid-apply (OOM, crash loop). Zero preserves unbounded
    /// retries.
    pub max_attempts: u32,
}

impl Default for ProjectionWorkerConfig {
    fn default() -> Self {
        Self {
            batch_size: 64,
            lease_duration_ms: 30_000,
            max_attempts: DEFAULT_PROJECTION_MAX_ATTEMPTS,
        }
    }
}

impl ProjectionWorkerConfig {
    /// Config with [`PROJECTION_MAX_ATTEMPTS_ENV`] applied when it parses.
    ///
    /// Unset or unparsable values keep the product default instead of failing
    /// boot, matching the other storage tunables.
    pub fn from_env() -> Self {
        let mut config = Self::default();
        if let Some(parsed) =
            parse_max_attempts(&std::env::var(PROJECTION_MAX_ATTEMPTS_ENV).unwrap_or_default())
        {
            config.max_attempts = parsed;
        }
        config
    }
}

/// Pure parser for [`PROJECTION_MAX_ATTEMPTS_ENV`].
///
/// `0` is meaningful (unbounded retries), so it is accepted; anything that is
/// not an unsigned integer is ignored by the caller.
pub fn parse_max_attempts(raw: &str) -> Option<u32> {
    raw.trim().parse::<u32>().ok()
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
    /// (budget `c*q + c0` with `c = 1`, `c0 = 0`), plus bounded single-item
    /// isolation for deliveries that reach the retry ceiling in a failed batch.
    pub async fn run_once(&self) -> AccessResult<ProjectionRunReport> {
        self.counters.claim_calls.fetch_add(1, Ordering::Relaxed);
        let items = self
            .ledger
            .claim_work_for_providers(
                &ClaimDeliveries {
                    owner_token: self.owner_token,
                    limit: self.config.batch_size,
                    lease_duration_ms: self.config.lease_duration_ms,
                },
                self.graph.provider_name(),
                self.vector.provider_name(),
            )
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
                    item.binding.require_provider(self.graph.provider_name())
                }
                ProjectionTarget::Vector => {
                    item.binding.require_vector()?;
                    item.binding.require_provider(self.vector.provider_name())
                }
            } {
                self.quarantine(&item, error.to_string(), report).await?;
                continue;
            }
            if self.attempts_over_budget(&item) {
                self.quarantine_exhausted(
                    &item,
                    "no provider call left under the retry limit",
                    report,
                )
                .await?;
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

        self.count_apply_call(target);
        let pairs: Vec<(&ProjectionEvent, &DataBindingDescriptor)> = ready
            .iter()
            .map(|item| (&item.event, &item.binding))
            .collect();
        let result = match target {
            ProjectionTarget::Graph => self.graph.apply_batch(&pairs).await,
            ProjectionTarget::Vector => self.vector.apply_batch(&pairs).await,
        };
        drop(pairs);

        let (ready, receipts) = match result {
            // A receipt list of a different length is a provider contract
            // violation, not a transient failure: fail the iteration loudly
            // instead of acking a mismatched pair.
            Ok(batch_receipts) if batch_receipts.len() != ready.len() => {
                return Err(AccessError::CorruptData(format!(
                    "apply_batch returned {} receipts for {} items",
                    batch_receipts.len(),
                    ready.len()
                )));
            }
            Ok(batch_receipts) => (ready, batch_receipts),
            Err(error) if is_poison(&error) => {
                let reason = format!("poison projection payload: {error}");
                for item in ready {
                    self.quarantine(&item, reason.clone(), report).await?;
                }
                return Ok(());
            }
            Err(error) => {
                self.retry_failed_batch(ready, target, &error, report)
                    .await?
            }
        };

        if ready.is_empty() {
            return Ok(());
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

        let applied = match self.ledger.acknowledge_batch(&ack_requests).await {
            Ok(applied) => applied,
            Err(error) if is_poison(&error) => return Err(error),
            Err(error) => {
                // WHY: Ack/visibility failures must not leave deliveries leased
                // until expiry (seen as projecting stalls with climbing attempts).
                report.transient_failures += ack_requests.len() as u64;
                let backoff_ms = self.config.lease_duration_ms.min(60_000);
                for request in &ack_requests {
                    let renew = RenewDelivery {
                        event_id: request.event_id,
                        binding_id: request.binding_id,
                        owner_token: self.owner_token,
                        epoch: request.epoch,
                        lease_duration_ms: self.config.lease_duration_ms,
                    };
                    if let Err(release_error) =
                        self.ledger.release_for_retry(&renew, backoff_ms).await
                    {
                        tracing::warn!(
                            event_id = %request.event_id,
                            error = %release_error,
                            "projection retry release after ack failure failed"
                        );
                    }
                }
                tracing::warn!(
                    error = %error,
                    count = ack_requests.len(),
                    "projection acknowledge_batch failed; deliveries returned to retry"
                );
                return Ok(());
            }
        };
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

    /// Whether this claim already exceeded the configured attempt budget.
    ///
    /// The claim counted this attempt, so `attempt > max` means the budget is
    /// spent and no provider call may run. `0` disables the ceiling.
    fn attempts_over_budget(&self, item: &ProjectionWorkItem) -> bool {
        self.config.max_attempts > 0 && item.delivery.attempt > self.config.max_attempts
    }

    /// Whether this claim used the last attempt the ceiling allows.
    ///
    /// A failure on this claim cannot be retried, so it must quarantine. The
    /// asymmetry with [`Self::attempts_over_budget`] is deliberate: attempt
    /// `max_attempts` is still a real attempt and must be given its provider
    /// call before the delivery is retired.
    fn attempts_exhausted(&self, item: &ProjectionWorkItem) -> bool {
        self.config.max_attempts > 0 && item.delivery.attempt >= self.config.max_attempts
    }

    fn backoff_ms(&self, item: &ProjectionWorkItem) -> u64 {
        self.config
            .lease_duration_ms
            .saturating_mul(item.delivery.attempt.max(1) as u64)
            .min(60_000)
    }

    fn renew_for(&self, item: &ProjectionWorkItem) -> RenewDelivery {
        RenewDelivery {
            event_id: item.event.event_id,
            binding_id: item.binding_id(),
            owner_token: self.owner_token,
            epoch: item.delivery.epoch,
            lease_duration_ms: self.config.lease_duration_ms,
        }
    }

    fn count_apply_call(&self, target: ProjectionTarget) {
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
    }

    async fn apply_one(
        &self,
        target: ProjectionTarget,
        item: &ProjectionWorkItem,
    ) -> AccessResult<Vec<ProjectionApplyReceipt>> {
        self.count_apply_call(target);
        let single = [(&item.event, &item.binding)];
        match target {
            ProjectionTarget::Graph => self.graph.apply_batch(&single).await,
            ProjectionTarget::Vector => self.vector.apply_batch(&single).await,
        }
    }

    /// Return one delivery to the retry queue.
    async fn release_retry(
        &self,
        item: &ProjectionWorkItem,
        backoff_ms: u64,
        stage: &str,
    ) -> AccessResult<()> {
        if let Err(error) = self
            .ledger
            .release_for_retry(&self.renew_for(item), backoff_ms)
            .await
        {
            tracing::warn!(
                event_id = %item.event.event_id,
                error = %error,
                stage,
                "projection retry release failed"
            );
        }
        Ok(())
    }

    /// Quarantine a delivery that has used every attempt the ceiling allows.
    async fn quarantine_exhausted(
        &self,
        item: &ProjectionWorkItem,
        detail: &str,
        report: &mut ProjectionRunReport,
    ) -> AccessResult<()> {
        let reason = format!(
            "projection retry limit {} reached after {} claimed attempts: {detail}",
            self.config.max_attempts, item.delivery.attempt
        );
        self.quarantine(item, reason, report).await
    }

    /// Settle a batch whose provider apply failed.
    ///
    /// Healthy members go back to the retry queue. A member that reached the
    /// ceiling is re-applied on its own, so one deterministic failure cannot
    /// quarantine the rest of the batch; only the culprit is quarantined.
    /// Returns the deliveries that the isolated retry applied, with receipts.
    async fn retry_failed_batch(
        &self,
        items: Vec<ProjectionWorkItem>,
        target: ProjectionTarget,
        error: &AccessError,
        report: &mut ProjectionRunReport,
    ) -> AccessResult<(Vec<ProjectionWorkItem>, Vec<ProjectionApplyReceipt>)> {
        let count = items.len();
        report.transient_failures += count as u64;
        let mut applied = Vec::new();
        let mut receipts = Vec::new();
        for item in items {
            if !self.attempts_exhausted(&item) {
                let backoff_ms = self.backoff_ms(&item);
                self.release_retry(&item, backoff_ms, "apply").await?;
                continue;
            }
            // Isolation only distinguishes a culprit from healthy siblings. In a
            // one-delivery batch the failed call already was that delivery's
            // only call, so re-applying it would pay the same failing provider
            // call twice (the expensive timeout/OOM path) and learn nothing.
            if count == 1 {
                let detail = format!("single-delivery batch apply failed ({error})");
                self.quarantine_exhausted(&item, &detail, report).await?;
                continue;
            }
            if let Some(receipt) = self.isolate_exhausted(&item, target, error, report).await? {
                applied.push(item);
                receipts.push(receipt);
            }
        }
        tracing::warn!(
            error = %error,
            count,
            "projection apply_batch failed; deliveries returned to retry"
        );
        Ok((applied, receipts))
    }

    /// Re-apply one exhausted delivery alone and classify the outcome.
    ///
    /// Returns the receipt when the isolated retry succeeded. Every other
    /// outcome quarantines the delivery with the reason that explains it.
    async fn isolate_exhausted(
        &self,
        item: &ProjectionWorkItem,
        target: ProjectionTarget,
        batch_error: &AccessError,
        report: &mut ProjectionRunReport,
    ) -> AccessResult<Option<ProjectionApplyReceipt>> {
        let outcome = match self.apply_one(target, item).await {
            Ok(mut receipts) if receipts.len() == 1 => Ok(receipts.remove(0)),
            Ok(receipts) => Err(AccessError::CorruptData(format!(
                "single-item apply returned {} receipts",
                receipts.len()
            ))),
            Err(error) => Err(error),
        };
        match outcome {
            Ok(receipt) => Ok(Some(receipt)),
            Err(error) if is_poison(&error) => {
                let detail = format!("poison projection payload: {error}");
                self.quarantine_exhausted(item, &detail, report)
                    .await
                    .map(|()| None)
            }
            Err(error) => {
                let detail = format!(
                    "batch apply failed ({batch_error}); single-item apply failed ({error})"
                );
                self.quarantine_exhausted(item, &detail, report)
                    .await
                    .map(|()| None)
            }
        }
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

    /// Graph applier whose behaviour depends on how many items it receives.
    ///
    /// `fail_batches` models a failure caused by one member of a multi-item
    /// batch: a single-item call succeeds, so the culprit is the batch itself.
    struct SelectiveGraphApplier {
        calls: AtomicU64,
        fail_batches: bool,
        fail_everything: bool,
    }

    impl SelectiveGraphApplier {
        fn failing_batches() -> Self {
            Self {
                calls: AtomicU64::new(0),
                fail_batches: true,
                fail_everything: false,
            }
        }

        fn failing_everything() -> Self {
            Self {
                calls: AtomicU64::new(0),
                fail_batches: false,
                fail_everything: true,
            }
        }
    }

    #[async_trait]
    impl GraphProjectionApplier for SelectiveGraphApplier {
        async fn apply_batch(
            &self,
            items: &[(&ProjectionEvent, &DataBindingDescriptor)],
        ) -> AccessResult<Vec<ProjectionApplyReceipt>> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            if self.fail_everything {
                return Err(AccessError::Unavailable("projection apply failed".into()));
            }
            if self.fail_batches && items.len() > 1 {
                return Err(AccessError::Unavailable(
                    "one member poisoned the batch".into(),
                ));
            }
            Ok(items
                .iter()
                .map(|(event, binding)| noop_receipt("graph", event, binding.binding_id))
                .collect())
        }
    }

    /// Ledger that records which deliveries the worker released or quarantined.
    #[derive(Default)]
    struct RecordingLedger {
        items: Mutex<Vec<ProjectionWorkItem>>,
        released: Mutex<Vec<Uuid>>,
        quarantined: Mutex<Vec<Uuid>>,
    }

    impl RecordingLedger {
        fn with_items(items: Vec<ProjectionWorkItem>) -> Self {
            Self {
                items: Mutex::new(items),
                ..Self::default()
            }
        }

        fn released(&self) -> Vec<Uuid> {
            self.released.lock().unwrap().clone()
        }

        fn quarantined(&self) -> Vec<Uuid> {
            self.quarantined.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl ProjectionWorkLedger for RecordingLedger {
        async fn claim_work(
            &self,
            _request: &ClaimDeliveries,
        ) -> AccessResult<Vec<ProjectionWorkItem>> {
            Ok(std::mem::take(&mut *self.items.lock().unwrap()))
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

        async fn acknowledge(&self, request: &AckDelivery) -> AccessResult<ProjectionDelivery> {
            Ok(self.acked(request))
        }

        async fn acknowledge_batch(
            &self,
            requests: &[AckDelivery],
        ) -> AccessResult<Vec<(Uuid, Uuid)>> {
            Ok(requests
                .iter()
                .map(|r| (r.event_id, r.binding_id))
                .collect())
        }

        async fn quarantine_work(
            &self,
            request: &QuarantineDelivery,
        ) -> AccessResult<ProjectionDelivery> {
            self.quarantined.lock().unwrap().push(request.event_id);
            Ok(delivery(1))
        }

        async fn release_for_retry(
            &self,
            request: &RenewDelivery,
            _backoff_ms: u64,
        ) -> AccessResult<()> {
            self.released.lock().unwrap().push(request.event_id);
            Ok(())
        }
    }

    impl RecordingLedger {
        fn acked(&self, request: &AckDelivery) -> ProjectionDelivery {
            let mut acked = delivery(request.epoch);
            acked.event_id = request.event_id;
            acked.binding_id = request.binding_id;
            acked
        }
    }

    fn item_for(event_id: u128, attempt: u32) -> ProjectionWorkItem {
        let mut item = work(PROJECTION_SCHEMA_V1);
        item.event.event_id = Uuid::from_u128(event_id);
        item.delivery.event_id = Uuid::from_u128(event_id);
        item.delivery.attempt = attempt;
        item
    }

    fn worker_with(
        ledger: Arc<RecordingLedger>,
        graph: Arc<dyn GraphProjectionApplier>,
        max_attempts: u32,
    ) -> ProjectionWorker {
        ProjectionWorker::new(
            Uuid::from_u128(3),
            ledger,
            graph,
            Arc::new(NoopVectorProjectionApplier),
            ProjectionWorkerConfig {
                max_attempts,
                ..ProjectionWorkerConfig::default()
            },
        )
    }

    #[tokio::test]
    async fn exhausted_single_delivery_is_quarantined_instead_of_retried() {
        let max_attempts = 3;
        let ledger = Arc::new(RecordingLedger::with_items(vec![item_for(1, max_attempts)]));
        let graph = Arc::new(SelectiveGraphApplier::failing_everything());
        let worker = worker_with(ledger.clone(), graph.clone(), max_attempts);

        let report = worker.run_once().await.unwrap();

        assert_eq!(report.quarantined, 1);
        assert_eq!(ledger.quarantined(), vec![Uuid::from_u128(1)]);
        assert!(ledger.released().is_empty());
        // The batch call was this delivery's only call, so no redundant
        // single-item re-apply.
        assert_eq!(graph.calls.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn delivery_already_over_the_ceiling_is_quarantined_before_any_provider_call() {
        let max_attempts = 2;
        // A crash after the claim consumes an attempt without a provider call.
        // Once the budget is spent there is nothing left to apply, so the
        // delivery must be retired without another paid call.
        let ledger = Arc::new(RecordingLedger::with_items(vec![item_for(
            1,
            max_attempts + 1,
        )]));
        let graph = Arc::new(SelectiveGraphApplier::failing_everything());
        let worker = worker_with(ledger.clone(), graph.clone(), max_attempts);

        let report = worker.run_once().await.unwrap();

        assert_eq!(report.quarantined, 1);
        assert_eq!(ledger.quarantined(), vec![Uuid::from_u128(1)]);
        assert_eq!(graph.calls.load(Ordering::Relaxed), 0);
        assert_eq!(worker.counters().snapshot().graph_apply_calls, 0);
    }

    #[tokio::test]
    async fn last_allowed_attempt_still_gets_its_provider_call() {
        let max_attempts = 2;
        let ledger = Arc::new(RecordingLedger::with_items(vec![item_for(1, max_attempts)]));
        let graph = Arc::new(SelectiveGraphApplier::failing_everything());
        let worker = worker_with(ledger.clone(), graph.clone(), max_attempts);

        let report = worker.run_once().await.unwrap();

        // The delivery can fail on its last allowed attempt, so the call runs and
        // the delivery is then retired without a redundant second call.
        assert_eq!(graph.calls.load(Ordering::Relaxed), 1);
        assert_eq!(report.quarantined, 1);
    }

    #[tokio::test]
    async fn healthy_batch_member_is_not_quarantined_with_the_exhausted_one() {
        let max_attempts = 3;
        let ledger = Arc::new(RecordingLedger::with_items(vec![
            item_for(1, max_attempts),
            item_for(2, 1),
        ]));
        let graph = Arc::new(SelectiveGraphApplier::failing_batches());
        let worker = worker_with(ledger.clone(), graph.clone(), max_attempts);

        let report = worker.run_once().await.unwrap();

        // The exhausted delivery is re-applied alone and goes through, so the
        // healthy delivery is only returned to the queue for a later attempt.
        assert_eq!(report.quarantined, 0);
        assert!(ledger.quarantined().is_empty());
        assert_eq!(ledger.released(), vec![Uuid::from_u128(2)]);
        assert_eq!(report.applied, 1);
        assert_eq!(report.transient_failures, 2);
        assert_eq!(graph.calls.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn exhausted_culprit_is_quarantined_and_its_sibling_still_retries() {
        let max_attempts = 2;
        let ledger = Arc::new(RecordingLedger::with_items(vec![
            item_for(1, max_attempts),
            item_for(2, 1),
        ]));
        let graph = Arc::new(SelectiveGraphApplier::failing_everything());
        let worker = worker_with(ledger.clone(), graph.clone(), max_attempts);

        let report = worker.run_once().await.unwrap();

        assert_eq!(report.quarantined, 1);
        assert_eq!(ledger.quarantined(), vec![Uuid::from_u128(1)]);
        assert_eq!(ledger.released(), vec![Uuid::from_u128(2)]);
        assert_eq!(report.applied, 0);
    }

    #[tokio::test]
    async fn zero_max_attempts_keeps_retrying_forever() {
        let ledger = Arc::new(RecordingLedger::with_items(vec![item_for(1, 500)]));
        let graph = Arc::new(SelectiveGraphApplier::failing_everything());
        let worker = worker_with(ledger.clone(), graph.clone(), 0);

        let report = worker.run_once().await.unwrap();

        assert_eq!(report.quarantined, 0);
        assert!(ledger.quarantined().is_empty());
        assert_eq!(ledger.released(), vec![Uuid::from_u128(1)]);
        // No isolation pass when the ceiling is disabled.
        assert_eq!(graph.calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn max_attempts_env_parses_zero_and_rejects_noise() {
        assert_eq!(parse_max_attempts("0"), Some(0));
        assert_eq!(parse_max_attempts(" 12 "), Some(12));
        assert_eq!(parse_max_attempts(""), None);
        assert_eq!(parse_max_attempts("ten"), None);
        assert_eq!(parse_max_attempts("-1"), None);
    }

    #[test]
    fn config_defaults_to_a_bounded_retry_budget() {
        let config = ProjectionWorkerConfig::default();
        assert_eq!(config.max_attempts, DEFAULT_PROJECTION_MAX_ATTEMPTS);
        assert!(config.max_attempts > 0, "default must bound retries");
    }

    struct AlternateGraphApplier;

    #[async_trait]
    impl GraphProjectionApplier for AlternateGraphApplier {
        fn provider_name(&self) -> &str {
            "alternate_graph"
        }

        async fn apply_batch(
            &self,
            items: &[(&ProjectionEvent, &DataBindingDescriptor)],
        ) -> AccessResult<Vec<ProjectionApplyReceipt>> {
            NoopGraphProjectionApplier.apply_batch(items).await
        }
    }

    #[tokio::test]
    async fn selected_graph_provider_accepts_its_binding_and_rejects_others() {
        for (provider, expected_applied, expected_quarantined) in
            [("alternate_graph", 1, 0), ("age", 0, 1)]
        {
            let mut item = work(PROJECTION_SCHEMA_V1);
            item.binding.provider = provider.into();
            let ledger = Arc::new(FakeLedger {
                item: Mutex::new(Some(item)),
                ack_result: Mutex::new(Some(Ok(delivery(1)))),
                quarantines: AtomicU64::new(0),
            });
            let worker = ProjectionWorker::new(
                Uuid::from_u128(3),
                ledger,
                Arc::new(AlternateGraphApplier),
                Arc::new(NoopVectorProjectionApplier),
                ProjectionWorkerConfig::default(),
            );
            let report = worker.run_once().await.unwrap();
            assert_eq!(report.applied, expected_applied);
            assert_eq!(report.quarantined, expected_quarantined);
            assert_eq!(
                worker.counters().snapshot().graph_apply_calls,
                expected_applied
            );
        }
    }

    struct AlternateVectorApplier;

    #[async_trait]
    impl VectorProjectionApplier for AlternateVectorApplier {
        fn provider_name(&self) -> &str {
            "alternate_vector"
        }

        async fn apply_batch(
            &self,
            items: &[(&ProjectionEvent, &DataBindingDescriptor)],
        ) -> AccessResult<Vec<ProjectionApplyReceipt>> {
            NoopVectorProjectionApplier.apply_batch(items).await
        }
    }

    #[tokio::test]
    async fn selected_vector_provider_accepts_its_binding_and_rejects_others() {
        for (provider, expected_applied, expected_quarantined) in
            [("alternate_vector", 1, 0), ("pgvector", 0, 1)]
        {
            let mut item = work(PROJECTION_SCHEMA_V1);
            item.binding.role = edgequake_storage_contracts::BindingRole::Vector;
            item.binding.provider = provider.into();
            let ledger = Arc::new(FakeLedger {
                item: Mutex::new(Some(item)),
                ack_result: Mutex::new(Some(Ok(delivery(1)))),
                quarantines: AtomicU64::new(0),
            });
            let worker = ProjectionWorker::new(
                Uuid::from_u128(3),
                ledger,
                Arc::new(NoopGraphProjectionApplier),
                Arc::new(AlternateVectorApplier),
                ProjectionWorkerConfig::default(),
            );
            let report = worker.run_once().await.unwrap();
            assert_eq!(report.applied, expected_applied);
            assert_eq!(report.quarantined, expected_quarantined);
            assert_eq!(
                worker.counters().snapshot().vector_apply_calls,
                expected_applied
            );
        }
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
