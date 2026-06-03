//! AGE agtype → domain type parsing (SPEC-017 P1-12).

use std::collections::HashMap;

use crate::traits::{GraphEdge, GraphNode};

use super::super::PostgresAGEGraphStorage;

impl PostgresAGEGraphStorage {
    pub(in crate::adapters::postgres::graph) fn parse_vertex(
        agtype_str: &str,
    ) -> Option<GraphNode> {
        let json_str = agtype_str.trim_end_matches("::vertex");
        let value: serde_json::Value = serde_json::from_str(json_str).ok()?;
        let obj = value.as_object()?;
        let properties = obj.get("properties")?.as_object()?;
        let node_id = properties.get("node_id")?.as_str()?.to_string();

        let mut props: HashMap<String, serde_json::Value> = HashMap::new();
        for (k, v) in properties.iter() {
            if k != "node_id" {
                props.insert(k.clone(), v.clone());
            }
        }

        Some(GraphNode {
            id: node_id,
            properties: props,
        })
    }

    pub(in crate::adapters::postgres::graph) fn parse_edge(agtype_str: &str) -> Option<GraphEdge> {
        let json_str = agtype_str.trim_end_matches("::edge");
        let value: serde_json::Value = serde_json::from_str(json_str).ok()?;
        let obj = value.as_object()?;
        let properties = obj.get("properties")?.as_object()?;
        let source = properties.get("source_id")?.as_str()?.to_string();
        let target = properties.get("target_id")?.as_str()?.to_string();

        let mut props: HashMap<String, serde_json::Value> = HashMap::new();
        for (k, v) in properties.iter() {
            if k != "source_id" && k != "target_id" {
                props.insert(k.clone(), v.clone());
            }
        }

        Some(GraphEdge {
            source,
            target,
            properties: props,
        })
    }

    pub(in crate::adapters::postgres::graph) fn parse_edge_from_props(
        props_json: serde_json::Value,
    ) -> Option<GraphEdge> {
        let properties = props_json.as_object()?;
        let source = properties.get("source_id")?.as_str()?.to_string();
        let target = properties.get("target_id")?.as_str()?.to_string();

        let mut props: HashMap<String, serde_json::Value> = HashMap::new();
        for (k, v) in properties.iter() {
            if k != "source_id" && k != "target_id" {
                props.insert(k.clone(), v.clone());
            }
        }

        Some(GraphEdge {
            source,
            target,
            properties: props,
        })
    }

    pub(in crate::adapters::postgres::graph) fn parse_properties_to_node(
        node_id: &str,
        props_json: &serde_json::Value,
    ) -> Option<GraphNode> {
        if props_json.is_null() {
            return None;
        }

        let properties_map = props_json.as_object()?;
        let properties: HashMap<String, serde_json::Value> = properties_map
            .iter()
            .filter(|(k, _)| k.as_str() != "node_id")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Some(GraphNode {
            id: node_id.to_string(),
            properties,
        })
    }

    pub(in crate::adapters::postgres::graph) fn parse_json_to_properties(
        props_json: &serde_json::Value,
    ) -> HashMap<String, serde_json::Value> {
        if let Some(obj) = props_json.as_object() {
            obj.iter()
                .filter(|(k, _)| k.as_str() != "source_id" && k.as_str() != "target_id")
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        } else {
            HashMap::new()
        }
    }
}
