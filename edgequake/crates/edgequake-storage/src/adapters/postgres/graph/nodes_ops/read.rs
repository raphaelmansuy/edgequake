//! Node read ops — GraphStorageReadOps surface (SPEC-054 ISP).

use std::collections::HashMap;

use sqlx::Row;

use super::super::PostgresAGEGraphStorage;
use crate::error::{Result, StorageError};
use crate::traits::{GraphEdge, GraphNode};

impl PostgresAGEGraphStorage {
    pub(in crate::adapters::postgres::graph) async fn pg_has_node(&self, node_id: &str) -> Result<bool> {
        let cypher = "MATCH (n:Node {node_id: $node_id}) RETURN n LIMIT 1";
        let params = serde_json::json!({ "node_id": node_id });
        let rows = self.cypher_query_bound(cypher, &["n"], &params).await?;
        Ok(!rows.is_empty())
    }


    pub(in crate::adapters::postgres::graph) async fn pg_get_node(&self, node_id: &str) -> Result<Option<GraphNode>> {
        let cypher = "MATCH (n:Node {node_id: $node_id}) RETURN n";
        let params = serde_json::json!({ "node_id": node_id });
        let rows = self.cypher_query_bound(cypher, &["n"], &params).await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let json_value: serde_json::Value = rows[0].get("n");
        let agtype_str = json_value.to_string();
        Ok(Self::parse_vertex(&agtype_str))
    }


    /// FAST OPTIMIZED: Get node degree using native SQL.
    ///
    /// Uses direct SQL query instead of slow Cypher OPTIONAL MATCH pattern.
    /// This is 10x+ faster as it leverages PostgreSQL's native aggregation and our node_id index.
    /// Counts BOTH incoming and outgoing edges (total degree).
    ///
    /// Performance: <50ms for single node (vs 500ms+ with Cypher approach)
    pub(in crate::adapters::postgres::graph) async fn pg_node_degree(&self, node_id: &str) -> Result<usize> {
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
    /// SPEC-053 HARDENED: Batch degree via "EDGE" property indexes (O(k log E) not O(V+E)).
    ///
    /// # WHY: Replaced parent-table query with indexed property scan
    ///
    /// The previous implementation looked up graphids from `_ag_label_vertex`
    /// (parent table — no indexes after M070) then joined `_ag_label_edge`
    /// (same — all parent-table indexes dropped). This was O(V + E) per call.
    ///
    /// Edge properties already store `source_id` / `target_id` (set by every upsert
    /// path). The `"EDGE"` child table has btree expression indexes on those columns:
    ///   - `idx_edge_source_id`  ON "EDGE" ((agtype_to_json(properties)->>'source_id'))
    ///   - `idx_edge_target_id`  ON "EDGE" ((agtype_to_json(properties)->>'target_id'))
    ///
    /// The new query counts edges from the indexed child table only. A VALUES CTE
    /// preserves all input node IDs so that nodes with no edges return degree 0.
    pub(in crate::adapters::postgres::graph) async fn pg_node_degrees_batch(
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

        // SQL escaping (double single-quotes) for inline literals.
        let escaped: Vec<String> = node_ids
            .iter()
            .map(|id| Self::escape_sql_string(id))
            .collect();

        let in_list = escaped
            .iter()
            .map(|id| format!("'{}'", id))
            .collect::<Vec<_>>()
            .join(", ");

        // VALUES list for the input CTE: ('id1'), ('id2'), …
        // This preserves degree-0 nodes via LEFT JOIN without a vertex-table lookup.
        let values_list = escaped
            .iter()
            .map(|id| format!("('{}')", id))
            .collect::<Vec<_>>()
            .join(", ");

        // WHY "EDGE" child table (not "_ag_label_edge" parent):
        //   idx_edge_source_id / idx_edge_target_id are defined on the child.
        //   M070 dropped all parent-table indexes — querying the parent forces
        //   a sequential scan.
        // WHY VALUES CTE (not "_ag_label_vertex" lookup):
        //   We don't need graphids; edge properties already hold the text node_id.
        //   The VALUES CTE is a constant — zero DB I/O — and lets LEFT JOIN return
        //   degree 0 for isolated nodes without a second table scan.
        let sql = format!(
            "WITH input(node_id) AS ( VALUES {values_list} ), \
             out_deg AS ( \
               SELECT ag_catalog.agtype_to_json(e.properties)->>'source_id' AS node_id, \
                      COUNT(*)::bigint AS cnt \
               FROM {graph}.\"EDGE\" e \
               WHERE ag_catalog.agtype_to_json(e.properties)->>'source_id' IN ({in_list}) \
               GROUP BY node_id \
             ), \
             in_deg AS ( \
               SELECT ag_catalog.agtype_to_json(e.properties)->>'target_id' AS node_id, \
                      COUNT(*)::bigint AS cnt \
               FROM {graph}.\"EDGE\" e \
               WHERE ag_catalog.agtype_to_json(e.properties)->>'target_id' IN ({in_list}) \
               GROUP BY node_id \
             ) \
             SELECT i.node_id, \
                    COALESCE(o.cnt, 0) + COALESCE(d.cnt, 0) AS degree \
             FROM input i \
             LEFT JOIN out_deg o ON o.node_id = i.node_id \
             LEFT JOIN in_deg  d ON d.node_id = i.node_id",
            values_list = values_list,
            graph = self.graph_name,
            in_list = in_list
        );

        tracing::debug!(
            target: "edgequake_storage",
            node_count = node_ids.len(),
            "Batch degree SQL (SPEC-053 indexed): {}",
            sql.chars().take(300).collect::<String>()
        );

        let rows = sqlx::query(&sql)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Batch degree query failed: {}", e)))?;

        let results = rows
            .iter()
            .map(|row| {
                let node_id: String = row.get("node_id");
                let degree: i64 = row.get("degree");
                (node_id, degree as usize)
            })
            .collect();

        Ok(results)
    }


    pub(in crate::adapters::postgres::graph) async fn pg_get_all_nodes(&self) -> Result<Vec<GraphNode>> {
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


    pub(in crate::adapters::postgres::graph) async fn pg_get_nodes_by_ids(&self, node_ids: &[String]) -> Result<Vec<GraphNode>> {
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
    pub(in crate::adapters::postgres::graph) async fn pg_get_nodes_batch(
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
    pub(in crate::adapters::postgres::graph) async fn pg_get_edges_for_nodes_batch(
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
    pub(in crate::adapters::postgres::graph) async fn pg_get_nodes_with_degrees_batch(
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
