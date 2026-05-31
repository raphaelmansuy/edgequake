use std::collections::HashMap;
use std::sync::atomic::Ordering;

use sqlx::Row;

use super::PostgresAGEGraphStorage;
use crate::error::{Result, StorageError};
use crate::traits::{GraphEdge, GraphNode};

impl PostgresAGEGraphStorage {
    pub(super) async fn pg_has_node(&self, node_id: &str) -> Result<bool> {
        let escaped_id = Self::escape_cypher_string(node_id);
        let cypher = format!(
            "MATCH (n:Node {{node_id: '{}'}}) RETURN n LIMIT 1",
            escaped_id
        );

        let rows = self.cypher_query(&cypher, &["n"]).await?;
        Ok(!rows.is_empty())
    }

    pub(super) async fn pg_get_node(&self, node_id: &str) -> Result<Option<GraphNode>> {
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
    pub(super) async fn pg_upsert_node(
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

    /// SC1: batched node upsert using a single `UNWIND ... MERGE` per chunk.
    ///
    /// WHY: the default trait impl issues one `cypher()` round trip per node.
    /// For a document yielding hundreds of entities that is hundreds of network
    /// + planner round trips. UNWIND expands an inline list literal into rows
    ///   inside ONE AGE query, so a 500-node batch becomes a single round trip.
    ///
    /// WHY inline literal (not a bound `$param`): AGE's `cypher()` SQL wrapper
    /// does not forward sqlx bind parameters into the Cypher scope, so the list
    /// must be materialized as a Cypher literal. Every value flows through
    /// `value_to_cypher`, which single-quote-escapes strings, so injection via
    /// node ids/properties is neutralized exactly as in the single-node path.
    pub(super) async fn pg_upsert_nodes_batch(
        &self,
        nodes: &[(String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        if nodes.is_empty() {
            return Ok(());
        }

        // WHY chunk: bound the generated query string so a pathological batch
        // cannot blow past PostgreSQL's statement size / planner limits.
        const CHUNK: usize = 500;

        for chunk in nodes.chunks(CHUNK) {
            let rows: Vec<String> = chunk
                .iter()
                .map(|(node_id, properties)| {
                    let mut map = properties.clone();
                    // node_id is the MERGE key; force it into the property map.
                    map.insert(
                        "node_id".to_string(),
                        serde_json::Value::String(node_id.clone()),
                    );
                    Self::properties_to_cypher(&map)
                })
                .collect();

            let cypher = format!(
                "UNWIND [{}] AS props \
                 MERGE (n:Node {{node_id: props.node_id}}) \
                 SET n = props",
                rows.join(", ")
            );
            self.cypher_execute(&cypher).await?;
        }

        // Lazily create indexes after the first successful batch (AGE builds the
        // Node table lazily, mirroring the single-node path).
        if !self.indexes_verified.load(Ordering::Relaxed) {
            self.ensure_indexes().await?;
            self.indexes_verified.store(true, Ordering::Relaxed);
            tracing::info!("Created AGE indexes after first node batch");
        }

        Ok(())
    }

    pub(super) async fn pg_delete_node(&self, node_id: &str) -> Result<()> {
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
    pub(super) async fn pg_node_degree(&self, node_id: &str) -> Result<usize> {
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
    pub(super) async fn pg_node_degrees_batch(
        &self,
        node_ids: &[String],
    ) -> Result<Vec<(String, usize)>> {
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

    pub(super) async fn pg_get_all_nodes(&self) -> Result<Vec<GraphNode>> {
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

    pub(super) async fn pg_get_nodes_by_ids(&self, node_ids: &[String]) -> Result<Vec<GraphNode>> {
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
    pub(super) async fn pg_get_nodes_batch(
        &self,
        node_ids: &[String],
    ) -> Result<HashMap<String, GraphNode>> {
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
    pub(super) async fn pg_get_edges_for_nodes_batch(
        &self,
        node_ids: &[String],
    ) -> Result<Vec<GraphEdge>> {
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
    pub(super) async fn pg_get_nodes_with_degrees_batch(
        &self,
        node_ids: &[String],
    ) -> Result<Vec<(GraphNode, usize, usize)>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }

        // WHY: All graphid comparisons must go via ::text — Apache AGE's graphid type has
        // no registered = operator in the PostgreSQL type system. Direct graphid = graphid
        // comparisons produce "operator does not exist: ag_catalog.graphid = ag_catalog.graphid".
        // The consistent fix (same pattern as node_degree / node_degrees_batch) is to cast
        // every graphid to text before comparing, turning all joins into text = text.
        let sql = format!(
            r#"
            WITH input(v, ord) AS (
              SELECT v, ord FROM unnest($1::text[]) WITH ORDINALITY AS t(v, ord)
            ),
            ids(node_id, ord) AS (
              SELECT (to_json(v)::text)::agtype AS node_id, ord FROM input
            ),
            vids AS (
              SELECT n.id::text AS vid_text, i.node_id, i.ord, n.properties
              FROM {}."Node" AS n
              JOIN ids i ON ag_catalog.agtype_access_operator(
                  VARIADIC ARRAY[n.properties, '"node_id"'::agtype]
              ) = i.node_id
            ),
            deg_out AS (
              SELECT e.start_id::text AS vid_text, COUNT(*)::bigint AS out_degree
              FROM {}."EDGE" AS e
              JOIN vids v ON v.vid_text = e.start_id::text
              GROUP BY e.start_id::text
            ),
            deg_in AS (
              SELECT e.end_id::text AS vid_text, COUNT(*)::bigint AS in_degree
              FROM {}."EDGE" AS e
              JOIN vids v ON v.vid_text = e.end_id::text
              GROUP BY e.end_id::text
            )
            SELECT v.node_id::text AS node_id,
                   ag_catalog.agtype_to_json(v.properties) AS properties,
                   COALESCE(o.out_degree, 0)::bigint AS out_degree,
                   COALESCE(n.in_degree, 0)::bigint AS in_degree
            FROM vids v
            LEFT JOIN deg_out o ON o.vid_text = v.vid_text
            LEFT JOIN deg_in n ON n.vid_text = v.vid_text
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
}
