//! PostgreSQL graph storage using Apache AGE extension.
//!
//! This module implements graph storage using Apache AGE (A Graph Extension)
//! for PostgreSQL. AGE provides native graph database capabilities with
//! Cypher query language support.
//!
//! # Features
//!
//! - Native Cypher query language support
//! - Variable-length path traversal
//! - ACID transactions with graph operations
//! - Native graph storage (vertices and edges)
//! - Efficient graph-optimized indexes
//!
//! # Requirements
//!
//! - PostgreSQL 11-17
//! - Apache AGE extension installed and loaded
//!
//! # Example
//!
//! ```ignore
//! use edgequake_storage::adapters::postgres::{PostgresConfig, PostgresAGEGraphStorage};
//!
//! let config = PostgresConfig::new("localhost", 5432, "edgequake", "user", "pass")
//!     .with_namespace("my-workspace");
//!
//! let storage = PostgresAGEGraphStorage::new(config);
//! storage.initialize().await?;
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use sqlx::Row;

use super::config::PostgresConfig;
use super::connection::PostgresPool;
use crate::error::{Result, StorageError};
use crate::traits::{GraphEdge, GraphNode, GraphStorage, KnowledgeGraph};

/// PostgreSQL graph storage using Apache AGE.
///
/// Uses the AGE extension for native graph operations with Cypher queries.
/// All operations use AGE's graph-optimized storage and query engine.
pub struct PostgresAGEGraphStorage {
    pool: PostgresPool,
    graph_name: String,
    namespace: String,
    prefix: String,
    initialized: AtomicBool,
}

impl PostgresAGEGraphStorage {
    /// Create a new Apache AGE graph storage.
    pub fn new(config: PostgresConfig) -> Self {
        let prefix = config.table_prefix();
        let graph_name = format!("eq_{}_graph", prefix);
        let namespace = config.namespace.clone();

        Self {
            pool: PostgresPool::new(config),
            graph_name,
            namespace,
            prefix,
            initialized: AtomicBool::new(false),
        }
    }

    /// Get the underlying pool.
    pub fn pool(&self) -> &PostgresPool {
        &self.pool
    }

    /// Get the graph name.
    pub fn graph_name(&self) -> &str {
        &self.graph_name
    }

    /// Execute a Cypher query that returns results.
    ///
    /// This acquires a single connection and runs LOAD 'age' + SET search_path
    /// before executing the Cypher query to ensure AGE is available.
    ///
    /// The `columns` parameter specifies the column name(s) for the AS clause.
    /// Each column will be cast to json using agtype_to_json() for sqlx compatibility.
    async fn cypher_query(
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
    async fn cypher_execute(&self, cypher: &str) -> Result<()> {
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
    async fn cypher_query_count(&self, cypher: &str) -> Result<i64> {
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
    fn parse_vertex(agtype_str: &str) -> Option<GraphNode> {
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
    fn parse_edge(agtype_str: &str) -> Option<GraphEdge> {
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

    /// Escape a string for use in Cypher queries.
    fn escape_cypher_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    /// Convert properties HashMap to Cypher map literal.
    fn properties_to_cypher(props: &HashMap<String, serde_json::Value>) -> String {
        if props.is_empty() {
            return "{}".to_string();
        }

        let parts: Vec<String> = props
            .iter()
            .map(|(k, v)| {
                let value_str = match v {
                    serde_json::Value::String(s) => format!("'{}'", Self::escape_cypher_string(s)),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => "null".to_string(),
                    _ => format!("'{}'", Self::escape_cypher_string(&v.to_string())),
                };
                format!("{}: {}", k, value_str)
            })
            .collect();

        format!("{{{}}}", parts.join(", "))
    }

    /// Create the AGE graph if it doesn't exist.
    async fn create_graph(&self) -> Result<()> {
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
}

#[async_trait]
impl GraphStorage for PostgresAGEGraphStorage {
    fn namespace(&self) -> &str {
        &self.namespace
    }

    async fn initialize(&self) -> Result<()> {
        if self.initialized.load(Ordering::Relaxed) {
            return Ok(());
        }

        self.pool.initialize().await?;
        self.create_graph().await?;
        self.initialized.store(true, Ordering::Relaxed);

        tracing::info!(
            "Initialized PostgresAGEGraphStorage with graph '{}'",
            self.graph_name
        );

        Ok(())
    }

    async fn finalize(&self) -> Result<()> {
        Ok(())
    }

    async fn has_node(&self, node_id: &str) -> Result<bool> {
        let escaped_id = Self::escape_cypher_string(node_id);
        let cypher = format!(
            "MATCH (n:Node {{node_id: '{}'}}) RETURN n LIMIT 1",
            escaped_id
        );

        let rows = self.cypher_query(&cypher, &["n"]).await?;
        Ok(!rows.is_empty())
    }

    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>> {
        let escaped_id = Self::escape_cypher_string(node_id);
        let cypher = format!("MATCH (n:Node {{node_id: '{}'}}) RETURN n", escaped_id);

        let rows = self.cypher_query(&cypher, &["n"]).await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let json_value: serde_json::Value = rows[0].get("n");
        let agtype_str = json_value.to_string();
        Ok(Self::parse_vertex(&agtype_str))
    }

    async fn upsert_node(
        &self,
        node_id: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let escaped_id = Self::escape_cypher_string(node_id);

        // Build properties with node_id included
        let mut props_with_id = properties.clone();
        props_with_id.insert(
            "node_id".to_string(),
            serde_json::Value::String(node_id.to_string()),
        );
        let props_cypher = Self::properties_to_cypher(&props_with_id);

        // Use MERGE to upsert the node
        let cypher = format!(
            "MERGE (n:Node {{node_id: '{}'}}) SET n = {}",
            escaped_id, props_cypher
        );

        self.cypher_execute(&cypher).await
    }

    async fn delete_node(&self, node_id: &str) -> Result<()> {
        let escaped_id = Self::escape_cypher_string(node_id);

        // Use DETACH DELETE to remove node and all connected edges
        let cypher = format!(
            "MATCH (n:Node {{node_id: '{}'}}) DETACH DELETE n",
            escaped_id
        );

        self.cypher_execute(&cypher).await
    }

    async fn node_degree(&self, node_id: &str) -> Result<usize> {
        let escaped_id = Self::escape_cypher_string(node_id);
        let cypher = format!(
            "MATCH (n:Node {{node_id: '{}'}})-[r]-() RETURN count(r)",
            escaped_id
        );

        let count = self.cypher_query_count(&cypher).await?;
        Ok(count as usize)
    }

    async fn get_all_nodes(&self) -> Result<Vec<GraphNode>> {
        let cypher = "MATCH (n:Node) RETURN n";
        let rows = self.cypher_query(cypher, &["n"]).await?;

        let nodes: Vec<GraphNode> = rows
            .iter()
            .filter_map(|row| {
                let json_value: serde_json::Value = row.get("n");
                let agtype_str = json_value.to_string();
                Self::parse_vertex(&agtype_str)
            })
            .collect();

        Ok(nodes)
    }

    async fn get_nodes_by_ids(&self, node_ids: &[String]) -> Result<Vec<GraphNode>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Build list of IDs for Cypher IN clause
        let ids_list: Vec<String> = node_ids
            .iter()
            .map(|id| format!("'{}'", Self::escape_cypher_string(id)))
            .collect();

        let cypher = format!(
            "MATCH (n:Node) WHERE n.node_id IN [{}] RETURN n",
            ids_list.join(", ")
        );

        let rows = self.cypher_query(&cypher, &["n"]).await?;

        let nodes: Vec<GraphNode> = rows
            .iter()
            .filter_map(|row| {
                let json_value: serde_json::Value = row.get("n");
                let agtype_str = json_value.to_string();
                Self::parse_vertex(&agtype_str)
            })
            .collect();

        Ok(nodes)
    }

    async fn has_edge(&self, source: &str, target: &str) -> Result<bool> {
        let escaped_source = Self::escape_cypher_string(source);
        let escaped_target = Self::escape_cypher_string(target);

        let cypher = format!(
            "MATCH (a:Node {{node_id: '{}'}})-[r:EDGE]->(b:Node {{node_id: '{}'}}) RETURN r LIMIT 1",
            escaped_source, escaped_target
        );

        let rows = self.cypher_query(&cypher, &["r"]).await?;
        Ok(!rows.is_empty())
    }

    async fn get_edge(&self, source: &str, target: &str) -> Result<Option<GraphEdge>> {
        let escaped_source = Self::escape_cypher_string(source);
        let escaped_target = Self::escape_cypher_string(target);

        let cypher = format!(
            "MATCH (a:Node {{node_id: '{}'}})-[r:EDGE]->(b:Node {{node_id: '{}'}}) RETURN r",
            escaped_source, escaped_target
        );

        let rows = self.cypher_query(&cypher, &["r"]).await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let json_value: serde_json::Value = rows[0].get("r");
        let agtype_str = json_value.to_string();
        Ok(Self::parse_edge(&agtype_str))
    }

    async fn upsert_edge(
        &self,
        source: &str,
        target: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let escaped_source = Self::escape_cypher_string(source);
        let escaped_target = Self::escape_cypher_string(target);

        // Build properties with source_id and target_id
        let mut props_with_ids = properties.clone();
        props_with_ids.insert(
            "source_id".to_string(),
            serde_json::Value::String(source.to_string()),
        );
        props_with_ids.insert(
            "target_id".to_string(),
            serde_json::Value::String(target.to_string()),
        );
        let props_cypher = Self::properties_to_cypher(&props_with_ids);

        // First ensure both nodes exist
        let create_nodes = format!(
            "MERGE (a:Node {{node_id: '{}'}}) MERGE (b:Node {{node_id: '{}'}})",
            escaped_source, escaped_target
        );
        self.cypher_execute(&create_nodes).await?;

        // Then create/update the edge
        // Use MATCH + DELETE + CREATE pattern for upsert since MERGE on edges can be tricky
        let delete_existing = format!(
            "MATCH (a:Node {{node_id: '{}'}})-[r:EDGE]->(b:Node {{node_id: '{}'}}) DELETE r",
            escaped_source, escaped_target
        );
        let _ = self.cypher_execute(&delete_existing).await; // Ignore if no edge exists

        let create_edge = format!(
            "MATCH (a:Node {{node_id: '{}'}}), (b:Node {{node_id: '{}'}}) CREATE (a)-[r:EDGE {}]->(b)",
            escaped_source, escaped_target, props_cypher
        );
        self.cypher_execute(&create_edge).await
    }

    async fn delete_edge(&self, source: &str, target: &str) -> Result<()> {
        let escaped_source = Self::escape_cypher_string(source);
        let escaped_target = Self::escape_cypher_string(target);

        let cypher = format!(
            "MATCH (a:Node {{node_id: '{}'}})-[r:EDGE]->(b:Node {{node_id: '{}'}}) DELETE r",
            escaped_source, escaped_target
        );

        self.cypher_execute(&cypher).await
    }

    async fn get_node_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>> {
        let escaped_id = Self::escape_cypher_string(node_id);

        // Get both outgoing and incoming edges
        let cypher = format!(
            "MATCH (n:Node {{node_id: '{}'}})-[r:EDGE]-() RETURN r",
            escaped_id
        );

        let rows = self.cypher_query(&cypher, &["r"]).await?;

        let edges: Vec<GraphEdge> = rows
            .iter()
            .filter_map(|row| {
                let json_value: serde_json::Value = row.get("r");
                let agtype_str = json_value.to_string();
                Self::parse_edge(&agtype_str)
            })
            .collect();

        Ok(edges)
    }

    async fn get_all_edges(&self) -> Result<Vec<GraphEdge>> {
        let cypher = "MATCH ()-[r:EDGE]->() RETURN r";
        let rows = self.cypher_query(cypher, &["r"]).await?;

        let edges: Vec<GraphEdge> = rows
            .iter()
            .filter_map(|row| {
                let json_value: serde_json::Value = row.get("r");
                let agtype_str = json_value.to_string();
                Self::parse_edge(&agtype_str)
            })
            .collect();

        Ok(edges)
    }

    async fn get_knowledge_graph(
        &self,
        start_node: &str,
        max_depth: usize,
        max_nodes: usize,
    ) -> Result<KnowledgeGraph> {
        let escaped_id = Self::escape_cypher_string(start_node);

        // Use AGE's variable-length path traversal
        let cypher = format!(
            "MATCH p = (start:Node {{node_id: '{}'}})-[*0..{}]-(connected) \
             RETURN DISTINCT connected LIMIT {}",
            escaped_id, max_depth, max_nodes
        );

        let rows = self.cypher_query(&cypher, &["connected"]).await?;

        let mut kg = KnowledgeGraph::new();
        let mut node_ids: Vec<String> = Vec::new();

        for row in &rows {
            let json_value: serde_json::Value = row.get("connected");
            let agtype_str = json_value.to_string();
            if let Some(node) = Self::parse_vertex(&agtype_str) {
                node_ids.push(node.id.clone());
                kg.add_node(node);
            }
        }

        // Get edges between discovered nodes
        if !node_ids.is_empty() {
            let ids_list: Vec<String> = node_ids
                .iter()
                .map(|id| format!("'{}'", Self::escape_cypher_string(id)))
                .collect();

            let edges_cypher = format!(
                "MATCH (a:Node)-[r:EDGE]->(b:Node) \
                 WHERE a.node_id IN [{}] AND b.node_id IN [{}] \
                 RETURN r",
                ids_list.join(", "),
                ids_list.join(", ")
            );

            let edge_rows = self.cypher_query(&edges_cypher, &["r"]).await?;

            for row in &edge_rows {
                let json_value: serde_json::Value = row.get("r");
                let agtype_str = json_value.to_string();
                if let Some(edge) = Self::parse_edge(&agtype_str) {
                    kg.add_edge(edge);
                }
            }
        }

        kg.is_truncated = kg.node_count() >= max_nodes;

        Ok(kg)
    }

    async fn get_popular_labels(&self, limit: usize) -> Result<Vec<String>> {
        // Get nodes with highest degree using AGE
        // NOTE: AGE 1.6.0 has a bug with ORDER BY on aggregation aliases in Cypher,
        // so we use SQL-level ordering instead
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

        // Use SQL-level ordering since AGE has issues with ORDER BY on aggregation aliases
        let sql = format!(
            "SELECT agtype_to_json(node_id) as node_id FROM ( \
                SELECT * FROM cypher('{}', $$ \
                    MATCH (n:Node)-[r]-() \
                    RETURN n.node_id as node_id, count(r) as degree \
                $$) AS (node_id agtype, degree agtype) \
             ) subq \
             ORDER BY degree DESC \
             LIMIT {}",
            self.graph_name, limit
        );

        let rows = sqlx::query(&sql)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Cypher query failed: {}", e)))?;

        let labels: Vec<String> = rows
            .iter()
            .filter_map(|row| {
                let json_value: serde_json::Value = row.get("node_id");
                let node_id_str = json_value.to_string();
                // Remove quotes from agtype string
                Some(node_id_str.trim_matches('"').to_string())
            })
            .collect();

        Ok(labels)
    }

    async fn search_labels(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        let escaped_query = Self::escape_cypher_string(&query.to_uppercase());

        // Use Cypher's string matching for case-insensitive search
        let cypher = format!(
            "MATCH (n:Node) \
             WHERE toUpper(n.node_id) CONTAINS '{}' \
             RETURN n.node_id \
             LIMIT {}",
            escaped_query, limit
        );

        let rows = self.cypher_query(&cypher, &["node_id"]).await?;

        let labels: Vec<String> = rows
            .iter()
            .filter_map(|row| {
                let json_value: serde_json::Value = row.get("node_id");
                let node_id_str = json_value.to_string();
                Some(node_id_str.trim_matches('"').to_string())
            })
            .collect();

        Ok(labels)
    }

    async fn get_neighbors(&self, node_id: &str, depth: usize) -> Result<Vec<GraphNode>> {
        let escaped_id = Self::escape_cypher_string(node_id);

        // Use variable-length path traversal to get neighbors at specified depth
        let cypher = format!(
            "MATCH (start:Node {{node_id: '{}'}})-[*1..{}]-(neighbor:Node) \
             WHERE neighbor.node_id <> '{}' \
             RETURN DISTINCT neighbor",
            escaped_id, depth, escaped_id
        );

        let rows = self.cypher_query(&cypher, &["neighbor"]).await?;

        let neighbors: Vec<GraphNode> = rows
            .iter()
            .filter_map(|row| {
                let json_value: serde_json::Value = row.get("neighbor");
                let agtype_str = json_value.to_string();
                Self::parse_vertex(&agtype_str)
            })
            .collect();

        Ok(neighbors)
    }

    async fn node_count(&self) -> Result<usize> {
        let cypher = "MATCH (n:Node) RETURN count(n)";
        let count = self.cypher_query_count(cypher).await?;
        Ok(count as usize)
    }

    async fn edge_count(&self) -> Result<usize> {
        let cypher = "MATCH ()-[r:EDGE]->() RETURN count(r)";
        let count = self.cypher_query_count(cypher).await?;
        Ok(count as usize)
    }

    async fn clear(&self) -> Result<()> {
        // Delete all nodes (edges will be deleted automatically with DETACH)
        let cypher = "MATCH (n:Node) DETACH DELETE n";
        self.cypher_execute(cypher).await
    }

    /// Optimized: Get popular nodes with degrees in single query.
    ///
    /// Uses a single Cypher query to return nodes with their connection counts,
    /// eliminating N+1 query patterns.
    async fn get_popular_nodes_with_degree(
        &self,
        limit: usize,
        min_degree: Option<usize>,
        entity_type: Option<&str>,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<(GraphNode, usize)>> {
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

        // Build WHERE conditions
        let mut conditions = Vec::new();

        if let Some(min) = min_degree {
            conditions.push(format!("degree >= {}", min));
        }

        if let Some(et) = entity_type {
            let escaped_et = Self::escape_cypher_string(et);
            conditions.push(format!("n.entity_type = '{}'", escaped_et));
        }

        if let Some(tid) = tenant_id {
            let escaped_tid = Self::escape_cypher_string(tid);
            conditions.push(format!(
                "(n.tenant_id IS NULL OR n.tenant_id = '{}')",
                escaped_tid
            ));
        }

        if let Some(wid) = workspace_id {
            let escaped_wid = Self::escape_cypher_string(wid);
            conditions.push(format!(
                "(n.workspace_id IS NULL OR n.workspace_id = '{}')",
                escaped_wid
            ));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Single optimized query that returns nodes with their degrees
        // Using SQL-level subquery to work around AGE ORDER BY limitations
        let sql = format!(
            "SELECT agtype_to_json(n) as node, degree::bigint as degree FROM ( \
                SELECT * FROM cypher('{}', $$ \
                    MATCH (n:Node) \
                    OPTIONAL MATCH (n)-[r]-() \
                    WITH n, count(r) as degree \
                    {} \
                    RETURN n, degree \
                    ORDER BY degree DESC \
                    LIMIT {} \
                $$) AS (n agtype, degree agtype) \
             ) subq",
            self.graph_name,
            where_clause,
            limit * 2 // Fetch extra to account for filtering
        );

        let rows = sqlx::query(&sql)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Cypher query failed: {}", e)))?;

        let mut results = Vec::with_capacity(limit);

        for row in rows {
            if results.len() >= limit {
                break;
            }

            let json_value: serde_json::Value = row.get("node");
            let agtype_str = json_value.to_string();

            if let Some(node) = Self::parse_vertex(&agtype_str) {
                let degree: i64 = row.get("degree");
                results.push((node, degree as usize));
            }
        }

        Ok(results)
    }

    /// Optimized: Get edges between nodes in a specified set.
    ///
    /// Uses a single Cypher query with WHERE IN clause to fetch only
    /// edges connecting the specified nodes.
    async fn get_edges_for_node_set(
        &self,
        node_ids: &[String],
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<GraphEdge>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Build list of IDs for Cypher IN clause
        let ids_list: Vec<String> = node_ids
            .iter()
            .map(|id| format!("'{}'", Self::escape_cypher_string(id)))
            .collect();
        let ids_str = ids_list.join(", ");

        // Build WHERE conditions for tenant/workspace filtering
        let mut conditions = vec![
            format!("a.node_id IN [{}]", ids_str),
            format!("b.node_id IN [{}]", ids_str),
        ];

        if let Some(tid) = tenant_id {
            let escaped_tid = Self::escape_cypher_string(tid);
            conditions.push(format!(
                "(r.tenant_id IS NULL OR r.tenant_id = '{}')",
                escaped_tid
            ));
        }

        if let Some(wid) = workspace_id {
            let escaped_wid = Self::escape_cypher_string(wid);
            conditions.push(format!(
                "(r.workspace_id IS NULL OR r.workspace_id = '{}')",
                escaped_wid
            ));
        }

        let cypher = format!(
            "MATCH (a:Node)-[r:EDGE]->(b:Node) \
             WHERE {} \
             RETURN r",
            conditions.join(" AND ")
        );

        let rows = self.cypher_query(&cypher, &["r"]).await?;

        let edges: Vec<GraphEdge> = rows
            .iter()
            .filter_map(|row| {
                let json_value: serde_json::Value = row.get("r");
                let agtype_str = json_value.to_string();
                Self::parse_edge(&agtype_str)
            })
            .collect();

        Ok(edges)
    }
}

impl std::fmt::Debug for PostgresAGEGraphStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresAGEGraphStorage")
            .field("namespace", &self.namespace)
            .field("graph_name", &self.graph_name)
            .field("prefix", &self.prefix)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_storage_creation() {
        let config = PostgresConfig::default().with_namespace("test");
        let storage = PostgresAGEGraphStorage::new(config);

        // Graph name includes eq_ prefix from table_prefix() which returns "eq_test"
        // Then format!("eq_{}_graph", prefix) creates "eq_eq_test_graph"
        assert_eq!(storage.graph_name, "eq_eq_test_graph");
        assert_eq!(storage.namespace, "test");
    }

    #[test]
    fn test_escape_cypher_string() {
        assert_eq!(
            PostgresAGEGraphStorage::escape_cypher_string("hello'world"),
            "hello\\'world"
        );
        assert_eq!(
            PostgresAGEGraphStorage::escape_cypher_string("line\nnew"),
            "line\\nnew"
        );
    }

    #[test]
    fn test_properties_to_cypher() {
        let mut props = HashMap::new();
        props.insert("name".to_string(), serde_json::json!("Alice"));
        props.insert("age".to_string(), serde_json::json!(30));
        props.insert("active".to_string(), serde_json::json!(true));

        let cypher = PostgresAGEGraphStorage::properties_to_cypher(&props);

        // Properties order is not guaranteed, so just check for presence
        assert!(cypher.starts_with('{'));
        assert!(cypher.ends_with('}'));
        assert!(cypher.contains("name: 'Alice'"));
        assert!(cypher.contains("age: 30"));
        assert!(cypher.contains("active: true"));
    }

    #[test]
    fn test_parse_vertex() {
        let agtype = r#"{"id": 123, "label": "Node", "properties": {"node_id": "test-1", "name": "Test Node"}}"#;

        let node = PostgresAGEGraphStorage::parse_vertex(agtype);
        assert!(node.is_some());

        let node = node.unwrap();
        assert_eq!(node.id, "test-1");
        assert_eq!(node.properties.get("name").unwrap(), "Test Node");
    }

    #[test]
    fn test_parse_edge() {
        let agtype = r#"{"id": 456, "label": "EDGE", "start_id": 123, "end_id": 789, "properties": {"source_id": "node-1", "target_id": "node-2", "weight": 0.5}}"#;

        let edge = PostgresAGEGraphStorage::parse_edge(agtype);
        assert!(edge.is_some());

        let edge = edge.unwrap();
        assert_eq!(edge.source, "node-1");
        assert_eq!(edge.target, "node-2");
        assert_eq!(edge.properties.get("weight").unwrap(), 0.5);
    }
}
