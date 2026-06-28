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
    /// WHY: per-document entity_count fallback for the Documents list. A
    /// single aggregate Cypher avoids materializing the nodes. We check both
    /// the modern `source_ids` array and the legacy `source_id` pipe-delimited
    /// string to match `collect_source_references` semantics.
    pub(super) async fn pg_node_count_by_source_prefix(&self, prefix: &str) -> Result<usize> {
        // WHY: avoid Cypher's `any(s IN n.source_ids WHERE s STARTS WITH ...)`
        // which this AGE version rejects with "syntax error at or near WHERE".
        // Mirror scan_ops.rs and do the prefix match in SQL against the JSONB
        // view of each node's properties — same semantics, AGE-version-safe.
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let escaped = Self::escape_sql_string(prefix);
        let chunk = Self::escape_sql_string(&format!("{prefix}-chunk-"));
        let props = "ag_catalog.agtype_to_json(v.properties)";
        let sql = format!(
            "SELECT count(*)::BIGINT FROM {graph}.\"_ag_label_vertex\" v \
             WHERE ({props}::jsonb)->>'source_id' LIKE '{esc}%' \
                OR ({props}::jsonb)->>'source_id' LIKE '%|{esc}%' \
                OR ({props}::jsonb)->>'source_id' LIKE '{chunk}%' \
                OR ({props}::jsonb)->>'source_id' LIKE '%|{chunk}%' \
                OR EXISTS ( \
                    SELECT 1 FROM jsonb_array_elements_text( \
                        CASE \
                            WHEN jsonb_typeof({props}::jsonb->'source_ids') = 'array' \
                            THEN {props}::jsonb->'source_ids' \
                            ELSE '[]'::jsonb \
                        END \
                    ) src \
                    WHERE src LIKE '{esc}%' OR src LIKE '{chunk}%' OR src = '{esc}' \
                )",
            graph = self.graph_name,
            props = props,
            esc = escaped,
            chunk = chunk
        );

        let count: i64 = sqlx::query_scalar(&sql)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("source-prefix node count failed: {e}")))?;
        Ok(count as usize)
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
