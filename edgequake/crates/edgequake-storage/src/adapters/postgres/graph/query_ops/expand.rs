//! Graph expand / neighbors — variable-length Cypher (SPEC-054).

use sqlx::Row;

use super::super::helpers::EdgeTenantFilterMode;
use super::super::PostgresAGEGraphStorage;
use crate::error::{Result, StorageError};
use crate::traits::{EdgeListFilter, GraphEdge, GraphNode, KnowledgeGraph, NodeListFilter};

impl PostgresAGEGraphStorage {
    pub(in crate::adapters::postgres::graph) async fn pg_get_knowledge_graph(
        &self,
        start_node: &str,
        max_depth: usize,
        max_nodes: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<KnowledgeGraph> {
        if let (Some(tenant), Some(workspace)) = (tenant_id, workspace_id) {
            return self
                .pg_get_knowledge_graph_scoped(start_node, max_depth, max_nodes, tenant, workspace)
                .await;
        }

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

    /// Tenant-scoped BFS using native SQL edge batch lookups (SPEC-027 IMP-022).
    ///
    /// Requires migration 046 expression indexes for production-scale graphs.
    async fn pg_get_knowledge_graph_scoped(
        &self,
        start_node: &str,
        max_depth: usize,
        max_nodes: usize,
        tenant_id: &str,
        workspace_id: &str,
    ) -> Result<KnowledgeGraph> {
        use std::collections::{HashSet, VecDeque};

        use crate::traits::{edge_matches_list_filter, node_matches_list_filter};

        let node_filter = NodeListFilter {
            tenant_id: Some(tenant_id.to_string()),
            workspace_id: Some(workspace_id.to_string()),
            ..Default::default()
        };
        let edge_filter = EdgeListFilter {
            tenant_id: Some(tenant_id.to_string()),
            workspace_id: Some(workspace_id.to_string()),
            relationship_type: None,
        };

        let Some(start) = self.pg_get_node(start_node).await? else {
            return Ok(KnowledgeGraph::new());
        };
        if !node_matches_list_filter(&start, &node_filter) {
            return Ok(KnowledgeGraph::new());
        }

        let mut kg = KnowledgeGraph::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut frontier: VecDeque<String> = VecDeque::new();
        visited.insert(start.id.clone());
        frontier.push_back(start.id.clone());
        kg.add_node(start);

        for _ in 0..max_depth {
            if frontier.is_empty() || kg.node_count() >= max_nodes {
                break;
            }

            let current_frontier: Vec<String> = frontier.drain(..).collect();
            let frontier_set: HashSet<&str> = current_frontier.iter().map(String::as_str).collect();
            let edges = self.pg_get_incident_edges_batch(&current_frontier).await?;

            for edge in edges {
                if !edge_matches_list_filter(&edge, &edge_filter) {
                    continue;
                }

                for (endpoint, other) in
                    [(&edge.source, &edge.target), (&edge.target, &edge.source)]
                {
                    if !frontier_set.contains(endpoint.as_str()) || visited.contains(other) {
                        continue;
                    }
                    if let Some(node) = self.pg_get_node(other).await? {
                        if node_matches_list_filter(&node, &node_filter)
                            && visited.insert(other.clone())
                        {
                            kg.add_node(node);
                            if kg.node_count() < max_nodes {
                                frontier.push_back(other.clone());
                            }
                        }
                    }
                }
            }
        }

        let node_ids: Vec<String> = kg.nodes.iter().map(|n| n.id.clone()).collect();
        if !node_ids.is_empty() {
            let edges = self
                .pg_get_edges_for_node_set(&node_ids, Some(tenant_id), Some(workspace_id))
                .await?;
            for edge in edges {
                kg.add_edge(edge);
            }
        }

        kg.is_truncated = kg.node_count() >= max_nodes;
        Ok(kg)
    }


    pub(in crate::adapters::postgres::graph) async fn pg_get_neighbors(
        &self,
        node_id: &str,
        depth: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<GraphNode>> {
        let escaped_id = Self::escape_cypher_string(node_id);

        let safe_depth = depth.clamp(1, 3);
        const MAX_NEIGHBORS: usize = 500;

        let mut tenant_where = String::new();
        if let Some(tid) = tenant_id {
            tenant_where.push_str(&format!(
                " AND neighbor.tenant_id = '{}'",
                Self::escape_cypher_string(tid)
            ));
        }
        if let Some(wid) = workspace_id {
            tenant_where.push_str(&format!(
                " AND neighbor.workspace_id = '{}'",
                Self::escape_cypher_string(wid)
            ));
        }

        let cypher = format!(
            "MATCH (start:Node {{node_id: '{escaped_id}'}})-[*1..{safe_depth}]-(neighbor:Node) \
             WHERE neighbor.node_id <> '{escaped_id}'{tenant_where} \
             RETURN DISTINCT neighbor \
             LIMIT {MAX_NEIGHBORS}"
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
    pub(in crate::adapters::postgres::graph) async fn pg_get_edges_for_node_set(
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

        // WHY: Tenant/workspace filters — legacy NULL-as-wildcard for pre-multitenancy edges.
        let edge_filter = EdgeListFilter {
            tenant_id: tenant_id.map(str::to_string),
            workspace_id: workspace_id.map(str::to_string),
            relationship_type: None,
        };
        let extra_where = Self::edge_and_clause(
            "e",
            &edge_filter,
            EdgeTenantFilterMode::LegacyNullAsWildcard,
        );

        // Native SQL: filter on edge properties directly.
        // `source_id` and `target_id` are stored in edge properties (not vertex joins needed).
        // Migration 036 adds expression indexes on these properties for fast lookups.
        let sql = format!(
            r#"SELECT ag_catalog.agtype_to_json(e.properties) AS edge_props
               FROM {}."_ag_label_edge" e
               WHERE ag_catalog.agtype_to_json(e.properties)->>'source_id' IN ({})
                 AND ag_catalog.agtype_to_json(e.properties)->>'target_id' IN ({})
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
