//! SPEC-091 W4 compensation-quarantine drain worker (migration 107 typed DLQ).
//!
//! `EDGEQUAKE_COMPENSATION_DRAIN` = off (default) | dry-run | on.
//! Claims due rows with `FOR UPDATE SKIP LOCKED` (safe with N API replicas),
//! applies the registered closure, and records the outcome with bounded
//! exponential-ish backoff (`attempt_count * 5 min`, dead after 6 attempts).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::drain_claim::{backoff_secs, parse_drain_mode, parse_interval_secs};

pub use crate::drain_claim::DrainMode;

pub const COMPENSATION_DRAIN_ENV: &str = "EDGEQUAKE_COMPENSATION_DRAIN";

#[derive(Debug, Clone)]
pub struct DrainConfig {
    pub mode: DrainMode,
    pub interval: Duration,
    pub batch: i64,
    pub max_attempts: i32,
    /// Optional workspace scope for the claim query (GAP-091-13, SPEC-091 IW0).
    /// `None` = global drain (production singleton — must drain ALL workspaces);
    /// `Some(ws)` claims only entries attributed to that workspace (tests, ops).
    pub workspace_id: Option<Uuid>,
}

impl DrainConfig {
    pub fn from_env() -> Self {
        let raw = std::env::var(COMPENSATION_DRAIN_ENV).unwrap_or_default();
        let mode = parse_drain_mode(&raw, DrainMode::Off);
        Self {
            mode,
            interval: parse_interval_secs("EDGEQUAKE_COMPENSATION_DRAIN_INTERVAL_SECS", 60, 5),
            batch: 50,
            max_attempts: 6,
            workspace_id: None,
        }
    }

    /// Scope the drain to a single workspace (tests / operator runs).
    pub fn with_workspace_scope(mut self, workspace_id: Uuid) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }
}

#[derive(Debug, Clone)]
pub struct QuarantineEntry {
    pub entry_id: Uuid,
    pub document_id: Uuid,
    /// Workspace attribution from the document parent (GAP-091-13); NULL for
    /// tombstone shells. Retry/applier logic uses this to re-derive scope.
    pub workspace_id: Option<Uuid>,
    pub payload: Value,
    pub attempt_count: i32,
}

static DRAIN_CLAIMED_TOTAL: AtomicU64 = AtomicU64::new(0);
static DRAIN_RESOLVED_TOTAL: AtomicU64 = AtomicU64::new(0);
static DRAIN_DEAD_TOTAL: AtomicU64 = AtomicU64::new(0);

pub fn drain_claimed_total() -> u64 {
    DRAIN_CLAIMED_TOTAL.load(Ordering::Relaxed)
}
pub fn drain_resolved_total() -> u64 {
    DRAIN_RESOLVED_TOTAL.load(Ordering::Relaxed)
}
pub fn drain_dead_total() -> u64 {
    DRAIN_DEAD_TOTAL.load(Ordering::Relaxed)
}

/// Spawn the periodic drain. Returns `None` when mode is `off`.
pub fn spawn_compensation_drain<F, Fut>(
    pool: PgPool,
    config: DrainConfig,
    applier: F,
) -> Option<tokio::task::JoinHandle<()>>
where
    F: Fn(QuarantineEntry) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<(), String>> + Send,
{
    if matches!(config.mode, DrainMode::Off) {
        return None;
    }
    tracing::info!(mode = ?config.mode, interval_s = config.interval.as_secs(),
        "SPEC-091 compensation drain worker started");
    Some(tokio::spawn(async move {
        let mut ticker = tokio::time::interval(config.interval);
        loop {
            ticker.tick().await;
            if let Err(e) = drain_once(&pool, &config, &applier).await {
                tracing::warn!(error = %e, "compensation drain round failed");
            }
        }
    }))
}

/// Global claim. Omits the workspace predicate so
/// `idx_compensation_quarantine_due` can stop at the batch.
const QUARANTINE_CLAIM_OPEN: &str = r#"
        UPDATE public.compensation_quarantine
        SET status = 'processing', attempt_count = attempt_count + 1, updated_at = now()
        WHERE entry_id IN (
            SELECT entry_id FROM public.compensation_quarantine
            WHERE status IN ('pending', 'failed') AND next_attempt_at <= now()
            ORDER BY next_attempt_at
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING entry_id, document_id, workspace_id, payload, attempt_count
        "#;

/// Scoped claim. `workspace_id = $2` uses
/// `idx_compensation_quarantine_workspace_next`.
const QUARANTINE_CLAIM_SCOPED: &str = r#"
        UPDATE public.compensation_quarantine
        SET status = 'processing', attempt_count = attempt_count + 1, updated_at = now()
        WHERE entry_id IN (
            SELECT entry_id FROM public.compensation_quarantine
            WHERE status IN ('pending', 'failed')
              AND next_attempt_at <= now()
              AND workspace_id = $2
            ORDER BY next_attempt_at
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING entry_id, document_id, workspace_id, payload, attempt_count
        "#;

pub async fn drain_once<F, Fut>(
    pool: &PgPool,
    config: &DrainConfig,
    applier: &F,
) -> Result<(), crate::error::StorageError>
where
    F: Fn(QuarantineEntry) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<(), String>> + Send,
{
    // `$2 IS NULL OR workspace_id = $2` hides the workspace from a generic
    // plan. A scoped drain then reads every other workspace's due rows.
    // The global claim omits that predicate so `(next_attempt_at)` can stop
    // at the batch. A scoped claim requires `workspace_id = $2`.
    let claimed = if let Some(workspace_id) = config.workspace_id {
        sqlx::query_as::<_, (Uuid, Uuid, Option<Uuid>, Value, i32)>(QUARANTINE_CLAIM_SCOPED)
            .bind(config.batch)
            .bind(workspace_id)
            .fetch_all(pool)
            .await
    } else {
        sqlx::query_as::<_, (Uuid, Uuid, Option<Uuid>, Value, i32)>(QUARANTINE_CLAIM_OPEN)
            .bind(config.batch)
            .fetch_all(pool)
            .await
    }
    .map_err(|e| crate::error::StorageError::Database(format!("drain claim failed: {e}")))?;

    if claimed.is_empty() {
        return Ok(());
    }
    DRAIN_CLAIMED_TOTAL.fetch_add(claimed.len() as u64, Ordering::Relaxed);

    for (entry_id, document_id, workspace_id, payload, attempt_count) in claimed {
        let entry = QuarantineEntry {
            entry_id,
            document_id,
            workspace_id,
            payload,
            attempt_count,
        };
        let outcome: Result<(), String> = if matches!(config.mode, DrainMode::DryRun) {
            tracing::info!(%entry_id, %document_id, attempt_count, "compensation drain DRY-RUN claim");
            Ok(())
        } else {
            applier(entry).await
        };

        match (outcome, matches!(config.mode, DrainMode::DryRun)) {
            (Ok(()), true) => {
                // Dry-run never mutates lifecycle: release back to pending.
                set_status(pool, entry_id, "pending", None, None).await?;
            }
            (Ok(()), false) => {
                DRAIN_RESOLVED_TOTAL.fetch_add(1, Ordering::Relaxed);
                set_status(pool, entry_id, "resolved", None, None).await?;
            }
            (Err(e), _) if attempt_count >= config.max_attempts => {
                DRAIN_DEAD_TOTAL.fetch_add(1, Ordering::Relaxed);
                tracing::error!(%entry_id, error = %e, "compensation entry DEAD after max attempts");
                set_status(pool, entry_id, "dead", Some(e), None).await?;
            }
            (Err(e), _) => {
                let backoff = backoff_secs(attempt_count, 300);
                set_status(pool, entry_id, "failed", Some(e), Some(backoff)).await?;
            }
        }
    }
    Ok(())
}

async fn set_status(
    pool: &PgPool,
    entry_id: Uuid,
    status: &str,
    last_error: Option<String>,
    backoff_secs_opt: Option<i64>,
) -> Result<(), crate::error::StorageError> {
    sqlx::query(
        r#"
        UPDATE public.compensation_quarantine
        SET status = $2,
            last_error = $3::jsonb,
            next_attempt_at = CASE WHEN $4::bigint IS NULL THEN next_attempt_at
                                   ELSE now() + make_interval(secs => $4) END,
            updated_at = now()
        WHERE entry_id = $1
        "#,
    )
    .bind(entry_id)
    .bind(status)
    .bind(last_error.map(|e| serde_json::json!({ "error": e }).to_string()))
    .bind(backoff_secs_opt)
    .execute(pool)
    .await
    .map_err(|e| crate::error::StorageError::Database(format!("drain set_status failed: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_spec091_drain_default_off() {
        std::env::remove_var(COMPENSATION_DRAIN_ENV);
        assert_eq!(DrainConfig::from_env().mode, DrainMode::Off);
    }

    #[test]
    fn contract_spec091_drain_backoff_bounded() {
        let cfg = DrainConfig::from_env();
        assert!(cfg.max_attempts <= 6);
        assert!(cfg.interval.as_secs() >= 5);
    }

    #[test]
    fn claim_sql_requires_the_workspace_only_when_scoped() {
        assert!(QUARANTINE_CLAIM_SCOPED.contains("workspace_id = $2"));
        assert!(!QUARANTINE_CLAIM_SCOPED.contains("$2::uuid IS NULL"));
        assert!(!QUARANTINE_CLAIM_OPEN.contains("$2"));
        assert!(QUARANTINE_CLAIM_OPEN.contains("ORDER BY next_attempt_at"));
    }
}
