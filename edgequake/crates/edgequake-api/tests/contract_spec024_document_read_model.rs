//! SPEC-024 Phase 4.6 — document read-model reconciliation SSOT contracts.

#[test]
fn contract_list_documents_merges_relational_backfill() {
    let list = include_str!("../src/handlers/documents/query/list.rs");
    assert!(
        list.contains("document_read_model::merge_document_summaries"),
        "documents list must merge relational rows missing from KV (G4)"
    );
    assert!(
        list.contains("document_read_model::reconcile_entity_counts_with_graph"),
        "documents list must reconcile entity counts with AGE graph"
    );
}

#[test]
fn contract_workspace_stats_merges_document_counts() {
    let stats = include_str!("../src/handlers/workspaces/stats.rs");
    assert!(
        stats.contains("document_read_model::merge_document_count"),
        "workspace stats must use max(postgresql, kv) merge"
    );
}

#[test]
fn contract_document_read_model_exports_merge_strategy() {
    let model = include_str!("../src/document_read_model.rs");
    assert!(
        model.contains("MERGE_STRATEGY"),
        "read model must expose operator-visible merge strategy"
    );
    assert!(
        model.contains("detect_document_drift"),
        "read model must expose drift detection helper"
    );
}
