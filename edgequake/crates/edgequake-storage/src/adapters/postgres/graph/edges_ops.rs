use std::collections::HashMap;

use sqlx::Row;

use super::PostgresAGEGraphStorage;
use crate::error::Result;
use crate::traits::GraphEdge;

impl PostgresAGEGraphStorage {
    pub(super) async fn pg_has_edge(&self, source: &str, target: &str) -> Result<bool> {
        let escaped_source = Self::escape_cypher_string(source);
        let escaped_target = Self::escape_cypher_string(target);

        let cypher = format!(
            "MATCH (a:Node {{node_id: '{}'}})-[r:EDGE]->(b:Node {{node_id: '{}'}}) RETURN r LIMIT 1",
            escaped_source, escaped_target
        );

        let rows = self.cypher_query(&cypher, &["r"]).await?;
        Ok(!rows.is_empty())
    }

    pub(super) async fn pg_get_edge(
        &self,
        source: &str,
        target: &str,
    ) -> Result<Option<GraphEdge>> {
        let escaped_source = Self::escape_cypher_string(source);
        let escaped_target = Self::escape_cypher_string(target);

        let cypher = format!(
            "MATCH (a:Node {{node_id: '{}'}})-[r:EDGE]->(b:Node {{node_id: '{}'}}) RETURN r",
            escaped_source, escaped_target
        );

        let rows = self.cypher_query(&cypher, &["r"]).await?;

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
        let props_cypher = Self::properties_to_cypher(&props_with_ids);

        // SC1: collapse the old 3-round-trip MERGE-nodes / DELETE-edge /
        // CREATE-edge dance into ONE idempotent Cypher statement.
        //
        // WHY single statement: the previous pattern issued three separate
        // `cypher_execute` calls per edge. Under concurrent ingestion the
        // DELETE-then-CREATE window also created a race where two writers could
        // both delete and then both create, yielding duplicate EDGE rows.
        //
        // WHY MERGE on (source_id, target_id): MERGE keyed only on the edge's
        // logical identity guarantees at-most-one edge between the pair, and the
        // trailing `SET r +=` overlays the latest properties (last-write-wins)
        // without ever producing a duplicate. This is the canonical AGE upsert.
        let cypher = format!(
            "MERGE (a:Node {{node_id: '{src}'}}) \
             MERGE (b:Node {{node_id: '{tgt}'}}) \
             MERGE (a)-[r:EDGE {{source_id: '{src}', target_id: '{tgt}'}}]->(b) \
             SET r += {props}",
            src = escaped_source,
            tgt = escaped_target,
            props = props_cypher
        );
        self.cypher_execute(&cypher).await
    }

    /// SC1: batched edge upsert using a single `UNWIND ... MERGE` per chunk.
    ///
    /// WHY: same round-trip collapse as `upsert_nodes_batch`. Each row carries
    /// `source_id`/`target_id` plus the edge properties; MERGE on the endpoint
    /// nodes then MERGE on the relationship keyed by (source_id, target_id)
    /// guarantees at-most-one edge per pair (no DELETE/CREATE race), and
    /// `SET r += props` applies last-write-wins property updates.
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

            let cypher = format!(
                "UNWIND [{}] AS e \
                 MERGE (a:Node {{node_id: e.source_id}}) \
                 MERGE (b:Node {{node_id: e.target_id}}) \
                 MERGE (a)-[r:EDGE {{source_id: e.source_id, target_id: e.target_id}}]->(b) \
                 SET r += e",
                rows.join(", ")
            );
            self.cypher_execute(&cypher).await?;
        }

        Ok(())
    }

    pub(super) async fn pg_delete_edge(&self, source: &str, target: &str) -> Result<()> {
        let escaped_source = Self::escape_cypher_string(source);
        let escaped_target = Self::escape_cypher_string(target);

        let cypher = format!(
            "MATCH (a:Node {{node_id: '{}'}})-[r:EDGE]->(b:Node {{node_id: '{}'}}) DELETE r",
            escaped_source, escaped_target
        );

        self.cypher_execute(&cypher).await
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
