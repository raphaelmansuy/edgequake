//! Point get, incident edges, degree, and BFS close (Q-GRAPH-001..007).

use std::collections::HashMap;

use sqlx::Row;

use super::super::PostgresAGEGraphStorage;
use super::rows::{edge_from_row, node_from_row, tenant_and, TenantMode};
use crate::error::{Result, StorageError};
use crate::traits::{GraphEdge, GraphNode};

#[allow(clippy::module_inception)]
pub(in crate::adapters::postgres::graph) mod read {
    use super::*;

    pub(in crate::adapters::postgres::graph) async fn get_nodes_batch(
        storage: &PostgresAGEGraphStorage,
        node_ids: &[String],
    ) -> Result<HashMap<String, GraphNode>> {
        if node_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let pool = storage.pool.get().await?;
        let rows = sqlx::query(
            "SELECT node_id, properties
             FROM public.eq_graph_nodes
             WHERE namespace = $1 AND node_id = ANY($2::text[])",
        )
        .bind(&storage.namespace)
        .bind(node_ids)
        .fetch_all(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain get_nodes_batch failed: {e}")))?;
        Ok(rows
            .iter()
            .filter_map(node_from_row)
            .map(|n| (n.id.clone(), n))
            .collect())
    }

    pub(in crate::adapters::postgres::graph) async fn node_degrees_batch(
        storage: &PostgresAGEGraphStorage,
        node_ids: &[String],
    ) -> Result<Vec<(String, usize)>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }
        let pool = storage.pool.get().await?;
        let rows = sqlx::query(
            "SELECT id AS node_id, COALESCE(n.degree, 0)::bigint AS degree
             FROM unnest($2::text[]) AS id
             LEFT JOIN public.eq_graph_nodes n
               ON n.namespace = $1 AND n.node_id = id",
        )
        .bind(&storage.namespace)
        .bind(node_ids)
        .fetch_all(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain node_degrees_batch failed: {e}")))?;
        let mut map: HashMap<String, usize> = HashMap::new();
        for row in &rows {
            let id: String = row.get("node_id");
            let degree: i64 = row.get("degree");
            map.insert(id, degree.max(0) as usize);
        }
        Ok(node_ids
            .iter()
            .map(|id| (id.clone(), map.get(id).copied().unwrap_or(0)))
            .collect())
    }

    pub(in crate::adapters::postgres::graph) async fn get_all_nodes(
        storage: &PostgresAGEGraphStorage,
    ) -> Result<Vec<GraphNode>> {
        let pool = storage.pool.get().await?;
        let rows = sqlx::query(
            "SELECT node_id, properties FROM public.eq_graph_nodes WHERE namespace = $1",
        )
        .bind(&storage.namespace)
        .fetch_all(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain get_all_nodes failed: {e}")))?;
        Ok(rows.iter().filter_map(node_from_row).collect())
    }

    pub(in crate::adapters::postgres::graph) async fn incident_edges(
        storage: &PostgresAGEGraphStorage,
        node_ids: &[String],
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<GraphEdge>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }
        let scope = tenant_and("e", tenant_id, workspace_id, TenantMode::Strict);
        let pool = storage.pool.get().await?;
        let mut unique = node_ids.to_vec();
        unique.sort();
        unique.dedup();
        const CHUNK: usize = 200;
        let mut edges = Vec::new();
        for chunk in unique.chunks(CHUNK) {
            let sql = format!(
                "SELECT source_id, target_id, properties
                 FROM public.eq_graph_edges e
                 WHERE e.namespace = $1
                   AND (e.source_id = ANY($2::text[]) OR e.target_id = ANY($2::text[]))
                   {scope}"
            );
            let rows = sqlx::query(&sql)
                .bind(&storage.namespace)
                .bind(chunk)
                .fetch_all(&pool)
                .await
                .map_err(|e| StorageError::Database(format!("plain incident edges failed: {e}")))?;
            edges.extend(rows.iter().filter_map(edge_from_row));
        }
        Ok(edges)
    }

    pub(in crate::adapters::postgres::graph) async fn get_edge(
        storage: &PostgresAGEGraphStorage,
        source: &str,
        target: &str,
    ) -> Result<Option<GraphEdge>> {
        let pool = storage.pool.get().await?;
        let row = sqlx::query(
            "SELECT source_id, target_id, properties
             FROM public.eq_graph_edges
             WHERE namespace = $1 AND source_id = $2 AND target_id = $3
             LIMIT 1",
        )
        .bind(&storage.namespace)
        .bind(source)
        .bind(target)
        .fetch_optional(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain get_edge failed: {e}")))?;
        Ok(row.as_ref().and_then(edge_from_row))
    }

    pub(in crate::adapters::postgres::graph) async fn get_all_edges(
        storage: &PostgresAGEGraphStorage,
    ) -> Result<Vec<GraphEdge>> {
        let pool = storage.pool.get().await?;
        let rows = sqlx::query(
            "SELECT source_id, target_id, properties
             FROM public.eq_graph_edges WHERE namespace = $1",
        )
        .bind(&storage.namespace)
        .fetch_all(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain get_all_edges failed: {e}")))?;
        Ok(rows.iter().filter_map(edge_from_row).collect())
    }

    /// Both endpoints in the set (Q-GRAPH-006 close). Legacy empty tenant matches.
    pub(in crate::adapters::postgres::graph) async fn edges_for_node_set(
        storage: &PostgresAGEGraphStorage,
        node_ids: &[String],
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<GraphEdge>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }
        let scope = tenant_and("e", tenant_id, workspace_id, TenantMode::LegacyEmpty);
        let pool = storage.pool.get().await?;
        let sql = format!(
            "SELECT source_id, target_id, properties
             FROM public.eq_graph_edges e
             WHERE e.namespace = $1
               AND e.source_id = ANY($2::text[])
               AND e.target_id = ANY($2::text[])
               {scope}"
        );
        let rows = sqlx::query(&sql)
            .bind(&storage.namespace)
            .bind(node_ids)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("plain edges_for_node_set failed: {e}")))?;
        Ok(rows.iter().filter_map(edge_from_row).collect())
    }

    pub(in crate::adapters::postgres::graph) async fn nodes_with_degrees(
        storage: &PostgresAGEGraphStorage,
        node_ids: &[String],
    ) -> Result<Vec<(GraphNode, usize, usize)>> {
        let nodes = get_nodes_batch(storage, node_ids).await?;
        let pool = storage.pool.get().await?;
        // Two index-only counts. An OR join fetches every incident edge row
        // just to count it. A self-loop is stored once and counted on both sides.
        let rows = sqlx::query(
            "SELECT id AS node_id,
                    (SELECT COUNT(*) FROM public.eq_graph_edges e
                      WHERE e.namespace = $1 AND e.target_id = id)::bigint AS in_degree,
                    (SELECT COUNT(*) FROM public.eq_graph_edges e
                      WHERE e.namespace = $1 AND e.source_id = id)::bigint AS out_degree
             FROM unnest($2::text[]) AS id",
        )
        .bind(&storage.namespace)
        .bind(node_ids)
        .fetch_all(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain nodes_with_degrees failed: {e}")))?;
        let mut deg: HashMap<String, (usize, usize)> = HashMap::new();
        for row in &rows {
            let id: String = row.get("node_id");
            let out_d: i64 = row.get("out_degree");
            let in_d: i64 = row.get("in_degree");
            deg.insert(id, (out_d.max(0) as usize, in_d.max(0) as usize));
        }
        Ok(node_ids
            .iter()
            .filter_map(|id| {
                let node = nodes.get(id)?.clone();
                let (out_d, in_d) = deg.get(id).copied().unwrap_or((0, 0));
                Some((node, in_d, out_d))
            })
            .collect())
    }
}
