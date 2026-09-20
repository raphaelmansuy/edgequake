//! SQLite projection ledger using immediate transactions and epoch CAS.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use edgequake_storage_contracts::{
    AccessError, AccessResult, AckDelivery, ClaimDeliveries, DeliveryState, ProjectionDelivery,
    ProjectionLedger, QuarantineDelivery, RenewDelivery,
};
use sqlx::{Connection, Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

const MAX_CLAIM_BATCH: u32 = 1_000;
const MAX_LEASE_MS: u64 = 86_400_000;

#[derive(Clone)]
pub struct SqliteProjectionLedger {
    pool: SqlitePool,
}

impl SqliteProjectionLedger {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProjectionLedger for SqliteProjectionLedger {
    async fn claim(&self, request: &ClaimDeliveries) -> AccessResult<Vec<ProjectionDelivery>> {
        validate_claim(request)?;
        let now = Utc::now().timestamp_millis();
        let lease_until = now
            .checked_add(checked_i64(request.lease_duration_ms, "lease duration")?)
            .ok_or_else(|| AccessError::InvalidInput("lease deadline overflow".into()))?;
        let mut connection = self.pool.acquire().await.map_err(database_error)?;
        let mut tx = connection
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(database_error)?;

        sqlx::query(
            "UPDATE projection_deliveries SET state='retry',next_attempt_at=?, \
                 lease_owner=NULL,lease_until=NULL,epoch=epoch+1 \
             WHERE state='leased' AND lease_until<=?",
        )
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;

        let keys = sqlx::query_as::<_, (String, String)>(
            "SELECT event_id,binding_id FROM projection_deliveries \
             WHERE state IN ('pending','retry') AND next_attempt_at<=? \
             ORDER BY next_attempt_at,event_id,binding_id LIMIT ?",
        )
        .bind(now)
        .bind(i64::from(request.limit))
        .fetch_all(&mut *tx)
        .await
        .map_err(database_error)?;

        let mut claimed = Vec::with_capacity(keys.len());
        for (event_id, binding_id) in keys {
            if let Some(row) = sqlx::query_as::<_, DeliveryRow>(
                "UPDATE projection_deliveries SET state='leased',lease_owner=?,lease_until=?, \
                     epoch=epoch+1,attempts=attempts+1 \
                 WHERE event_id=? AND binding_id=? \
                   AND state IN ('pending','retry') AND next_attempt_at<=? \
                 RETURNING event_id,binding_id,state,next_attempt_at,lease_until, \
                           lease_owner,epoch,attempts,receipt",
            )
            .bind(request.owner_token.to_string())
            .bind(lease_until)
            .bind(event_id)
            .bind(binding_id)
            .bind(now)
            .fetch_optional(&mut *tx)
            .await
            .map_err(database_error)?
            {
                claimed.push(row.try_into_delivery()?);
            }
        }
        tx.commit().await.map_err(database_error)?;
        Ok(claimed)
    }

    async fn renew(&self, request: &RenewDelivery) -> AccessResult<ProjectionDelivery> {
        validate_lease_duration(request.lease_duration_ms)?;
        let now = Utc::now().timestamp_millis();
        let lease_until = now
            .checked_add(checked_i64(request.lease_duration_ms, "lease duration")?)
            .ok_or_else(|| AccessError::InvalidInput("lease deadline overflow".into()))?;
        sqlx::query_as::<_, DeliveryRow>(
            "UPDATE projection_deliveries SET lease_until=? \
             WHERE event_id=? AND binding_id=? AND state='leased' \
               AND lease_owner=? AND epoch=? AND lease_until>? \
             RETURNING event_id,binding_id,state,next_attempt_at,lease_until, \
                       lease_owner,epoch,attempts,receipt",
        )
        .bind(lease_until)
        .bind(request.event_id.to_string())
        .bind(request.binding_id.to_string())
        .bind(request.owner_token.to_string())
        .bind(checked_i64(request.epoch, "epoch")?)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)?
        .ok_or_else(lost_ownership)?
        .try_into_delivery()
    }

    async fn ack(&self, request: &AckDelivery) -> AccessResult<ProjectionDelivery> {
        let now = Utc::now().timestamp_millis();
        let mut connection = self.pool.acquire().await.map_err(database_error)?;
        let mut tx = connection
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(database_error)?;
        let changed = sqlx::query(
            "UPDATE projection_deliveries SET state='applied',lease_owner=NULL, \
                 lease_until=NULL,receipt=? \
             WHERE event_id=? AND binding_id=? AND state='leased' \
               AND lease_owner=? AND epoch=? AND lease_until>?",
        )
        .bind(&request.completion_proof)
        .bind(request.event_id.to_string())
        .bind(request.binding_id.to_string())
        .bind(request.owner_token.to_string())
        .bind(checked_i64(request.epoch, "epoch")?)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(database_error)?
        .rows_affected();
        if changed == 0 {
            tx.rollback().await.map_err(database_error)?;
            return Err(lost_ownership());
        }
        insert_visibility(&mut tx, request).await?;
        let mut delivery = load_delivery(&mut tx, request.event_id, request.binding_id).await?;
        tx.commit().await.map_err(database_error)?;
        delivery.provider_receipt = Some(request.provider_receipt.clone());
        delivery.completion_proof = Some(request.completion_proof.clone());
        Ok(delivery)
    }

    async fn quarantine(&self, request: &QuarantineDelivery) -> AccessResult<ProjectionDelivery> {
        let now = Utc::now().timestamp_millis();
        let row = sqlx::query_as::<_, DeliveryRow>(
            "UPDATE projection_deliveries SET state='quarantined',lease_owner=NULL, \
                 lease_until=NULL,receipt=? \
             WHERE event_id=? AND binding_id=? AND state='leased' \
               AND lease_owner=? AND epoch=? AND lease_until>? \
             RETURNING event_id,binding_id,state,next_attempt_at,lease_until, \
                       lease_owner,epoch,attempts,receipt",
        )
        .bind(request.reason.as_bytes())
        .bind(request.event_id.to_string())
        .bind(request.binding_id.to_string())
        .bind(request.owner_token.to_string())
        .bind(checked_i64(request.epoch, "epoch")?)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)?
        .ok_or_else(lost_ownership)?;
        let mut delivery = row.try_into_delivery()?;
        delivery.provider_receipt = Some(request.reason.clone());
        Ok(delivery)
    }
}

async fn insert_visibility(
    tx: &mut Transaction<'_, Sqlite>,
    request: &AckDelivery,
) -> AccessResult<()> {
    sqlx::query(
        "INSERT INTO projection_visibility \
             (tenant_id,workspace_id,object_kind,object_id,object_revision,binding_id, \
              completion_receipt,verified_generation) \
         SELECT e.tenant_id,e.workspace_id,e.object_kind,e.object_id,e.object_revision, \
                d.binding_id,?,b.generation \
         FROM projection_deliveries d \
         JOIN projection_events e ON e.event_id=d.event_id \
         JOIN data_bindings b ON b.binding_id=d.binding_id \
         WHERE d.event_id=? AND d.binding_id=? \
         ON CONFLICT (tenant_id,workspace_id,object_kind,object_id,object_revision,binding_id) \
         DO UPDATE SET completion_receipt=excluded.completion_receipt, \
                       verified_generation=excluded.verified_generation",
    )
    .bind(&request.completion_proof)
    .bind(request.event_id.to_string())
    .bind(request.binding_id.to_string())
    .execute(&mut **tx)
    .await
    .map_err(database_error)?;
    Ok(())
}

async fn load_delivery(
    tx: &mut Transaction<'_, Sqlite>,
    event_id: Uuid,
    binding_id: Uuid,
) -> AccessResult<ProjectionDelivery> {
    sqlx::query_as::<_, DeliveryRow>(
        "SELECT event_id,binding_id,state,next_attempt_at,lease_until, \
                lease_owner,epoch,attempts,receipt \
         FROM projection_deliveries WHERE event_id=? AND binding_id=?",
    )
    .bind(event_id.to_string())
    .bind(binding_id.to_string())
    .fetch_one(&mut **tx)
    .await
    .map_err(database_error)?
    .try_into_delivery()
}

#[derive(sqlx::FromRow)]
struct DeliveryRow {
    event_id: String,
    binding_id: String,
    state: String,
    next_attempt_at: i64,
    lease_until: Option<i64>,
    lease_owner: Option<String>,
    epoch: i64,
    attempts: i64,
    receipt: Option<Vec<u8>>,
}

impl DeliveryRow {
    fn try_into_delivery(self) -> AccessResult<ProjectionDelivery> {
        Ok(ProjectionDelivery {
            event_id: parse_uuid(&self.event_id, "event id")?,
            binding_id: parse_uuid(&self.binding_id, "binding id")?,
            state: parse_state(&self.state)?,
            epoch: u64::try_from(self.epoch)
                .map_err(|_| AccessError::CorruptData("negative epoch".into()))?,
            owner_token: self
                .lease_owner
                .map(|value| parse_uuid(&value, "lease owner"))
                .transpose()?,
            attempt: u32::try_from(self.attempts)
                .map_err(|_| AccessError::CorruptData("invalid attempt count".into()))?,
            due_at: parse_time(self.next_attempt_at)?,
            lease_expires_at: self.lease_until.map(parse_time).transpose()?,
            provider_receipt: None,
            completion_proof: self.receipt,
        })
    }
}

fn parse_state(value: &str) -> AccessResult<DeliveryState> {
    match value {
        "pending" => Ok(DeliveryState::Pending),
        "retry" => Ok(DeliveryState::Retry),
        "leased" => Ok(DeliveryState::Leased),
        "applied" => Ok(DeliveryState::Applied),
        "quarantined" => Ok(DeliveryState::Quarantined),
        _ => Err(AccessError::CorruptData(format!(
            "unknown SQLite delivery state '{value}'"
        ))),
    }
}

fn parse_uuid(value: &str, field: &str) -> AccessResult<Uuid> {
    Uuid::parse_str(value)
        .map_err(|error| AccessError::CorruptData(format!("invalid {field}: {error}")))
}

fn parse_time(value: i64) -> AccessResult<DateTime<Utc>> {
    DateTime::from_timestamp_millis(value)
        .ok_or_else(|| AccessError::CorruptData(format!("invalid epoch millis {value}")))
}

fn validate_claim(request: &ClaimDeliveries) -> AccessResult<()> {
    if request.limit == 0 || request.limit > MAX_CLAIM_BATCH {
        return Err(AccessError::InvalidInput(format!(
            "projection claim limit must be between 1 and {MAX_CLAIM_BATCH}"
        )));
    }
    validate_lease_duration(request.lease_duration_ms)
}

fn validate_lease_duration(value: u64) -> AccessResult<()> {
    if value == 0 || value > MAX_LEASE_MS {
        return Err(AccessError::InvalidInput(format!(
            "projection lease must be between 1 and {MAX_LEASE_MS} milliseconds"
        )));
    }
    Ok(())
}

fn checked_i64(value: u64, field: &str) -> AccessResult<i64> {
    i64::try_from(value)
        .map_err(|_| AccessError::InvalidInput(format!("{field} exceeds SQLite INTEGER")))
}

fn lost_ownership() -> AccessError {
    AccessError::Conflict("projection delivery lease ownership was lost".into())
}

fn database_error(error: sqlx::Error) -> AccessError {
    AccessError::Unavailable(format!("SQLite projection ledger: {error}"))
}
