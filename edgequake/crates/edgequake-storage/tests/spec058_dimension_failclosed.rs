//! E2E / contract: vector dimension reconcile (SPEC-058 + PreferExisting boot).
//!
//! @implements SPEC-058 fail-closed · empty heal · PreferExisting at AppState boot

#[test]
fn contract_policy_module_exports_prefer_existing() {
    use edgequake_storage::{decide_dimension_action, DimensionAction, DimensionReconcilePolicy};

    assert_eq!(
        decide_dimension_action(
            Some(1024),
            768,
            false,
            false,
            DimensionReconcilePolicy::PreferExisting
        ),
        DimensionAction::KeepExisting
    );
    assert_eq!(
        decide_dimension_action(
            Some(1024),
            768,
            false,
            false,
            DimensionReconcilePolicy::FailClosed
        ),
        DimensionAction::FailClosed
    );
    assert_eq!(
        decide_dimension_action(
            Some(1024),
            768,
            true,
            false,
            DimensionReconcilePolicy::FailClosed
        ),
        DimensionAction::RecreateEmpty
    );
}

#[test]
fn contract_ensure_dimension_fail_closed_source() {
    let src = include_str!("../src/adapters/postgres/vector/migration.rs");
    assert!(src.contains("EDGEQUAKE_ALLOW_VECTOR_TABLE_REBUILD"));
    assert!(src.contains("Refusing DROP TABLE"));
    assert!(src.contains("PreferExisting"));
    assert!(src.contains("RecreateEmpty"));
    assert!(src.contains(r#""1" | "true" | "yes" | "on""#));
}

#[test]
fn contract_boot_uses_prefer_existing() {
    // The reconcile moved with the provider refactor: the vector provider owns
    // boot-time dimension policy and AppState::new_postgres delegates to it.
    let provider = include_str!("../../edgequake-api/src/state/data_access_providers/postgres.rs");
    assert!(
        provider.contains("DimensionReconcilePolicy::PreferExisting"),
        "the PostgreSQL vector provider must PreferExisting on the default vector table"
    );
    assert!(
        provider.contains("KeptExisting"),
        "boot must rebind on KeptExisting"
    );
    let state = include_str!("../../edgequake-api/src/state/postgres.rs");
    assert!(
        state.contains("PreferExisting"),
        "AppState must document that its vector factory reconciles with PreferExisting"
    );
}
