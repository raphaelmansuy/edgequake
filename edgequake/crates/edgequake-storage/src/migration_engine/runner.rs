//! SPEC-091 automatic migration engine runner (07-migration-engine.md).
//!
//! All long-running data movement runs here: descriptor → lease → preflight →
//! adaptive batches (data + ledger + progress in ONE transaction, LAW-D6) →
//! verification → completed. Boot spawns [`spawn_for_serving`]; in `verify`
//! mode it registers jobs and reports estimates without moving data.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, Transaction};

use super::adaptive::AdaptiveBatchSizer;
use super::lease;
use super::{MigrationMode, MIGRATION_MODE_ENV};
use crate::error::StorageError;

/// Rollout watch: pause when active backends exceed this (plan gate).
const MAX_ACTIVE_BACKENDS: i64 = 150;

/// Outcome of one batch inside the job's transaction.
#[derive(Debug, Clone)]
pub struct BatchOutcome {
    /// Rows *scanned* this batch (drives cursor + completion %).
    pub scanned: i64,
    /// Rows written/mutated (informational; logged).
    pub written: i64,
    /// Durable misses this batch (unresolved joins, bad metadata) — bumps `failed_count`.
    pub failed: i64,
    /// Next keyset cursor; `None` ⇒ source exhausted → verification phase.
    pub next_cursor: Option<Value>,
}

/// Verification verdict for a finished backfill.
#[derive(Debug, Clone, Serialize)]
pub struct VerifyReport {
    pub metric: String,
    pub expected: i64,
    pub actual: i64,
    pub sampled: usize,
    pub mismatches: usize,
}

impl VerifyReport {
    /// Coverage gate: typed counterparts for every expected legacy row (≡ DROP
    /// 126 / LAW-139-3). Sampled equality mismatches fail only when
    /// `EDGEQUAKE_MIGRATION_VERIFY_EQUALITY=1` (opt-in copy-path CI).
    pub fn passes(&self) -> bool {
        let coverage_ok = self.actual >= self.expected;
        if !coverage_ok {
            return false;
        }
        if verify_equality_required() {
            self.mismatches == 0
        } else {
            true
        }
    }
}

/// `EDGEQUAKE_MIGRATION_VERIFY_EQUALITY` — default **off** (coverage-only,
/// SPEC-139). `1` / `true` / `on` = also require sampled mismatches == 0.
pub fn verify_equality_required() -> bool {
    match std::env::var("EDGEQUAKE_MIGRATION_VERIFY_EQUALITY") {
        Ok(v) => {
            let t = v.trim();
            t == "1" || t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("on")
        }
        Err(_) => false,
    }
}

/// A data-movement descriptor (07-migration-engine.md: "migrations as data").
#[async_trait]
pub trait BackfillJob: Send + Sync {
    /// Stable step id (UNIQUE with generation in the job ledger).
    fn step_id(&self) -> &'static str;
    /// SHA-384 hex of the descriptor definition (drift guard).
    fn step_sha384(&self) -> String;
    /// Schema generation this job belongs to (migration 108 bumps to 1).
    fn schema_generation(&self) -> i32;
    /// Ledger `reversibility` classification.
    fn reversibility(&self) -> &'static str {
        "reversible"
    }
    /// Cursor value for a fresh run (before any committed batch).
    fn initial_cursor(&self) -> Value;
    /// Exact COUNT(*) of source rows to scan (backfills must be exact).
    async fn estimate_total(&self, pool: &PgPool) -> Result<i64, StorageError>;
    /// One batch of work, executed inside the runner's transaction together
    /// with the batch ledger insert and job progress update (LAW-D6).
    async fn run_batch(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        cursor: &Value,
        limit: i64,
    ) -> Result<BatchOutcome, StorageError>;
    /// Post-exhaustion verification (coverage/checksum metrics per plan).
    async fn verify(&self, pool: &PgPool) -> Result<VerifyReport, StorageError>;
}

/// Engine tuning (env-overridable; defaults from 07-migration-engine.md).
#[derive(Debug, Clone)]
pub struct MigrationEngineConfig {
    pub batch_min: u32,
    pub batch_max: u32,
    pub batch_target_ms: u64,
    pub batch_slow_ms: u64,
    pub lease_ttl_secs: i64,
    pub throttle_sleep: Duration,
    pub owner: String,
}

impl MigrationEngineConfig {
    pub fn from_env() -> Self {
        fn env_u32(key: &str, default: u32) -> u32 {
            std::env::var(key)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
        }
        fn env_u64(key: &str, default: u64) -> u64 {
            std::env::var(key)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
        }
        Self {
            batch_min: env_u32("EDGEQUAKE_MIGRATION_BATCH_MIN", 64).max(1),
            batch_max: env_u32("EDGEQUAKE_MIGRATION_BATCH_MAX", 32_000).max(1),
            batch_target_ms: env_u64("EDGEQUAKE_MIGRATION_BATCH_TARGET_MS", 250).max(10),
            batch_slow_ms: env_u64("EDGEQUAKE_MIGRATION_BATCH_SLOW_MS", 500).max(20),
            lease_ttl_secs: env_u64("EDGEQUAKE_MIGRATION_LEASE_TTL_SECS", 60).max(5) as i64,
            throttle_sleep: Duration::from_millis(env_u64(
                "EDGEQUAKE_MIGRATION_THROTTLE_MS",
                5_000,
            )),
            owner: std::env::var("EDGEQUAKE_MIGRATION_OWNER")
                .unwrap_or_else(|_| format!("{}:{}", hostname_fallback(), std::process::id())),
        }
    }
}

fn hostname_fallback() -> String {
    std::env::var("HOSTNAME").unwrap_or_else(|_| "edgequake".into())
}

/// Boot entry point. Registers job descriptors, then:
/// - `off` / `verify` (default): returns immediately — **no ledger writes** on serve
///   (SPEC-150 WP-4). Use `edgequake migrate drain` for foreground data movement.
/// - `automatic`: spawns the runner task (resumable, leased) — dedicated worker only.
pub fn spawn_for_serving(
    pool: &PgPool,
    kv_table: String,
    vectors_table: String,
) -> Option<tokio::task::JoinHandle<()>> {
    let mode = MigrationMode::from_env();
    // SPEC-150 WP-4: serving never writes. Only explicit automatic opts in.
    if !matches!(mode, MigrationMode::Automatic) {
        tracing::debug!(
            env = MIGRATION_MODE_ENV,
            ?mode,
            "SPEC-150: migration engine not spawned on serve (need EDGEQUAKE_MIGRATION_MODE=automatic)"
        );
        return None;
    }

    let jobs = default_backfill_jobs(kv_table, vectors_table);
    let config = MigrationEngineConfig::from_env();
    let pool = pool.clone();

    Some(tokio::spawn(async move {
        if let Err(e) = run_engine(pool, jobs, config, mode).await {
            tracing::error!(error = %e, "SPEC-091 migration engine terminated with error");
        }
    }))
}

/// SPEC-150: foreground drain used by `edgequake migrate drain`.
pub async fn run_drain_foreground(
    pool: PgPool,
    kv_table: String,
    vectors_table: String,
) -> Result<(), StorageError> {
    let jobs = default_backfill_jobs(kv_table, vectors_table);
    let config = MigrationEngineConfig::from_env();
    run_engine(pool, jobs, config, MigrationMode::Automatic).await
}

fn default_backfill_jobs(
    kv_table: String,
    vectors_table: String,
) -> Vec<std::sync::Arc<dyn BackfillJob>> {
    let model = std::env::var("EDGEQUAKE_EMBEDDING_MODEL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "text-embedding-3-small".to_string());
    vec![
        std::sync::Arc::new(super::chunk_text_backfill::ChunkTextBackfillJob::new(
            kv_table,
        )),
        std::sync::Arc::new(super::family_remainder::DedupRemainderJob::new()),
        std::sync::Arc::new(super::family_remainder::ShellRemainderJob::new()),
        std::sync::Arc::new(super::family_remainder::ArtifactRemainderJob::new()),
        std::sync::Arc::new(
            super::chunk_embedding_backfill::ChunkEmbeddingBackfillJob::new(
                vectors_table,
                model.clone(),
            ),
        ),
        std::sync::Arc::new(super::fleet_embedding_backfill::FleetEmbeddingBackfillJob::new(model)),
        std::sync::Arc::new(super::fleet_provenance_stamp::FleetProvenanceStampJob::new()),
    ]
}

pub async fn run_engine(
    pool: PgPool,
    jobs: Vec<std::sync::Arc<dyn BackfillJob>>,
    config: MigrationEngineConfig,
    mode: MigrationMode,
) -> Result<(), StorageError> {
    for job in &jobs {
        let estimated = job.estimate_total(&pool).await.unwrap_or_else(|e| {
            tracing::warn!(step = job.step_id(), error = %e, "migration estimate failed");
            -1
        });
        lease::ensure_job_row(
            &pool,
            job.step_id(),
            &job.step_sha384(),
            job.schema_generation(),
            job.reversibility(),
            config.batch_min as i32,
            (estimated >= 0).then_some(estimated),
        )
        .await?;
        tracing::info!(
            step = job.step_id(),
            generation = job.schema_generation(),
            estimated_total = estimated,
            mode = ?mode,
            "SPEC-091 migration job registered"
        );
    }

    let reclaimed = lease::reclaim_verify_failed_jobs(&pool).await.unwrap_or(0);
    if reclaimed > 0 {
        tracing::info!(
            reclaimed,
            "SPEC-139: reclaimed verify-failed migration jobs for retry"
        );
    }

    if !mode.runs_jobs() {
        tracing::info!(
            env = MIGRATION_MODE_ENV,
            "SPEC-091 migration engine in verify mode — set EDGEQUAKE_MIGRATION_MODE=automatic to run"
        );
        return Ok(());
    }

    for job in &jobs {
        if let Err(e) = run_job(&pool, job.as_ref(), &config).await {
            tracing::error!(
                step = job.step_id(),
                error = %e,
                "SPEC-139: migration job failed — continuing remaining jobs"
            );
        }
    }
    Ok(())
}

async fn run_job(
    pool: &PgPool,
    job: &dyn BackfillJob,
    config: &MigrationEngineConfig,
) -> Result<(), StorageError> {
    let Some(lease_row) = lease::claim_lease(
        pool,
        job.step_id(),
        &job.step_sha384(),
        job.schema_generation(),
        &config.owner,
        config.lease_ttl_secs,
    )
    .await?
    else {
        tracing::info!(
            step = job.step_id(),
            "migration job already completed/failed or leased by another instance — skipped"
        );
        return Ok(());
    };

    tracing::info!(
        step = job.step_id(),
        job_id = %lease_row.job_id,
        owner = %config.owner,
        "migration lease claimed — starting batches"
    );

    let mut sizer = AdaptiveBatchSizer::new(
        config.batch_min,
        config.batch_max,
        config.batch_target_ms,
        config.batch_slow_ms,
    );
    let mut cursor = lease_row.cursor_position.clone();
    if cursor.is_null() || cursor == json!({}) {
        cursor = job.initial_cursor();
    }
    let mut batch_seq: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(batch_seq), 0) FROM edgequake.edgequake_migration_batch WHERE job_id = $1",
    )
    .bind(lease_row.job_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    loop {
        // Operator control (P1): honor pause/cancel between batches. The
        // batch-progress UPDATE also refuses to resurrect paused/cancelled
        // states, closing the race at the SQL level.
        match lease::current_state(pool, lease_row.job_id).await {
            Ok(Some(state)) if state == "paused" => {
                // Keep the lease alive across long pauses.
                if let Err(e) =
                    lease::heartbeat(pool, lease_row.job_id, &config.owner, config.lease_ttl_secs)
                        .await
                {
                    tracing::warn!(step = job.step_id(), error = %e, "paused heartbeat failed");
                }
                tokio::time::sleep(config.throttle_sleep).await;
                continue;
            }
            Ok(Some(state)) if state == "cancelled" => {
                // control_job already set the terminal state + released the lease.
                tracing::info!(step = job.step_id(), "migration job cancelled by operator");
                return Ok(());
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(step = job.step_id(), error = %e, "migration state poll failed");
            }
        }

        // Preflight gate (rollout watch): pause under serving load.
        let throttled = active_backends(pool).await.map(|n| n > MAX_ACTIVE_BACKENDS);
        if throttled == Some(true) {
            tracing::warn!(
                step = job.step_id(),
                "rollout watch: >{MAX_ACTIVE_BACKENDS} active backends — pausing migration batches"
            );
            tokio::time::sleep(config.throttle_sleep).await;
            continue;
        }

        let started = Instant::now();
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| StorageError::Database(format!("migration batch begin failed: {e}")))?;

        let outcome = job
            .run_batch(&mut tx, &cursor, i64::from(sizer.size()))
            .await?;

        let duration_ms = started.elapsed().as_millis() as i64;

        if let Some(next) = &outcome.next_cursor {
            batch_seq += 1;
            lease::insert_batch_ledger(
                &mut *tx,
                lease_row.job_id,
                batch_seq,
                &cursor,
                next,
                outcome.scanned as i32,
                duration_ms as i32,
            )
            .await?;
            lease::record_batch_progress(
                &mut *tx,
                lease_row.job_id,
                &config.owner,
                config.lease_ttl_secs,
                outcome.scanned,
                outcome.failed,
                next,
                sizer.size() as i32,
                None,
            )
            .await?;
        }
        tx.commit()
            .await
            .map_err(|e| StorageError::Database(format!("migration batch commit failed: {e}")))?;

        let adjustment = sizer.record(duration_ms as u64, false);
        tracing::debug!(
            step = job.step_id(),
            scanned = outcome.scanned,
            written = outcome.written,
            failed = outcome.failed,
            duration_ms,
            batch_size = sizer.size(),
            ?adjustment,
            "migration batch committed"
        );

        // P3: structured rate + ETA log every 25 batches (ledger-derived —
        // same numbers as the detail endpoint and `migrate status` CLI).
        if batch_seq % 25 == 0 {
            if let Ok(Some(detail)) = lease::job_detail(pool, lease_row.job_id).await {
                tracing::info!(
                    step = job.step_id(),
                    processed = detail.processed_count,
                    total = ?detail.estimated_total,
                    pct = ?detail.completion_pct.map(|p| (p * 100.0).round() / 100.0),
                    rows_per_sec = ?detail.rows_per_sec.map(|r| (r * 10.0).round() / 10.0),
                    eta_seconds = ?detail.eta_seconds.map(|e| e.round() as i64),
                    "migration progress"
                );
            }
        }

        match outcome.next_cursor {
            Some(next) => cursor = next,
            None => break,
        }
    }

    // Verification phase (07 §Gates: coverage/checksum before completed).
    tracing::info!(step = job.step_id(), "backfill exhausted — verifying");
    match job.verify(pool).await {
        Ok(report) if report.passes() => {
            lease::finish_job(pool, lease_row.job_id, &config.owner, "completed", None).await?;
            tracing::info!(
                step = job.step_id(),
                metric = %report.metric,
                expected = report.expected,
                actual = report.actual,
                sampled = report.sampled,
                "migration job completed and verified"
            );
        }
        Ok(report) => {
            let err = json!({ "verify_failed": &report });
            lease::finish_job(pool, lease_row.job_id, &config.owner, "failed", Some(err)).await?;
            tracing::error!(
                step = job.step_id(),
                ?report,
                "migration verification FAILED"
            );
        }
        Err(e) => {
            let err = json!({ "verify_error": e.to_string() });
            lease::finish_job(pool, lease_row.job_id, &config.owner, "failed", Some(err)).await?;
            return Err(e);
        }
    }
    Ok(())
}

/// `pg_stat_activity` active backend count (rollout watch gate).
async fn active_backends(pool: &PgPool) -> Option<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM pg_stat_activity WHERE state = 'active' AND pid <> pg_backend_pid()",
    )
    .fetch_one(pool)
    .await
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_spec091_engine_config_defaults_match_plan() {
        let cfg = MigrationEngineConfig::from_env();
        assert!(cfg.batch_min >= 1);
        assert!(cfg.batch_max >= cfg.batch_min);
        assert!(cfg.batch_target_ms >= 10);
        assert!(cfg.lease_ttl_secs >= 5);
    }

    #[test]
    fn contract_spec091_verify_report_gate() {
        let pass = VerifyReport {
            metric: "coverage".into(),
            expected: 10,
            actual: 10,
            sampled: 10,
            mismatches: 0,
        };
        assert!(pass.passes());
        let under = VerifyReport {
            actual: 9,
            ..pass.clone()
        };
        assert!(!under.passes());
        let mismatches_ok_by_default = VerifyReport {
            mismatches: 1,
            ..pass.clone()
        };
        assert!(
            mismatches_ok_by_default.passes(),
            "LAW-139: equality is opt-in; coverage-only is default"
        );
        let _g = crate::test_env_lock::test_env_lock();
        std::env::set_var("EDGEQUAKE_MIGRATION_VERIFY_EQUALITY", "1");
        assert!(!mismatches_ok_by_default.passes());
        std::env::remove_var("EDGEQUAKE_MIGRATION_VERIFY_EQUALITY");
    }
}
