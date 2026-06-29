//! Vertex/edge property filter SQL — tenant isolation SSOT (SPEC-006 / SPEC-017).
//!
//! All graph read paths that filter AGE tables by tenant/workspace MUST use
//! these builders so isolation semantics stay consistent with `GraphScanOps`
//! and `handlers/isolation.rs`.

use crate::traits::{EdgeListFilter, NodeListFilter};

use super::super::PostgresAGEGraphStorage;

/// Edge tenant filter mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::adapters::postgres::graph) enum EdgeTenantFilterMode {
    /// Strict equality — missing tenant/workspace on edge → excluded (list APIs).
    Strict,
    /// Legacy read compat — NULL tenant/workspace on edge still visible (graph viewer).
    LegacyNullAsWildcard,
}

impl PostgresAGEGraphStorage {
    /// Build a SQL predicate for vertex property filters.
    ///
    /// Returns `TRUE` when no filters are set (admin / internal callers only).
    /// Strict tenant/workspace equality — vertices missing those properties are excluded.
    pub(in crate::adapters::postgres::graph) fn build_vertex_property_where(
        table_alias: &str,
        filter: &NodeListFilter,
    ) -> String {
        let props = format!("ag_catalog.agtype_to_json({table_alias}.properties)");
        let mut conditions = Vec::new();

        if let Some(tid) = filter.tenant_id.as_deref() {
            conditions.push(format!(
                "{props}->>'tenant_id' = '{}'",
                Self::escape_sql_string(tid)
            ));
        }
        if let Some(wid) = filter.workspace_id.as_deref() {
            conditions.push(format!(
                "{props}->>'workspace_id' = '{}'",
                Self::escape_sql_string(wid)
            ));
        }
        if let Some(etype) = filter.entity_type.as_deref() {
            conditions.push(format!(
                "UPPER({props}->>'entity_type') = UPPER('{}')",
                Self::escape_sql_string(etype)
            ));
        }
        if let Some(search) = filter.search.as_deref() {
            let q = Self::escape_sql_string(&search.to_lowercase());
            conditions.push(format!(
                "(LOWER({props}->>'node_id') LIKE '%{q}%' \
                 OR LOWER(COALESCE({props}->>'description', '')) LIKE '%{q}%')"
            ));
        }
        if let Some(ref community_ids) = filter.community_ids {
            if community_ids.is_empty() {
                conditions.push("FALSE".to_string());
            } else {
                let id_list = community_ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                conditions.push(format!("({props}->>'community_id')::bigint IN ({id_list})"));
            }
        }

        if conditions.is_empty() {
            "TRUE".to_string()
        } else {
            conditions.join(" AND ")
        }
    }

    /// `WHERE {predicate}` or empty string when predicate is `TRUE`.
    pub(in crate::adapters::postgres::graph) fn vertex_where_clause(
        table_alias: &str,
        filter: &NodeListFilter,
    ) -> String {
        let predicate = Self::build_vertex_property_where(table_alias, filter);
        if predicate == "TRUE" {
            String::new()
        } else {
            format!("WHERE {predicate}")
        }
    }

    /// Build edge property predicate for tenant/workspace isolation.
    pub(in crate::adapters::postgres::graph) fn build_edge_property_where(
        table_alias: &str,
        filter: &EdgeListFilter,
        mode: EdgeTenantFilterMode,
    ) -> String {
        let props = format!("ag_catalog.agtype_to_json({table_alias}.properties)");
        let mut conditions = Vec::new();

        if let Some(tid) = filter.tenant_id.as_deref() {
            let escaped = Self::escape_sql_string(tid);
            conditions.push(match mode {
                EdgeTenantFilterMode::Strict => {
                    format!("{props}->>'tenant_id' = '{escaped}'")
                }
                EdgeTenantFilterMode::LegacyNullAsWildcard => format!(
                    "({props}->>'tenant_id' IS NULL OR {props}->>'tenant_id' = '{escaped}')"
                ),
            });
        }
        if let Some(wid) = filter.workspace_id.as_deref() {
            let escaped = Self::escape_sql_string(wid);
            conditions.push(match mode {
                EdgeTenantFilterMode::Strict => {
                    format!("{props}->>'workspace_id' = '{escaped}'")
                }
                EdgeTenantFilterMode::LegacyNullAsWildcard => format!(
                    "({props}->>'workspace_id' IS NULL OR {props}->>'workspace_id' = '{escaped}')"
                ),
            });
        }
        if let Some(rel) = filter.relationship_type.as_deref() {
            conditions.push(format!(
                "UPPER({props}->>'relation_type') = UPPER('{}')",
                Self::escape_sql_string(rel)
            ));
        }

        if conditions.is_empty() {
            "TRUE".to_string()
        } else {
            conditions.join(" AND ")
        }
    }

    /// Extra `AND …` clause for edge queries (empty when no filters).
    pub(in crate::adapters::postgres::graph) fn edge_and_clause(
        table_alias: &str,
        filter: &EdgeListFilter,
        mode: EdgeTenantFilterMode,
    ) -> String {
        let predicate = Self::build_edge_property_where(table_alias, filter, mode);
        if predicate == "TRUE" {
            String::new()
        } else {
            format!(" AND {predicate}")
        }
    }

    /// `SELECT COUNT(*) …` for vertices matching `filter` (analytics SSOT).
    pub(in crate::adapters::postgres::graph) fn vertex_count_sql(
        graph_name: &str,
        table_alias: &str,
        filter: &NodeListFilter,
    ) -> String {
        format!(
            "SELECT COUNT(*)::bigint FROM {graph_name}.\"_ag_label_vertex\" {table_alias} {}",
            Self::vertex_where_clause(table_alias, filter)
        )
    }

    /// `SELECT COUNT(*) …` for edges matching tenant/workspace (strict).
    pub(in crate::adapters::postgres::graph) fn edge_count_sql(
        graph_name: &str,
        table_alias: &str,
        filter: &EdgeListFilter,
    ) -> String {
        let predicate =
            Self::build_edge_property_where(table_alias, filter, EdgeTenantFilterMode::Strict);
        if predicate == "TRUE" {
            format!("SELECT COUNT(*)::bigint FROM {graph_name}.\"_ag_label_edge\" {table_alias}")
        } else {
            format!(
                "SELECT COUNT(*)::bigint FROM {graph_name}.\"_ag_label_edge\" {table_alias} WHERE {predicate}"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::PostgresAGEGraphStorage;
    use super::EdgeTenantFilterMode;
    use crate::traits::{EdgeListFilter, NodeListFilter};

    #[test]
    fn tenant_workspace_strict_equality() {
        let filter = NodeListFilter {
            tenant_id: Some("t1".into()),
            workspace_id: Some("w1".into()),
            ..Default::default()
        };
        let sql = PostgresAGEGraphStorage::build_vertex_property_where("v", &filter);
        assert!(sql.contains("tenant_id' = 't1'"));
        assert!(sql.contains("workspace_id' = 'w1'"));
        assert!(!sql.contains("IS NULL"));
    }

    #[test]
    fn empty_filter_is_true_predicate() {
        let sql =
            PostgresAGEGraphStorage::build_vertex_property_where("v", &NodeListFilter::default());
        assert_eq!(sql, "TRUE");
    }

    #[test]
    fn edge_strict_vs_legacy_tenant_filter() {
        let filter = EdgeListFilter {
            tenant_id: Some("t1".into()),
            workspace_id: Some("w1".into()),
            relationship_type: None,
        };
        let strict = PostgresAGEGraphStorage::build_edge_property_where(
            "e",
            &filter,
            EdgeTenantFilterMode::Strict,
        );
        assert!(strict.contains("tenant_id' = 't1'"));
        assert!(!strict.contains("IS NULL"));

        let legacy = PostgresAGEGraphStorage::build_edge_property_where(
            "e",
            &filter,
            EdgeTenantFilterMode::LegacyNullAsWildcard,
        );
        assert!(legacy.contains("IS NULL"));
    }
}
