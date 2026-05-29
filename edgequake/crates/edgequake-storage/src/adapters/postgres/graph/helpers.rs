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
    /// QW1 (F2): build the AGE per-connection session-setup statements as a
    /// single simple-query batch (`LOAD 'age'; SET search_path; SET timeout`).
    ///
    /// # WHY one batched statement instead of 2-3 separate `execute` calls
    /// Every graph op previously issued `LOAD 'age'` + `SET search_path` (and
    /// sometimes `SET statement_timeout`) as independent round trips before the
    /// actual Cypher. On the SPEC-011 shared pool — whose `after_connect` only
    /// sets `search_path=public` and never loads AGE — this session tax is paid
    /// on *every* call. Collapsing the setup into one `raw_sql` batch removes
    /// 1-2 network round trips per graph operation. We keep it per-call (rather
    /// than in `after_connect`) because the shared pool is constructed by the
    /// API layer and the bootstrap ordering of `CREATE EXTENSION age` cannot be
    /// guaranteed before the first connections are established.
    ///
    /// # WHY executed on `&PgPool` (not a reborrowed `&mut PgConnection`)
    /// `sqlx::raw_sql(..).execute(&mut *conn)` makes the surrounding async-trait
    /// future fail the `Send`/higher-ranked `Executor` bound (a known sqlx 0.8
    /// limitation). Running the *entire* batch (setup + terminal Cypher) on
    /// `&PgPool` — which is `'static` and `Send` — sidesteps that while still
    /// executing as a single simple-query round trip on one pooled connection.
    /// This is only safe for self-contained batches that do not need session
    /// state to survive into a *separate* follow-up query (writes qualify;
    /// the typed read helpers below keep their dedicated-connection setup).
    fn age_session_setup_sql() -> String {
        // Configurable via EDGEQUAKE_GRAPH_QUERY_TIMEOUT_SECS (default 30s).
        // WHY parse to integer: prevents injection through the env var into the
        // SET statement (a non-numeric value would be a syntax error at best,
        // an injection vector at worst).
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

    /// F8: choose a dollar-quote tag that is guaranteed not to occur in `body`.
    ///
    /// # WHY: defends against `$$`-based SQL injection through Cypher bodies
    /// The Cypher body is embedded into `cypher('graph', $$ ...body... $$)`.
    /// `escape_cypher_string` neutralizes single/double quotes and backslashes
    /// but does **not** neutralize the `$$` dollar-quote terminator. A property
    /// value (e.g. an LLM-extracted entity description sourced from an attacker
    /// document) containing `$$` could close the SQL string literal early and
    /// inject arbitrary SQL. By selecting a unique tag we *verify* is absent
    /// from the body, premature termination becomes impossible.
    pub(super) fn dollar_quote_tag(body: &str) -> String {
        const BASE: &str = "$eqcy$";
        if !body.contains(BASE) {
            return BASE.to_string();
        }
        // Extremely unlikely fallback: find a numbered tag not present in body.
        let mut n: u64 = 0;
        loop {
            let tag = format!("$eqcy{}$", n);
            if !body.contains(&tag) {
                return tag;
            }
            n += 1;
        }
    }

    /// QW1 helper for paths that need session state to persist across a
    /// *separate* follow-up query on the **same dedicated connection** (typed
    /// reads, graph/index creation). These cannot use the batched `raw_sql`
    /// trick (it would run on a different pooled connection), so we issue the
    /// three setup statements on the provided connection. Kept in one place for
    /// DRY and to guarantee identical session configuration everywhere.
    async fn setup_age_session(conn: &mut sqlx::PgConnection) -> Result<()> {
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

    pub(super) async fn cypher_query(
        &self,
        cypher: &str,
        columns: &[&str],
    ) -> Result<Vec<sqlx::postgres::PgRow>> {
        let pool = self.pool.get().await?;

        // Acquire a dedicated connection so session state persists into the
        // typed SELECT that follows.
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        Self::setup_age_session(&mut conn).await?;

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

        // F8: dollar-quote with a verified-unique tag instead of bare `$$`.
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

    /// Execute a Cypher query that doesn't return results (terminal clause).
    ///
    /// QW1: session setup and the Cypher statement are sent as a single
    /// simple-query batch — one network round trip for the entire write.
    pub(super) async fn cypher_execute(&self, cypher: &str) -> Result<()> {
        let pool = self.pool.get().await?;

        // F8: verified-unique dollar-quote tag.
        let tag = Self::dollar_quote_tag(cypher);
        // QW1: setup + cypher in ONE round trip (writes are the hot path: F1/F3).
        // Executed on `&PgPool` (see `age_session_setup_sql` doc) so the whole
        // self-contained batch runs on a single pooled connection without
        // tripping the sqlx `raw_sql` + `&mut PgConnection` Send limitation.
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

    /// Execute a Cypher query that returns a single scalar value (count, degree, etc.)
    ///
    /// For scalar values (integers, strings), agtype_to_json() doesn't work,
    /// so we use agtype_to_int8 for counts.
    pub(super) async fn cypher_query_count(&self, cypher: &str) -> Result<i64> {
        let pool = self.pool.get().await?;

        // Acquire a dedicated connection so session state persists into the
        // typed scalar SELECT that follows.
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        Self::setup_age_session(&mut conn).await?;

        // F8: verified-unique dollar-quote tag.
        let tag = Self::dollar_quote_tag(cypher);
        // Use agtype_to_int8 to convert count to bigint
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

        // QW1: batched AGE session setup (DRY, single helper).
        Self::setup_age_session(&mut conn).await?;

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

        // QW1: batched AGE session setup (DRY, single helper).
        Self::setup_age_session(&mut conn).await?;

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

        // QW1: batched AGE session setup (DRY, single helper).
        Self::setup_age_session(&mut conn).await?;

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

#[cfg(test)]
mod helper_tests {
    use super::PostgresAGEGraphStorage;

    #[test]
    fn test_dollar_quote_tag_default_when_absent() {
        // Common case: body has no `$eqcy$`, so the base tag is used.
        let body = "MATCH (n:Node {node_id: 'abc'}) RETURN n";
        assert_eq!(PostgresAGEGraphStorage::dollar_quote_tag(body), "$eqcy$");
    }

    #[test]
    fn test_dollar_quote_tag_avoids_collision_with_base() {
        // F8: if the body literally contains the base tag, a numbered tag that
        // is NOT present in the body must be chosen.
        let body = "RETURN '$eqcy$ injection attempt'";
        let tag = PostgresAGEGraphStorage::dollar_quote_tag(body);
        assert_ne!(tag, "$eqcy$");
        assert!(!body.contains(&tag), "chosen tag must be absent from body");
    }

    #[test]
    fn test_dollar_quote_tag_avoids_double_dollar_injection() {
        // F8: a body carrying a bare `$$` (the legacy terminator) must still be
        // safely wrappable — the chosen tag must not appear in the body.
        let body = "SET n.desc = 'ends here $$ DROP TABLE foo; --'";
        let tag = PostgresAGEGraphStorage::dollar_quote_tag(body);
        assert!(!body.contains(&tag));
        // The base tag is fine here because the body has `$$` not `$eqcy$`.
        assert_eq!(tag, "$eqcy$");
    }

    #[test]
    fn test_dollar_quote_tag_escalates_through_numbered_tags() {
        // Pathological: body contains base AND the first numbered tag.
        let body = "x $eqcy$ y $eqcy0$ z";
        let tag = PostgresAGEGraphStorage::dollar_quote_tag(body);
        assert!(!body.contains(&tag));
        assert_eq!(tag, "$eqcy1$");
    }

    #[test]
    fn test_age_session_setup_is_single_batched_statement() {
        // QW1: setup must contain LOAD + search_path + statement_timeout so the
        // whole session bootstrap is one simple-query round trip.
        let sql = PostgresAGEGraphStorage::age_session_setup_sql();
        assert!(sql.contains("LOAD 'age'"));
        assert!(sql.contains("search_path = ag_catalog"));
        assert!(sql.contains("statement_timeout"));
        // Three statements separated by `;`.
        assert_eq!(sql.matches(';').count(), 3);
    }
}
