//! Cypher execution helpers (SPEC-017 P1-12, SPEC-022 P-H7 parameterized Cypher).

use sqlx::Row;

use crate::error::{Result, StorageError};

use super::super::PostgresAGEGraphStorage;

impl PostgresAGEGraphStorage {
    /// Build the outer SQL for a parameterized Cypher call (AGE prepared-statement pattern).
    fn cypher_bound_sql(graph_name: &str, cypher: &str, columns: &[&str], execute: bool) -> String {
        let tag = Self::dollar_quote_tag(cypher);
        // WHY bare $1: AGE requires the third argument to cypher() to be a Param
        // node — any cast expression (e.g. $1::agtype) is rejected with "third
        // argument of cypher function must be a parameter". We bind the agtype
        // map as text (String) and let PostgreSQL's text→agtype input function
        // handle the coercion via the declared parameter type.
        if execute {
            return format!(
                "{} SELECT * FROM cypher('{}', {} {} {}, $1) AS (a agtype);",
                Self::age_session_setup_sql(),
                graph_name,
                tag,
                cypher,
                tag
            );
        }
        let as_clause = columns
            .iter()
            .map(|c| format!("{} agtype", c))
            .collect::<Vec<_>>()
            .join(", ");
        let select_clause = columns
            .iter()
            .map(|c| format!("agtype_to_json({}) as {}", c, c))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "SELECT {} FROM cypher('{}', {} {} {}, $1) AS ({})",
            select_clause, graph_name, tag, cypher, tag, as_clause
        )
    }

    fn cypher_params_text(params: &serde_json::Value) -> String {
        serde_json::to_string(params).unwrap_or_else(|_| "{}".to_string())
    }

    /// Run read Cypher with bound agtype parameters (no string interpolation of user values).
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
        let params_text = Self::cypher_params_text(params);
        let rows = sqlx::query(&sql)
            .bind(params_text)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Parameterized Cypher query failed: {}", e))
            })?;

        Ok(rows)
    }

    /// Run write Cypher with bound agtype parameters.
    pub(in crate::adapters::postgres::graph) async fn cypher_execute_bound(
        &self,
        cypher: &str,
        params: &serde_json::Value,
    ) -> Result<()> {
        let pool = self.pool.get().await?;
        let sql = Self::cypher_bound_sql(&self.graph_name, cypher, &[], true);
        let params_text = Self::cypher_params_text(params);
        sqlx::query(&sql)
            .bind(params_text)
            .execute(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Parameterized Cypher execute failed: {}", e))
            })?;
        Ok(())
    }

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
            .map(|c| format!("{} agtype", c))
            .collect::<Vec<_>>()
            .join(", ");
        let select_clause = columns
            .iter()
            .map(|c| format!("agtype_to_json({}) as {}", c, c))
            .collect::<Vec<_>>()
            .join(", ");

        let tag = Self::dollar_quote_tag(cypher);
        let sql = format!(
            "SELECT {} FROM cypher('{}', {} {} {}) AS ({})",
            select_clause, self.graph_name, tag, cypher, tag, as_clause
        );

        let rows = sqlx::query(&sql)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Cypher query failed: {}", e)))?;

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
            .map_err(|e| StorageError::Database(format!("Cypher execute failed: {}", e)))?;

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
            .map_err(|e| StorageError::Database(format!("Cypher count query failed: {}", e)))?;

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
            .map_err(|e| StorageError::Database(format!("Batch query failed: {}", e)))?;

        Ok(rows)
    }
}
