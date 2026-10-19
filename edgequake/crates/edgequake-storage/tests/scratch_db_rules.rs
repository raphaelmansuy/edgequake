//! Safety rules of the scratch-database rebuild (`support/scratch_db.rs`).
#![cfg(feature = "postgres")]

#[path = "support/scratch_db.rs"]
mod scratch_db;

use scratch_db::is_disposable;

#[test]
fn only_scratch_databases_are_disposable() {
    assert!(is_disposable("edgequake_test"));
    assert!(is_disposable("edgequake_lineage_check_test"));
    for kept in [
        "edgequake",
        "postgres",
        "_test",
        "test",
        "edgequake_test_backup",
        "",
    ] {
        assert!(!is_disposable(kept), "`{kept}` must never be dropped");
    }
}
