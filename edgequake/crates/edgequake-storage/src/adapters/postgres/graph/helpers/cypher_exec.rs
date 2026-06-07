//! Cypher execution helpers (SPEC-017 P1-12).

use sqlx::Row;

use crate::error::{Result, StorageError};

use super::super::PostgresAGEGraphStorage;

impl PostgresAGEGraphStorage {
    pub(in crate::adapters::postgres::graph) async fn cypher_query(
        &self,
        cypher: &str,
        columns: &[&str],
    ) -> Result<Vec<sqlx::postgres::PgRow>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        Self::setup_age_session(&mut conn).await?;

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

        Self::setup_age_session(&mut conn).await?;

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

        Self::setup_age_session(&mut conn).await?;

        let rows = sqlx::query(sql)
            .bind(ids)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Batch query failed: {}", e)))?;

        Ok(rows)
    }
}
