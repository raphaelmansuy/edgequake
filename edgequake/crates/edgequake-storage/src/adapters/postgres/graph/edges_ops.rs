use std::collections::HashMap;

use sqlx::Row;

use super::PostgresAGEGraphStorage;
use crate::error::Result;
use crate::traits::GraphEdge;

impl PostgresAGEGraphStorage {
    pub(super) async fn pg_has_edge(&self, source: &str, target: &str) -> Result<bool> {
        let cypher = "MATCH (a:Node {node_id: $source_id})-[r:EDGE]->(b:Node {node_id: $target_id}) RETURN r LIMIT 1";
        let params = serde_json::json!({ "source_id": source, "target_id": target });
        let rows = self.cypher_query_bound(cypher, &["r"], &params).await?;
        Ok(!rows.is_empty())
    }

    pub(super) async fn pg_get_edge(
        &self,
        source: &str,
        target: &str,
    ) -> Result<Option<GraphEdge>> {
        let cypher =
            "MATCH (a:Node {node_id: $source_id})-[r:EDGE]->(b:Node {node_id: $target_id}) RETURN r";
        let params = serde_json::json!({ "source_id": source, "target_id": target });
        let rows = self.cypher_query_bound(cypher, &["r"], &params).await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let json_value: serde_json::Value = rows[0].get("r");
        let agtype_str = json_value.to_string();
        Ok(Self::parse_edge(&agtype_str))
    }

    pub(super) async fn pg_upsert_edge(
        &self,
        source: &str,
        target: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let escaped_source = Self::escape_cypher_string(source);
        let escaped_target = Self::escape_cypher_string(target);

        // Build properties with source_id and target_id
        let mut props_with_ids = properties.clone();
        props_with_ids.insert(
            "source_id".to_string(),
            serde_json::Value::String(source.to_string()),
        );
        props_with_ids.insert(
            "target_id".to_string(),
            serde_json::Value::String(target.to_string()),
        );
        // WHY: AGE 1.6.0 does NOT support `ON CREATE SET` (apache/age#2347 is
        // unreleased) — "syntax error at or near ON". `SET r = <variable map>`
        // fails (apache/age#1634). The version-safe pattern is per-key
        // `SET r.key = <literal>` expanded inline — verified against AGE 1.6.0
        // to persist on both freshly-MERGEd and existing edges. source_id /
        // target_id are the MERGE key and are persisted by the MERGE pattern.
        let mut set_clauses: Vec<String> = Vec::with_capacity(props_with_ids.len());
        for (k, v) in &props_with_ids {
            if k == "source_id" || k == "target_id" {
                continue;
            }
            set_clauses.push(format!("r.{} = {}", k, Self::value_to_cypher(v)));
        }
        let set_clause = if set_clauses.is_empty() {
            String::new()
        } else {
            format!(" SET {}", set_clauses.join(", "))
        };
        let cypher = format!(
            "MERGE (a:Node {{node_id: '{src}'}}) \
             MERGE (b:Node {{node_id: '{tgt}'}}) \
             MERGE (a)-[r:EDGE {{source_id: '{src}', target_id: '{tgt}'}}]->(b){set_clause}",
            src = escaped_source,
            tgt = escaped_target,
            set_clause = set_clause
        );
        self.cypher_execute(&cypher).await
    }

    /// SC1: batched edge upsert using a single `UNWIND ... MERGE` per chunk.
    ///
    /// WHY: same round-trip collapse as `upsert_nodes_batch`. Each row carries
    /// `source_id`/`target_id` plus the edge properties; MERGE on the endpoint
    /// nodes then MERGE on the relationship keyed by (source_id, target_id)
    /// guarantees at-most-one edge per pair (no DELETE/CREATE race), and
    /// `SET r.key = e.key` (per-key) applies last-write-wins property updates.
    pub(super) async fn pg_upsert_edges_batch(
        &self,
        edges: &[(String, String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        if edges.is_empty() {
            return Ok(());
        }

        const CHUNK: usize = 500;

        for chunk in edges.chunks(CHUNK) {
            let rows: Vec<String> = chunk
                .iter()
                .map(|(source, target, properties)| {
                    let mut map = properties.clone();
                    map.insert(
                        "source_id".to_string(),
                        serde_json::Value::String(source.clone()),
                    );
                    map.insert(
                        "target_id".to_string(),
                        serde_json::Value::String(target.clone()),
                    );
                    Self::properties_to_cypher(&map)
                })
                .collect();

            // WHY: AGE 1.6.0 does NOT support `ON CREATE SET` (apache/age#2347
            // is unreleased) and `SET r = <variable map>` fails
            // (apache/age#1634). The version-safe pattern is per-key
            // `SET r.key = e.key` referencing the UNWIND row — verified against
            // AGE 1.6.0 to persist on both fresh and existing edges.
            // source_id/target_id are the MERGE key (persisted by MERGE).
            let mut set_keys: Vec<&str> = Vec::with_capacity(32);
            if let Some((_, _, props)) = chunk.first() {
                for k in props.keys() {
                    if k != "source_id" && k != "target_id" {
                        set_keys.push(k.as_str());
                    }
                }
            }
            let set_clause = if set_keys.is_empty() {
                String::new()
            } else {
                let sets: Vec<String> = set_keys
                    .iter()
                    .map(|k| format!("r.{} = e.{}", k, k))
                    .collect();
                format!(" SET {}", sets.join(", "))
            };
            let cypher = format!(
                "UNWIND [{}] AS e \
                 MERGE (a:Node {{node_id: e.source_id}}) \
                 MERGE (b:Node {{node_id: e.target_id}}) \
                 MERGE (a)-[r:EDGE {{source_id: e.source_id, target_id: e.target_id}}]->(b){}",
                rows.join(", "),
                set_clause
            );
            self.cypher_execute(&cypher).await?;
        }

        Ok(())
    }

    pub(super) async fn pg_delete_edge(&self, source: &str, target: &str) -> Result<()> {
        let cypher =
            "MATCH (a:Node {node_id: $source_id})-[r:EDGE]->(b:Node {node_id: $target_id}) DELETE r";
        let params = serde_json::json!({ "source_id": source, "target_id": target });
        self.cypher_execute_bound(cypher, &params).await
    }

    pub(super) async fn pg_get_node_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>> {
        let escaped_id = Self::escape_cypher_string(node_id);

        // Get both outgoing and incoming edges
        let cypher = format!(
            "MATCH (n:Node {{node_id: '{}'}})-[r:EDGE]-() RETURN r",
            escaped_id
        );

        let rows = self.cypher_query(&cypher, &["r"]).await?;

        let edges: Vec<GraphEdge> = rows
            .iter()
            .filter_map(|row| {
                let json_value: serde_json::Value = row.get("r");
                let agtype_str = json_value.to_string();
                Self::parse_edge(&agtype_str)
            })
            .collect();

        Ok(edges)
    }

    pub(super) async fn pg_get_incident_edges_batch(
        &self,
        node_ids: &[String],
    ) -> Result<Vec<GraphEdge>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut unique: Vec<String> = node_ids.to_vec();
        unique.sort();
        unique.dedup();

        const CHUNK: usize = 100;
        let mut all_edges = Vec::new();

        for chunk in unique.chunks(CHUNK) {
            let id_literals: Vec<String> = chunk
                .iter()
                .map(|id| format!("'{}'", Self::escape_cypher_string(id)))
                .collect();
            let cypher = format!(
                "UNWIND [{}] AS nid MATCH (n:Node {{node_id: nid}})-[r:EDGE]-() RETURN r",
                id_literals.join(", ")
            );

            let rows = self.cypher_query(&cypher, &["r"]).await?;
            let edges: Vec<GraphEdge> = rows
                .iter()
                .filter_map(|row| {
                    let json_value: serde_json::Value = row.get("r");
                    let agtype_str = json_value.to_string();
                    Self::parse_edge(&agtype_str)
                })
                .collect();
            all_edges.extend(edges);
        }

        Ok(all_edges)
    }

    pub(super) async fn pg_get_all_edges(&self) -> Result<Vec<GraphEdge>> {
        let cypher = "MATCH ()-[r:EDGE]->() RETURN r";
        let rows = self.cypher_query(cypher, &["r"]).await?;

        let edges: Vec<GraphEdge> = rows
            .iter()
            .filter_map(|row| {
                let json_value: serde_json::Value = row.get("r");
                let agtype_str = json_value.to_string();
                Self::parse_edge(&agtype_str)
            })
            .collect();

        Ok(edges)
    }
}
