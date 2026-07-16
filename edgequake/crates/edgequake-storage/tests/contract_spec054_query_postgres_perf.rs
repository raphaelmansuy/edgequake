//! SPEC-054 / specs/054 — Query · Postgres · AGE · pgvector performance invariants.
//!
//! Wiring contracts (no live DB). Budgets that need Postgres live in
//! `e2e_spec054_query_perf_smoke.rs`.
//!
//! Cross-ref: `specs/054-fix-bugs-17/`.

#[test]
fn contract_filtered_query_modes_use_query_filtered() {
    let local = include_str!("../../edgequake-query/src/engine_impl/modes/local.rs");
    let global = include_str!("../../edgequake-query/src/engine_impl/modes/global.rs");
    let naive = include_str!("../../edgequake-query/src/engine_impl/modes/naive.rs");
    let chunk = include_str!("../../edgequake-query/src/engine_impl/modes/chunk_retrieval.rs");
    for (name, src) in [
        ("local", local),
        ("global", global),
        ("naive", naive),
        ("chunk_retrieval", chunk),
    ] {
        assert!(
            src.contains("query_filtered"),
            "{name} must use query_filtered for scoped ANN (iterative_scan path)"
        );
    }
}

#[test]
fn contract_hybrid_mix_delegate_to_filtered_arms() {
    // Hybrid/Mix do not call query_filtered directly — they fan out to
    // local/global/naive arms that own the filtered ANN path (DRY).
    let hybrid = include_str!("../../edgequake-query/src/engine_impl/modes/hybrid.rs");
    let mix = include_str!("../../edgequake-query/src/engine_impl/modes/mix.rs");
    for (name, src) in [("hybrid", hybrid), ("mix", mix)] {
        assert!(
            src.contains("query_local_with_vector_storage"),
            "{name} must delegate to local arm (query_filtered)"
        );
        assert!(
            src.contains("query_global_with_vector_storage"),
            "{name} must delegate to global arm (query_filtered)"
        );
        assert!(
            src.contains("query_naive_with_vector_storage"),
            "{name} must delegate to naive arm (query_filtered)"
        );
    }
}

#[test]
fn contract_docs_054_complexity_and_alignment_exist() {
    let catalog = include_str!("../../../../specs/054-fix-bugs-17/005-query-complexity-catalog.md");
    assert!(catalog.contains("node_counts_by_source_prefixes"));
    assert!(catalog.contains("FORBIDDEN"));
    let align = include_str!("../../../../specs/054-fix-bugs-17/006-july-2026-alignment.md");
    assert!(align.contains("0.8.2"));
    assert!(align.contains("EDGEQUAKE_HNSW_EF_CONSTRUCTION"));
}

#[test]
fn contract_batched_lineage_counts_wired() {
    let analytics = include_str!("../src/adapters/postgres/graph/analytics_ops.rs");
    assert!(analytics.contains("pg_node_counts_by_source_prefixes"));
    let trait_src = include_str!("../src/traits/graph_analytics_ops.rs");
    assert!(trait_src.contains("node_counts_by_source_prefixes"));
    let read_model = include_str!("../../edgequake-api/src/document_read_model.rs");
    assert!(
        read_model.contains("node_counts_by_source_prefixes"),
        "documents list must use batched AGE reconcile (SPEC-054 L1-a)"
    );
    assert!(
        !read_model.contains(".node_count_by_source_prefix(&prefix)"),
        "per-doc N+1 prefix loop must not remain in document_read_model"
    );
}

#[test]
fn contract_search_tuning_enables_iterative_scan_when_filtered() {
    let src = include_str!("../src/adapters/postgres/vector/search_tuning.rs");
    assert!(src.contains("hnsw.iterative_scan"));
    assert!(src.contains("max_scan_tuples"));
    assert!(src.contains("relaxed_order"));
    assert!(src.contains("pgvector_supports_iterative_scan"));
    assert!(
        src.contains("if filtered && iterative_scan_supported"),
        "iterative_scan must be gated on filtered + version support"
    );
}

#[test]
fn contract_vector_count_uses_stats_not_raw_scan_first() {
    let src = include_str!("../src/adapters/postgres/vector/storage_impl.rs");
    assert!(
        src.contains("SELECT row_count FROM"),
        "count() must prefer stats table (QUERY_CATALOG VEC-03 mitigated)"
    );
    assert!(
        src.contains("ensure_row_count_stats"),
        "stats self-heal must exist"
    );
}

#[test]
fn contract_native_upsert_targets_unique_index_names() {
    let nodes = include_str!("../src/adapters/postgres/graph/nodes_ops/mutate.rs");
    let edges = include_str!("../src/adapters/postgres/graph/edges_ops.rs");
    assert!(nodes.contains("idx_node_prop_node_id_unique"));
    assert!(edges.contains("idx_edge_source_target_unique") || edges.contains("source_id"));
    assert!(nodes.contains("pg_upsert_nodes_batch_native"));
    assert!(edges.contains("pg_upsert_edges_batch_native"));
}

#[test]
fn contract_escape_ssot_and_bound_cypher_hot_reads() {
    let escape = include_str!("../src/adapters/postgres/graph/helpers/escape.rs");
    assert!(escape.contains("fn escape_cypher_string"));
    assert!(escape.contains("fn escape_sql_literal"));
    let cypher_fmt = include_str!("../src/adapters/postgres/graph/helpers/cypher_format.rs");
    assert!(
        cypher_fmt.contains("super::escape::"),
        "cypher_format must delegate to escape SSOT"
    );
    let read = include_str!("../src/adapters/postgres/graph/nodes_ops/read.rs");
    assert!(
        read.contains("cypher_query_bound"),
        "hot node reads must use bound Cypher"
    );
    let isp = include_str!("../src/adapters/postgres/graph/nodes_ops/mod.rs");
    assert!(isp.contains("GraphStorageReadOps") || isp.contains("read"));
}

#[test]
fn contract_bootstrap_skips_dedup_when_unique_valid() {
    let lifecycle =
        include_str!("../src/adapters/postgres/graph/helpers/graph_lifecycle.rs");
    assert!(
        lifecycle.contains("index_validity"),
        "must probe pg_index.indisvalid before O(N) dedup"
    );
    assert!(
        lifecycle.contains("already valid — skip"),
        "valid UNIQUE must short-circuit dedup/create"
    );
    assert!(
        lifecycle.contains("dedup_nodes_for_unique_index"),
        "dedup remains for missing/INVALID index path"
    );
}

#[test]
fn contract_m083_support_skips_when_unique_exists() {
    let apply = include_str!("../../../migrations/support/083/apply.sql");
    assert!(
        apply.contains("already exists"),
        "M083 boot SSOT must skip O(N) work when UNIQUE exists"
    );
    assert!(
        apply.contains("skip dedup"),
        "M083 must document skip dedup/ANALYZE fast path"
    );
    assert!(
        apply.contains("CHECKSUM SAFETY") || apply.contains("checksum"),
        "M083 support must warn not to edit locked sqlx migration"
    );
    // Frozen sqlx migration must remain present (checksum-locked). Do not require
    // byte-identity with support/ — boot SSOT may diverge for fast-path.
    let locked = include_str!("../../../migrations/083_age_native_unique_index_reconcile.sql");
    assert!(locked.contains("idx_node_prop_node_id_unique"));
}

#[test]
fn contract_docs_054_pack_exists() {
    let readme = include_str!("../../../../specs/054-fix-bugs-17/README.md");
    assert!(readme.contains("First Principles"));
    assert!(readme.contains("pgvector"));
    let fp = include_str!("../../../../specs/054-fix-bugs-17/001-first-principles.md");
    assert!(fp.contains("iterative_scan"));
    assert!(fp.contains("O(N)"));
}

#[test]
fn contract_native_graph_writes_default_on() {
    let src = include_str!("../src/adapters/postgres/graph/mod.rs");
    assert!(
        src.contains("Unset → enabled") || src.contains("Err(_) => true"),
        "native_graph_writes must default ON (specs/054 best performance)"
    );
    assert!(
        src.contains("\"0\" | \"false\" | \"off\" | \"no\""),
        "must support explicit opt-out"
    );
}
