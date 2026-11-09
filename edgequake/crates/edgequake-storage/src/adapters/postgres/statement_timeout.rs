//! Transaction-scoped `statement_timeout` (SPEC-089 / LAW-H2).
//!
//! WHY: `tokio::time::timeout` abandons the Rust future while Postgres keeps
//! executing — zombie pool holders (GH-336). `SET LOCAL` inside a transaction
//! cancels the statement and cannot leak into the pool after COMMIT/ROLLBACK
//! ([PostgreSQL SET](https://www.postgresql.org/docs/16/sql-set.html)).

use sqlx::{Acquire, PgConnection, Postgres, Row, Transaction};

use crate::error::{Result, StorageError};

/// Open transaction with `SET LOCAL statement_timeout` already applied.
///
/// Public so interactive HTTP read paths (documents list/detail) can cancel
/// SQL before a Rust deadline abandons the future (LAW-H2 / GH-400).
pub struct LocalTimeoutTx<'c> {
    inner: Transaction<'c, Postgres>,
}

impl<'c> LocalTimeoutTx<'c> {
    pub async fn begin(conn: &'c mut PgConnection, timeout_ms: u32) -> Result<Self> {
        let mut inner = conn
            .begin()
            .await
            .map_err(|e| StorageError::Database(format!("statement_timeout begin failed: {e}")))?;
        sqlx::query(&format!("SET LOCAL statement_timeout = '{timeout_ms}ms'"))
            .execute(&mut *inner)
            .await
            .map_err(|e| {
                StorageError::Database(format!("SET LOCAL statement_timeout failed: {e}"))
            })?;
        Ok(Self { inner })
    }

    pub async fn commit(self) -> Result<()> {
        self.inner
            .commit()
            .await
            .map_err(|e| StorageError::Database(format!("statement_timeout commit failed: {e}")))
    }

    pub async fn rollback(self) -> Result<()> {
        self.inner
            .rollback()
            .await
            .map_err(|e| StorageError::Database(format!("statement_timeout rollback failed: {e}")))
    }
}

/// Borrow the PG connection for sqlx queries (`Executor`).
///
/// WHY: `&mut Transaction` is **not** `Executor` in sqlx 0.8 — callers need
/// the deref'd `PgConnection` (GH-400 interactive reads).
impl<'c> AsMut<PgConnection> for LocalTimeoutTx<'c> {
    fn as_mut(&mut self) -> &mut PgConnection {
        &mut self.inner
    }
}

/// Headroom so Postgres cancels before `tokio::time::timeout` abandons the
/// future (LAW-H2 / GH-336). Covers commit/rollback + error mapping.
const GRAPH_QUERY_PG_HEADROOM_MS: u32 = 250;

/// Resolve graph interactive query timeout in milliseconds (native SQL paths).
///
/// Mirrors app budget (`EDGEQUAKE_GRAPH_QUERY_TIMEOUT_SECS`, 15s) minus
/// [`GRAPH_QUERY_PG_HEADROOM_MS`] so the server wins the cancel race.
pub(in crate::adapters::postgres) fn graph_query_statement_timeout_ms() -> u32 {
    let secs = std::env::var("EDGEQUAKE_GRAPH_QUERY_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(15)
        .max(1);
    secs.saturating_mul(1000)
        .saturating_sub(GRAPH_QUERY_PG_HEADROOM_MS)
        .max(1)
}

/// Interactive HTTP read-path PG kill (SPEC-089 Phase 4 / F-336-13).
///
/// Mirrors `EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS` (default 2500) minus headroom.
/// Worker wall-clock (<=7200s) is LLM-bound — per-statement kills stay on
/// `LocalTimeoutTx` / AGE session GUCs, not this interactive budget.
pub fn interactive_statement_timeout_ms() -> u32 {
    let ms = std::env::var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(2_500)
        .clamp(500, 30_000);
    ms.saturating_sub(GRAPH_QUERY_PG_HEADROOM_MS).max(1)
}

/// Interactive vector statement budget before headroom (`EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS`).
///
/// Same ceiling as graph reads. Filtered HNSW on the query path sets
/// `hnsw.ef_search` up to 1000 and `hnsw.max_scan_tuples` to 20000. A 2s kill
/// cancelled that scan while hybrid `query_execute` was still under 5s
/// (keyword + embed, then SQL). This GUC is the kill: there is no separate
/// tokio deadline on the vector path (LAW-H2 / SPEC-090 F-090-27).
pub const VECTOR_QUERY_TIMEOUT_MS_DEFAULT: u32 = 15_000;

/// Floor and ceiling for `EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS` (milliseconds).
const VECTOR_QUERY_TIMEOUT_MS_MIN: u32 = 500;
const VECTOR_QUERY_TIMEOUT_MS_MAX: u32 = 120_000;

/// Vector ANN / BM25 / FTS statement kill (SPEC-090 F-090-27).
///
/// [`VECTOR_QUERY_TIMEOUT_MS_DEFAULT`] minus [`GRAPH_QUERY_PG_HEADROOM_MS`],
/// so Postgres cancels before any caller waiting on the same budget.
/// Override with `EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS`.
pub fn vector_query_statement_timeout_ms() -> u32 {
    let ms = std::env::var("EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(VECTOR_QUERY_TIMEOUT_MS_DEFAULT)
        .clamp(VECTOR_QUERY_TIMEOUT_MS_MIN, VECTOR_QUERY_TIMEOUT_MS_MAX);
    ms.saturating_sub(GRAPH_QUERY_PG_HEADROOM_MS).max(1)
}

/// Keep SQLSTATE 57014 as [`StorageError::DeadlineExceeded`] and name the budget.
///
/// Typed ANN used to surface the bare Postgres message, so a 4.6s hybrid span
/// did not say which kill fired.
pub(in crate::adapters::postgres) fn map_vector_query_error(
    error: sqlx::Error,
    timeout_ms: u32,
) -> StorageError {
    match StorageError::from(error) {
        StorageError::DeadlineExceeded(message) => StorageError::DeadlineExceeded(format!(
            "{message} (vector statement_timeout {timeout_ms}ms)"
        )),
        other => other,
    }
}

/// Env: cap for plain-graph cutover statements (GitHub #494).
///
/// Unset or empty means `0` (disabled). This is not
/// `EDGEQUAKE_MIGRATE_STATEMENT_TIMEOUT`: a short migrate override must not
/// become the cutover cap.
pub const CUTOVER_STATEMENT_TIMEOUT_ENV: &str = "EDGEQUAKE_CUTOVER_STATEMENT_TIMEOUT";

/// Literal passed to `SET LOCAL statement_timeout` / `transaction_timeout`.
///
/// `0` disables the timeout. Any other value must be a short unit literal
/// such as `50ms`, `5s`, or `30min`.
pub(in crate::adapters::postgres) fn cutover_statement_timeout_literal() -> Result<String> {
    let raw = std::env::var(CUTOVER_STATEMENT_TIMEOUT_ENV).unwrap_or_default();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok("0".to_string());
    }
    if !valid_cutover_timeout_literal(trimmed) {
        return Err(StorageError::InvalidConfig(format!(
            "{CUTOVER_STATEMENT_TIMEOUT_ENV}={trimmed:?} is not 0 or a unit literal (50ms, 5s, 30min)"
        )));
    }
    Ok(trimmed.to_string())
}

fn valid_cutover_timeout_literal(value: &str) -> bool {
    if value == "0" {
        return true;
    }
    if value.len() >= 32 {
        return false;
    }
    let unit_at = value
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(value.len());
    if unit_at == 0 || unit_at == value.len() {
        return false;
    }
    matches!(&value[unit_at..], "ms" | "s" | "min" | "h" | "d")
}

fn render_timeout_literal(literal: &str) -> String {
    if literal == "0" {
        "0".to_string()
    } else {
        format!("'{literal}'")
    }
}

/// `SET LOCAL` the cutover statement budget on an open transaction.
///
/// Role and database defaults return at COMMIT or ROLLBACK
/// ([SET](https://www.postgresql.org/docs/current/sql-set.html)). Call this
/// only after `BEGIN`. A session `SET` would survive on a pooled connection
/// if `RESET ALL` did not run.
///
/// PostgreSQL 17 `transaction_timeout` still cancels the whole transaction
/// when `statement_timeout` is 0, so the same literal is applied when that
/// setting exists. PostgreSQL 16 has no such GUC.
pub(in crate::adapters::postgres) async fn apply_local_cutover_timeout(
    conn: &mut PgConnection,
) -> Result<()> {
    let literal = cutover_statement_timeout_literal()?;
    let rendered = render_timeout_literal(&literal);
    sqlx::query(&format!("SET LOCAL statement_timeout = {rendered}"))
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("SET LOCAL statement_timeout failed: {e}")))?;
    let transaction_timeout: Option<String> =
        sqlx::query_scalar("SELECT current_setting('transaction_timeout', true)")
            .fetch_one(&mut *conn)
            .await
            .unwrap_or(None);
    if transaction_timeout.is_some() {
        sqlx::query(&format!("SET LOCAL transaction_timeout = {rendered}"))
            .execute(&mut *conn)
            .await
            .map_err(|e| {
                StorageError::Database(format!("SET LOCAL transaction_timeout failed: {e}"))
            })?;
    }
    Ok(())
}

/// Community snapshot scan statement budget (GH-404).
///
/// Default 30s per page so a bad plan is cancelled in Postgres instead of
/// becoming a multi-hour zombie pool holder. Override with
/// `EDGEQUAKE_COMMUNITY_STATEMENT_TIMEOUT_MS`.
pub(in crate::adapters::postgres) fn community_statement_timeout_ms() -> u32 {
    let ms = std::env::var("EDGEQUAKE_COMMUNITY_STATEMENT_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(30_000)
        .clamp(1_000, 300_000);
    ms.saturating_sub(GRAPH_QUERY_PG_HEADROOM_MS).max(1)
}

/// How an operator literal is accepted (SPEC-171 LAW-171-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutLiteralPolicy {
    /// Empty uses `fallback`. `0` stays `0` (disabled).
    AllowZero,
    /// Empty uses `fallback`. `0` is replaced by `fallback`.
    RejectZero,
    /// Empty is an error. `0` stays `0`.
    RequiredAllowZero,
}

/// One checker for migrate, vector DDL, and ingest timeouts.
///
/// Accepts `0` or a short unit literal (`30s`, `5min`). Rejects anything that
/// could be interpolated into `SET`.
pub fn resolve_timeout_literal(
    raw: &str,
    fallback: &str,
    policy: TimeoutLiteralPolicy,
) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return match policy {
            TimeoutLiteralPolicy::RequiredAllowZero => Err(StorageError::InvalidConfig(
                "timeout literal is empty".into(),
            )),
            TimeoutLiteralPolicy::AllowZero | TimeoutLiteralPolicy::RejectZero => {
                Ok(fallback.to_string())
            }
        };
    }
    if trimmed == "0" {
        return match policy {
            TimeoutLiteralPolicy::RejectZero => Ok(fallback.to_string()),
            TimeoutLiteralPolicy::AllowZero | TimeoutLiteralPolicy::RequiredAllowZero => {
                Ok("0".to_string())
            }
        };
    }
    if !valid_cutover_timeout_literal(trimmed) {
        return Err(StorageError::InvalidConfig(format!(
            "timeout literal {trimmed:?} is not 0 or a unit literal (50ms, 5s, 30min)"
        )));
    }
    Ok(trimmed.to_string())
}

/// `lock_timeout` must be strictly shorter than `statement_timeout` or it never fires.
pub fn ingest_lock_statement_pair(lock_secs: u64, statement_secs: u64) -> Result<()> {
    if statement_secs == 0 || lock_secs >= statement_secs {
        return Err(StorageError::InvalidConfig(format!(
            "lock_timeout {lock_secs}s must be shorter than statement_timeout {statement_secs}s"
        )));
    }
    Ok(())
}

/// `transaction_timeout` exists on PostgreSQL 17+. PostgreSQL 16 must not see the GUC.
pub fn transaction_timeout_literal(server_version_num: i32, budget_secs: u64) -> Option<String> {
    if server_version_num < 170_000 || budget_secs == 0 {
        None
    } else {
        Some(format!("{budget_secs}s"))
    }
}

/// Ingest session lock wait. Default 30s, clamp 1..=120. `0` does not disable it.
pub fn ingest_lock_timeout_secs() -> u64 {
    let raw = std::env::var("EDGEQUAKE_DB_INGEST_LOCK_TIMEOUT_SECS").unwrap_or_default();
    let literal = resolve_timeout_literal(raw.trim(), "30s", TimeoutLiteralPolicy::RejectZero)
        .unwrap_or_else(|_| "30s".to_string());
    let digits: String = literal.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<u64>().unwrap_or(30).clamp(1, 120)
}

/// Two-key transaction advisory lock. Dies at commit or rollback.
pub const WORKSPACE_MERGE_LOCK_SQL: &str =
    "SELECT pg_advisory_xact_lock(hashtext('edgequake.merge'), hashtext($1))";

/// Activity snapshot. `left(query, 120)` is the server-side cap. Rust redacts secrets.
pub const ACTIVITY_SNAPSHOT_SQL: &str = "\
SELECT a.pid, a.application_name, a.state::text AS state, a.wait_event_type, a.wait_event, \
       (now() - a.xact_start)::text AS xact_age, pg_blocking_pids(a.pid)::text AS blocked_by, \
       left(a.query, 120) AS query \
FROM pg_stat_activity a \
WHERE a.application_name LIKE 'edgequake:%' \
  AND (a.wait_event_type = 'Lock' OR now() - a.xact_start > interval '5 minutes')";

/// Drop `sk-` tokens and cap the text. A snapshot failure must not decide the task.
pub fn redact_activity_text(query: &str) -> String {
    let mut out = String::new();
    let mut rest = query;
    while let Some(index) = rest.find("sk-") {
        out.push_str(&rest[..index]);
        out.push_str("[redacted]");
        rest = &rest[index + 3..];
        let skip = rest.find(char::is_whitespace).unwrap_or(rest.len());
        rest = &rest[skip..];
    }
    out.push_str(rest);
    if out.len() > 120 {
        out.truncate(120);
    }
    out
}

pub fn snapshot_is_best_effort<T, E>(result: std::result::Result<T, E>) -> Option<T> {
    result.ok()
}

/// Bound for the diagnostic snapshot. Shorter than any ingest lock wait.
pub const ACTIVITY_SNAPSHOT_TIMEOUT: &str = "5s";

/// Best-effort lock graph. Failure is returned; callers must not fail the task.
pub async fn log_activity_snapshot(pool: &sqlx::PgPool) -> Result<()> {
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| StorageError::Connection(format!("activity snapshot acquire failed: {e}")))?;
    let mut tx = conn
        .begin()
        .await
        .map_err(|e| StorageError::Database(format!("activity snapshot begin failed: {e}")))?;
    sqlx::query(&format!(
        "SET LOCAL statement_timeout = '{ACTIVITY_SNAPSHOT_TIMEOUT}'"
    ))
    .execute(&mut *tx)
    .await
    .map_err(|e| StorageError::Database(format!("activity snapshot timeout failed: {e}")))?;
    let rows = sqlx::query(ACTIVITY_SNAPSHOT_SQL)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("activity snapshot query failed: {e}")))?;
    for row in &rows {
        let query: String = row.try_get("query").unwrap_or_default();
        tracing::warn!(
            pid = row.try_get::<i32, _>("pid").unwrap_or(0),
            application_name = row.try_get::<String, _>("application_name").unwrap_or_default(),
            state = row.try_get::<String, _>("state").unwrap_or_default(),
            wait_event_type = row
                .try_get::<Option<String>, _>("wait_event_type")
                .ok()
                .flatten()
                .unwrap_or_default(),
            wait_event = row
                .try_get::<Option<String>, _>("wait_event")
                .ok()
                .flatten()
                .unwrap_or_default(),
            xact_age = row.try_get::<String, _>("xact_age").unwrap_or_default(),
            blocked_by = row.try_get::<String, _>("blocked_by").unwrap_or_default(),
            query = %redact_activity_text(&query),
            "SPEC-171 ingest activity snapshot"
        );
    }
    let _ = tx.rollback().await;
    Ok(())
}

/// Append a stable SQLSTATE token so the task classifier can see it without sqlx.
pub fn sqlstate_hint(error: &sqlx::Error) -> String {
    let Some(code) = error.as_database_error().and_then(|db| db.code()) else {
        return error.to_string();
    };
    match code.as_ref() {
        "55P03" => format!("sqlstate 55P03 lock_not_available: {error}"),
        "40P01" => format!("sqlstate 40P01 deadlock_detected: {error}"),
        "57014" => format!("sqlstate 57014 query_canceled: {error}"),
        _ => error.to_string(),
    }
}

/// `SET LOCAL` lock and statement timeouts, plus `transaction_timeout` on PostgreSQL 17+.
///
/// `statement_secs` is the persist budget minus one second so it stays under
/// `transaction_timeout` when that GUC is set.
pub async fn apply_ingest_write_guards(
    conn: &mut sqlx::PgConnection,
    workspace_id: Option<&str>,
    statement_secs: u64,
) -> Result<()> {
    let lock_secs = ingest_lock_timeout_secs();
    let statement_secs = statement_secs.max(lock_secs.saturating_add(1));
    ingest_lock_statement_pair(lock_secs, statement_secs)?;
    let lock_literal = format!("{lock_secs}s");
    let statement_literal = format!("{statement_secs}s");
    sqlx::query(&format!("SET LOCAL lock_timeout = '{lock_literal}'"))
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("SET LOCAL lock_timeout failed: {e}")))?;
    sqlx::query(&format!(
        "SET LOCAL statement_timeout = '{statement_literal}'"
    ))
    .execute(&mut *conn)
    .await
    .map_err(|e| StorageError::Database(format!("SET LOCAL statement_timeout failed: {e}")))?;
    let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
        .fetch_one(&mut *conn)
        .await
        .unwrap_or(0);
    if let Some(txn) = transaction_timeout_literal(version, statement_secs.saturating_add(1)) {
        sqlx::query(&format!("SET LOCAL transaction_timeout = '{txn}'"))
            .execute(&mut *conn)
            .await
            .map_err(|e| {
                StorageError::Database(format!("SET LOCAL transaction_timeout failed: {e}"))
            })?;
    }
    if let Some(workspace_id) = workspace_id.filter(|id| !id.is_empty()) {
        sqlx::query(WORKSPACE_MERGE_LOCK_SQL)
            .bind(workspace_id)
            .execute(&mut *conn)
            .await
            .map_err(|e| {
                StorageError::Database(format!(
                    "workspace merge lock failed: {}",
                    sqlstate_hint(&e)
                ))
            })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_query_timeout_ms_default_is_under_app_budget() {
        let prev = std::env::var("EDGEQUAKE_GRAPH_QUERY_TIMEOUT_SECS").ok();
        std::env::remove_var("EDGEQUAKE_GRAPH_QUERY_TIMEOUT_SECS");
        assert_eq!(graph_query_statement_timeout_ms(), 14_750);
        assert!(graph_query_statement_timeout_ms() < 15_000);
        if let Some(v) = prev {
            std::env::set_var("EDGEQUAKE_GRAPH_QUERY_TIMEOUT_SECS", v);
        }
    }

    #[test]
    fn interactive_timeout_ms_default_is_under_read_path_budget() {
        let prev = std::env::var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS").ok();
        std::env::remove_var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS");
        assert_eq!(interactive_statement_timeout_ms(), 2_250);
        assert!(interactive_statement_timeout_ms() < 2_500);
        if let Some(v) = prev {
            std::env::set_var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS", v);
        }
    }

    #[test]
    fn typed_ann_search_uses_the_vector_statement_budget() {
        let ann = include_str!("typed_ann_search.rs");
        assert!(
            ann.contains("vector_query_statement_timeout_ms()"),
            "typed ANN must install the vector statement budget"
        );
        assert!(include_str!("chunk_embedding_index.rs").contains("map_vector_query_error"));
        assert!(include_str!("fleet_embedding_index.rs").contains("map_vector_query_error"));
    }

    #[test]
    fn vector_query_timeout_ms_default_covers_hybrid_and_stays_under_budget() {
        let prev = std::env::var("EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS").ok();
        std::env::remove_var("EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS");
        let budget = vector_query_statement_timeout_ms();
        // Production hybrid `query_execute` died at 4.6s on a 1.75s kill.
        assert!(
            budget > 5_000,
            "vector kill {budget}ms is inside the failure window"
        );
        assert_eq!(
            budget,
            VECTOR_QUERY_TIMEOUT_MS_DEFAULT - GRAPH_QUERY_PG_HEADROOM_MS
        );
        assert!(budget < VECTOR_QUERY_TIMEOUT_MS_DEFAULT);

        std::env::set_var("EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS", "8000");
        assert_eq!(vector_query_statement_timeout_ms(), 7_750);
        // Below the floor: clamp, then subtract headroom.
        std::env::set_var("EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS", "10");
        assert_eq!(
            vector_query_statement_timeout_ms(),
            VECTOR_QUERY_TIMEOUT_MS_MIN - GRAPH_QUERY_PG_HEADROOM_MS
        );
        // Above the ceiling: same shape as the graph interactive max (120s).
        std::env::set_var("EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS", "999999");
        assert_eq!(
            vector_query_statement_timeout_ms(),
            VECTOR_QUERY_TIMEOUT_MS_MAX - GRAPH_QUERY_PG_HEADROOM_MS
        );

        match prev {
            Some(v) => std::env::set_var("EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS", v),
            None => std::env::remove_var("EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS"),
        }
    }

    #[test]
    fn cutover_timeout_literal_defaults_accepts_units_and_rejects_injection() {
        let prev = std::env::var(CUTOVER_STATEMENT_TIMEOUT_ENV).ok();
        std::env::remove_var(CUTOVER_STATEMENT_TIMEOUT_ENV);
        assert_eq!(cutover_statement_timeout_literal().unwrap(), "0");
        std::env::set_var(CUTOVER_STATEMENT_TIMEOUT_ENV, "  ");
        assert_eq!(cutover_statement_timeout_literal().unwrap(), "0");
        for ok in ["0", "50ms", "5s", "30min", "1h", "1d"] {
            std::env::set_var(CUTOVER_STATEMENT_TIMEOUT_ENV, ok);
            assert_eq!(cutover_statement_timeout_literal().unwrap(), ok);
        }
        for bad in [
            "30 min", "'30min'", "30min;", "1;select", "min", "-1s", "1.5s",
        ] {
            std::env::set_var(CUTOVER_STATEMENT_TIMEOUT_ENV, bad);
            assert!(
                cutover_statement_timeout_literal().is_err(),
                "{bad} must be rejected"
            );
        }
        match prev {
            Some(v) => std::env::set_var(CUTOVER_STATEMENT_TIMEOUT_ENV, v),
            None => std::env::remove_var(CUTOVER_STATEMENT_TIMEOUT_ENV),
        }
    }

    #[test]
    fn community_statement_timeout_default_is_under_30s() {
        let prev = std::env::var("EDGEQUAKE_COMMUNITY_STATEMENT_TIMEOUT_MS").ok();
        std::env::remove_var("EDGEQUAKE_COMMUNITY_STATEMENT_TIMEOUT_MS");
        assert_eq!(community_statement_timeout_ms(), 29_750);
        assert!(community_statement_timeout_ms() < 30_000);
        if let Some(v) = prev {
            std::env::set_var("EDGEQUAKE_COMMUNITY_STATEMENT_TIMEOUT_MS", v);
        }
    }

    #[test]
    fn t171_15_timeout_literal_rejects_sql() {
        let err = resolve_timeout_literal("1s; DROP TABLE", "30s", TimeoutLiteralPolicy::AllowZero);
        assert!(err.is_err());
    }

    #[test]
    fn t171_16_zero_policy() {
        assert_eq!(
            resolve_timeout_literal("0", "30s", TimeoutLiteralPolicy::RejectZero).unwrap(),
            "30s"
        );
        assert_eq!(
            resolve_timeout_literal("0", "30s", TimeoutLiteralPolicy::AllowZero).unwrap(),
            "0"
        );
        assert!(
            resolve_timeout_literal("", "30s", TimeoutLiteralPolicy::RequiredAllowZero).is_err()
        );
    }

    #[test]
    fn t171_17_lock_timeout_must_be_shorter_than_statement() {
        assert!(ingest_lock_statement_pair(30, 30).is_err());
        assert!(ingest_lock_statement_pair(30, 899).is_ok());
    }

    #[test]
    fn t171_18_transaction_timeout_is_pg17_only() {
        assert!(transaction_timeout_literal(160_000, 900).is_none());
        assert_eq!(
            transaction_timeout_literal(170_000, 900).as_deref(),
            Some("900s")
        );
    }

    #[test]
    fn t171_19_merge_lock_sql_is_transaction_scoped() {
        assert!(WORKSPACE_MERGE_LOCK_SQL.contains("pg_advisory_xact_lock"));
        assert!(WORKSPACE_MERGE_LOCK_SQL.contains("hashtext('edgequake.merge')"));
        assert!(WORKSPACE_MERGE_LOCK_SQL.contains("hashtext($1)"));
        assert!(!WORKSPACE_MERGE_LOCK_SQL.contains("pg_advisory_lock("));
    }

    #[test]
    fn t171_20_snapshot_redacts_and_is_best_effort() {
        let text = redact_activity_text("select sk-secret-token from docs");
        assert!(!text.contains("sk-secret"));
        assert!(text.contains("[redacted]"));
        assert!(ACTIVITY_SNAPSHOT_SQL.contains("left(a.query, 120)"));
        assert!(ACTIVITY_SNAPSHOT_SQL.contains("pg_stat_activity"));
        assert_eq!(ACTIVITY_SNAPSHOT_TIMEOUT, "5s");
        assert!(snapshot_is_best_effort::<(), &str>(Err("down")).is_none());
    }
}
