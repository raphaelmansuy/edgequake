//! Internal helpers for Apache AGE graph operations.
//!
//! Cypher query execution, AGE type parsing, SQL escaping,
//! graph creation, and index management.

use std::collections::HashMap;

use sqlx::Row;

use crate::error::{Result, StorageError};
use crate::traits::{GraphEdge, GraphNode};

use super::PostgresAGEGraphStorage;

impl PostgresAGEGraphStorage {
    pub(super) async fn cypher_query(
        &self,
        cypher: &str,
        columns: &[&str],
    ) -> Result<Vec<sqlx::postgres::PgRow>> {
        let pool = self.pool.get().await?;

        // Acquire a dedicated connection so session state persists
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        // Set up AGE session on this connection
        sqlx::query("LOAD 'age'")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to load AGE: {}", e)))?;

        sqlx::query("SET search_path = ag_catalog, \"$user\", public")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set AGE search path: {}", e)))?;

        // Set statement timeout to 30 seconds to allow complex graph queries to complete
        // Application-level timeouts (5s) will trigger fallback before this for most cases
        sqlx::query("SET statement_timeout = '30s'")
            .execute(&mut *conn)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Failed to set statement timeout: {}", e))
            })?;

        // Build AS clause with all columns as agtype
        let as_clause = columns
            .iter()
            .map(|c| format!("{} agtype", c))
            .collect::<Vec<_>>()
            .join(", ");

        // Build SELECT clause with agtype_to_json for each column
        let select_clause = columns
            .iter()
            .map(|c| format!("agtype_to_json({}) as {}", c, c))
            .collect::<Vec<_>>()
            .join(", ");

        // Execute: SELECT agtype_to_json(col) FROM cypher(...) AS (col agtype)
        let sql = format!(
            "SELECT {} FROM cypher('{}', $$ {} $$) AS ({})",
            select_clause, self.graph_name, cypher, as_clause
        );

        let rows = sqlx::query(&sql)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Cypher query failed: {}", e)))?;

        Ok(rows)
    }

    /// Execute a Cypher query that doesn't return results (terminal clause).
    ///
    /// This acquires a single connection and runs LOAD 'age' + SET search_path
    /// before executing the Cypher query.
    pub(super) async fn cypher_execute(&self, cypher: &str) -> Result<()> {
        let pool = self.pool.get().await?;

        // Acquire a dedicated connection so session state persists
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        // Set up AGE session on this connection
        sqlx::query("LOAD 'age'")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to load AGE: {}", e)))?;

        sqlx::query("SET search_path = ag_catalog, \"$user\", public")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set AGE search path: {}", e)))?;

        // Now execute the Cypher query on the same connection
        let sql = format!(
            "SELECT * FROM cypher('{}', $$ {} $$) AS (a agtype)",
            self.graph_name, cypher
        );

        sqlx::query(&sql)
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Cypher execute failed: {}", e)))?;

        Ok(())
    }

    /// Execute a Cypher query that returns a single scalar value (count, degree, etc.)
    ///
    /// For scalar values (integers, strings), agtype_to_json() doesn't work,
    /// so we use agtype_to_int8 for counts.
    pub(super) async fn cypher_query_count(&self, cypher: &str) -> Result<i64> {
        let pool = self.pool.get().await?;

        // Acquire a dedicated connection so session state persists
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        // Set up AGE session on this connection
        sqlx::query("LOAD 'age'")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to load AGE: {}", e)))?;

        sqlx::query("SET search_path = ag_catalog, \"$user\", public")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set AGE search path: {}", e)))?;

        // Use agtype_to_int8 to convert count to bigint
        let sql = format!(
            "SELECT agtype_to_int8(count) FROM cypher('{}', $$ {} $$) AS (count agtype)",
            self.graph_name, cypher
        );

        let row = sqlx::query(&sql)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Cypher count query failed: {}", e)))?;

        Ok(row.map(|r| r.get::<i64, _>(0)).unwrap_or(0))
    }

    /// Parse an AGE vertex agtype into a GraphNode.
    pub(super) fn parse_vertex(agtype_str: &str) -> Option<GraphNode> {
        // AGE returns: {"id": 123, "label": "Node", "properties": {...}}::vertex
        let json_str = agtype_str.trim_end_matches("::vertex");

        let value: serde_json::Value = serde_json::from_str(json_str).ok()?;
        let obj = value.as_object()?;

        // The node ID is stored in properties.node_id (our custom field)
        let properties = obj.get("properties")?.as_object()?;
        let node_id = properties.get("node_id")?.as_str()?.to_string();

        // Convert properties to HashMap, excluding node_id
        let mut props: HashMap<String, serde_json::Value> = HashMap::new();
        for (k, v) in properties.iter() {
            if k != "node_id" {
                props.insert(k.clone(), v.clone());
            }
        }

        Some(GraphNode {
            id: node_id,
            properties: props,
        })
    }

    /// Parse an AGE edge agtype into a GraphEdge.
    pub(super) fn parse_edge(agtype_str: &str) -> Option<GraphEdge> {
        // AGE returns: {"id": 123, "label": "EDGE", "start_id": 1, "end_id": 2, "properties": {...}}::edge
        let json_str = agtype_str.trim_end_matches("::edge");

        let value: serde_json::Value = serde_json::from_str(json_str).ok()?;
        let obj = value.as_object()?;

        let properties = obj.get("properties")?.as_object()?;
        let source = properties.get("source_id")?.as_str()?.to_string();
        let target = properties.get("target_id")?.as_str()?.to_string();

        // Convert properties to HashMap, excluding source_id and target_id
        let mut props: HashMap<String, serde_json::Value> = HashMap::new();
        for (k, v) in properties.iter() {
            if k != "source_id" && k != "target_id" {
                props.insert(k.clone(), v.clone());
            }
        }

        Some(GraphEdge {
            source,
            target,
            properties: props,
        })
    }

    /// Parse a `GraphEdge` from a plain edge-properties JSON value.
    ///
    /// # WHY: Native SQL path
    ///
    /// When edges are fetched via native SQL (`ag_catalog.agtype_to_json(properties)`),
    /// the result is already the flat properties object — NOT the full AGE edge envelope
    /// `{"id":…,"label":…,"properties":{…}}::edge` returned by Cypher.
    /// This helper avoids the extra unwrapping done by `parse_edge`.
    pub(super) fn parse_edge_from_props(props_json: serde_json::Value) -> Option<GraphEdge> {
        let properties = props_json.as_object()?;
        let source = properties.get("source_id")?.as_str()?.to_string();
        let target = properties.get("target_id")?.as_str()?.to_string();

        let mut props: HashMap<String, serde_json::Value> = HashMap::new();
        for (k, v) in properties.iter() {
            if k != "source_id" && k != "target_id" {
                props.insert(k.clone(), v.clone());
            }
        }

        Some(GraphEdge {
            source,
            target,
            properties: props,
        })
    }

    /// Escape a string for use in Cypher queries.
    pub(super) fn escape_cypher_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    /// Escape a string for use in SQL queries.
    ///
    /// # WHY: SQL uses different escaping than Cypher
    /// SQL uses doubled single quotes ('') to escape single quotes,
    /// not backslash (\') like Cypher. Using backslash in SQL IN clauses
    /// causes "syntax error at or near \" errors in PostgreSQL.
    pub(super) fn escape_sql_string(s: &str) -> String {
        s.replace('\'', "''")
    }

    /// Convert properties HashMap to Cypher map literal.
    ///
    /// # WHY: Proper array serialization
    /// Arrays must be converted to Cypher list syntax `[a, b, c]` not JSON strings.
    /// Without this, source_chunk_ids and other array properties become strings
    /// like `'["chunk1", "chunk2"]'` instead of proper lists, breaking array operations.
    pub(super) fn properties_to_cypher(props: &HashMap<String, serde_json::Value>) -> String {
        if props.is_empty() {
            return "{}".to_string();
        }

        let parts: Vec<String> = props
            .iter()
            .map(|(k, v)| {
                let value_str = Self::value_to_cypher(v);
                format!("{}: {}", k, value_str)
            })
            .collect();

        format!("{{{}}}", parts.join(", "))
    }

    /// Convert a single JSON value to Cypher literal syntax.
    ///
    /// Handles:
    /// - Strings: escaped with single quotes
    /// - Numbers: raw numeric literals
    /// - Booleans: `true`/`false`
    /// - Null: `null`
    /// - Arrays: `[val1, val2, ...]` (recursive)
    /// - Objects: `{key1: val1, ...}` (recursive)
    pub(super) fn value_to_cypher(v: &serde_json::Value) -> String {
        match v {
            serde_json::Value::String(s) => format!("'{}'", Self::escape_cypher_string(s)),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => "null".to_string(),
            serde_json::Value::Array(arr) => {
                // Convert to Cypher list: [val1, val2, val3]
                let items: Vec<String> = arr.iter().map(Self::value_to_cypher).collect();
                format!("[{}]", items.join(", "))
            }
            serde_json::Value::Object(obj) => {
                // Convert to nested Cypher map: {key1: val1, key2: val2}
                let items: Vec<String> = obj
                    .iter()
                    .map(|(k, val)| format!("{}: {}", k, Self::value_to_cypher(val)))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
        }
    }

    /// Create the AGE graph if it doesn't exist.
    pub(super) async fn create_graph(&self) -> Result<()> {
        let pool = self.pool.get().await?;

        // Acquire a dedicated connection so session state persists
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        // Set up AGE session on this connection
        sqlx::query("LOAD 'age'")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to load AGE: {}", e)))?;

        sqlx::query("SET search_path = ag_catalog, \"$user\", public")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set AGE search path: {}", e)))?;

        // Check if graph exists
        let check_sql = format!(
            "SELECT 1 FROM ag_catalog.ag_graph WHERE name = '{}'",
            self.graph_name
        );

        let exists = sqlx::query(&check_sql)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Graph check failed: {}", e)))?;

        if exists.is_none() {
            // Create graph
            let create_sql = format!(
                "SELECT * FROM ag_catalog.create_graph('{}')",
                self.graph_name
            );

            sqlx::query(&create_sql)
                .execute(&mut *conn)
                .await
                .map_err(|e| {
                    StorageError::Database(format!("Failed to create AGE graph: {}", e))
                })?;

            tracing::info!("Created AGE graph: {}", self.graph_name);
        }

        Ok(())
    }

    /// Create or ensure indexes exist on AGE graph tables for query performance.
    ///
    /// This is CRITICAL for query performance. Without these indexes, Cypher queries
    /// like `MATCH (n:Node {node_id: 'xxx'})` perform full table scans.
    ///
    /// Creates indexes on:
    /// - Node table: node_id property expression index, GIN index on properties
    /// - EDGE table: start_id, end_id, and composite indexes
    pub(super) async fn ensure_indexes(&self) -> Result<()> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        // Set up AGE session
        sqlx::query("LOAD 'age'")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to load AGE: {}", e)))?;

        sqlx::query("SET search_path = ag_catalog, \"$user\", public")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set AGE search path: {}", e)))?;

        // Define all indexes to create - order matters for dependencies
        let index_queries = [
            // Node table indexes (CRITICAL for query performance)
            (
                "idx_node_prop_node_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_node_prop_node_id 
                       ON {}."Node" (ag_catalog.agtype_access_operator(properties, '"node_id"'::agtype))"#,
                    self.graph_name
                ),
            ),
            (
                "idx_node_props_gin",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_node_props_gin 
                       ON {}."Node" USING gin(properties)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_node_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_node_id 
                       ON {}."Node" (id)"#,
                    self.graph_name
                ),
            ),
            // EDGE table indexes
            (
                "idx_edge_start_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_edge_start_id 
                       ON {}."EDGE" (start_id)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_edge_end_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_edge_end_id 
                       ON {}."EDGE" (end_id)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_edge_start_end",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_edge_start_end 
                       ON {}."EDGE" (start_id, end_id)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_edge_props_gin",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_edge_props_gin 
                       ON {}."EDGE" USING gin(properties)"#,
                    self.graph_name
                ),
            ),
            // Fallback indexes on AGE internal tables
            (
                "idx_ag_vertex_props_gin",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_ag_vertex_props_gin 
                       ON {}."_ag_label_vertex" USING gin(properties)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_ag_edge_start_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_ag_edge_start_id 
                       ON {}."_ag_label_edge" (start_id)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_ag_edge_end_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_ag_edge_end_id 
                       ON {}."_ag_label_edge" (end_id)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_ag_edge_start_end",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_ag_edge_start_end 
                       ON {}."_ag_label_edge" (start_id, end_id)"#,
                    self.graph_name
                ),
            ),
        ];

        let mut indexes_created = 0;
        let mut indexes_skipped = 0;

        for (name, sql) in &index_queries {
            match sqlx::query(sql).execute(&mut *conn).await {
                Ok(_) => {
                    indexes_created += 1;
                    tracing::debug!("Created/verified index: {}", name);
                }
                Err(e) => {
                    // Check if it's a "table does not exist" error - this is OK
                    // The table will be created on first node/edge insertion
                    let err_str = e.to_string();
                    if err_str.contains("does not exist")
                        || err_str.contains("undefined_table")
                        || err_str.contains("relation")
                    {
                        indexes_skipped += 1;
                        tracing::debug!(
                            "Skipped index {} (table not yet created): {}",
                            name,
                            err_str
                        );
                    } else {
                        tracing::warn!("Failed to create index {}: {}", name, e);
                    }
                }
            }
        }

        if indexes_created > 0 {
            tracing::info!(
                "AGE graph indexes: {} created/verified, {} skipped (tables pending)",
                indexes_created,
                indexes_skipped
            );
        }

        Ok(())
    }

    /// Execute a batch SQL query with array parameter binding.
    ///
    /// This is the LightRAG-inspired pattern using UNNEST with ORDINALITY
    /// for efficient batch queries with preserved ordering.
    pub(super) async fn batch_sql_query(
        &self,
        sql: &str,
        ids: &[String],
    ) -> Result<Vec<sqlx::postgres::PgRow>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        // Set up AGE session
        sqlx::query("LOAD 'age'")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to load AGE: {}", e)))?;

        sqlx::query("SET search_path = ag_catalog, \"$user\", public")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set search path: {}", e)))?;

        // Set statement timeout
        sqlx::query("SET statement_timeout = '30s'")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set timeout: {}", e)))?;

        let rows = sqlx::query(sql)
            .bind(ids)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Batch query failed: {}", e)))?;

        Ok(rows)
    }

    /// Parse JSON properties into a GraphNode.
    pub(super) fn parse_properties_to_node(
        node_id: &str,
        props_json: &serde_json::Value,
    ) -> Option<GraphNode> {
        if props_json.is_null() {
            return None;
        }

        let properties_map = props_json.as_object()?;
        let properties: HashMap<String, serde_json::Value> = properties_map
            .iter()
            .filter(|(k, _)| k.as_str() != "node_id")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Some(GraphNode {
            id: node_id.to_string(),
            properties,
        })
    }

    /// Parse JSON into properties HashMap.
    pub(super) fn parse_json_to_properties(
        props_json: &serde_json::Value,
    ) -> HashMap<String, serde_json::Value> {
        if let Some(obj) = props_json.as_object() {
            obj.iter()
                .filter(|(k, _)| k.as_str() != "source_id" && k.as_str() != "target_id")
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        } else {
            HashMap::new()
        }
    }
}
