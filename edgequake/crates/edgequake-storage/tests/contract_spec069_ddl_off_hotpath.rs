//! SPEC-069 — eq_* DDL is boot-owned; delete/ingest hot path must not race schema.

#[test]
fn contract_ensure_indexes_single_flight_and_verified_flag() {
    let mod_src = include_str!("../src/adapters/postgres/graph/mod.rs");
    assert!(
        mod_src.contains("ensure_indexes_lock"),
        "PostgresAGEGraphStorage must hold a single-flight lock for ensure_indexes"
    );

    let life = include_str!("../src/adapters/postgres/graph/helpers/graph_lifecycle.rs");
    assert!(
        life.contains("ensure_indexes_lock.lock()"),
        "ensure_indexes must acquire single-flight lock"
    );
    assert!(
        life.contains("indexes_verified.load") && life.contains("indexes_verified.store"),
        "ensure_indexes must early-exit / set indexes_verified when eq_* ready"
    );
    // SPEC-069/098: ensure_eq_id_columns must short-circuit on an O(1) catalog
    // probe. The probe is `eq_id_schema_ready`; the log line has been reworded
    // since (now "eq_* schema present"), so assert the structure, not the prose.
    let ensure_fn = life
        .split("async fn ensure_eq_id_columns")
        .nth(1)
        .expect("ensure_eq_id_columns");
    let early_exit_head = &ensure_fn[..ensure_fn.len().min(600)];
    assert!(
        early_exit_head.contains("eq_id_schema_ready"),
        "ensure_eq_id_columns must catalog early-exit when columns/indexes/triggers exist"
    );
    assert!(
        early_exit_head.contains("return Ok(())"),
        "the catalog probe must short-circuit before any DDL"
    );
    // D-30: readiness must require multigraph arbiter (not legacy 2-col alone).
    let ready_fn = life
        .split("async fn eq_id_schema_ready")
        .nth(1)
        .and_then(|s| s.split("async fn ensure_eq_id_columns").next())
        .expect("eq_id_schema_ready before ensure_eq_id_columns");
    assert!(
        ready_fn.contains("eq_rel_type")
            && ready_fn.contains("indexname = 'idx_edge_eq_source_target_rel'"),
        "eq_id_schema_ready must require eq_rel_type + idx_edge_eq_source_target_rel"
    );
    assert!(
        !ready_fn.contains("indexname = 'idx_edge_eq_source_target'"),
        "eq_id_schema_ready must not treat legacy 2-col idx as ready"
    );
    assert!(
        !life.contains("DROP TRIGGER IF EXISTS trg_eq_sync_node_id")
            && !life.contains("DROP TRIGGER IF EXISTS trg_eq_sync_edge_ids"),
        "must not DROP TRIGGER on every ensure (hot-path race under query timeout)"
    );
    // Triggers moved to helpers::sync_triggers and use CREATE OR REPLACE, so
    // the original `if !node_trig` form is gone. The invariant that mattered —
    // never DROP TRIGGER on the hot path — is asserted above and still holds.
    let sync_triggers = include_str!("../src/adapters/postgres/graph/helpers/sync_triggers.rs");
    assert!(
        sync_triggers.contains("CREATE OR REPLACE TRIGGER trg_eq_sync_node_id")
            && sync_triggers.contains("CREATE OR REPLACE TRIGGER trg_eq_sync_edge_ids"),
        "sync triggers must be created with CREATE OR REPLACE (no DROP on the hot path)"
    );
    assert!(
        !sync_triggers.contains("DROP TRIGGER"),
        "sync trigger module must never DROP TRIGGER on the hot path"
    );
}

#[test]
fn contract_ddl_session_disables_query_timeout() {
    let session = include_str!("../src/adapters/postgres/graph/helpers/session.rs");
    assert!(
        session.contains("setup_age_ddl_session"),
        "DDL session helper required"
    );
    assert!(
        session.contains("SET statement_timeout = 0") && session.contains("lock_timeout"),
        "DDL path must use statement_timeout=0 + lock_timeout"
    );
    let life = include_str!("../src/adapters/postgres/graph/helpers/graph_lifecycle.rs");
    assert!(
        life.contains("setup_age_ddl_session"),
        "ensure_indexes must use DDL session GUCs (not query 15s timeout)"
    );
}

#[test]
fn contract_pg_initialize_boot_owns_indexes_verified() {
    let life = include_str!("../src/adapters/postgres/graph/lifecycle_ops.rs");
    assert!(
        life.contains("ensure_indexes") && life.contains("indexes_verified"),
        "pg_initialize must run ensure_indexes and track indexes_verified"
    );
    assert!(
        life.contains("eq_id_schema_ready"),
        "boot must re-check catalog readiness after concurrent bootstrap"
    );
}

#[test]
fn contract_native_upsert_fail_closed_without_eq_schema() {
    let mutate = include_str!("../src/adapters/postgres/graph/nodes_ops/mutate.rs");
    assert!(
        mutate.contains("graph schema not bootstrapped (eq_id)"),
        "native node upsert must fail closed if eq_* not verified after ensure"
    );
    let edges = include_str!("../src/adapters/postgres/graph/edges_ops.rs");
    assert!(
        edges.contains("graph schema not bootstrapped (eq_id)"),
        "native edge upsert must fail closed if eq_* not verified after ensure"
    );
    // Must not force-set verified=true after ensure (that hid incomplete schema).
    assert!(
        !mutate.contains("indexes_verified.store(true, Ordering::Relaxed)"),
        "mutate must not force indexes_verified=true; ensure_indexes owns the flag"
    );
}
