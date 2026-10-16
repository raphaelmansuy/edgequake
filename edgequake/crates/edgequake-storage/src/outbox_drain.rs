//! SPEC-091 RM0 — outbox_events drain worker (migration 134).
//!
//! `EDGEQUAKE_OUTBOX_DRAIN` = off | dry-run | **on** (default).
//! Claims unprocessed rows with `FOR UPDATE SKIP LOCKED`, invokes the applier,
//! marks `processed_at`, and TTL-deletes old processed rows. Never mutates
//! document/task status (LAW-RM5).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::drain_claim::{backoff_secs, parse_drain_mode, parse_interval_secs, DrainMode};

pub const OUTBOX_DRAIN_ENV: &str = "EDGEQUAKE_OUTBOX_DRAIN";
pub const OUTBOX_DRAIN_INTERVAL_ENV: &str = "EDGEQUAKE_OUTBOX_DRAIN_INTERVAL_SECS";
pub const OUTBOX_DRAIN_TTL_DAYS_ENV: &str = "EDGEQUAKE_OUTBOX_DRAIN_TTL_DAYS";

#[derive(Debug, Clone)]
pub struct OutboxDrainConfig {
    pub mode: DrainMode,
    pub interval: Duration,
    pub batch: i64,
    pub max_attempts: i32,
    pub ttl_days: i32,
    pub workspace_id: Option<Uuid>,
}

impl OutboxDrainConfig {
    /// Default mode is **on** (RM0 locked decision).
    pub fn from_env() -> Self {
        let raw = std::env::var(OUTBOX_DRAIN_ENV).unwrap_or_default();
        let mode = parse_drain_mode(&raw, DrainMode::On);
        let ttl_days = std::env::var(OUTBOX_DRAIN_TTL_DAYS_ENV)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(7)
            .max(1);
        Self {
            mode,
            interval: parse_interval_secs(OUTBOX_DRAIN_INTERVAL_ENV, 30, 5),
            batch: 50,
            max_attempts: 6,
            ttl_days,
            workspace_id: None,
        }
    }

    pub fn with_workspace_scope(mut self, workspace_id: Uuid) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }
}

#[derive(Debug, Clone)]
pub struct OutboxEvent {
    pub id: Uuid,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub payload: Value,
    pub workspace_id: Option<Uuid>,
    pub attempt_count: i32,
}

static CLAIMED: AtomicU64 = AtomicU64::new(0);
static PROCESSED: AtomicU64 = AtomicU64::new(0);
static FAILED: AtomicU64 = AtomicU64::new(0);
static DEAD: AtomicU64 = AtomicU64::new(0);

pub fn outbox_drain_claimed_total() -> u64 {
    CLAIMED.load(Ordering::Relaxed)
}
pub fn outbox_drain_processed_total() -> u64 {
    PROCESSED.load(Ordering::Relaxed)
}
pub fn outbox_drain_failed_total() -> u64 {
    FAILED.load(Ordering::Relaxed)
}
pub fn outbox_drain_dead_total() -> u64 {
    DEAD.load(Ordering::Relaxed)
}

/// Max age (seconds) of the oldest unprocessed row, or 0 when empty.
pub async fn outbox_lag_seconds(pool: &PgPool) -> Result<i64, crate::error::StorageError> {
    let lag: Option<f64> = sqlx::query_scalar(
        r#"
        SELECT EXTRACT(EPOCH FROM (now() - MIN(created_at)))
        FROM public.outbox_events
        WHERE processed_at IS NULL
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| crate::error::StorageError::Database(format!("outbox lag: {e}")))?;
    Ok(lag.unwrap_or(0.0).max(0.0) as i64)
}

/// Spawn the periodic outbox drain. Returns `None` when mode is `off`.
pub fn spawn_outbox_drain<F, Fut>(
    pool: PgPool,
    config: OutboxDrainConfig,
    applier: F,
) -> Option<tokio::task::JoinHandle<()>>
where
    F: Fn(OutboxEvent) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<(), String>> + Send,
{
    if matches!(config.mode, DrainMode::Off) {
        return None;
    }
    tracing::info!(
        mode = ?config.mode,
        interval_s = config.interval.as_secs(),
        "SPEC-091 RM0 outbox drain worker started"
    );
    Some(tokio::spawn(async move {
        let mut ticker = tokio::time::interval(config.interval);
        loop {
            ticker.tick().await;
            if let Err(e) = drain_once(&pool, &config, &applier).await {
                tracing::warn!(error = %e, "outbox drain round failed");
            }
            if let Err(e) = ttl_delete_processed(&pool, config.ttl_days).await {
                tracing::warn!(error = %e, "outbox TTL delete failed");
            }
        }
    }))
}

/// Global claim. No workspace predicate, so `idx_outbox_events_unprocessed`
/// can return the oldest due rows and stop at `LIMIT`.
const OUTBOX_CLAIM_OPEN: &str = r#"
        UPDATE public.outbox_events
        SET attempt_count = attempt_count + 1
        WHERE id IN (
            SELECT id FROM public.outbox_events
            WHERE processed_at IS NULL
              AND available_at <= now()
            ORDER BY available_at, created_at
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id, aggregate_type, aggregate_id, event_type, payload, workspace_id, attempt_count
        "#;

/// Scoped claim. `workspace_id = $2` lets
/// `idx_outbox_events_workspace_unprocessed` start at that workspace.
const OUTBOX_CLAIM_SCOPED: &str = r#"
        UPDATE public.outbox_events
        SET attempt_count = attempt_count + 1
        WHERE id IN (
            SELECT id FROM public.outbox_events
            WHERE processed_at IS NULL
              AND available_at <= now()
              AND workspace_id = $2
            ORDER BY available_at, created_at
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id, aggregate_type, aggregate_id, event_type, payload, workspace_id, attempt_count
        "#;

/// One drain round (public for contracts / chaos).
pub async fn drain_once<F, Fut>(
    pool: &PgPool,
    config: &OutboxDrainConfig,
    applier: &F,
) -> Result<(), crate::error::StorageError>
where
    F: Fn(OutboxEvent) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<(), String>> + Send,
{
    // `$2 IS NULL OR workspace_id = $2` hides the workspace from a generic
    // plan, so a scoped drain walks every older event and throws the other
    // workspaces away. Require the workspace when the caller set one.
    let claimed = if let Some(workspace_id) = config.workspace_id {
        sqlx::query_as::<_, (Uuid, String, Uuid, String, Value, Option<Uuid>, i32)>(
            OUTBOX_CLAIM_SCOPED,
        )
        .bind(config.batch)
        .bind(workspace_id)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, (Uuid, String, Uuid, String, Value, Option<Uuid>, i32)>(
            OUTBOX_CLAIM_OPEN,
        )
        .bind(config.batch)
        .fetch_all(pool)
        .await
    }
    .map_err(|e| crate::error::StorageError::Database(format!("outbox claim failed: {e}")))?;

    if claimed.is_empty() {
        return Ok(());
    }
    CLAIMED.fetch_add(claimed.len() as u64, Ordering::Relaxed);

    for (id, aggregate_type, aggregate_id, event_type, payload, workspace_id, attempt_count) in
        claimed
    {
        let event = OutboxEvent {
            id,
            aggregate_type,
            aggregate_id,
            event_type,
            payload,
            workspace_id,
            attempt_count,
        };
        let outcome: Result<(), String> = if matches!(config.mode, DrainMode::DryRun) {
            tracing::info!(
                event_id = %event.id,
                event_type = %event.event_type,
                "outbox drain DRY-RUN claim"
            );
            Ok(())
        } else {
            applier(event.clone()).await
        };

        match (outcome, matches!(config.mode, DrainMode::DryRun)) {
            (Ok(()), true) => {
                // Dry-run: leave unprocessed (reset available_at so it can be reclaimed).
                sqlx::query(
                    "UPDATE public.outbox_events SET available_at = now() WHERE id = $1 AND processed_at IS NULL",
                )
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| {
                    crate::error::StorageError::Database(format!("outbox dry-run release: {e}"))
                })?;
            }
            (Ok(()), false) => {
                PROCESSED.fetch_add(1, Ordering::Relaxed);
                mark_processed(pool, id).await?;
            }
            (Err(e), _) if attempt_count >= config.max_attempts => {
                DEAD.fetch_add(1, Ordering::Relaxed);
                tracing::error!(%id, error = %e, "outbox event DEAD after max attempts");
                // Mark processed so it stops blocking the queue; payload retained for audit.
                mark_processed(pool, id).await?;
            }
            (Err(e), _) => {
                FAILED.fetch_add(1, Ordering::Relaxed);
                let backoff = backoff_secs(attempt_count, 300);
                tracing::warn!(%id, error = %e, backoff_secs = backoff, "outbox apply failed");
                sqlx::query(
                    r#"
                    UPDATE public.outbox_events
                    SET available_at = now() + make_interval(secs => $2)
                    WHERE id = $1 AND processed_at IS NULL
                    "#,
                )
                .bind(id)
                .bind(backoff)
                .execute(pool)
                .await
                .map_err(|err| {
                    crate::error::StorageError::Database(format!("outbox backoff: {err}"))
                })?;
            }
        }
    }
    Ok(())
}

async fn mark_processed(pool: &PgPool, id: Uuid) -> Result<(), crate::error::StorageError> {
    sqlx::query(
        "UPDATE public.outbox_events SET processed_at = now() WHERE id = $1 AND processed_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| crate::error::StorageError::Database(format!("outbox mark processed: {e}")))?;
    Ok(())
}

async fn ttl_delete_processed(
    pool: &PgPool,
    ttl_days: i32,
) -> Result<u64, crate::error::StorageError> {
    let res = sqlx::query(
        r#"
        DELETE FROM public.outbox_events
        WHERE processed_at IS NOT NULL
          AND processed_at < now() - make_interval(days => $1)
        "#,
    )
    .bind(ttl_days)
    .execute(pool)
    .await
    .map_err(|e| crate::error::StorageError::Database(format!("outbox TTL delete: {e}")))?;
    Ok(res.rows_affected())
}

/// Simulate connection abort after claim: leave rows unprocessed with bumped attempt_count.
/// Used by chaos contract (RM-AC-02).
pub async fn chaos_claim_without_ack(
    pool: &PgPool,
    batch: i64,
) -> Result<usize, crate::error::StorageError> {
    let claimed = sqlx::query_as::<_, (Uuid,)>(
        r#"
        UPDATE public.outbox_events
        SET attempt_count = attempt_count + 1
        WHERE id IN (
            SELECT id FROM public.outbox_events
            WHERE processed_at IS NULL AND available_at <= now()
            ORDER BY available_at, created_at
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id
        "#,
    )
    .bind(batch)
    .fetch_all(pool)
    .await
    .map_err(|e| crate::error::StorageError::Database(format!("chaos claim: {e}")))?;
    Ok(claimed.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_spec091_outbox_drain_default_on() {
        std::env::remove_var(OUTBOX_DRAIN_ENV);
        assert_eq!(OutboxDrainConfig::from_env().mode, DrainMode::On);
    }

    #[test]
    fn contract_spec091_outbox_drain_explicit_off() {
        std::env::set_var(OUTBOX_DRAIN_ENV, "off");
        assert_eq!(OutboxDrainConfig::from_env().mode, DrainMode::Off);
        std::env::remove_var(OUTBOX_DRAIN_ENV);
    }

    #[test]
    fn claim_sql_requires_the_workspace_only_when_scoped() {
        assert!(OUTBOX_CLAIM_SCOPED.contains("workspace_id = $2"));
        assert!(!OUTBOX_CLAIM_SCOPED.contains("$2::uuid IS NULL"));
        assert!(!OUTBOX_CLAIM_OPEN.contains("$2"));
        assert!(OUTBOX_CLAIM_OPEN.contains("ORDER BY available_at, created_at"));
    }
}
