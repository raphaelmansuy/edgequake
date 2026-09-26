//! Contract: source-prefix discovery must use child-table GIN JOIN probes (SPEC-071).
//!
//! Regression for lineage/delete timeouts:
//! `Source-prefix edge query failed: canceling statement due to statement timeout`

#[test]
fn contract_find_nodes_by_source_prefixes_uses_gin_join_probes() {
    let scan = include_str!("../src/adapters/postgres/graph/scan_ops.rs");
    assert!(
        scan.contains("generate_series"),
        "modern discovery must probe chunk ids via generate_series JOIN"
    );
    assert!(
        scan.contains("unnest($1::text[])"),
        "modern discovery must unnest exact source ids for GIN @>"
    );
    assert!(
        scan.contains("pg_find_nodes_by_source_prefixes"),
        "node discovery entrypoint must remain"
    );
    assert!(
        scan.contains("pg_find_edges_by_source_prefixes"),
        "edge discovery entrypoint must remain"
    );
}

#[test]
fn contract_modern_discovery_targets_child_node_and_edge_tables() {
    let scan = include_str!("../src/adapters/postgres/graph/scan_ops.rs");

    // Hot path must query indexed child tables, not AGE parents.
    assert!(
        scan.contains(r#"."Node" v"#) || scan.contains(".\"Node\" v"),
        "node discovery must FROM child \"Node\""
    );
    assert!(
        scan.contains(r#"."EDGE" e"#) || scan.contains(".\"EDGE\" e"),
        "edge discovery must FROM child \"EDGE\""
    );

    // Extract modern edge SQL region (between modern_sql for edges and legacy enable check).
    let edge_fn = scan
        .find("pub(super) async fn pg_find_edges_by_source_prefixes")
        .expect("edge discovery fn");
    let edge_body = &scan[edge_fn..];
    let modern_end = edge_body
        .find("if Self::source_prefix_legacy_enabled()")
        .unwrap_or(edge_body.len().min(4500));
    let modern_edge = &edge_body[..modern_end];

    assert!(
        modern_edge.contains("eq_source_id") && modern_edge.contains("eq_target_id"),
        "edge modern path must resolve endpoints via eq_* (no parent text-cast JOINs)"
    );
    assert!(
        !modern_edge.contains("_ag_label_edge"),
        "edge modern hot path must not scan AGE parent _ag_label_edge"
    );
    assert!(
        !modern_edge.contains("start_id::text"),
        "edge modern hot path must not text-cast JOIN on start_id"
    );
}

#[test]
fn contract_legacy_source_prefix_not_unconditional() {
    let scan = include_str!("../src/adapters/postgres/graph/scan_ops.rs");
    assert!(
        scan.contains("EDGEQUAKE_SOURCE_PREFIX_LEGACY"),
        "legacy residual path must be gated by env"
    );
    assert!(
        scan.contains("fn source_prefix_legacy_enabled"),
        "legacy enable helper must exist"
    );
    // Legacy must not run unconditionally after modern (old bug).
    assert!(
        scan.contains("if Self::source_prefix_legacy_enabled()"),
        "legacy SQL must be behind source_prefix_legacy_enabled()"
    );
    // Giant-OR modern helper removed from hot path (dead / SeqScan risk).
    assert!(
        !scan.contains("fn build_source_prefix_clause_modern"),
        "unused giant-OR modern helper must stay removed"
    );
}

#[test]
fn contract_modern_source_prefix_helper_probes_indexed_lineage_arrays() {
    let helper = include_str!("../src/adapters/postgres/graph/helpers/source_lineage_sql.rs");
    // Modern fn body only: from its signature to the first top-level `}`.
    let start = helper
        .find("fn jsonb_matches_doc_source_prefix_modern")
        .expect("modern helper");
    let end = start
        + helper[start..]
            .find("\n}\n")
            .expect("modern helper body end");
    let modern = &helper[start..end];
    // SPEC-149: source_chunk_ids has its own GIN (idx_*_source_chunk_ids_gin);
    // probing it is required for durable-committer documents that omit source_ids.
    assert!(
        modern.contains("INDEXED_LINEAGE_ARRAY_KEYS"),
        "modern helper must iterate the shared indexed lineage keys"
    );
    assert_eq!(
        edgequake_storage::INDEXED_LINEAGE_ARRAY_KEYS,
        ["source_ids", "source_chunk_ids"],
        "both GIN-indexed lineage arrays must be probed"
    );
    assert!(
        !modern.contains("LIKE"),
        "modern helper must stay GIN-only (no LIKE)"
    );
}

/// Migration 157 and runtime DDL must bound the same four lineage GIN indexes.
#[cfg(feature = "postgres")]
#[test]
fn contract_lineage_gin_pending_limit_matches_migration_157() {
    let migration = include_str!("../../../migrations/157_graph_lineage_gin_pending_limit.sql");
    let support = include_str!("../../../migrations/support/157/apply.sql");
    assert_eq!(migration, support, "support/157/apply.sql must mirror M157");
    let limit = format!(
        "gin_pending_list_limit = {}",
        edgequake_storage::LINEAGE_GIN_PENDING_LIST_LIMIT_KB
    );
    assert!(migration.contains(&limit), "M157 must set {limit}");
    for index in edgequake_storage::LINEAGE_GIN_INDEXES {
        assert!(migration.contains(index), "M157 must bound {index}");
    }
}
