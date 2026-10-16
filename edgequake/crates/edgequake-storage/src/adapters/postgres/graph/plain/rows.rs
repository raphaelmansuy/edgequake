//! Row mapping for plain graph tables.

use std::collections::HashMap;

use serde_json::Value;
use sqlx::Row;

use crate::traits::{GraphEdge, GraphNode};

pub(super) fn lit(value: &str) -> String {
    value.replace('\'', "''")
}

pub(super) fn text_prop(props: &HashMap<String, Value>, key: &str) -> String {
    props
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

pub(super) fn node_from_row(row: &sqlx::postgres::PgRow) -> Option<GraphNode> {
    let id: String = row.try_get("node_id").ok()?;
    if id.is_empty() {
        return None;
    }
    let props: Value = row.try_get("properties").unwrap_or(Value::Null);
    let mut properties = props
        .as_object()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect::<HashMap<_, _>>();
    properties
        .entry("node_id".to_string())
        .or_insert_with(|| Value::String(id.clone()));
    Some(GraphNode { id, properties })
}

pub(super) fn edge_from_row(row: &sqlx::postgres::PgRow) -> Option<GraphEdge> {
    let source: String = row.try_get("source_id").ok()?;
    let target: String = row.try_get("target_id").ok()?;
    if source.is_empty() || target.is_empty() {
        return None;
    }
    let props: Value = row.try_get("properties").unwrap_or(Value::Null);
    let mut properties = props
        .as_object()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect::<HashMap<_, _>>();
    properties
        .entry("source_id".to_string())
        .or_insert_with(|| Value::String(source.clone()));
    properties
        .entry("target_id".to_string())
        .or_insert_with(|| Value::String(target.clone()));
    Some(GraphEdge {
        source,
        target,
        properties,
    })
}

#[derive(Clone, Copy)]
pub(super) enum TenantMode {
    Strict,
    /// Missing tenant/workspace (`''`) still matches (EC-13).
    LegacyEmpty,
}

pub(super) fn tenant_and(
    alias: &str,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
    mode: TenantMode,
) -> String {
    let mut sql = String::new();
    if let Some(tid) = tenant_id {
        let tid = lit(tid);
        sql.push_str(&match mode {
            TenantMode::Strict => format!(" AND {alias}.tenant_id = '{tid}'"),
            TenantMode::LegacyEmpty => {
                format!(" AND ({alias}.tenant_id = '' OR {alias}.tenant_id = '{tid}')")
            }
        });
    }
    if let Some(wid) = workspace_id {
        let wid = lit(wid);
        sql.push_str(&match mode {
            TenantMode::Strict => format!(" AND {alias}.workspace_id = '{wid}'"),
            TenantMode::LegacyEmpty => {
                format!(" AND ({alias}.workspace_id = '' OR {alias}.workspace_id = '{wid}')")
            }
        });
    }
    sql
}
