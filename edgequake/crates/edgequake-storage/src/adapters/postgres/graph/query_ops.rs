use std::collections::HashMap;

use sqlx::Row;

use super::PostgresAGEGraphStorage;
use crate::error::{Result, StorageError};
use crate::traits::{GraphEdge, GraphNode, KnowledgeGraph};

impl PostgresAGEGraphStorage {
    pub(super) async fn pg_get_knowledge_graph(
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

    pub(super) async fn pg_get_popular_labels(&self, limit: usize) -> Result<Vec<String>> {
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
    pub(super) async fn pg_search_labels(&self, query: &str, limit: usize) -> Result<Vec<String>> {
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
    pub(super) async fn pg_search_nodes(
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

    pub(super) async fn pg_get_neighbors(
        &self,
        node_id: &str,
        depth: usize,
    ) -> Result<Vec<GraphNode>> {
        let escaped_id = Self::escape_cypher_string(node_id);

        // QW4: clamp traversal depth to a hard ceiling of 3 hops. Variable-length
        // path expansion in AGE is combinatorial — each extra hop multiplies the
        // intermediate row set by the average degree, so an unbounded `depth`
        // (e.g. a caller passing usize::MAX) can OOM the backend or hang the
        // connection. 3 hops is the practical limit for "related context" in
        // graph-RAG retrieval; anything deeper is noise.
        let safe_depth = depth.clamp(1, 3);

        // QW4: cap the neighbor result set. Hub nodes (high-degree entities like
        // a country or a common topic) can expand to tens of thousands of
        // neighbors; without a LIMIT we'd buffer all of them into memory and
        // flood downstream ranking. 500 is generous for context assembly.
        const MAX_NEIGHBORS: usize = 500;

        // Use variable-length path traversal to get neighbors at specified depth
        let cypher = format!(
            "MATCH (start:Node {{node_id: '{}'}})-[*1..{}]-(neighbor:Node) \
             WHERE neighbor.node_id <> '{}' \
             RETURN DISTINCT neighbor \
             LIMIT {}",
            escaped_id, safe_depth, escaped_id, MAX_NEIGHBORS
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

    pub(super) async fn pg_get_popular_nodes_with_degree(
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

        // WHY: graphid::text::bigint is unreliable across AGE versions — if the graphid
        // output format is not a bare integer string, the ::bigint cast fails.
        // The consistent pattern (same as node_degree / node_degrees_batch) is to cast
        // graphid to text and join on text = text, which AGE always supports.
        let sql = format!(
            "WITH edge_counts AS ( \
                SELECT \
                    start_id::text AS start_id_text, \
                    COUNT(*) as out_degree \
                FROM {}.\"_ag_label_edge\" \
                GROUP BY start_id::text \
            ), \
            node_degrees AS ( \
                SELECT \
                    v.id::text AS id_text, \
                    v.properties, \
                    COALESCE(ec.out_degree, 0) as degree \
                FROM {}.\"_ag_label_vertex\" v \
                LEFT JOIN edge_counts ec ON v.id::text = ec.start_id_text \
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

    /// FAST OPTIMIZED: Get edges between nodes in a specified set using native SQL.
    ///
    /// # WHY: Replace Cypher with native SQL (9s → <200ms)
    ///
    /// The previous Cypher `MATCH (a:Node)-[r:EDGE]->(b:Node) WHERE a.node_id IN [...]`
    /// required AGE to traverse the full vertex table twice (once per endpoint) even with
    /// expression indexes, because the AGE query planner does not push SQL indexes into
    /// Cypher IN-list evaluations for large node sets.
    ///
    /// The native SQL approach directly queries `_ag_label_edge` properties, which stores
    /// `source_id` and `target_id` as top-level properties. With expression indexes on
    /// these fields (migration 036), this becomes an indexed ANY($) lookup.
    pub(super) async fn pg_get_edges_for_node_set(
        &self,
        node_ids: &[String],
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<GraphEdge>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }

        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        // WHY: Build SQL IN clause using escaped string literals to avoid the AGE Cypher
        // overhead. This is the same pattern as `get_popular_nodes_with_degree` — native
        // SQL with direct table access. `escape_sql_string` uses '' (not \') for safety.
        let ids_list: Vec<String> = node_ids
            .iter()
            .map(|id| format!("'{}'", Self::escape_sql_string(id)))
            .collect();
        let ids_str = ids_list.join(", ");

        // WHY: Tenant/workspace filters use IS NULL OR = pattern for backward compatibility
        // with edges that were written before multi-tenancy was enforced. New edges always
        // have these fields set, but old edges may have NULL values.
        let mut extra_filters = Vec::new();
        if let Some(tid) = tenant_id {
            let escaped_tid = Self::escape_sql_string(tid);
            extra_filters.push(format!(
                "(ag_catalog.agtype_to_json(properties)->>'tenant_id' IS NULL \
                 OR ag_catalog.agtype_to_json(properties)->>'tenant_id' = '{}')",
                escaped_tid
            ));
        }
        if let Some(wid) = workspace_id {
            let escaped_wid = Self::escape_sql_string(wid);
            extra_filters.push(format!(
                "(ag_catalog.agtype_to_json(properties)->>'workspace_id' IS NULL \
                 OR ag_catalog.agtype_to_json(properties)->>'workspace_id' = '{}')",
                escaped_wid
            ));
        }

        let extra_where = if extra_filters.is_empty() {
            String::new()
        } else {
            format!(" AND {}", extra_filters.join(" AND "))
        };

        // Native SQL: filter on edge properties directly.
        // `source_id` and `target_id` are stored in edge properties (not vertex joins needed).
        // Migration 036 adds expression indexes on these properties for fast lookups.
        let sql = format!(
            r#"SELECT ag_catalog.agtype_to_json(properties) AS edge_props
               FROM {}."_ag_label_edge"
               WHERE ag_catalog.agtype_to_json(properties)->>'source_id' IN ({})
                 AND ag_catalog.agtype_to_json(properties)->>'target_id' IN ({})
                 {}"#,
            self.graph_name, ids_str, ids_str, extra_where
        );

        // WHY: No LOAD 'age' / search_path required for native SQL on AGE tables.
        // The ag_catalog.agtype_to_json function is callable from any search_path
        // when the schema is fully qualified.
        sqlx::query("SET search_path = ag_catalog, \"$user\", public")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set search_path: {}", e)))?;

        let rows = sqlx::query(&sql).fetch_all(&mut *conn).await.map_err(|e| {
            StorageError::Database(format!("get_edges_for_node_set SQL failed: {}", e))
        })?;

        let edges: Vec<GraphEdge> = rows
            .iter()
            .filter_map(|row| {
                let props_json: serde_json::Value = row.get("edge_props");
                Self::parse_edge_from_props(props_json)
            })
            .collect();

        Ok(edges)
    }
}
