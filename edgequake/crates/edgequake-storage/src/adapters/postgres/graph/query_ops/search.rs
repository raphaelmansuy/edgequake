//! Label/node search helpers (SPEC-054 ISP).

use std::collections::HashMap;

use sqlx::Row;

use super::super::PostgresAGEGraphStorage;
use crate::error::{Result, StorageError};
use crate::traits::{GraphNode, NodeListFilter};

impl PostgresAGEGraphStorage {

    pub(in crate::adapters::postgres::graph) async fn pg_get_popular_labels(
        &self,
        limit: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<String>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let filter = NodeListFilter {
            tenant_id: tenant_id.map(str::to_string),
            workspace_id: workspace_id.map(str::to_string),
            ..Default::default()
        };
        let vertex_where = Self::vertex_where_clause("v", &filter);

        // Scoped SQL — same filter-first pattern as pg_get_popular_nodes_with_degree.
        let sql = format!(
            "WITH filtered_nodes AS MATERIALIZED ( \
                SELECT v.id::text AS id_text, v.properties \
                FROM {}.\"_ag_label_vertex\" v \
                {} \
            ), \
            edge_counts AS ( \
                SELECT e.start_id::text AS start_id_text, COUNT(*) AS out_degree \
                FROM {}.\"_ag_label_edge\" e \
                INNER JOIN filtered_nodes fn ON e.start_id::text = fn.id_text \
                GROUP BY e.start_id::text \
            ) \
            SELECT ag_catalog.agtype_to_json(fn.properties)->>'node_id' AS label \
            FROM filtered_nodes fn \
            LEFT JOIN edge_counts ec ON fn.id_text = ec.start_id_text \
            ORDER BY COALESCE(ec.out_degree, 0) DESC \
            LIMIT {}",
            self.graph_name, vertex_where, self.graph_name, limit
        );

        let rows = sqlx::query(&sql)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("popular labels SQL failed: {}", e)))?;

        Ok(rows
            .iter()
            .filter_map(|row| row.get::<Option<String>, _>("label"))
            .collect())
    }


    /// FAST OPTIMIZED: Search node labels with full-text search and fuzzy matching.
    ///
    /// Uses PostgreSQL's full-text search (ts_vector) and trigram similarity (pg_trgm).
    /// Supports fuzzy matching, ranking by relevance, and handles typos.
    ///
    /// Performance: <100ms for fuzzy search across 10k+ nodes
    pub(in crate::adapters::postgres::graph) async fn pg_search_labels(
        &self,
        query: &str,
        limit: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<String>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let tenant_predicate = Self::build_vertex_property_where(
            "v",
            &NodeListFilter {
                tenant_id: tenant_id.map(str::to_string),
                workspace_id: workspace_id.map(str::to_string),
                ..Default::default()
            },
        );
        let tenant_and = if tenant_predicate == "TRUE" {
            String::new()
        } else {
            format!(" AND {tenant_predicate}")
        };

        let escaped_query = Self::escape_sql_string(query);
        tracing::debug!(query = %query, escaped = %escaped_query, "search_labels starting");

        // Try full-text search first (best for word matching)
        let fts_sql = format!(
            "SELECT \
                ag_catalog.agtype_to_json(v.properties)->>'node_id' as label, \
                ts_rank( \
                    to_tsvector('english', ag_catalog.agtype_to_json(v.properties)->>'node_id'), \
                    plainto_tsquery('english', '{0}') \
                ) as rank \
             FROM {1}.\"_ag_label_vertex\" v \
             WHERE to_tsvector('english', ag_catalog.agtype_to_json(v.properties)->>'node_id') \
                   @@ plainto_tsquery('english', '{0}'){2} \
             ORDER BY rank DESC \
             LIMIT {3}",
            escaped_query, self.graph_name, tenant_and, limit
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
                ag_catalog.agtype_to_json(v.properties)->>'node_id' as label, \
                ag_catalog.similarity( \
                    ag_catalog.agtype_to_json(v.properties)->>'node_id', \
                    '{0}' \
                ) as sim \
             FROM {1}.\"_ag_label_vertex\" v \
             WHERE ag_catalog.agtype_to_json(v.properties)->>'node_id' OPERATOR(ag_catalog.%) '{0}' \
             {2} \
             ORDER BY sim DESC \
             LIMIT {3}",
            escaped_query, self.graph_name, tenant_and, limit
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
            "SELECT ag_catalog.agtype_to_json(v.properties)->>'node_id' as label \
             FROM {0}.\"_ag_label_vertex\" v \
             WHERE LOWER(ag_catalog.agtype_to_json(v.properties)->>'node_id') LIKE LOWER('{1}%') \
             {2} \
             ORDER BY ag_catalog.agtype_to_json(v.properties)->>'node_id' \
             LIMIT {3}",
            self.graph_name, escaped_query, tenant_and, limit
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
    pub(in crate::adapters::postgres::graph) async fn pg_search_nodes(
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

        tracing::debug!(query = %query, "search_nodes starting");

        let filter = NodeListFilter {
            tenant_id: tenant_id.map(str::to_string),
            workspace_id: workspace_id.map(str::to_string),
            entity_type: entity_type.map(str::to_string),
            search: Some(query.to_string()),
            ..Default::default()
        };
        let vertex_where = Self::vertex_where_clause("v", &filter);

        // WHY: Filter vertices first (MATERIALIZED), scope degree counts to that set,
        // and join via graphid::text (AGE has no graphid = operator). Global edge
        // GROUP BY + nested-loop join against tenant filters caused 22s+ timeouts.
        let sql = format!(
            "WITH filtered_nodes AS MATERIALIZED ( \
                SELECT v.id::text AS id_text, ag_catalog.agtype_to_json(v.properties) AS props \
                FROM {graph}.\"_ag_label_vertex\" v \
                {vertex_where} \
            ), \
            out_degrees AS ( \
                SELECT e.start_id::text AS id_text, COUNT(*) AS out_degree \
                FROM {graph}.\"_ag_label_edge\" e \
                INNER JOIN filtered_nodes fn ON e.start_id::text = fn.id_text \
                GROUP BY e.start_id::text \
            ), \
            in_degrees AS ( \
                SELECT e.end_id::text AS id_text, COUNT(*) AS in_degree \
                FROM {graph}.\"_ag_label_edge\" e \
                INNER JOIN filtered_nodes fn ON e.end_id::text = fn.id_text \
                GROUP BY e.end_id::text \
            ) \
            SELECT \
                fn.props, \
                COALESCE(o.out_degree, 0) + COALESCE(i.in_degree, 0) AS degree \
            FROM filtered_nodes fn \
            LEFT JOIN out_degrees o ON fn.id_text = o.id_text \
            LEFT JOIN in_degrees i ON fn.id_text = i.id_text \
            ORDER BY degree DESC \
            LIMIT {limit}",
            graph = self.graph_name,
            vertex_where = vertex_where,
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


    pub(in crate::adapters::postgres::graph) async fn pg_get_popular_nodes_with_degree(
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

        let filter = NodeListFilter {
            tenant_id: tenant_id.map(str::to_string),
            workspace_id: workspace_id.map(str::to_string),
            entity_type: entity_type.map(str::to_string),
            ..Default::default()
        };
        let vertex_where = Self::vertex_where_clause("v", &filter);
        let min_degree_val = min_degree.unwrap_or(0);

        // WHY: Filter vertices first (MATERIALIZED), then hash-join edge counts scoped
        // to those nodes. The previous plan computed global edge_counts first and
        // nested-loop joined against tenant/workspace filters — PostgreSQL estimated
        // rows=1 with filters present, producing a nested-loop cartesian (~31k × 15k
        // comparisons, 22s+) that exceeded the 15s graph query budget (SPEC-006).
        // graphid::text joins are the AGE-safe pattern (see node_degrees_batch).
        let sql = format!(
            "WITH filtered_nodes AS MATERIALIZED ( \
                SELECT v.id::text AS id_text, v.properties \
                FROM {}.\"_ag_label_vertex\" v \
                {} \
            ), \
            edge_counts AS ( \
                SELECT e.start_id::text AS start_id_text, COUNT(*) AS out_degree \
                FROM {}.\"_ag_label_edge\" e \
                INNER JOIN filtered_nodes fn ON e.start_id::text = fn.id_text \
                GROUP BY e.start_id::text \
            ) \
            SELECT \
                ag_catalog.agtype_to_json(fn.properties) AS node_props, \
                COALESCE(ec.out_degree, 0) AS degree \
            FROM filtered_nodes fn \
            LEFT JOIN edge_counts ec ON fn.id_text = ec.start_id_text \
            WHERE COALESCE(ec.out_degree, 0) >= {} \
            ORDER BY degree DESC \
            LIMIT {}",
            self.graph_name, vertex_where, self.graph_name, min_degree_val, limit
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

}
