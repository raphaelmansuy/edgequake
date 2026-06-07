//! Bounded graph scan — SPEC-006 postgres push-down.

use super::PostgresAGEGraphStorage;
use crate::error::{Result, StorageError};
use crate::traits::{EdgeListFilter, GraphEdge, GraphNode, NodeListFilter, PagedGraphResult};
use sqlx::Row;

impl PostgresAGEGraphStorage {
    fn build_node_where_clause(filter: &NodeListFilter) -> String {
        let mut conditions = Vec::new();

        if let Some(tid) = filter.tenant_id.as_deref() {
            conditions.push(format!(
                "ag_catalog.agtype_to_json(v.properties)->>'tenant_id' = '{}'",
                Self::escape_sql_string(tid)
            ));
        }
        if let Some(wid) = filter.workspace_id.as_deref() {
            conditions.push(format!(
                "ag_catalog.agtype_to_json(v.properties)->>'workspace_id' = '{}'",
                Self::escape_sql_string(wid)
            ));
        }
        if let Some(etype) = filter.entity_type.as_deref() {
            conditions.push(format!(
                "UPPER(ag_catalog.agtype_to_json(v.properties)->>'entity_type') = UPPER('{}')",
                Self::escape_sql_string(etype)
            ));
        }
        if let Some(search) = filter.search.as_deref() {
            let q = Self::escape_sql_string(&search.to_lowercase());
            conditions.push(format!(
                "(LOWER(ag_catalog.agtype_to_json(v.properties)->>'node_id') LIKE '%{q}%' \
                 OR LOWER(COALESCE(ag_catalog.agtype_to_json(v.properties)->>'description', '')) LIKE '%{q}%')"
            ));
        }

        if conditions.is_empty() {
            "TRUE".to_string()
        } else {
            conditions.join(" AND ")
        }
    }

    fn build_edge_where_clause(filter: &EdgeListFilter) -> String {
        let mut conditions = Vec::new();

        if let Some(tid) = filter.tenant_id.as_deref() {
            conditions.push(format!(
                "ag_catalog.agtype_to_json(e.properties)->>'tenant_id' = '{}'",
                Self::escape_sql_string(tid)
            ));
        }
        if let Some(wid) = filter.workspace_id.as_deref() {
            conditions.push(format!(
                "ag_catalog.agtype_to_json(e.properties)->>'workspace_id' = '{}'",
                Self::escape_sql_string(wid)
            ));
        }
        if let Some(rel) = filter.relationship_type.as_deref() {
            conditions.push(format!(
                "UPPER(ag_catalog.agtype_to_json(e.properties)->>'relation_type') = UPPER('{}')",
                Self::escape_sql_string(rel)
            ));
        }

        if conditions.is_empty() {
            "TRUE".to_string()
        } else {
            conditions.join(" AND ")
        }
    }

    pub(super) async fn pg_list_nodes_filtered(
        &self,
        filter: &NodeListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<PagedGraphResult<GraphNode>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let where_clause = Self::build_node_where_clause(filter);

        let count_sql = format!(
            "SELECT COUNT(*)::BIGINT AS total
             FROM {graph}.\"_ag_label_vertex\" v
             WHERE {where_clause}",
            graph = self.graph_name,
            where_clause = where_clause
        );

        let total: i64 = sqlx::query_scalar(&count_sql)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Node count query failed: {}", e)))?;

        let page_sql = format!(
            "SELECT ag_catalog.agtype_to_json(v.properties) AS props
             FROM {graph}.\"_ag_label_vertex\" v
             WHERE {where_clause}
             ORDER BY ag_catalog.agtype_to_json(v.properties)->>'node_id'
             OFFSET {offset} LIMIT {limit}",
            graph = self.graph_name,
            where_clause = where_clause,
            offset = offset,
            limit = limit
        );

        let rows = sqlx::query(&page_sql)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Node list query failed: {}", e)))?;

        let items: Vec<GraphNode> = rows
            .iter()
            .filter_map(|row| {
                let props: serde_json::Value = row.get("props");
                let node_id = props.get("node_id")?.as_str()?.to_string();
                let properties = props.as_object()?.clone().into_iter().collect();
                Some(GraphNode {
                    id: node_id,
                    properties,
                })
            })
            .collect();

        Ok(PagedGraphResult {
            items,
            total: total as usize,
            offset,
            limit,
        })
    }

    pub(super) async fn pg_list_edges_filtered(
        &self,
        filter: &EdgeListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<PagedGraphResult<GraphEdge>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let where_clause = Self::build_edge_where_clause(filter);

        let count_sql = format!(
            "SELECT COUNT(*)::BIGINT AS total
             FROM {graph}.\"_ag_label_edge\" e
             WHERE {where_clause}",
            graph = self.graph_name,
            where_clause = where_clause
        );

        let total: i64 = sqlx::query_scalar(&count_sql)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Edge count query failed: {}", e)))?;

        let page_sql = format!(
            "SELECT
                ag_catalog.agtype_to_json(e.properties) AS props,
                ag_catalog.agtype_to_json(sv.properties)->>'node_id' AS source_id,
                ag_catalog.agtype_to_json(tv.properties)->>'node_id' AS target_id
             FROM {graph}.\"_ag_label_edge\" e
             JOIN {graph}.\"_ag_label_vertex\" sv ON e.start_id = sv.id
             JOIN {graph}.\"_ag_label_vertex\" tv ON e.end_id = tv.id
             WHERE {where_clause}
             ORDER BY source_id, target_id
             OFFSET {offset} LIMIT {limit}",
            graph = self.graph_name,
            where_clause = where_clause,
            offset = offset,
            limit = limit
        );

        let rows = sqlx::query(&page_sql)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Edge list query failed: {}", e)))?;

        let items: Vec<GraphEdge> = rows
            .iter()
            .filter_map(|row| {
                let props: serde_json::Value = row.get("props");
                let source: String = row.get("source_id");
                let target: String = row.get("target_id");
                let properties = props.as_object()?.clone().into_iter().collect();
                Some(GraphEdge {
                    source,
                    target,
                    properties,
                })
            })
            .collect();

        Ok(PagedGraphResult {
            items,
            total: total as usize,
            offset,
            limit,
        })
    }

    fn build_source_prefix_clause(props_expr: &str, source_prefixes: &[String]) -> String {
        let mut conditions = Vec::new();
        for prefix in source_prefixes {
            let esc = Self::escape_sql_string(prefix);
            let chunk = Self::escape_sql_string(&format!("{prefix}-chunk-"));
            conditions.push(format!(
                "({props}->>'source_id' LIKE '{esc}%' \
                 OR {props}->>'source_id' LIKE '%|{esc}%' \
                 OR {props}->>'source_id' LIKE '%|{chunk}%' \
                 OR {props}->>'source_id' LIKE '{chunk}%' \
                 OR EXISTS (
                     SELECT 1 FROM jsonb_array_elements_text(
                         CASE
                             WHEN jsonb_typeof({props}->'source_ids') = 'array'
                             THEN {props}->'source_ids'
                             ELSE '[]'::jsonb
                         END
                     ) src
                     WHERE src LIKE '{esc}%' OR src LIKE '{chunk}%' OR src = '{esc}'
                 ))",
                props = props_expr,
                esc = esc,
                chunk = chunk
            ));
        }
        if conditions.is_empty() {
            "FALSE".to_string()
        } else {
            conditions.join(" OR ")
        }
    }

    pub(super) async fn pg_find_nodes_by_source_prefixes(
        &self,
        filter: &NodeListFilter,
        source_prefixes: &[String],
    ) -> Result<Vec<GraphNode>> {
        if source_prefixes.is_empty() {
            return Ok(Vec::new());
        }

        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let tenant_where = Self::build_node_where_clause(filter);
        let props_expr = "ag_catalog.agtype_to_json(v.properties)";
        let source_where = Self::build_source_prefix_clause(props_expr, source_prefixes);

        let sql = format!(
            "SELECT {props} AS props
             FROM {graph}.\"_ag_label_vertex\" v
             WHERE {tenant_where} AND ({source_where})
             ORDER BY {props}->>'node_id'",
            props = props_expr,
            graph = self.graph_name,
            tenant_where = tenant_where,
            source_where = source_where
        );

        let rows = sqlx::query(&sql).fetch_all(&mut *conn).await.map_err(|e| {
            StorageError::Database(format!("Source-prefix node query failed: {}", e))
        })?;

        Ok(rows
            .iter()
            .filter_map(|row| {
                let props: serde_json::Value = row.get("props");
                let node_id = props.get("node_id")?.as_str()?.to_string();
                let properties = props.as_object()?.clone().into_iter().collect();
                Some(GraphNode {
                    id: node_id,
                    properties,
                })
            })
            .collect())
    }

    pub(super) async fn pg_find_edges_by_source_prefixes(
        &self,
        filter: &EdgeListFilter,
        source_prefixes: &[String],
    ) -> Result<Vec<GraphEdge>> {
        if source_prefixes.is_empty() {
            return Ok(Vec::new());
        }

        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let tenant_where = Self::build_edge_where_clause(filter);
        let props_expr = "ag_catalog.agtype_to_json(e.properties)";
        let source_where = Self::build_source_prefix_clause(props_expr, source_prefixes);

        let sql = format!(
            "SELECT
                {props} AS props,
                ag_catalog.agtype_to_json(sv.properties)->>'node_id' AS source_id,
                ag_catalog.agtype_to_json(tv.properties)->>'node_id' AS target_id
             FROM {graph}.\"_ag_label_edge\" e
             JOIN {graph}.\"_ag_label_vertex\" sv ON e.start_id = sv.id
             JOIN {graph}.\"_ag_label_vertex\" tv ON e.end_id = tv.id
             WHERE {tenant_where} AND ({source_where})
             ORDER BY source_id, target_id",
            props = props_expr,
            graph = self.graph_name,
            tenant_where = tenant_where,
            source_where = source_where
        );

        let rows = sqlx::query(&sql).fetch_all(&mut *conn).await.map_err(|e| {
            StorageError::Database(format!("Source-prefix edge query failed: {}", e))
        })?;

        Ok(rows
            .iter()
            .filter_map(|row| {
                let props: serde_json::Value = row.get("props");
                let source: String = row.get("source_id");
                let target: String = row.get("target_id");
                let properties = props.as_object()?.clone().into_iter().collect();
                Some(GraphEdge {
                    source,
                    target,
                    properties,
                })
            })
            .collect())
    }

    pub(super) async fn pg_find_edge_by_relationship_id(
        &self,
        filter: &EdgeListFilter,
        relationship_id: &str,
    ) -> Result<Option<GraphEdge>> {
        if relationship_id.is_empty() {
            return Ok(None);
        }

        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let tenant_where = Self::build_edge_where_clause(filter);
        let esc_id = Self::escape_sql_string(relationship_id);
        let props_expr = "ag_catalog.agtype_to_json(e.properties)";

        let sql = format!(
            "SELECT
                {props} AS props,
                ag_catalog.agtype_to_json(sv.properties)->>'node_id' AS source_id,
                ag_catalog.agtype_to_json(tv.properties)->>'node_id' AS target_id
             FROM {graph}.\"_ag_label_edge\" e
             JOIN {graph}.\"_ag_label_vertex\" sv ON e.start_id = sv.id
             JOIN {graph}.\"_ag_label_vertex\" tv ON e.end_id = tv.id
             WHERE {tenant_where}
               AND (
                 {props}->>'id' = '{esc_id}'
                 OR CONCAT(
                   ag_catalog.agtype_to_json(sv.properties)->>'node_id',
                   '_',
                   ag_catalog.agtype_to_json(tv.properties)->>'node_id'
                 ) = '{esc_id}'
               )
             LIMIT 1",
            props = props_expr,
            graph = self.graph_name,
            tenant_where = tenant_where,
            esc_id = esc_id
        );

        let row = sqlx::query(&sql)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Relationship id lookup failed: {}", e)))?;

        Ok(row.map(|row| {
            let props: serde_json::Value = row.get("props");
            let source: String = row.get("source_id");
            let target: String = row.get("target_id");
            let properties = props
                .as_object()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect();
            GraphEdge {
                source,
                target,
                properties,
            }
        }))
    }
}
