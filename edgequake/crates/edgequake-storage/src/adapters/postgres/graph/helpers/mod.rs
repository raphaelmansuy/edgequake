//! Internal helpers for Apache AGE graph operations (SPEC-017 P1-12).
//!
//! Split by responsibility:
//! - `session` — AGE bootstrap + dollar-quote safety
//! - `cypher_exec` — query / execute / batch SQL
//! - `age_parse` — agtype → GraphNode/GraphEdge
//! - `cypher_format` — escaping and property literals
//! - `graph_lifecycle` — graph DDL and indexes

mod age_parse;
mod agtype_bind;
mod cypher_exec;
mod cypher_format;
mod eq_id_sql;
mod escape;
mod graph_lifecycle;
mod session;
mod source_lineage_sql;
mod vertex_filter;

pub(in crate::adapters::postgres::graph) use eq_id_sql::{
    coalesce_endpoint, eq_id_fallback_env_enabled, prop_only_endpoint,
};

pub(in crate::adapters::postgres::graph) use source_lineage_sql::{
    jsonb_matches_doc_source_prefix_legacy, normalize_doc_chunk_prefix,
    source_ids_count_probes_cte_sql, source_ids_probes_cte_sql, SOURCE_CHUNK_PROBE_LIMIT,
};

#[cfg(test)]
pub(in crate::adapters::postgres::graph) use source_lineage_sql::jsonb_matches_doc_source_prefix_modern;

pub(in crate::adapters::postgres::graph) use vertex_filter::{
    EdgeTenantFilterMode, VertexTenantFilterMode,
};

#[cfg(test)]
mod helper_tests {
    use super::super::PostgresAGEGraphStorage;

    #[test]
    fn test_dollar_quote_tag_default_when_absent() {
        let body = "MATCH (n:Node {node_id: 'abc'}) RETURN n";
        assert_eq!(PostgresAGEGraphStorage::dollar_quote_tag(body), "$eqcy$");
    }

    #[test]
    fn test_dollar_quote_tag_avoids_collision_with_base() {
        let body = "RETURN '$eqcy$ injection attempt'";
        let tag = PostgresAGEGraphStorage::dollar_quote_tag(body);
        assert_ne!(tag, "$eqcy$");
        assert!(!body.contains(&tag), "chosen tag must be absent from body");
    }

    #[test]
    fn test_dollar_quote_tag_avoids_double_dollar_injection() {
        let body = "SET n.desc = 'ends here $$ DROP TABLE foo; --'";
        let tag = PostgresAGEGraphStorage::dollar_quote_tag(body);
        assert!(!body.contains(&tag));
        assert_eq!(tag, "$eqcy$");
    }

    #[test]
    fn test_dollar_quote_tag_escalates_through_numbered_tags() {
        let body = "x $eqcy$ y $eqcy0$ z";
        let tag = PostgresAGEGraphStorage::dollar_quote_tag(body);
        assert!(!body.contains(&tag));
        assert_eq!(tag, "$eqcy1$");
    }

    #[test]
    fn test_age_session_setup_is_single_batched_statement() {
        let sql = PostgresAGEGraphStorage::age_session_setup_sql();
        assert!(sql.contains("LOAD 'age'"));
        assert!(sql.contains("search_path = ag_catalog"));
        assert!(sql.contains("statement_timeout"));
        assert_eq!(sql.matches(';').count(), 3);
    }
}
