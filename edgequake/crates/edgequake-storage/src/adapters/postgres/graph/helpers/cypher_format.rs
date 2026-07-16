//! Cypher/SQL string formatting and property literals (SPEC-017 P1-12).
//!
//! Escaping SSOT: [`super::escape`]. Methods on [`PostgresAGEGraphStorage`]
//! remain as thin wrappers for call-site ergonomics.

use std::collections::HashMap;

use super::super::PostgresAGEGraphStorage;
use super::escape::{escape_cypher_string, escape_sql_literal};

impl PostgresAGEGraphStorage {
    pub(in crate::adapters::postgres::graph) fn escape_cypher_string(s: &str) -> String {
        escape_cypher_string(s)
    }

    /// Alias for [`escape_sql_literal`] (historical name on the storage type).
    pub(in crate::adapters::postgres::graph) fn escape_sql_string(s: &str) -> String {
        escape_sql_literal(s)
    }

    pub(in crate::adapters::postgres::graph) fn properties_to_cypher(
        props: &HashMap<String, serde_json::Value>,
    ) -> String {
        if props.is_empty() {
            return "{}".to_string();
        }

        let parts: Vec<String> = props
            .iter()
            .map(|(k, v)| format!("{}: {}", k, Self::value_to_cypher(v)))
            .collect();

        format!("{{{}}}", parts.join(", "))
    }

    pub(in crate::adapters::postgres::graph) fn value_to_cypher(v: &serde_json::Value) -> String {
        match v {
            serde_json::Value::String(s) => format!("'{}'", Self::escape_cypher_string(s)),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => "null".to_string(),
            serde_json::Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(Self::value_to_cypher).collect();
                format!("[{}]", items.join(", "))
            }
            serde_json::Value::Object(obj) => {
                let items: Vec<String> = obj
                    .iter()
                    .map(|(k, val)| format!("{}: {}", k, Self::value_to_cypher(val)))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
        }
    }
}
