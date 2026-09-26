//! SPEC-150 migrate runner — advisory lock, session timeouts, dirty messaging.
//!
//! Wraps the existing sqlx apply path with:
//! - `pg_try_advisory_lock` loop + `EDGEQUAKE_MIGRATE_LOCK_DEADLINE` (exit 75)
//! - session `lock_timeout` / `statement_timeout` from lock_class defaults
//! - actionable Dirty-version messages pointing at the ops runbook
//! - optional `migration_run` / `_step` telemetry (after migration 159)

use std::time::{Duration, Instant};

use sqlx::PgPool;
use tracing::{info, warn};

/// EX_TEMPFAIL — another migrate holds the run lock past the deadline.
pub const MIGRATE_LOCK_EXIT_CODE: i32 = 75;

/// EX_DATAERR — unknown checksum / data error (fossil refuse).
pub const MIGRATE_DATA_EXIT_CODE: i32 = 65;

const RUN_LOCK_KEY_SQL: &str = "hashtext('edgequake.migrate.run')";

/// Env: max seconds to wait for the migrate advisory lock (default 60).
pub const MIGRATE_LOCK_DEADLINE_ENV: &str = "EDGEQUAKE_MIGRATE_LOCK_DEADLINE";
/// Env: session lock_timeout (default `5s`).
pub const MIGRATE_LOCK_TIMEOUT_ENV: &str = "EDGEQUAKE_MIGRATE_LOCK_TIMEOUT";
/// Env: session statement_timeout override (default per lock_class).
pub const MIGRATE_STATEMENT_TIMEOUT_ENV: &str = "EDGEQUAKE_MIGRATE_STATEMENT_TIMEOUT";

#[derive(Debug, Clone)]
pub struct RunnerConfig {
    pub lock_deadline: Duration,
    pub lock_timeout: String,
    pub statement_timeout: Option<String>,
}

impl RunnerConfig {
    pub fn from_env() -> Self {
        let lock_deadline = std::env::var(MIGRATE_LOCK_DEADLINE_ENV)
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(60));
        let lock_timeout = std::env::var(MIGRATE_LOCK_TIMEOUT_ENV).unwrap_or_else(|_| "5s".into());
        let statement_timeout = std::env::var(MIGRATE_STATEMENT_TIMEOUT_ENV)
            .ok()
            .filter(|s| !s.trim().is_empty());
        Self {
            lock_deadline,
            lock_timeout,
            statement_timeout,
        }
    }
}

/// Default statement_timeout for a manifest `lock_class`.
pub fn statement_timeout_for_class(lock_class: &str) -> &'static str {
    match lock_class {
        "ddl_share" => "10min",
        "ddl_cic" => "0",
        "dml_batch" => "30s",
        "contract" | "graph_rewrite" => "10min",
        _ => "30s", // ddl_short / ddl_access_exclusive
    }
}

/// Acquire the migrate run advisory lock, or return a Protocol error after deadline.
///
/// The lock is session-scoped: the guard holds the acquiring connection until
/// [`MigrateLockGuard::release`] so unlock cannot land on a different pool checkout.
pub async fn acquire_migrate_run_lock(
    pool: &PgPool,
    cfg: &RunnerConfig,
) -> Result<MigrateLockGuard, sqlx::Error> {
    let deadline = Instant::now() + cfg.lock_deadline;
    let mut attempt = 0u32;
    let mut conn = pool.acquire().await?;
    loop {
        let got: bool =
            sqlx::query_scalar(&format!("SELECT pg_try_advisory_lock({RUN_LOCK_KEY_SQL})"))
                .fetch_one(&mut *conn)
                .await?;
        if got {
            info!(
                target: "edgequake.migration",
                attempt,
                "Acquired edgequake.migrate.run advisory lock"
            );
            return Ok(MigrateLockGuard { conn: Some(conn) });
        }
        if Instant::now() >= deadline {
            // Drop the idle checkout before returning so we do not pin a pool slot.
            drop(conn);
            return Err(sqlx::Error::Protocol(format!(
                "MIGRATE_LOCK_BUSY: another edgequake migrate holds the run lock \
                 past {deadline_secs}s (EDGEQUAKE_MIGRATE_LOCK_DEADLINE). \
                 Exit {MIGRATE_LOCK_EXIT_CODE} (EX_TEMPFAIL). Retry later.",
                deadline_secs = cfg.lock_deadline.as_secs()
            )));
        }
        attempt += 1;
        let sleep_ms = (50u64 * 2u64.saturating_pow(attempt.min(6))).min(2000);
        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
    }
}

/// RAII unlock for the migrate run advisory lock (same session as acquire).
pub struct MigrateLockGuard {
    conn: Option<sqlx::pool::PoolConnection<sqlx::Postgres>>,
}

impl MigrateLockGuard {
    pub async fn release(mut self) {
        if let Some(mut conn) = self.conn.take() {
            let _ = sqlx::query(&format!("SELECT pg_advisory_unlock({RUN_LOCK_KEY_SQL})"))
                .execute(&mut *conn)
                .await;
        }
    }
}

impl Drop for MigrateLockGuard {
    fn drop(&mut self) {
        // Best-effort: dropping the connection releases session advisory locks.
        self.conn.take();
    }
}

/// Apply session timeouts on a dedicated connection from the pool.
pub async fn apply_session_timeouts(
    pool: &PgPool,
    cfg: &RunnerConfig,
    lock_class: &str,
) -> Result<(), sqlx::Error> {
    let stmt = cfg
        .statement_timeout
        .clone()
        .unwrap_or_else(|| statement_timeout_for_class(lock_class).to_string());
    // SET does not accept parameters for timeout values in all PG versions via
    // bind — use validated literals from our allowlist / env.
    validate_timeout_literal(&cfg.lock_timeout)?;
    validate_timeout_literal(&stmt)?;
    sqlx::query("SET application_name = 'edgequake-migrate'")
        .execute(pool)
        .await?;
    sqlx::query(&format!("SET lock_timeout = '{}'", cfg.lock_timeout))
        .execute(pool)
        .await?;
    sqlx::query(&format!("SET statement_timeout = '{stmt}'"))
        .execute(pool)
        .await?;
    Ok(())
}

fn validate_timeout_literal(s: &str) -> Result<(), sqlx::Error> {
    let ok = s == "0"
        || s.chars().all(|c| c.is_ascii_alphanumeric() || c == ' ')
            && !s.is_empty()
            && s.len() < 32;
    if ok {
        Ok(())
    } else {
        Err(sqlx::Error::Protocol(format!(
            "invalid timeout literal {s:?} (expected e.g. 5s, 30s, 10min, 0)"
        )))
    }
}

/// Enrich a sqlx migrate error with Dirty-version recovery hints.
pub fn enrich_migrate_error(err: sqlx::Error) -> sqlx::Error {
    let msg = err.to_string();
    if msg.contains("Dirty") || msg.contains("dirty") {
        sqlx::Error::Protocol(format!(
            "{msg}\n\
             Recovery: inspect public._sqlx_migrations WHERE success = false; \
             fix the failed version SQLSTATE, DELETE the dirty row only after \
             confirming the DDL did not partially apply, then re-run \
             `edgequake migrate`. See specs/150-reliable-migration-system/11-ops-runbook.md."
        ))
    } else if msg.contains("55P03") || msg.contains("lock_not_available") {
        sqlx::Error::Protocol(format!(
            "{msg}\n\
             lock_timeout (55P03): retry with backoff, or raise \
             EDGEQUAKE_MIGRATE_LOCK_TIMEOUT / wait for blockers \
             (pg_stat_activity / pg_locks). Spec: 10-performance-budget.md."
        ))
    } else {
        err
    }
}

/// Insert a migration_run row when table 159 exists; returns None otherwise.
pub async fn begin_migration_run(
    pool: &PgPool,
    binary_version: &str,
) -> Result<Option<uuid::Uuid>, sqlx::Error> {
    let exists: bool =
        sqlx::query_scalar("SELECT to_regclass('edgequake.migration_run') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !exists {
        return Ok(None);
    }
    let id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO edgequake.migration_run (binary_version, outcome) \
         VALUES ($1, 'running') RETURNING id",
    )
    .bind(binary_version)
    .fetch_one(pool)
    .await?;
    Ok(Some(id))
}

pub async fn finish_migration_run(
    pool: &PgPool,
    run_id: Option<uuid::Uuid>,
    outcome: &str,
    applied_count: i32,
) -> Result<(), sqlx::Error> {
    let Some(id) = run_id else {
        return Ok(());
    };
    sqlx::query(
        "UPDATE edgequake.migration_run \
         SET finished_at = now(), outcome = $2, applied_count = $3 \
         WHERE id = $1",
    )
    .bind(id)
    .bind(outcome)
    .bind(applied_count)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn record_migration_step(
    pool: &PgPool,
    run_id: Option<uuid::Uuid>,
    version: i64,
    phase: &str,
    duration_ms: i64,
    sqlstate: Option<&str>,
    outcome: &str,
) -> Result<(), sqlx::Error> {
    let Some(id) = run_id else {
        return Ok(());
    };
    let exists: bool =
        sqlx::query_scalar("SELECT to_regclass('edgequake.migration_run_step') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !exists {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO edgequake.migration_run_step \
         (run_id, version, phase, duration_ms, sqlstate, outcome) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (run_id, version) DO UPDATE SET \
           duration_ms = EXCLUDED.duration_ms, \
           sqlstate = EXCLUDED.sqlstate, \
           outcome = EXCLUDED.outcome",
    )
    .bind(id)
    .bind(version)
    .bind(phase)
    .bind(duration_ms)
    .bind(sqlstate)
    .bind(outcome)
    .execute(pool)
    .await?;
    Ok(())
}

/// Bump `schema_compat.min_binary_schema` to at least `floor`.
pub async fn bump_schema_compat(pool: &PgPool, floor: i64) -> Result<(), sqlx::Error> {
    let exists: bool =
        sqlx::query_scalar("SELECT to_regclass('edgequake.schema_compat') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !exists {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO edgequake.schema_compat (id, min_binary_schema, updated_at) \
         VALUES (1, $1, now()) \
         ON CONFLICT (id) DO UPDATE SET \
           min_binary_schema = GREATEST(edgequake.schema_compat.min_binary_schema, EXCLUDED.min_binary_schema), \
           updated_at = now()",
    )
    .bind(floor)
    .execute(pool)
    .await?;
    Ok(())
}

/// Read `schema_compat.min_binary_schema` if present.
pub async fn read_min_binary_schema(pool: &PgPool) -> Result<Option<i64>, sqlx::Error> {
    let exists: bool =
        sqlx::query_scalar("SELECT to_regclass('edgequake.schema_compat') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !exists {
        return Ok(None);
    }
    let v: Option<i64> =
        sqlx::query_scalar("SELECT min_binary_schema FROM edgequake.schema_compat WHERE id = 1")
            .fetch_optional(pool)
            .await?;
    Ok(v)
}

/// Retry helper for 55P03 lock_not_available (max 8 attempts).
pub async fn with_lock_retry<F, Fut, T>(label: &str, mut f: F) -> Result<T, sqlx::Error>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
{
    let mut attempt = 0u32;
    loop {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                let msg = e.to_string();
                let is_lock = msg.contains("55P03") || msg.contains("lock_not_available");
                if !is_lock || attempt >= 8 {
                    return Err(enrich_migrate_error(e));
                }
                attempt += 1;
                let sleep_ms = (100u64 * 2u64.saturating_pow(attempt.min(5))).min(5000);
                warn!(
                    target: "edgequake.migration",
                    label,
                    attempt,
                    sleep_ms,
                    "55P03 lock_not_available — retrying"
                );
                tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_literals_validated() {
        assert!(validate_timeout_literal("5s").is_ok());
        assert!(validate_timeout_literal("10min").is_ok());
        assert!(validate_timeout_literal("0").is_ok());
        assert!(validate_timeout_literal("30s; DROP").is_err());
        assert!(validate_timeout_literal("").is_err());
    }

    #[test]
    fn class_defaults_match_budget() {
        assert_eq!(statement_timeout_for_class("ddl_share"), "10min");
        assert_eq!(statement_timeout_for_class("dml_batch"), "30s");
        assert_eq!(statement_timeout_for_class("ddl_cic"), "0");
    }
}
