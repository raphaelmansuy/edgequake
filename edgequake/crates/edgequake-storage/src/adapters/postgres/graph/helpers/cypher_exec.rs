//! Cypher execution helpers (SPEC-017 P1-12, SPEC-022 P-H7, SPEC-044).
//!
//! ## AGE parameter binding (SPEC-044)
//!
//! AGE's `cypher()` third argument must be a bare Postgres parameter (`$1`) inside
//! a prepared statement — inline literals and cast expressions on the third
//! argument are rejected. See:
//! <https://age.apache.org/age-manual/master/advanced/prepared_statements.html>
//!
//! Bind the parameter map as a JSON **string** wrapped in [`PgAgtype`] so sqlx
//! sends PostgreSQL type `agtype` (bare `$1` third argument).

use sqlx::Row;

use crate::error::{Result, StorageError};

use super::super::PostgresAGEGraphStorage;
use super::agtype_bind::PgAgtype;

impl PostgresAGEGraphStorage {
    /// Build SQL for parameterized Cypher (bare `$1` third arg — SPEC-044 SSOT).
    fn cypher_bound_sql(graph_name: &str, cypher: &str, columns: &[&str], execute: bool) -> String {
        let tag = Self::dollar_quote_tag(cypher);
        if execute {
            return format!(
                "SELECT * FROM cypher('{graph}', {tag} {cypher} {tag}, $1) AS (a agtype)",
                graph = graph_name,
            );
        }

        let as_clause = columns
            .iter()
            .map(|c| format!("{c} agtype"))
            .collect::<Vec<_>>()
            .join(", ");
        let select_clause = columns
            .iter()
            .map(|c| format!("agtype_to_json({c}) as {c}"))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "SELECT {select_clause} FROM cypher('{graph}', {tag} {cypher} {tag}, $1) AS ({as_clause})",
            graph = graph_name,
        )
    }

    /// Run read Cypher with agtype parameters (injection-safe via serde_json + `$1`).
    /// Bound Cypher helper retained for debug / AGE fallbacks (request path prefers native SQL — IMP-031).
    #[allow(dead_code)]
    pub(in crate::adapters::postgres::graph) async fn cypher_query_bound(
        &self,
        cypher: &str,
        columns: &[&str],
        params: &serde_json::Value,
    ) -> Result<Vec<sqlx::postgres::PgRow>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        Self::setup_age_session_scoped(&mut conn, None).await?;

        let sql = Self::cypher_bound_sql(&self.graph_name, cypher, columns, false);
        let params_bind = PgAgtype::from_json(params)?;

        let rows = sqlx::query(&sql)
            .bind(params_bind)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Parameterized Cypher query failed: {e}"))
            })?;

        Ok(rows)
    }

    /// Run write Cypher with agtype parameters.
    pub(in crate::adapters::postgres::graph) async fn cypher_execute_bound(
        &self,
        cypher: &str,
        params: &serde_json::Value,
    ) -> Result<()> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        Self::setup_age_session_scoped(&mut conn, None).await?;

        let sql = Self::cypher_bound_sql(&self.graph_name, cypher, &[], true);
        let params_bind = PgAgtype::from_json(params)?;

        sqlx::query(&sql)
            .bind(params_bind)
            .execute(&mut *conn)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Parameterized Cypher execute failed: {e}"))
            })?;

        Ok(())
    }

    // ── non-parameterized helpers (unchanged) ───────────────────────────────

    pub(in crate::adapters::postgres::graph) async fn cypher_query(
        &self,
        cypher: &str,
        columns: &[&str],
    ) -> Result<Vec<sqlx::postgres::PgRow>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        Self::setup_age_session_scoped(&mut conn, None).await?;

        let as_clause = columns
            .iter()
            .map(|c| format!("{c} agtype"))
            .collect::<Vec<_>>()
            .join(", ");
        let select_clause = columns
            .iter()
            .map(|c| format!("agtype_to_json({c}) as {c}"))
            .collect::<Vec<_>>()
            .join(", ");

        let tag = Self::dollar_quote_tag(cypher);
        let sql = format!(
            "SELECT {select_clause} FROM cypher('{graph}', {tag} {cypher} {tag}) AS ({as_clause})",
            graph = self.graph_name,
        );

        let rows = sqlx::query(&sql)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Cypher query failed: {e}")))?;

        Ok(rows)
    }

    pub(in crate::adapters::postgres::graph) async fn cypher_execute(
        &self,
        cypher: &str,
    ) -> Result<()> {
        let pool = self.pool.get().await?;
        let tag = Self::dollar_quote_tag(cypher);
        let sql = format!(
            "{} SELECT * FROM cypher('{}', {} {} {}) AS (a agtype);",
            Self::age_session_setup_sql(),
            self.graph_name,
            tag,
            cypher,
            tag
        );

        sqlx::raw_sql(&sql)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Cypher execute failed: {e}")))?;

        Ok(())
    }

    /// Kept for rare debug/admin Cypher aggregates (request path uses native COUNT — IMP-031-06).
    #[allow(dead_code)]
    pub(in crate::adapters::postgres::graph) async fn cypher_query_count(
        &self,
        cypher: &str,
    ) -> Result<i64> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        Self::setup_age_session_scoped(&mut conn, None).await?;

        let tag = Self::dollar_quote_tag(cypher);
        let sql = format!(
            "SELECT agtype_to_int8(count) FROM cypher('{}', {} {} {}) AS (count agtype)",
            self.graph_name, tag, cypher, tag
        );

        let row = sqlx::query(&sql)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Cypher count query failed: {e}")))?;

        Ok(row.map(|r| r.get::<i64, _>(0)).unwrap_or(0))
    }

    pub(in crate::adapters::postgres::graph) async fn batch_sql_query(
        &self,
        sql: &str,
        ids: &[String],
    ) -> Result<Vec<sqlx::postgres::PgRow>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        Self::setup_age_session_scoped(&mut conn, None).await?;

        let rows = sqlx::query(sql)
            .bind(ids)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Batch query failed: {e}")))?;

        Ok(rows)
    }
}
