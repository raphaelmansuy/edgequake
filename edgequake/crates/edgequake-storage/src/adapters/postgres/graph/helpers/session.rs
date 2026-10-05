//! AGE session bootstrap and dollar-quote safety (SPEC-017 P1-12).

use crate::error::{Result, StorageError};

use super::super::PostgresAGEGraphStorage;
use super::graph_query_statement_timeout_ms;

impl PostgresAGEGraphStorage {
    /// QW1 (F2): build the AGE per-connection session-setup statements as a
    /// single simple-query batch (`LOAD 'age'; SET search_path; SET timeout`).
    pub(in crate::adapters::postgres::graph) fn age_session_setup_sql() -> String {
        let timeout_ms = graph_query_statement_timeout_ms();
        format!(
            "LOAD 'age'; SET search_path = ag_catalog, \"$user\", public; \
             SET statement_timeout = '{}ms';",
            timeout_ms
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
        let timeout_ms = graph_query_statement_timeout_ms();
        sqlx::query(&format!("SET statement_timeout = '{timeout_ms}ms'"))
            .execute(&mut *conn)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Failed to set statement timeout: {}", e))
            })?;
        Ok(())
    }

    /// SPEC-042-E E-02: set session tenant for AGE graph RLS policies.
    pub(in crate::adapters::postgres::graph) async fn apply_age_tenant_rls_context(
        conn: &mut sqlx::PgConnection,
        tenant_id: Option<&str>,
    ) -> Result<()> {
        use super::super::super::capabilities::age_rls_requested;
        if !age_rls_requested() {
            return Ok(());
        }
        if let Some(tid) = tenant_id.filter(|s| !s.is_empty()) {
            sqlx::query("SELECT set_config('edgequake.tenant_id', $1, true)")
                .bind(tid)
                .execute(&mut *conn)
                .await
                .map_err(|e| {
                    StorageError::Database(format!("Failed to set AGE tenant context: {}", e))
                })?;
        }
        Ok(())
    }

    /// Session setup with optional AGE RLS tenant context.
    pub(in crate::adapters::postgres::graph) async fn setup_age_session_scoped(
        conn: &mut sqlx::PgConnection,
        tenant_id: Option<&str>,
    ) -> Result<()> {
        Self::setup_age_session(conn).await?;
        Self::apply_age_tenant_rls_context(conn, tenant_id).await
    }

    /// Filtered graph reads apply the same transaction-local RLS envelope as relational reads.
    /// Both absent is the explicit legacy administration scan path.
    pub(in crate::adapters::postgres::graph) async fn enforce_graph_read_scope(
        conn: &mut sqlx::PgConnection,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<()> {
        if tenant_id.is_none() && workspace_id.is_none() {
            return Ok(());
        }
        let parse = |raw: &str| {
            uuid::Uuid::parse_str(raw)
                .map_err(|_| StorageError::InvalidInput("Malformed graph scope".into()))
        };
        let tenant = tenant_id.map(parse).transpose()?;
        let workspace = workspace_id.map(parse).transpose()?;
        if workspace.is_some() {
            super::super::super::rls::enforce_workspace_context(conn, tenant, workspace).await
        } else {
            super::super::super::rls::enforce_tenant_context(
                conn,
                tenant.ok_or_else(|| {
                    StorageError::InvalidInput("Graph tenant scope is required".into())
                })?,
                None,
                None,
            )
            .await
        }
    }

    /// SPEC-069: DDL-only session GUCs — never apply the query `statement_timeout`.
    ///
    /// Postgres ops guidance (2026): `statement_timeout = 0` so legitimate DDL can
    /// finish; short `lock_timeout` so contended ShareLock/ACCESS EXCLUSIVE waits
    /// fail fast instead of stacking behind delete/ingest traffic.
    pub(in crate::adapters::postgres::graph) async fn setup_age_ddl_session(
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
        // SPEC-083 / P0: maintenance window prefers longer lock waits so ADD COLUMN
        // can succeed on large AGE graphs; boot path stays fail-fast at 5s.
        let maintenance = matches!(
            std::env::var("EDGEQUAKE_EQ_MAINTENANCE").as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
        );
        let lock_timeout = std::env::var("EDGEQUAKE_GRAPH_DDL_LOCK_TIMEOUT")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| {
                if maintenance {
                    "120s".to_string()
                } else {
                    "5s".to_string()
                }
            });
        sqlx::query("SET statement_timeout = 0")
            .execute(&mut *conn)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Failed to clear statement_timeout for DDL: {}", e))
            })?;
        sqlx::query(&format!(
            "SET lock_timeout = '{}'",
            lock_timeout.replace('\'', "")
        ))
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to set DDL lock_timeout: {}", e)))?;
        Ok(())
    }
}
