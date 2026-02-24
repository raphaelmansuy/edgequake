//! PostgreSQL graph storage using Apache AGE extension.
//!
//! This module implements graph storage using Apache AGE (A Graph Extension)
//! for PostgreSQL. AGE provides native graph database capabilities with
//! Cypher query language support.
//!
//! ## Implements
//!
//! - [`FEAT0203`]: Apache AGE graph storage
//! - [`FEAT0310`]: Cypher query language support
//! - [`FEAT0311`]: Variable-length path traversal
//! - [`FEAT0312`]: Multi-tenant graph isolation
//!
//! ## Use Cases
//!
//! - [`UC0602`]: System stores entities and relationships
//! - [`UC0701`]: System traverses knowledge graph
//! - [`UC0702`]: System finds entity relationships
//!
//! ## Enforces
//!
//! - [`BR0203`]: ACID transactions for graph operations
//! - [`BR0310`]: Namespace isolation per tenant
//! - [`BR0311`]: Lazy index creation after first insert
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
/// # WHY: Apache AGE for Graph Storage
///
/// We chose Apache AGE (A Graph Extension) over alternatives:
///
/// 1. **Native PostgreSQL Integration**
///    - WHY: Leverages PostgreSQL's ACID guarantees, replication, and ecosystem
///    - WHY: No separate graph database to manage; uses existing Postgres infra
///
/// 2. **Cypher Query Language**
///    - WHY: Industry-standard graph query language (Neo4j compatible)
///    - WHY: Rich traversal syntax (variable-length paths, pattern matching)
///
/// 3. **Performance Optimizations**
///    - WHY indexes_verified: AGE creates label tables lazily; indexes created on first use
///    - WHY SQL fallback for degree: Cypher OPTIONAL MATCH is 10x slower than native SQL
///    - WHY graphid::text: AGE's graphid type lacks native equality operator
///
/// 4. **Multi-Tenancy via Namespace**
///    - WHY graph_name includes prefix: Each tenant gets isolated graph
///    - WHY namespace tracking: Enables per-tenant vector filtering
///
/// Uses the AGE extension for native graph operations with Cypher queries.
/// All operations use AGE's graph-optimized storage and query engine.
mod helpers;

pub struct PostgresAGEGraphStorage {
    pool: PostgresPool,
    graph_name: String,
    namespace: String,
    prefix: String,
    initialized: AtomicBool,
    /// Track if indexes have been created after first node insertion.
    /// AGE creates label tables lazily on first use, so indexes must be
    /// created after the first node/edge is inserted.
    indexes_verified: AtomicBool,
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
            indexes_verified: AtomicBool::new(false),
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

        // CRITICAL: Create indexes for query performance
        // Without these, Cypher queries like MATCH (n:Node {node_id: 'xxx'}) scan all vertices
        self.ensure_indexes().await?;

        self.initialized.store(true, Ordering::Relaxed);

        tracing::info!(
            "Initialized PostgresAGEGraphStorage with graph '{}' (indexes verified)",
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

    /// Upsert a node into the graph.
    ///
    /// # WHY: MERGE-Based Upsert
    ///
    /// Uses Cypher MERGE instead of separate CREATE/UPDATE:
    /// - Atomic: No race conditions between check and insert
    /// - Idempotent: Safe to retry on network failures
    /// - Efficient: Single round-trip vs two queries
    ///
    /// Also triggers lazy index creation on first node.
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

        self.cypher_execute(&cypher).await?;

        // Ensure indexes exist after first node insertion
        // AGE creates the Node table lazily, so we need to create indexes
        // after the first node is inserted
        if !self.indexes_verified.load(Ordering::Relaxed) {
            self.ensure_indexes().await?;
            self.indexes_verified.store(true, Ordering::Relaxed);
            tracing::info!("Created AGE indexes after first node insertion");
        }

        Ok(())
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

    /// FAST OPTIMIZED: Get node degree using native SQL.
    ///
    /// Uses direct SQL query instead of slow Cypher OPTIONAL MATCH pattern.
    /// This is 10x+ faster as it leverages PostgreSQL's native aggregation and our node_id index.
    /// Counts BOTH incoming and outgoing edges (total degree).
    ///
    /// Performance: <50ms for single node (vs 500ms+ with Cypher approach)
    async fn node_degree(&self, node_id: &str) -> Result<usize> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let escaped_id = Self::escape_sql_string(node_id);

        // WHY: Use ::text cast for graphid comparison - Apache AGE's graphid type
        // lacks a native equality operator, but text comparison works correctly.
        let sql = format!(
            "WITH node_vid AS ( \
                SELECT id::text as id_text FROM {}.\"_ag_label_vertex\" \
                WHERE ag_catalog.agtype_to_json(properties)->>'node_id' = '{}' \
             ), \
             out_edges AS ( \
                SELECT COUNT(*) as cnt FROM {}.\"_ag_label_edge\" e \
                JOIN node_vid n ON e.start_id::text = n.id_text \
             ), \
             in_edges AS ( \
                SELECT COUNT(*) as cnt FROM {}.\"_ag_label_edge\" e \
                JOIN node_vid n ON e.end_id::text = n.id_text \
             ) \
             SELECT COALESCE(o.cnt, 0) + COALESCE(i.cnt, 0) as degree \
             FROM out_edges o, in_edges i",
            self.graph_name, escaped_id, self.graph_name, self.graph_name
        );

        let row = sqlx::query(&sql)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Node degree query failed: {}", e)))?;

        let degree: i64 = row.get("degree");
        Ok(degree as usize)
    }

    /// FAST OPTIMIZED: Get degrees for multiple nodes in a single query.
    ///
    /// Uses SQL IN clause with GROUP BY to calculate all degrees in one query.
    /// This is N times faster than calling node_degree() N times (1 query vs N queries).
    ///
    /// Performance: <100ms for 100 nodes (vs 5000ms+ with N separate queries)
    async fn node_degrees_batch(&self, node_ids: &[String]) -> Result<Vec<(String, usize)>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }

        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        // Build escaped ID list for SQL ANY clause
        // Use SQL escaping (doubling single quotes) not Cypher escaping (backslash)
        let ids_list: Vec<String> = node_ids
            .iter()
            .map(|id| Self::escape_sql_string(id))
            .collect();

        // WHY: Use ::text cast for graphid comparison - Apache AGE's graphid type
        // lacks a native equality operator, but text comparison works correctly.
        let sql = format!(
            "WITH target_nodes AS ( \
                SELECT id::text as id_text, ag_catalog.agtype_to_json(properties)->>'node_id' as node_id \
                FROM {}.\"_ag_label_vertex\" \
                WHERE ag_catalog.agtype_to_json(properties)->>'node_id' IN ({}) \
             ), \
             out_degrees AS ( \
                SELECT n.node_id, COUNT(*) as out_deg \
                FROM {}.\"_ag_label_edge\" e \
                JOIN target_nodes n ON e.start_id::text = n.id_text \
                GROUP BY n.node_id \
             ), \
             in_degrees AS ( \
                SELECT n.node_id, COUNT(*) as in_deg \
                FROM {}.\"_ag_label_edge\" e \
                JOIN target_nodes n ON e.end_id::text = n.id_text \
                GROUP BY n.node_id \
             ) \
             SELECT t.node_id, COALESCE(o.out_deg, 0) + COALESCE(i.in_deg, 0) as degree \
             FROM target_nodes t \
             LEFT JOIN out_degrees o ON o.node_id = t.node_id \
             LEFT JOIN in_degrees i ON i.node_id = t.node_id",
            self.graph_name,
            ids_list
                .iter()
                .map(|id| format!("'{}'", id))
                .collect::<Vec<_>>()
                .join(", "),
            self.graph_name,
            self.graph_name
        );

        // WHY: Truncate SQL for logging, but respect UTF-8 char boundaries.
        // Direct byte slicing (&sql[..500]) can panic if it falls inside a multi-byte character.
        // Instead, take chars up to a safe byte limit.
        let sql_preview = sql.chars().take(500).collect::<String>();
        tracing::debug!(target: "edgequake_storage", "Batch degree SQL: {}", sql_preview);

        let rows = sqlx::query(&sql)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Batch degree query failed: {}", e)))?;

        let mut results = Vec::new();
        let mut found_ids = std::collections::HashSet::new();

        for row in rows {
            let node_id: String = row.get("node_id");
            let degree: i64 = row.get("degree");
            found_ids.insert(node_id.clone());
            results.push((node_id, degree as usize));
        }

        // Add nodes with 0 degree (not in edge_counts CTE)
        for node_id in node_ids {
            if !found_ids.contains(node_id) {
                results.push((node_id.clone(), 0));
            }
        }

        Ok(results)
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

    /// OPTIMIZED: LightRAG-inspired batch node retrieval using UNNEST with ORDINALITY.
    ///
    /// This method uses a single SQL query with array binding to fetch multiple nodes
    /// in O(1) database round-trips, matching LightRAG's performance pattern.
    ///
    /// Performance: ~10ms for 100 nodes (vs ~500ms with individual queries)
    async fn get_nodes_batch(&self, node_ids: &[String]) -> Result<HashMap<String, GraphNode>> {
        if node_ids.is_empty() {
            return Ok(HashMap::new());
        }

        // Use direct SQL with UNNEST for batch parameter binding (LightRAG pattern)
        let sql = format!(
            r#"
            WITH input(v, ord) AS (
              SELECT v, ord FROM unnest($1::text[]) WITH ORDINALITY AS t(v, ord)
            ),
            ids(node_id, ord) AS (
              SELECT (to_json(v)::text)::agtype AS node_id, ord FROM input
            )
            SELECT i.node_id::text AS node_id,
                   ag_catalog.agtype_to_json(n.properties) AS properties
            FROM {}."Node" AS n
            JOIN ids i ON ag_catalog.agtype_access_operator(
                VARIADIC ARRAY[n.properties, '"node_id"'::agtype]
            ) = i.node_id
            ORDER BY i.ord
            "#,
            self.graph_name
        );

        let rows = self.batch_sql_query(&sql, node_ids).await?;

        let mut result = HashMap::new();
        for row in rows {
            let raw_node_id: String = row.get("node_id");
            // Remove surrounding quotes from agtype string conversion
            let node_id = raw_node_id.trim_matches('"').to_string();
            let props_json: serde_json::Value = row.get("properties");

            if let Some(node) = Self::parse_properties_to_node(&node_id, &props_json) {
                result.insert(node_id, node);
            }
        }

        Ok(result)
    }

    /// OPTIMIZED: LightRAG-inspired batch edge retrieval for node set.
    ///
    /// Gets all edges where BOTH endpoints are in the specified node set.
    /// Uses JOINs instead of fetch-all-then-filter pattern.
    ///
    /// Performance: Single query for any number of nodes
    async fn get_edges_for_nodes_batch(&self, node_ids: &[String]) -> Result<Vec<GraphEdge>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Use direct SQL with UNNEST for batch parameter binding
        let sql = format!(
            r#"
            WITH input(v, ord) AS (
              SELECT v, ord FROM unnest($1::text[]) WITH ORDINALITY AS t(v, ord)
            ),
            ids(node_id, ord) AS (
              SELECT (to_json(v)::text)::agtype AS node_id, ord FROM input
            ),
            vids AS (
              SELECT n.id AS vid, i.node_id
              FROM {}."Node" AS n
              JOIN ids i ON ag_catalog.agtype_access_operator(
                  VARIADIC ARRAY[n.properties, '"node_id"'::agtype]
              ) = i.node_id
            )
            SELECT ag_catalog.agtype_to_json(e.properties) AS properties,
                   src.node_id::text AS source_id,
                   tgt.node_id::text AS target_id
            FROM {}."EDGE" AS e
            JOIN vids src ON src.vid = e.start_id
            JOIN vids tgt ON tgt.vid = e.end_id
            "#,
            self.graph_name, self.graph_name
        );

        let rows = self.batch_sql_query(&sql, node_ids).await?;

        let mut edges = Vec::new();
        for row in rows {
            let raw_source: String = row.get("source_id");
            let raw_target: String = row.get("target_id");
            // Remove surrounding quotes from agtype string conversion
            let source = raw_source.trim_matches('"').to_string();
            let target = raw_target.trim_matches('"').to_string();
            let props_json: serde_json::Value = row.get("properties");

            let properties = Self::parse_json_to_properties(&props_json);
            edges.push(GraphEdge {
                source,
                target,
                properties,
            });
        }

        Ok(edges)
    }

    /// OPTIMIZED: LightRAG-inspired batch degree calculation.
    ///
    /// Calculates in-degree and out-degree for multiple nodes in a single query.
    /// Returns total degree (in + out) for each node.
    ///
    /// Performance: Single query for any number of nodes
    async fn get_nodes_with_degrees_batch(
        &self,
        node_ids: &[String],
    ) -> Result<Vec<(GraphNode, usize, usize)>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Combined query for nodes and their degrees
        let sql = format!(
            r#"
            WITH input(v, ord) AS (
              SELECT v, ord FROM unnest($1::text[]) WITH ORDINALITY AS t(v, ord)
            ),
            ids(node_id, ord) AS (
              SELECT (to_json(v)::text)::agtype AS node_id, ord FROM input
            ),
            vids AS (
              SELECT n.id AS vid, i.node_id, i.ord, n.properties
              FROM {}."Node" AS n
              JOIN ids i ON ag_catalog.agtype_access_operator(
                  VARIADIC ARRAY[n.properties, '"node_id"'::agtype]
              ) = i.node_id
            ),
            deg_out AS (
              SELECT e.start_id AS vid, COUNT(*)::bigint AS out_degree
              FROM {}."EDGE" AS e
              JOIN vids v ON v.vid = e.start_id
              GROUP BY e.start_id
            ),
            deg_in AS (
              SELECT e.end_id AS vid, COUNT(*)::bigint AS in_degree
              FROM {}."EDGE" AS e
              JOIN vids v ON v.vid = e.end_id
              GROUP BY e.end_id
            )
            SELECT v.node_id::text AS node_id,
                   ag_catalog.agtype_to_json(v.properties) AS properties,
                   COALESCE(o.out_degree, 0)::bigint AS out_degree,
                   COALESCE(n.in_degree, 0)::bigint AS in_degree
            FROM vids v
            LEFT JOIN deg_out o ON o.vid = v.vid
            LEFT JOIN deg_in n ON n.vid = v.vid
            ORDER BY v.ord
            "#,
            self.graph_name, self.graph_name, self.graph_name
        );

        let rows = self.batch_sql_query(&sql, node_ids).await?;

        let mut result = Vec::new();
        for row in rows {
            let raw_node_id: String = row.get("node_id");
            let node_id = raw_node_id.trim_matches('"').to_string();
            let props_json: serde_json::Value = row.get("properties");
            let out_degree: i64 = row.get("out_degree");
            let in_degree: i64 = row.get("in_degree");

            if let Some(node) = Self::parse_properties_to_node(&node_id, &props_json) {
                result.push((node, in_degree as usize, out_degree as usize));
            }
        }

        Ok(result)
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
            .map(|row| {
                let json_value: serde_json::Value = row.get("node_id");
                let node_id_str = json_value.to_string();
                // Remove quotes from agtype string
                node_id_str.trim_matches('"').to_string()
            })
            .collect();

        Ok(labels)
    }

    /// FAST OPTIMIZED: Search node labels with full-text search and fuzzy matching.
    ///
    /// Uses PostgreSQL's full-text search (ts_vector) and trigram similarity (pg_trgm).
    /// Supports fuzzy matching, ranking by relevance, and handles typos.
    ///
    /// Performance: <100ms for fuzzy search across 10k+ nodes
    async fn search_labels(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let escaped_query = Self::escape_sql_string(query);
        tracing::debug!(query = %query, escaped = %escaped_query, "search_labels starting");

        // Try full-text search first (best for word matching)
        let fts_sql = format!(
            "SELECT \
                ag_catalog.agtype_to_json(properties)->>'node_id' as label, \
                ts_rank( \
                    to_tsvector('english', ag_catalog.agtype_to_json(properties)->>'node_id'), \
                    plainto_tsquery('english', '{}') \
                ) as rank \
             FROM {}.\"_ag_label_vertex\" \
             WHERE to_tsvector('english', ag_catalog.agtype_to_json(properties)->>'node_id') \
                   @@ plainto_tsquery('english', '{}') \
             ORDER BY rank DESC \
             LIMIT {}",
            escaped_query, self.graph_name, escaped_query, limit
        );

        let fts_rows = sqlx::query(&fts_sql).fetch_all(&mut *conn).await;

        // If full-text search finds results, return them
        if let Ok(rows) = fts_rows {
            if !rows.is_empty() {
                let labels: Vec<String> = rows
                    .iter()
                    .filter_map(|row| row.get::<Option<String>, _>("label"))
                    .collect();

                if !labels.is_empty() {
                    return Ok(labels);
                }
            }
        }

        // WHY: Fallback to trigram similarity for fuzzy matching (typos, partial matches)
        // WHY: pg_trgm extension is in ag_catalog schema, so we must use OPERATOR(ag_catalog.%)
        //      and ag_catalog.similarity() explicitly to avoid "function not found" errors
        let trgm_sql = format!(
            "SELECT \
                ag_catalog.agtype_to_json(properties)->>'node_id' as label, \
                ag_catalog.similarity( \
                    ag_catalog.agtype_to_json(properties)->>'node_id', \
                    '{}' \
                ) as sim \
             FROM {}.\"_ag_label_vertex\" \
             WHERE ag_catalog.agtype_to_json(properties)->>'node_id' OPERATOR(ag_catalog.%) '{}' \
             ORDER BY sim DESC \
             LIMIT {}",
            escaped_query, self.graph_name, escaped_query, limit
        );

        let trgm_rows = sqlx::query(&trgm_sql).fetch_all(&mut *conn).await;
        tracing::debug!(sql = %trgm_sql, result = ?trgm_rows.as_ref().map(|r| r.len()).unwrap_or(0), "trigram search");

        // If trigram search finds results, return them
        if let Ok(rows) = trgm_rows {
            if !rows.is_empty() {
                let labels: Vec<String> = rows
                    .iter()
                    .filter_map(|row| row.get::<Option<String>, _>("label"))
                    .collect();
                tracing::debug!(labels = ?labels, "trigram search found labels");

                if !labels.is_empty() {
                    return Ok(labels);
                }
            }
        }

        // Final fallback to simple ILIKE prefix matching (always works)
        let prefix_sql = format!(
            "SELECT ag_catalog.agtype_to_json(properties)->>'node_id' as label \
             FROM {}.\"_ag_label_vertex\" \
             WHERE LOWER(ag_catalog.agtype_to_json(properties)->>'node_id') LIKE LOWER('{}%') \
             ORDER BY ag_catalog.agtype_to_json(properties)->>'node_id' \
             LIMIT {}",
            self.graph_name, escaped_query, limit
        );

        let prefix_rows = sqlx::query(&prefix_sql)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Search labels query failed: {}", e)))?;

        let labels: Vec<String> = prefix_rows
            .iter()
            .filter_map(|row| row.get::<Option<String>, _>("label"))
            .collect();

        Ok(labels)
    }

    /// Search for nodes with full text matching on label and description.
    ///
    /// Returns nodes with their degree, filtered by tenant/workspace context.
    /// Uses a combination of full-text search and ILIKE for best coverage.
    async fn search_nodes(
        &self,
        query: &str,
        limit: usize,
        entity_type: Option<&str>,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<(GraphNode, usize)>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let query_lower = query.to_lowercase();
        tracing::debug!(query = %query, "search_nodes starting");

        // Build WHERE conditions for tenant/workspace filtering
        let mut where_conditions = vec![format!(
            "(LOWER(ag_catalog.agtype_to_json(v.properties)->>'node_id') LIKE '%{}%' \
             OR LOWER(COALESCE(ag_catalog.agtype_to_json(v.properties)->>'description', '')) LIKE '%{}%')",
            query_lower, query_lower
        )];

        if let Some(tid) = tenant_id {
            let escaped_tid = Self::escape_sql_string(tid);
            where_conditions.push(format!(
                "ag_catalog.agtype_to_json(v.properties)->>'tenant_id' = '{}'",
                escaped_tid
            ));
        }

        if let Some(wid) = workspace_id {
            let escaped_wid = Self::escape_sql_string(wid);
            where_conditions.push(format!(
                "ag_catalog.agtype_to_json(v.properties)->>'workspace_id' = '{}'",
                escaped_wid
            ));
        }

        if let Some(etype) = entity_type {
            let escaped_etype = Self::escape_sql_string(etype);
            where_conditions.push(format!(
                "ag_catalog.agtype_to_json(v.properties)->>'entity_type' = '{}'",
                escaped_etype
            ));
        }

        let where_clause = where_conditions.join(" AND ");

        // CTE query to get nodes with degree count in one query
        let sql = format!(
            "WITH node_props AS (
                SELECT 
                    v.id as vertex_id,
                    ag_catalog.agtype_to_json(v.properties) as props
                FROM {graph}.\"_ag_label_vertex\" v
                WHERE {where_clause}
            ),
            edge_counts AS (
                SELECT 
                    e.start_id as node_id,
                    COUNT(*) as out_degree
                FROM {graph}.\"_ag_label_edge\" e
                GROUP BY e.start_id
            ),
            in_edge_counts AS (
                SELECT 
                    e.end_id as node_id,
                    COUNT(*) as in_degree
                FROM {graph}.\"_ag_label_edge\" e
                GROUP BY e.end_id
            )
            SELECT 
                np.props,
                COALESCE(ec.out_degree, 0) + COALESCE(ic.in_degree, 0) as degree
            FROM node_props np
            LEFT JOIN edge_counts ec ON np.vertex_id = ec.node_id
            LEFT JOIN in_edge_counts ic ON np.vertex_id = ic.node_id
            ORDER BY degree DESC
            LIMIT {limit}",
            graph = self.graph_name,
            where_clause = where_clause,
            limit = limit
        );

        tracing::debug!(sql = %sql, "search_nodes SQL");

        let rows = sqlx::query(&sql)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Search nodes query failed: {}", e)))?;

        let results: Vec<(GraphNode, usize)> = rows
            .iter()
            .filter_map(|row| {
                let props: serde_json::Value = row.get("props");
                let degree: i64 = row.get("degree");

                // Extract node_id from properties
                let node_id = props.get("node_id")?.as_str()?.to_string();

                let node = GraphNode {
                    id: node_id,
                    properties: props.as_object()?.clone().into_iter().collect(),
                };

                Some((node, degree as usize))
            })
            .collect();

        tracing::debug!(results_count = results.len(), "search_nodes completed");
        Ok(results)
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

    /// Get node count for a specific workspace (OODA-03: Fix dashboard stats).
    ///
    /// WHY: Dashboard was showing 0 entities because it only checked PostgreSQL
    /// tables (empty) and KV metadata (no entity_count field). The actual data
    /// is in Apache AGE graph storage.
    ///
    /// This method uses the same property-based filtering pattern as clear_workspace()
    /// for consistency with existing workspace isolation logic.
    async fn node_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
        let workspace_id_str = workspace_id.to_string();
        let escaped_wid = Self::escape_sql_string(&workspace_id_str);
        let cypher = format!(
            "MATCH (n:Node) WHERE n.workspace_id = '{}' RETURN count(n)",
            escaped_wid
        );
        let count = self.cypher_query_count(&cypher).await?;
        Ok(count as usize)
    }

    /// Get edge count for a specific workspace (OODA-03: Fix dashboard stats).
    ///
    /// WHY: Counts edges where either endpoint belongs to the workspace.
    /// This matches the deletion logic in clear_workspace() for consistency.
    async fn edge_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
        let workspace_id_str = workspace_id.to_string();
        let escaped_wid = Self::escape_sql_string(&workspace_id_str);
        let cypher = format!(
            "MATCH (n:Node)-[r:EDGE]->(m:Node) WHERE n.workspace_id = '{}' OR m.workspace_id = '{}' RETURN count(r)",
            escaped_wid, escaped_wid
        );
        let count = self.cypher_query_count(&cypher).await?;
        Ok(count as usize)
    }

    /// Get distinct entity type count for a workspace using Cypher DISTINCT.
    ///
    /// WHY: Eliminates the O(N) fetch-all-nodes pattern that made the dashboard
    /// EntityTypes KPI card extremely slow (8000+ nodes transferred over the
    /// network just to count unique types). A single aggregate query brings
    /// this down to milliseconds.
    async fn distinct_node_type_count_by_workspace(
        &self,
        workspace_id: &uuid::Uuid,
    ) -> Result<usize> {
        let workspace_id_str = workspace_id.to_string();
        let escaped_wid = Self::escape_sql_string(&workspace_id_str);

        // Cypher: collect distinct entity_type values, then count them.
        // We use collect + size because AGE's Cypher doesn't support
        // COUNT(DISTINCT n.entity_type) directly in all versions.
        let cypher = format!(
            "MATCH (n:Node) WHERE n.workspace_id = '{}' AND n.entity_type IS NOT NULL \
             WITH collect(DISTINCT n.entity_type) AS types \
             RETURN size(types)",
            escaped_wid
        );

        let count = self.cypher_query_count(&cypher).await.unwrap_or(0);
        Ok(count as usize)
    }

    async fn clear(&self) -> Result<()> {
        // Delete all nodes (edges will be deleted automatically with DETACH)
        let cypher = "MATCH (n:Node) DETACH DELETE n";
        self.cypher_execute(cypher).await
    }

    /// Clear nodes and edges for a specific workspace.
    ///
    /// Uses workspace_id property filtering to delete only data
    /// belonging to the specified workspace. Edges connected to
    /// deleted nodes are automatically removed via DETACH DELETE.
    ///
    /// Returns (nodes_deleted, edges_deleted).
    async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> Result<(usize, usize)> {
        let pool = self.pool.get().await?;

        // Acquire a dedicated connection so AGE session state persists
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        // OODA-224: CRITICAL - Must load AGE extension and set search path before
        // using any AGE functions like ag_catalog.cypher or agtype
        sqlx::query("LOAD 'age'")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to load AGE: {}", e)))?;

        sqlx::query("SET search_path = ag_catalog, \"$user\", public")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set AGE search path: {}", e)))?;

        // First, count nodes/edges that will be deleted
        let workspace_id_str = workspace_id.to_string();
        let escaped_wid = Self::escape_sql_string(&workspace_id_str);

        // Count nodes before deletion
        let count_cypher = format!(
            "MATCH (n:Node) WHERE n.workspace_id = '{}' RETURN count(n)",
            escaped_wid
        );
        let node_count = self.cypher_query_count(&count_cypher).await.unwrap_or(0) as usize;

        // Count edges before deletion (edges where either endpoint belongs to workspace)
        let edge_count_cypher = format!(
            "MATCH (n:Node)-[r:EDGE]->(m:Node) WHERE n.workspace_id = '{}' OR m.workspace_id = '{}' RETURN count(r)",
            escaped_wid, escaped_wid
        );
        let edge_count = self
            .cypher_query_count(&edge_count_cypher)
            .await
            .unwrap_or(0) as usize;

        // Delete nodes with DETACH (automatically removes connected edges)
        let delete_cypher = format!(
            "MATCH (n:Node) WHERE n.workspace_id = '{}' DETACH DELETE n",
            escaped_wid
        );

        // Execute deletion using the AGE-enabled connection
        let cypher_query = format!(
            "SELECT * FROM cypher('{}', $$ {} $$) AS (result agtype)",
            self.graph_name, delete_cypher
        );

        sqlx::query(&cypher_query)
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to clear workspace: {}", e)))?;

        tracing::info!(
            workspace_id = %workspace_id,
            nodes_deleted = node_count,
            edges_deleted = edge_count,
            "Cleared workspace from graph storage"
        );

        Ok((node_count, edge_count))
    }

    /// FAST OPTIMIZED: Get popular nodes with degrees using native SQL.
    ///
    /// Uses direct SQL with CTE for relationship counting instead of slow Cypher OPTIONAL MATCH.
    /// This is 10x+ faster as it leverages PostgreSQL's native aggregation and our indexes.
    ///
    /// Performance: <500ms (vs 4s+ timeout with Cypher approach)
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

        // Build WHERE conditions for property filtering (using our indexes!)
        let mut where_conditions = Vec::new();

        if let Some(et) = entity_type {
            let escaped_et = Self::escape_sql_string(et);
            where_conditions.push(format!(
                "ag_catalog.agtype_to_json(v.properties)->>'entity_type' = '{}'",
                escaped_et
            ));
        }

        // WHY: Strict multi-tenant filtering - only include nodes with MATCHING tenant_id
        // Nodes without tenant_id are EXCLUDED to prevent cross-tenant data leakage
        if let Some(tid) = tenant_id {
            let escaped_tid = Self::escape_sql_string(tid);
            where_conditions.push(format!(
                "ag_catalog.agtype_to_json(v.properties)->>'tenant_id' = '{}'",
                escaped_tid
            ));
        }

        // WHY: Strict workspace filtering - only include nodes with MATCHING workspace_id
        // Nodes without workspace_id are EXCLUDED to prevent cross-workspace data leakage
        if let Some(wid) = workspace_id {
            let escaped_wid = Self::escape_sql_string(wid);
            where_conditions.push(format!(
                "ag_catalog.agtype_to_json(v.properties)->>'workspace_id' = '{}'",
                escaped_wid
            ));
        }

        let where_clause = if where_conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_conditions.join(" AND "))
        };

        // FAST SQL query using CTE for degree calculation
        // This avoids expensive Cypher OPTIONAL MATCH and uses native SQL GROUP BY
        // Note: Cast graphid to text for comparison - AGE's graphid type doesn't have direct = operator
        let min_degree_filter = if let Some(min) = min_degree {
            format!("AND degree >= {}", min)
        } else {
            String::new()
        };

        let sql = format!(
            "WITH edge_counts AS ( \
                SELECT \
                    ag_catalog.graphid_to_agtype(start_id)::text as start_id_text, \
                    COUNT(*) as out_degree \
                FROM {}.\"_ag_label_edge\" \
                GROUP BY ag_catalog.graphid_to_agtype(start_id)::text \
            ), \
            node_degrees AS ( \
                SELECT \
                    v.id, \
                    v.properties, \
                    COALESCE(ec.out_degree, 0) as degree \
                FROM {}.\"_ag_label_vertex\" v \
                LEFT JOIN edge_counts ec ON ag_catalog.graphid_to_agtype(v.id)::text = ec.start_id_text \
                {} \
            ) \
            SELECT \
                ag_catalog.agtype_to_json(properties) as node_props, \
                degree \
            FROM node_degrees \
            WHERE degree >= 0 {} \
            ORDER BY degree DESC \
            LIMIT {}",
            self.graph_name, self.graph_name, where_clause, min_degree_filter, limit
        );

        let rows = sqlx::query(&sql)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Optimized SQL query failed: {}", e)))?;

        let mut results = Vec::with_capacity(limit);

        for row in rows {
            let json_value: serde_json::Value = row.get("node_props");
            let degree: i64 = row.get("degree");

            // Parse node properties
            if let Ok(properties_map) =
                serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(json_value)
            {
                // Convert Map to HashMap
                let properties: HashMap<String, serde_json::Value> =
                    properties_map.into_iter().collect();

                let node = GraphNode {
                    id: properties
                        .get("node_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    properties,
                };
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
