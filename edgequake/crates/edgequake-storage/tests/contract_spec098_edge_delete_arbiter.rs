//! SPEC-098 LAW-098-13: edge delete matches trigger `eq_rel_type` formula — no
//! Rust/Postgres dual-case heuristics.

#[test]
fn contract_delete_edges_batch_uses_trigger_upper_arbiter() {
    let edges = include_str!("../src/adapters/postgres/graph/edges_ops.rs");
    let start = edges
        .find("pub(super) async fn pg_delete_edges_batch")
        .expect("pg_delete_edges_batch");
    let body = &edges[start..];
    let end = body
        .find("pub(super) async fn pg_delete_edge_scoped")
        .unwrap_or(body.len().min(8000));
    let delete_fn = &body[..end];

    assert!(
        delete_fn.contains("sql_eq_rel_type_arbiter_expr")
            || delete_fn.contains("UPPER(COALESCE(NULLIF(TRIM("),
        "delete must use trigger-identical UPPER(COALESCE(NULLIF(TRIM(...)))) arbiter"
    );
    assert!(
        !delete_fn.contains("to_ascii_uppercase"),
        "delete must not use ASCII-only upper (French é/É drift)"
    );
    // Flaky dual-upper on already-COALESCE'd row expr is not the SSOT.
    assert!(
        !delete_fn.contains("UPPER(TRIM({rel_expr}))")
            && !delete_fn.contains("UPPER(TRIM(rel_expr))"),
        "delete must compare row arbiter key = pair arbiter key, not UPPER(TRIM(both))"
    );
}

#[test]
fn contract_sql_eq_rel_type_arbiter_expr_matches_trigger_shape() {
    let expr = edgequake_storage::sql_eq_rel_type_arbiter_expr("pairs.rel_type");
    assert_eq!(
        expr,
        "UPPER(COALESCE(NULLIF(TRIM(pairs.rel_type), ''), 'RELATED_TO'))"
    );
    let lifecycle = include_str!("../src/adapters/postgres/graph/helpers/graph_lifecycle.rs");
    assert!(
        lifecycle.contains("self.ensure_sync_triggers(conn).await?"),
        "graph lifecycle must provision the shared sync trigger implementation"
    );
    let triggers = include_str!("../src/adapters/postgres/graph/helpers/sync_triggers.rs");
    assert!(
        triggers.contains("NEW.eq_rel_type := UPPER(COALESCE(")
            && triggers.contains("NULLIF(TRIM(")
            && triggers.contains("'RELATED_TO'"),
        "trigger must keep the same UPPER(COALESCE(NULLIF(TRIM))) shape"
    );
}
