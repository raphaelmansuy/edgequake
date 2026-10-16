//! Transaction-scoped `statement_timeout` (SPEC-089 / LAW-H2).
//!
//! WHY: `tokio::time::timeout` abandons the Rust future while Postgres keeps
//! executing — zombie pool holders (GH-336). `SET LOCAL` inside a transaction
//! cancels the statement and cannot leak into the pool after COMMIT/ROLLBACK
//! ([PostgreSQL SET](https://www.postgresql.org/docs/16/sql-set.html)).

use sqlx::{Acquire, PgConnection, Postgres, Transaction};

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

/// Vector ANN search PG kill (SPEC-090 F-090-27).
///
/// Default 2000ms minus headroom so Postgres cancels before the app abandons
/// the future (LAW-H2). Override with `EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS`.
pub(in crate::adapters::postgres) fn vector_query_statement_timeout_ms() -> u32 {
    let ms = std::env::var("EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(2_000)
        .clamp(500, 30_000);
    ms.saturating_sub(GRAPH_QUERY_PG_HEADROOM_MS).max(1)
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
    fn vector_query_timeout_ms_default_is_under_budget() {
        let prev = std::env::var("EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS").ok();
        std::env::remove_var("EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS");
        assert_eq!(vector_query_statement_timeout_ms(), 1_750);
        assert!(vector_query_statement_timeout_ms() < 2_000);
        if let Some(v) = prev {
            std::env::set_var("EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS", v);
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
}
