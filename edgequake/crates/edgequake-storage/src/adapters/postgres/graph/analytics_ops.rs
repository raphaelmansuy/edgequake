use sqlx::Row;

use super::PostgresAGEGraphStorage;
use crate::error::{Result, StorageError};

impl PostgresAGEGraphStorage {
    pub(super) async fn pg_node_count(&self) -> Result<usize> {
        // WHY: Native SQL COUNT(*) on AGE vertex table is ~10x faster than
        // Cypher `MATCH (n:Node) RETURN count(n)` which does a full graph scan
        // through the AGE extension layer. No LOAD 'age' / search_path setup needed.
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;
        let sql = format!(
            r#"SELECT COUNT(*)::bigint FROM {}."_ag_label_vertex""#,
            self.graph_name
        );
        let row = sqlx::query(&sql)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("node_count SQL failed: {}", e)))?;
        let count: i64 = row.get(0);
        Ok(count as usize)
    }

    pub(super) async fn pg_edge_count(&self) -> Result<usize> {
        // WHY: Native SQL COUNT(*) on AGE edge table is ~10x faster than
        // Cypher `MATCH ()-[r:EDGE]->()` which traverses the full graph.
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;
        let sql = format!(
            r#"SELECT COUNT(*)::bigint FROM {}."_ag_label_edge""#,
            self.graph_name
        );
        let row = sqlx::query(&sql)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("edge_count SQL failed: {}", e)))?;
        let count: i64 = row.get(0);
        Ok(count as usize)
    }

    /// O(1) node count estimate from the planner statistics.
    ///
    /// SPEC-011 iter 02 Fix B: graph stream / popular / traversal endpoints
    /// poll node counts every ~30 s for UI display. Exact COUNT(*) is O(N)
    /// and grows with the graph; `pg_class.reltuples` is an in-memory catalog
    /// lookup and accurate within autovacuum's threshold (typically <10 %).
    pub(super) async fn pg_node_count_fast(&self) -> Result<usize> {
        self.reltuples_estimate("_ag_label_vertex").await
    }

    /// O(1) edge count estimate. See [`node_count_fast`].
    pub(super) async fn pg_edge_count_fast(&self) -> Result<usize> {
        self.reltuples_estimate("_ag_label_edge").await
    }

    /// Get node count for a specific workspace (OODA-03: Fix dashboard stats).
    ///
    /// WHY: Dashboard was showing 0 entities because it only checked PostgreSQL
    /// tables (empty) and KV metadata (no entity_count field). The actual data
    /// is in Apache AGE graph storage.
    ///
    /// This method uses the same property-based filtering pattern as clear_workspace()
    /// for consistency with existing workspace isolation logic.
    pub(super) async fn pg_node_count_by_workspace(
        &self,
        workspace_id: &uuid::Uuid,
    ) -> Result<usize> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let filter = crate::traits::NodeListFilter {
            tenant_id: None,
            workspace_id: Some(workspace_id.to_string()),
            ..Default::default()
        };
        let sql = Self::vertex_count_sql(&self.graph_name, "v", &filter);
        let row = sqlx::query(&sql)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("node_count_by_workspace failed: {e}")))?;
        let count: i64 = row.get(0);
        Ok(count as usize)
    }

    /// Get edge count for a specific workspace (OODA-03: Fix dashboard stats).
    pub(super) async fn pg_edge_count_by_workspace(
        &self,
        workspace_id: &uuid::Uuid,
    ) -> Result<usize> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let filter = crate::traits::EdgeListFilter {
            tenant_id: None,
            workspace_id: Some(workspace_id.to_string()),
            relationship_type: None,
        };
        let sql = Self::edge_count_sql(&self.graph_name, "e", &filter);
        let row = sqlx::query(&sql)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("edge_count_by_workspace failed: {e}")))?;
        let count: i64 = row.get(0);
        Ok(count as usize)
    }

    /// Distinct entity types in a workspace — native SQL (no full-graph Cypher scan).
    pub(super) async fn pg_distinct_node_type_count_by_workspace(
        &self,
        workspace_id: &uuid::Uuid,
    ) -> Result<usize> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let filter = crate::traits::NodeListFilter {
            tenant_id: None,
            workspace_id: Some(workspace_id.to_string()),
            ..Default::default()
        };
        let vertex_where = Self::vertex_where_clause("v", &filter);
        let extra = if vertex_where.is_empty() {
            "WHERE ag_catalog.agtype_to_json(v.properties)->>'entity_type' IS NOT NULL"
        } else {
            "AND ag_catalog.agtype_to_json(v.properties)->>'entity_type' IS NOT NULL"
        };
        let sql = format!(
            "SELECT COUNT(DISTINCT ag_catalog.agtype_to_json(v.properties)->>'entity_type')::bigint \
             FROM {}.\"_ag_label_vertex\" v {} {}",
            self.graph_name, vertex_where, extra
        );
        let count: i64 = sqlx::query_scalar(&sql)
            .fetch_one(&mut *conn)
            .await
            .unwrap_or(0);
        Ok(count as usize)
    }

    /// Count nodes whose `source_ids` array (or legacy `source_id`) contains
    /// any entry starting with `prefix` (SPEC-021 P-A3).
    ///
    /// WHY: per-document entity_count fallback for the Documents list. The
    /// previous LIKE/`jsonb_array_elements_text` predicate Seq-Scanned the
    /// full AGE vertex table (~seconds per zero-count doc → UI "stuck" on
    /// Documents). Production nodes store exact chunk ids in `source_ids`;
    /// probing those with GIN `@>` uses `idx_*_source_ids_gin` (~ms).
    pub(super) async fn pg_node_count_by_source_prefix(&self, prefix: &str) -> Result<usize> {
        let map = self
            .pg_node_counts_by_source_prefixes(&[prefix.to_string()])
            .await?;
        Ok(map.get(prefix).copied().unwrap_or(0))
    }

    /// Batch GIN `@>` probes for D document chunk prefixes — **one** round-trip
    /// (SPEC-054 L1-a). Same cap semantics as the single-prefix path.
    pub(super) async fn pg_node_counts_by_source_prefixes(
        &self,
        prefixes: &[String],
    ) -> Result<std::collections::HashMap<String, usize>> {
        use std::collections::HashMap;

        let mut out = HashMap::with_capacity(prefixes.len());
        if prefixes.is_empty() {
            return Ok(out);
        }

        // Normalize once so SQL `prefix || i` matches `source_chunk_id_candidates`.
        let normalized: Vec<String> = prefixes
            .iter()
            .map(|p| super::helpers::normalize_doc_chunk_prefix(p))
            .collect();

        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let probe_limit = super::helpers::SOURCE_CHUNK_PROBE_LIMIT as i32;

        // SPEC-084 / LAW-9 / GH-331: query child "Node" (owns idx_node_source_ids_gin),
        // not parent `_ag_label_vertex` (≈0 rows; GIN dropped by M070). Same locality
        // as SPEC-071 discovery in scan_ops.rs.
        // GIN-only on `source_ids` (indexed). Do NOT OR `source_chunk_ids`
        // here — that expression has no GIN and the planner falls back to a
        // Nested Loop Seq Scan over all vertices.
        let sql = format!(
            r#"
            WITH prefixes AS (
              SELECT prefix, ord
              FROM unnest($1::text[]) WITH ORDINALITY AS t(prefix, ord)
            ),
            probes AS (
              SELECT p.prefix, p.ord, (p.prefix || gs.i::text) AS chunk_id
              FROM prefixes p
              CROSS JOIN generate_series(0, $2::int - 1) AS gs(i)
            ),
            hits AS (
              SELECT pr.prefix, pr.ord, v.id
              FROM probes pr
              JOIN {graph}."Node" v
                ON ((ag_catalog.agtype_to_json(v.properties))::jsonb -> 'source_ids')
                   @> to_jsonb(pr.chunk_id)
            )
            SELECT p.prefix, count(DISTINCT h.id)::BIGINT AS cnt
            FROM prefixes p
            LEFT JOIN hits h ON h.prefix = p.prefix
            GROUP BY p.prefix, p.ord
            ORDER BY p.ord
            "#,
            graph = self.graph_name,
        );

        let rows: Vec<(String, i64)> = sqlx::query_as(&sql)
            .bind(&normalized)
            .bind(probe_limit)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| {
                StorageError::Database(format!("batched source-prefix GIN node count failed: {e}"))
            })?;

        // Map normalized prefix → count, then re-key to caller prefixes.
        let mut by_normalized: HashMap<String, usize> = HashMap::with_capacity(rows.len());
        for (prefix, cnt) in rows {
            by_normalized.insert(prefix, cnt as usize);
        }
        for (original, norm) in prefixes.iter().zip(normalized.iter()) {
            out.insert(
                original.clone(),
                by_normalized.get(norm).copied().unwrap_or(0),
            );
        }
        Ok(out)
    }

    pub(super) async fn pg_clear(&self) -> Result<()> {
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
    pub(super) async fn pg_clear_workspace(
        &self,
        workspace_id: &uuid::Uuid,
    ) -> Result<(usize, usize)> {
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
}
