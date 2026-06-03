//! Cypher/SQL string formatting and property literals (SPEC-017 P1-12).

use std::collections::HashMap;

use super::super::PostgresAGEGraphStorage;

impl PostgresAGEGraphStorage {
    pub(in crate::adapters::postgres::graph) fn escape_cypher_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    pub(in crate::adapters::postgres::graph) fn escape_sql_string(s: &str) -> String {
        s.replace('\'', "''")
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
