//! AGE session bootstrap and dollar-quote safety (SPEC-017 P1-12).

use crate::error::{Result, StorageError};

use super::super::PostgresAGEGraphStorage;

impl PostgresAGEGraphStorage {
    /// QW1 (F2): build the AGE per-connection session-setup statements as a
    /// single simple-query batch (`LOAD 'age'; SET search_path; SET timeout`).
    pub(in crate::adapters::postgres::graph) fn age_session_setup_sql() -> String {
        let timeout_secs: u32 = std::env::var("EDGEQUAKE_GRAPH_QUERY_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);
        format!(
            "LOAD 'age'; SET search_path = ag_catalog, \"$user\", public; \
             SET statement_timeout = '{}s';",
            timeout_secs
        )
    }

    /// F8: choose a dollar-quote tag guaranteed not to occur in `body`.
    pub(in crate::adapters::postgres::graph) fn dollar_quote_tag(body: &str) -> String {
        const BASE: &str = "$eqcy$";
        if !body.contains(BASE) {
            return BASE.to_string();
        }
        let mut n: u64 = 0;
        loop {
            let tag = format!("$eqcy{}$", n);
            if !body.contains(&tag) {
                return tag;
            }
            n += 1;
        }
    }

    /// Session setup on a dedicated connection (typed reads, graph/index DDL).
    pub(in crate::adapters::postgres::graph) async fn setup_age_session(
        conn: &mut sqlx::PgConnection,
    ) -> Result<()> {
        sqlx::query("LOAD 'age'")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to load AGE: {}", e)))?;
        sqlx::query("SET search_path = ag_catalog, \"$user\", public")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set AGE search path: {}", e)))?;
        let timeout_secs: u32 = std::env::var("EDGEQUAKE_GRAPH_QUERY_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);
        sqlx::query(&format!("SET statement_timeout = '{}s'", timeout_secs))
            .execute(&mut *conn)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Failed to set statement timeout: {}", e))
            })?;
        Ok(())
    }
}
