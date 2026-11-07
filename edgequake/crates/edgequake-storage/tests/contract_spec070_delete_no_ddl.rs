//! SPEC-070 — delete/cascade hot path must not migrate schema mid-task.

#[test]
fn contract_cascade_and_deletion_avoid_schema_ddl() {
    let cascade = include_str!("../../edgequake-api/src/services/document_graph_cascade.rs");
    let deletion = include_str!("../../edgequake-api/src/services/document_deletion.rs");
    for (name, src) in [("cascade", cascade), ("deletion", deletion)] {
        assert!(
            !src.contains("ensure_eq_id_columns") && !src.contains("ensure_indexes"),
            "{name} must not call graph schema ensure/DDL (boot-owned, SPEC-069/070)"
        );
        assert!(
            !src.contains("ALTER TABLE") && !src.contains("CREATE TRIGGER"),
            "{name} must not embed schema DDL SQL"
        );
    }
}

#[test]
fn contract_discovery_gin_join_still_required() {
    let scan = include_str!("../src/adapters/postgres/graph/scan_ops.rs");
    // Issue #478 / migration 207: discovery moved off inline `@> to_jsonb(...)`
    // probes (which the tenant role cannot plan against the GIN index, because
    // the AGE guard calls non-leakproof agtype_to_json) into a SECURITY DEFINER
    // lookup that applies the session tenant inside the indexed plan.
    assert!(
        scan.contains("eq_age_lineage_discover_nodes"),
        "discovery must use the migration-207 owner lookup for the indexed plan"
    );
    // The GIN-eligible probe shape lives in the owner-side SQL: `?|` against
    // the jsonb source-id arrays, driven by an unnest+generate_series probe CTE.
    let discover = include_str!("../../../migrations/207_age_lineage_discovery_rls.sql");
    assert!(
        discover.contains("?|"),
        "the owner-side discovery SQL must use a GIN-eligible array-contains probe"
    );
    assert!(
        discover.contains("generate_series") && discover.contains("unnest"),
        "discovery must build its probes with unnest + generate_series"
    );
    assert!(
        scan.contains("source_prefix_legacy_enabled"),
        "the inline probe shape must survive behind the legacy escape hatch"
    );
}
