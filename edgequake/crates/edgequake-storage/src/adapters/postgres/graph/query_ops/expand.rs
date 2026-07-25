//! Graph expand / neighbors — native BFS (SPEC-054 / IMP-031-04).
//!
//! First principles: request-path expand must be **O(depth × F log E + K log N)**
//! via indexed incident-edge batches + node batch fetch — never variable-length Cypher.

use sqlx::Row;

use super::super::helpers::EdgeTenantFilterMode;
use super::super::PostgresAGEGraphStorage;
use crate::error::{Result, StorageError};
use crate::traits::{
    edge_matches_list_filter, node_matches_list_filter, EdgeListFilter, GraphEdge, GraphNode,
    KnowledgeGraph, NodeListFilter,
};
use std::collections::{HashSet, VecDeque};

impl PostgresAGEGraphStorage {
    /**
     * @dataop      DATA-AGE-GRAPH-GET-KNOWLEDGE-GRAPH-038
     * @engine      apache_age (native BFS; IMP-031-04)
     * @intent      Bounded k-hop subgraph from start node; tenant/ws optional.
     * @complexity  time: O(depth × F × log E + K log N); space: O(K + E′)
     * @limits      max_depth / max_nodes hard caps; no unbounded MATCH
     */
    pub(in crate::adapters::postgres::graph) async fn pg_get_knowledge_graph(
        &self,
        start_node: &str,
        max_depth: usize,
        max_nodes: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<KnowledgeGraph> {
        // DRY: single native BFS path for scoped and unscoped (filters optional).
        self.pg_bfs_expand(start_node, max_depth, max_nodes, tenant_id, workspace_id)
            .await
    }

    /**
     * @dataop      DATA-AGE-GRAPH-GET-NEIGHBORS-042
     * @engine      apache_age (native BFS; IMP-031-04)
     * @intent      Distinct neighbors within depth 1..3 (excludes start).
     * @complexity  time: O(depth × F log E + K log N); space: O(K)
     * @limits      depth clamped to 3; max 500 neighbors
     */
    pub(in crate::adapters::postgres::graph) async fn pg_get_neighbors(
        &self,
        node_id: &str,
        depth: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<GraphNode>> {
        let safe_depth = depth.clamp(1, 3);
        const MAX_NEIGHBORS: usize = 500;
        let kg = self
            .pg_bfs_expand(
                node_id,
                safe_depth,
                MAX_NEIGHBORS.saturating_add(1),
                tenant_id,
                workspace_id,
            )
            .await?;
        Ok(kg
            .nodes
            .into_iter()
            .filter(|n| n.id != node_id)
            .take(MAX_NEIGHBORS)
            .collect())
    }

    /// Native multi-hop BFS (SSOT for expand + neighbors).
    ///
    /// Per hop:
    /// 1. `pg_get_incident_edges_batch(frontier)` — O(F log E)
    /// 2. Collect neighbor IDs, `pg_get_nodes_batch` once — O(K log N) one RT
    async fn pg_bfs_expand(
        &self,
        start_node: &str,
        max_depth: usize,
        max_nodes: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<KnowledgeGraph> {
        let node_filter = NodeListFilter {
            tenant_id: tenant_id.map(str::to_string),
            workspace_id: workspace_id.map(str::to_string),
            ..Default::default()
        };
        let edge_filter = EdgeListFilter {
            tenant_id: tenant_id.map(str::to_string),
            workspace_id: workspace_id.map(str::to_string),
            relationship_type: None,
        };
        let filter_nodes = tenant_id.is_some() || workspace_id.is_some();
        let filter_edges = filter_nodes;

        let Some(start) = self.pg_get_node(start_node).await? else {
            return Ok(KnowledgeGraph::new());
        };
        if filter_nodes && !node_matches_list_filter(&start, &node_filter) {
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
            let edges = self
                .pg_get_incident_edges_batch(
                    &current_frontier,
                    edge_filter.tenant_id.as_deref(),
                    edge_filter.workspace_id.as_deref(),
                )
                .await?;

            let mut candidate_ids: Vec<String> = Vec::new();
            let mut candidate_seen: HashSet<String> = HashSet::new();
            for edge in &edges {
                if filter_edges && !edge_matches_list_filter(edge, &edge_filter) {
                    continue;
                }
                for (endpoint, other) in
                    [(&edge.source, &edge.target), (&edge.target, &edge.source)]
                {
                    if !frontier_set.contains(endpoint.as_str()) || visited.contains(other) {
                        continue;
                    }
                    if candidate_seen.insert(other.clone()) {
                        candidate_ids.push(other.clone());
                    }
                }
            }

            if candidate_ids.is_empty() {
                continue;
            }

            let batch = self.pg_get_nodes_batch(&candidate_ids).await?;
            for id in candidate_ids {
                let Some(node) = batch.get(&id) else {
                    continue;
                };
                if filter_nodes && !node_matches_list_filter(node, &node_filter) {
                    continue;
                }
                if !visited.insert(id.clone()) {
                    continue;
                }
                kg.add_node(node.clone());
                if kg.node_count() < max_nodes {
                    frontier.push_back(id);
                }
                if kg.node_count() >= max_nodes {
                    break;
                }
            }
        }

        let node_ids: Vec<String> = kg.nodes.iter().map(|n| n.id.clone()).collect();
        if !node_ids.is_empty() {
            let edges = self
                .pg_get_edges_for_node_set(&node_ids, tenant_id, workspace_id)
                .await?;
            for edge in edges {
                if !filter_edges || edge_matches_list_filter(&edge, &edge_filter) {
                    kg.add_edge(edge);
                }
            }
        }

        kg.is_truncated = kg.node_count() >= max_nodes;
        Ok(kg)
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
