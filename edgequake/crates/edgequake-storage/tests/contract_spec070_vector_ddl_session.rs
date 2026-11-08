//! SPEC-070 — vector index DDL uses dedicated session GUCs (not query timeout).

#[test]
fn contract_vector_ddl_session_gucs() {
    let ddl = include_str!("../src/adapters/postgres/vector/ddl.rs");
    assert!(
        ddl.contains("apply_vector_ddl_gucs"),
        "vector ddl must expose its DDL session GUC helper"
    );
    // The clause is assembled from a `set` variable so the same code serves
    // `SET` and `SET LOCAL`; assert the fragments rather than one literal.
    assert!(
        ddl.contains("statement_timeout = 0"),
        "vector DDL must clear statement_timeout"
    );
    assert!(
        ddl.contains("lock_timeout = "),
        "vector DDL must set lock_timeout"
    );
    assert!(
        ddl.contains("maintenance_work_mem = "),
        "vector DDL must set maintenance_work_mem"
    );
    assert!(
        ddl.contains("\"SET LOCAL\"") || ddl.contains("SET LOCAL"),
        "vector DDL must support the transaction-scoped form"
    );
    assert!(
        ddl.contains("EDGEQUAKE_INDEX_MAINTENANCE_WORK_MEM"),
        "maintenance_work_mem must be env-tunable"
    );
    assert!(
        ddl.contains("execute_index_ddl"),
        "ANN/FTS index CREATE must go through execute_index_ddl"
    );
    // Query path must not set maintenance_work_mem (ANN search is SET LOCAL only).
    let tuning = include_str!("../src/adapters/postgres/vector/search_tuning.rs");
    assert!(
        !tuning.contains("maintenance_work_mem"),
        "search_tuning must not raise maintenance_work_mem on query path"
    );
}
