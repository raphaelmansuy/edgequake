//! PostgreSQL projection delivery ledger with lease fencing.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use edgequake_storage_contracts::{
    AccessError, AccessResult, AccessScope, AckDelivery, ClaimDeliveries, DeliveryState,
    ProjectionDelivery, ProjectionEvent, ProjectionLedger as ProjectionLedgerContract,
    ProjectionOperation, QuarantineDelivery, RenewDelivery, TenantId, WorkspaceId,
};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::payload::ProjectionWorkItem;

const MAX_CLAIM_BATCH: u32 = 1_000;
const MAX_LEASE_MS: u64 = 86_400_000;

/// Worker-facing ledger extension that returns event and binding metadata.
#[async_trait]
pub trait ProjectionWorkLedger: Send + Sync {
    async fn claim_work(&self, request: &ClaimDeliveries) -> AccessResult<Vec<ProjectionWorkItem>>;

    async fn renew_work(&self, request: &RenewDelivery) -> AccessResult<()>;

    async fn acknowledge(&self, request: &AckDelivery) -> AccessResult<ProjectionDelivery>;

    async fn quarantine_work(
        &self,
        request: &QuarantineDelivery,
    ) -> AccessResult<ProjectionDelivery>;
}

/// PostgreSQL implementation of the durable delivery ledger.
#[derive(Clone)]
pub struct PgProjectionLedger {
    pool: PgPool,
}

impl PgProjectionLedger {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl ProjectionWorkLedger for PgProjectionLedger {
    async fn claim_work(&self, request: &ClaimDeliveries) -> AccessResult<Vec<ProjectionWorkItem>> {
        validate_claim(request)?;
        let mut tx = self.pool.begin().await.map_err(database_error)?;
        sqlx::query(RECLAIM_EXPIRED_SQL)
            .bind(i64::from(request.limit))
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;
        let rows = sqlx::query_as::<_, ClaimedRow>(CLAIM_SQL)
            .bind(i64::from(request.limit))
            .bind(request.owner_token)
            .bind(lease_ms(request.lease_duration_ms)?)
            .fetch_all(&mut *tx)
            .await
            .map_err(database_error)?;

        // The short claim transaction is committed before work leaves this
        // method, so callers cannot hold row locks during provider I/O.
        tx.commit().await.map_err(database_error)?;
        rows.into_iter().map(ClaimedRow::try_into_work).collect()
    }

    async fn renew_work(&self, request: &RenewDelivery) -> AccessResult<()> {
        let _ = self.renew(request).await?;
        Ok(())
    }

    async fn acknowledge(&self, request: &AckDelivery) -> AccessResult<ProjectionDelivery> {
        acknowledge(&self.pool, request).await
    }

    async fn quarantine_work(
        &self,
        request: &QuarantineDelivery,
    ) -> AccessResult<ProjectionDelivery> {
        quarantine(&self.pool, request).await
    }
}

#[async_trait]
impl ProjectionLedgerContract for PgProjectionLedger {
    async fn claim(&self, request: &ClaimDeliveries) -> AccessResult<Vec<ProjectionDelivery>> {
        Ok(self
            .claim_work(request)
            .await?
            .into_iter()
            .map(|item| item.delivery)
            .collect())
    }

    async fn renew(&self, request: &RenewDelivery) -> AccessResult<ProjectionDelivery> {
        validate_lease_duration(request.lease_duration_ms)?;
        let row = sqlx::query_as::<_, DeliveryRow>(RENEW_SQL)
            .bind(request.event_id)
            .bind(request.binding_id)
            .bind(request.owner_token)
            .bind(checked_epoch(request.epoch)?)
            .bind(lease_ms(request.lease_duration_ms)?)
            .fetch_optional(&self.pool)
            .await
            .map_err(database_error)?
            .ok_or_else(lost_ownership)?;
        row.try_into_delivery()
    }

    async fn ack(&self, request: &AckDelivery) -> AccessResult<ProjectionDelivery> {
        acknowledge(&self.pool, request).await
    }

    async fn quarantine(&self, request: &QuarantineDelivery) -> AccessResult<ProjectionDelivery> {
        quarantine(&self.pool, request).await
    }
}

async fn acknowledge(pool: &PgPool, request: &AckDelivery) -> AccessResult<ProjectionDelivery> {
    let mut tx = pool.begin().await.map_err(database_error)?;
    let acknowledged = sqlx::query_scalar::<_, Uuid>(ACK_AND_VISIBILITY_SQL)
        .bind(request.event_id)
        .bind(request.binding_id)
        .bind(request.owner_token)
        .bind(checked_epoch(request.epoch)?)
        .bind(&request.completion_proof)
        .fetch_optional(&mut *tx)
        .await
        .map_err(database_error)?;

    if acknowledged.is_none() {
        tx.rollback().await.map_err(database_error)?;
        return Err(lost_ownership());
    }

    let mut delivery = load_delivery(&mut tx, request.event_id, request.binding_id).await?;
    tx.commit().await.map_err(database_error)?;
    delivery.provider_receipt = Some(request.provider_receipt.clone());
    delivery.completion_proof = Some(request.completion_proof.clone());
    Ok(delivery)
}

async fn quarantine(
    pool: &PgPool,
    request: &QuarantineDelivery,
) -> AccessResult<ProjectionDelivery> {
    let row = sqlx::query_as::<_, DeliveryRow>(QUARANTINE_SQL)
        .bind(request.event_id)
        .bind(request.binding_id)
        .bind(request.owner_token)
        .bind(checked_epoch(request.epoch)?)
        .bind(request.reason.as_bytes())
        .fetch_optional(pool)
        .await
        .map_err(database_error)?
        .ok_or_else(lost_ownership)?;
    let mut delivery = row.try_into_delivery()?;
    delivery.provider_receipt = Some(request.reason.clone());
    Ok(delivery)
}

async fn load_delivery(
    tx: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    binding_id: Uuid,
) -> AccessResult<ProjectionDelivery> {
    sqlx::query_as::<_, DeliveryRow>(LOAD_DELIVERY_SQL)
        .bind(event_id)
        .bind(binding_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(database_error)?
        .try_into_delivery()
}

fn validate_claim(request: &ClaimDeliveries) -> AccessResult<()> {
    if request.limit == 0 || request.limit > MAX_CLAIM_BATCH {
        return Err(AccessError::InvalidInput(format!(
            "projection claim limit must be between 1 and {MAX_CLAIM_BATCH}"
        )));
    }
    validate_lease_duration(request.lease_duration_ms)
}

fn validate_lease_duration(duration_ms: u64) -> AccessResult<()> {
    if duration_ms == 0 || duration_ms > MAX_LEASE_MS {
        return Err(AccessError::InvalidInput(format!(
            "projection lease must be between 1 and {MAX_LEASE_MS} milliseconds"
        )));
    }
    Ok(())
}

fn lease_ms(value: u64) -> AccessResult<i64> {
    validate_lease_duration(value)?;
    i64::try_from(value)
        .map_err(|_| AccessError::InvalidInput("projection lease exceeds i64".into()))
}

fn checked_epoch(value: u64) -> AccessResult<i64> {
    i64::try_from(value)
        .map_err(|_| AccessError::InvalidInput("projection epoch exceeds i64".into()))
}

fn lost_ownership() -> AccessError {
    AccessError::Conflict("projection delivery lease ownership was lost".into())
}

fn database_error(error: sqlx::Error) -> AccessError {
    crate::error::StorageError::from(error).into()
}

fn parse_state(value: &str) -> AccessResult<DeliveryState> {
    match value {
        "pending" => Ok(DeliveryState::Pending),
        "retry" => Ok(DeliveryState::Retry),
        "leased" => Ok(DeliveryState::Leased),
        "applied" => Ok(DeliveryState::Applied),
        "quarantined" => Ok(DeliveryState::Quarantined),
        unknown => Err(AccessError::CorruptData(format!(
            "unknown projection delivery state '{unknown}'"
        ))),
    }
}

fn parse_operation(value: &str) -> ProjectionOperation {
    if value == "delete" || value.starts_with("delete:") {
        ProjectionOperation::Delete
    } else {
        ProjectionOperation::Upsert
    }
}

#[derive(sqlx::FromRow)]
struct DeliveryRow {
    event_id: Uuid,
    binding_id: Uuid,
    state: String,
    next_attempt_at: DateTime<Utc>,
    lease_until: Option<DateTime<Utc>>,
    lease_owner: Option<Uuid>,
    epoch: i64,
    attempts: i32,
    receipt: Option<Vec<u8>>,
}

impl DeliveryRow {
    fn try_into_delivery(self) -> AccessResult<ProjectionDelivery> {
        Ok(ProjectionDelivery {
            event_id: self.event_id,
            binding_id: self.binding_id,
            state: parse_state(&self.state)?,
            epoch: u64::try_from(self.epoch)
                .map_err(|_| AccessError::CorruptData("negative delivery epoch".into()))?,
            owner_token: self.lease_owner,
            attempt: u32::try_from(self.attempts)
                .map_err(|_| AccessError::CorruptData("negative delivery attempts".into()))?,
            due_at: self.next_attempt_at,
            lease_expires_at: self.lease_until,
            provider_receipt: None,
            completion_proof: self.receipt,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ClaimedRow {
    event_id: Uuid,
    binding_id: Uuid,
    state: String,
    next_attempt_at: DateTime<Utc>,
    lease_until: Option<DateTime<Utc>>,
    lease_owner: Option<Uuid>,
    epoch: i64,
    attempts: i32,
    receipt: Option<Vec<u8>>,
    tenant_id: Uuid,
    workspace_id: Uuid,
    object_kind: String,
    object_id: Uuid,
    object_revision: i64,
    schema_version: i32,
    operation: String,
    manifest_ref: String,
    digest: Vec<u8>,
    binding_role: String,
    binding_generation: i64,
    binding_provider: String,
    binding_config_ref: String,
    binding_layout: String,
    binding_physical_index: String,
    binding_model_descriptor: Option<String>,
    binding_state: String,
}

impl ClaimedRow {
    fn try_into_work(self) -> AccessResult<ProjectionWorkItem> {
        let digest: [u8; 32] = self
            .digest
            .try_into()
            .map_err(|_| AccessError::CorruptData("projection digest is not 32 bytes".into()))?;
        let scope = AccessScope::new(
            TenantId::new(self.tenant_id),
            WorkspaceId::new(self.workspace_id),
        );
        let event = ProjectionEvent {
            event_id: self.event_id,
            schema_version: u32::try_from(self.schema_version).map_err(|_| {
                AccessError::CorruptData("invalid projection schema version".into())
            })?,
            scope,
            object_kind: self.object_kind,
            object_id: self.object_id,
            object_revision: u64::try_from(self.object_revision)
                .map_err(|_| AccessError::CorruptData("invalid projection revision".into()))?,
            operation: parse_operation(&self.operation),
            payload_reference: self.manifest_ref,
            payload_digest: digest,
        };
        let binding_generation = u64::try_from(self.binding_generation)
            .map_err(|_| AccessError::CorruptData("invalid binding generation".into()))?;
        let delivery = DeliveryRow {
            event_id: self.event_id,
            binding_id: self.binding_id,
            state: self.state,
            next_attempt_at: self.next_attempt_at,
            lease_until: self.lease_until,
            lease_owner: self.lease_owner,
            epoch: self.epoch,
            attempts: self.attempts,
            receipt: self.receipt,
        }
        .try_into_delivery()?;
        let binding = edgequake_storage_contracts::DataBindingDescriptor {
            binding_id: self.binding_id,
            scope,
            role: edgequake_storage_contracts::BindingRole::parse(&self.binding_role)?,
            provider: self.binding_provider,
            config_ref: self.binding_config_ref,
            layout: self.binding_layout,
            physical_index: self.binding_physical_index,
            model_descriptor: self.binding_model_descriptor,
            generation: binding_generation,
            state: edgequake_storage_contracts::BindingState::parse(&self.binding_state)?,
        };
        Ok(ProjectionWorkItem {
            event,
            delivery,
            binding,
        })
    }
}

const CLAIM_SQL: &str = r#"
WITH due AS (
    SELECT event_id, binding_id
    FROM public.projection_deliveries
    WHERE state IN ('pending', 'retry') AND next_attempt_at <= now()
    ORDER BY next_attempt_at, event_id, binding_id
    LIMIT $1
    FOR UPDATE SKIP LOCKED
),
claimed AS (
    UPDATE public.projection_deliveries AS d
    SET state = 'leased',
        lease_owner = $2,
        lease_until = now() + ($3 * interval '1 millisecond'),
        epoch = d.epoch + 1,
        attempts = d.attempts + 1
    FROM due
    WHERE d.event_id = due.event_id AND d.binding_id = due.binding_id
    RETURNING d.*
)
SELECT c.event_id, c.binding_id, c.state, c.next_attempt_at, c.lease_until,
       c.lease_owner, c.epoch, c.attempts, c.receipt,
       e.tenant_id, e.workspace_id, e.object_kind, e.object_id,
       e.object_revision, e.schema_version, e.operation, e.manifest_ref, e.digest,
       b.role AS binding_role, b.generation AS binding_generation,
       b.provider AS binding_provider, b.config_ref AS binding_config_ref,
       b.layout AS binding_layout, b.physical_index AS binding_physical_index,
       b.model_descriptor AS binding_model_descriptor, b.state AS binding_state
FROM claimed c
JOIN public.projection_events e ON e.event_id = c.event_id
JOIN public.data_bindings b ON b.binding_id = c.binding_id
ORDER BY c.next_attempt_at, c.event_id, c.binding_id
"#;

const RECLAIM_EXPIRED_SQL: &str = r#"
WITH expired AS (
    SELECT event_id, binding_id
    FROM public.projection_deliveries
    WHERE state = 'leased' AND lease_until <= now()
    ORDER BY lease_until, event_id, binding_id
    LIMIT $1
    FOR UPDATE SKIP LOCKED
)
UPDATE public.projection_deliveries AS d
SET state = 'retry',
    next_attempt_at = now(),
    lease_owner = NULL,
    lease_until = NULL,
    epoch = d.epoch + 1
FROM expired
WHERE d.event_id = expired.event_id AND d.binding_id = expired.binding_id
"#;

const RENEW_SQL: &str = r#"
UPDATE public.projection_deliveries
SET lease_until = now() + ($5 * interval '1 millisecond')
WHERE event_id = $1 AND binding_id = $2
  AND state = 'leased' AND lease_owner = $3 AND epoch = $4
  AND lease_until > now()
RETURNING event_id, binding_id, state, next_attempt_at, lease_until,
          lease_owner, epoch, attempts, receipt
"#;

const ACK_AND_VISIBILITY_SQL: &str = r#"
WITH acknowledged AS (
    UPDATE public.projection_deliveries
    SET state = 'applied', lease_owner = NULL, lease_until = NULL, receipt = $5
    WHERE event_id = $1 AND binding_id = $2
      AND state = 'leased' AND lease_owner = $3 AND epoch = $4
      AND lease_until > now()
    RETURNING event_id, binding_id
),
visible AS (
    INSERT INTO public.projection_visibility (
        tenant_id, workspace_id, object_kind, object_id, object_revision,
        binding_id, completion_receipt, verified_generation
    )
    SELECT e.tenant_id, e.workspace_id, e.object_kind, e.object_id,
           e.object_revision, a.binding_id, $5, b.generation
    FROM acknowledged a
    JOIN public.projection_events e ON e.event_id = a.event_id
    JOIN public.data_bindings b ON b.binding_id = a.binding_id
    ON CONFLICT (
        tenant_id, workspace_id, object_kind, object_id, object_revision, binding_id
    ) DO UPDATE SET
        completion_receipt = EXCLUDED.completion_receipt,
        verified_generation = EXCLUDED.verified_generation
    RETURNING object_id
)
SELECT object_id FROM visible
"#;

const QUARANTINE_SQL: &str = r#"
UPDATE public.projection_deliveries
SET state = 'quarantined', lease_owner = NULL, lease_until = NULL, receipt = $5
WHERE event_id = $1 AND binding_id = $2
  AND state = 'leased' AND lease_owner = $3 AND epoch = $4
  AND lease_until > now()
RETURNING event_id, binding_id, state, next_attempt_at, lease_until,
          lease_owner, epoch, attempts, receipt
"#;

const LOAD_DELIVERY_SQL: &str = r#"
SELECT event_id, binding_id, state, next_attempt_at, lease_until,
       lease_owner, epoch, attempts, receipt
FROM public.projection_deliveries
WHERE event_id = $1 AND binding_id = $2
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claim_is_bounded_and_skip_locked() {
        assert!(CLAIM_SQL.contains("FOR UPDATE SKIP LOCKED"));
        assert!(CLAIM_SQL.contains("LIMIT $1"));
        assert!(CLAIM_SQL.contains("epoch = d.epoch + 1"));
        assert!(RECLAIM_EXPIRED_SQL.contains("epoch = d.epoch + 1"));
    }

    #[test]
    fn zero_row_ack_is_lost_ownership() {
        assert!(matches!(lost_ownership(), AccessError::Conflict(_)));
        assert!(ACK_AND_VISIBILITY_SQL.contains("lease_owner = $3 AND epoch = $4"));
        assert!(ACK_AND_VISIBILITY_SQL.contains("lease_until > now()"));
    }

    #[test]
    fn epoch_mismatch_cannot_ack_or_quarantine() {
        for statement in [ACK_AND_VISIBILITY_SQL, QUARANTINE_SQL, RENEW_SQL] {
            assert!(statement.contains("epoch = $4"));
            assert!(statement.contains("state = 'leased'"));
        }
    }
}
