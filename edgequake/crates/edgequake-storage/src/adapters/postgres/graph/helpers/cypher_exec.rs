//! Cypher execution helpers (SPEC-017 P1-12, SPEC-022 P-H7 parameterized Cypher).
//!
//! ## AGE parameter binding
//!
//! AGE's `cypher()` function requires the third (parameter-map) argument to be
//! a bare `$1` inside a `PREPARE`d statement — any cast expression (e.g.
//! `$1::agtype`) is rejected.  We therefore inline the agtype literal as
//! `'<escaped-json>'::agtype` directly in the SQL.  Because the JSON is
//! produced by `serde_json` (no user-controlled SQL fragments) and single
//! quotes are escaped, this is safe against injection.

use sqlx::Row;

use crate::error::{Result, StorageError};

use super::super::PostgresAGEGraphStorage;

impl PostgresAGEGraphStorage {
    /// Escape a JSON string for use as an agtype literal in SQL.
    fn escape_agtype_literal(json: &str) -> String {
        json.replace('\'', "''")
    }

    /// Run read Cypher with agtype parameters (injection-safe via serde_json).
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

        let params_json = serde_json::to_string(params).unwrap_or_else(|_| "{}".to_string());
        let params_lit = Self::escape_agtype_literal(&params_json);

        let tag = Self::dollar_quote_tag(cypher);
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

        let sql = format!(
            "SELECT {select_clause} FROM cypher('{graph}', {tag} {cypher} {tag}, \
             '{params_lit}'::agtype) AS ({as_clause})",
            graph = self.graph_name,
        );

        let rows = sqlx::query(&sql).fetch_all(&mut *conn).await.map_err(|e| {
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

        let params_json = serde_json::to_string(params).unwrap_or_else(|_| "{}".to_string());
        let params_lit = Self::escape_agtype_literal(&params_json);

        let tag = Self::dollar_quote_tag(cypher);
        let sql = format!(
            "{setup} SELECT * FROM cypher('{graph}', {tag} {cypher} {tag}, \
             '{params_lit}'::agtype) AS (a agtype);",
            setup = Self::age_session_setup_sql(),
            graph = self.graph_name,
        );

        sqlx::raw_sql(&sql).execute(&pool).await.map_err(|e| {
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
