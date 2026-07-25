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

        // SPEC-084 / LAW-9 / GH-331 + IMP-031-08: child "Node" + MATERIALIZED
        // probe-first GIN (same planner law as cascade discovery).
        // GIN-only on `source_ids` — never OR unindexed source_chunk_ids.
        let probes_cte = super::helpers::source_ids_count_probes_cte_sql();
        let sql = format!(
            r#"
            WITH {probes_cte},
            hits AS MATERIALIZED (
              SELECT pr.prefix, pr.ord, v.id
              FROM probes pr
              INNER JOIN {graph}."Node" v
                ON ((ag_catalog.agtype_to_json(v.properties))::jsonb -> 'source_ids')
                   @> to_jsonb(pr.chunk_id)
            )
            SELECT p.prefix, count(DISTINCT h.id)::BIGINT AS cnt
            FROM prefixes p
            LEFT JOIN hits h ON h.prefix = p.prefix
            GROUP BY p.prefix, p.ord
            ORDER BY p.ord
            "#,
            probes_cte = probes_cte.trim(),
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

    /// ADMIN: wipe entire graph (IMP-031-06 native TRUNCATE-equivalent DELETE).
    ///
    /// Complexity: O(N+E) unavoidable; avoids AGE Cypher planner overhead.
    pub(super) async fn pg_clear(&self) -> Result<()> {
        let pool = self.pool.get().await?;
        let graph = &self.graph_name;
        // Edges first (no FK, but keeps AGE label tables consistent), then nodes.
        let del_e = format!(r#"/* DATA-AGE-GRAPH-CLEAR */ DELETE FROM {graph}."EDGE""#);
        let del_n = format!(r#"/* DATA-AGE-GRAPH-CLEAR */ DELETE FROM {graph}."Node""#);
        sqlx::query(&del_e)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("native clear edges failed: {e}")))?;
        sqlx::query(&del_n)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("native clear nodes failed: {e}")))?;
        Ok(())
    }

    /// Clear nodes and edges for a specific workspace (IMP-031-06 native).
    ///
    /// 1. Count nodes with workspace_id (native COUNT)
    /// 2. Collect node_ids, detach incident edges, delete nodes (reuse batch delete)
    ///
    /// Returns (nodes_deleted, edges_deleted).
    pub(super) async fn pg_clear_workspace(
        &self,
        workspace_id: &uuid::Uuid,
    ) -> Result<(usize, usize)> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;
        let graph = &self.graph_name;
        let wid = workspace_id.to_string();
        let eq_present = self.eq_columns_present(&mut conn).await?;
        let node_key = if eq_present {
            super::helpers::coalesce_endpoint("n", "node")
        } else {
            super::helpers::prop_only_endpoint("n", "node")
        };
        let ws_expr =
            "COALESCE(ag_catalog.agtype_to_json(n.properties)->>'workspace_id', '')".to_string();

        // Collect node_ids in workspace (one RT).
        let list_sql = format!(
            r#"/* DATA-AGE-GRAPH-CLEAR-WORKSPACE list */
               SELECT {node_key} AS node_id
               FROM {graph}."Node" n
               WHERE {ws_expr} = $1"#
        );
        let rows = sqlx::query(&list_sql)
            .bind(&wid)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("clear_workspace list failed: {e}")))?;
        let node_ids: Vec<String> = rows
            .iter()
            .filter_map(|r| r.try_get::<String, _>("node_id").ok())
            .filter(|s| !s.is_empty())
            .collect();
        let node_count = node_ids.len();
        if node_ids.is_empty() {
            return Ok((0, 0));
        }

        // Count incident edges before detach (optional metric).
        let src = if eq_present {
            super::helpers::coalesce_endpoint("e", "source")
        } else {
            super::helpers::prop_only_endpoint("e", "source")
        };
        let tgt = if eq_present {
            super::helpers::coalesce_endpoint("e", "target")
        } else {
            super::helpers::prop_only_endpoint("e", "target")
        };
        let edge_cnt_sql = format!(
            r#"SELECT COUNT(*)::bigint FROM {graph}."EDGE" e
               WHERE {src} = ANY($1::text[]) OR {tgt} = ANY($1::text[])"#
        );
        let edge_count: i64 = sqlx::query_scalar(&edge_cnt_sql)
            .bind(&node_ids)
            .fetch_one(&mut *conn)
            .await
            .unwrap_or(0);

        // Reuse native batch detach+delete (DRY with delete_nodes_batch).
        drop(conn);
        self.pg_delete_nodes_batch(&node_ids).await?;

        tracing::info!(
            workspace_id = %workspace_id,
            nodes_deleted = node_count,
            edges_deleted = edge_count,
            "Cleared workspace from graph storage (native)"
        );

        Ok((node_count, edge_count as usize))
    }
}
