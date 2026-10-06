//! SPEC-083 / X-03: SQL fragments for eq_* endpoints with property fallback.
//!
//! LAW-2: hot paths must not assume denormalized columns are populated.
//! When columns exist, `COALESCE(eq_*, props->>…)` covers NULL backfill.
//! When columns are absent, callers must use [`prop_only_endpoint`] fragments.

/// Endpoint expression when `eq_*` columns exist (NULL-safe).
pub(in crate::adapters::postgres::graph) fn coalesce_endpoint(alias: &str, side: &str) -> String {
    // side: "source" | "target" | "node" (node uses eq_node_id / node_id)
    match side {
        "node" => format!(
            "COALESCE({a}.eq_node_id, ag_catalog.agtype_to_json({a}.properties)->>'node_id')",
            a = alias
        ),
        "source" => format!(
            "COALESCE({a}.eq_source_id, ag_catalog.agtype_to_json({a}.properties)->>'source_id')",
            a = alias
        ),
        "target" => format!(
            "COALESCE({a}.eq_target_id, ag_catalog.agtype_to_json({a}.properties)->>'target_id')",
            a = alias
        ),
        other => panic!("eq_id_sql: unknown side {other}"),
    }
}

/// Endpoint expression when `eq_*` columns are absent (property path only).
pub(in crate::adapters::postgres::graph) fn prop_only_endpoint(alias: &str, side: &str) -> String {
    match side {
        "node" => format!(
            "(ag_catalog.agtype_to_json({a}.properties)->>'node_id')",
            a = alias
        ),
        "source" => format!(
            "(ag_catalog.agtype_to_json({a}.properties)->>'source_id')",
            a = alias
        ),
        "target" => format!(
            "(ag_catalog.agtype_to_json({a}.properties)->>'target_id')",
            a = alias
        ),
        other => panic!("eq_id_sql: unknown side {other}"),
    }
}

/// Project degree endpoints without decoding properties on populated native rows.
/// The disjoint UNION ALL branches preserve COALESCE semantics and duplicates,
/// while letting PostgreSQL use the native endpoint's covering partial index.
pub(in crate::adapters::postgres::graph) fn degree_endpoint_rows(
    graph: &str,
    side: &str,
    eq_present: bool,
) -> String {
    assert!(matches!(side, "source" | "target"));
    let property = prop_only_endpoint("e", side);
    if !eq_present {
        return format!("(SELECT {property} AS node_id FROM {graph}.\"EDGE\" e)");
    }
    format!(
        "(SELECT e.eq_{side}_id AS node_id FROM {graph}.\"EDGE\" e \
         WHERE e.eq_{side}_id IS NOT NULL UNION ALL \
         SELECT {property} AS node_id FROM {graph}.\"EDGE\" e \
         WHERE e.eq_{side}_id IS NULL)"
    )
}

/// True when EDGEQUAKE_EQ_ID_FALLBACK forces property-aware SQL (COALESCE / prop-only).
pub(in crate::adapters::postgres::graph) fn eq_id_fallback_env_enabled() -> bool {
    matches!(
        std::env::var("EDGEQUAKE_EQ_ID_FALLBACK").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_degrees_coalesce_eq_and_props() {
        // Matrix Cluster 00: COALESCE(eq_*, props) for readiness/fallback parity.
        let s = coalesce_endpoint("e", "source");
        assert!(s.contains("eq_source_id"));
        assert!(s.contains("source_id"));
        assert!(s.contains("COALESCE"));
        let node = coalesce_endpoint("n", "node");
        assert!(node.contains("eq_node_id"));
        assert!(node.contains("COALESCE"));
    }

    #[test]
    fn prop_only_has_no_eq_column() {
        let s = prop_only_endpoint("e", "target");
        assert!(!s.contains("eq_target_id"));
        assert!(s.contains("target_id"));
    }
}
