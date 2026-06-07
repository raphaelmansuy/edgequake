//! MetadataFilter DRY contract: in-memory predicate fields align with SQL builder (SPEC-017).

use edgequake_storage::MetadataFilter;

#[test]
fn build_sql_emits_condition_per_active_field() {
    let mf = MetadataFilter {
        document_ids: Some(vec!["doc-a".into()]),
        tenant_id: Some("t1".into()),
        workspace_id: Some("ws1".into()),
        vector_type: Some("chunk".into()),
    };

    let with_ids = mf.build_sql(true, 2);
    assert_eq!(with_ids.conditions.len(), 5);
    assert!(with_ids
        .conditions
        .iter()
        .any(|c| c.contains("document_id")));
    assert!(with_ids.conditions.iter().any(|c| c.contains("tenant_id")));
    assert!(with_ids
        .conditions
        .iter()
        .any(|c| c.contains("workspace_id")));
    assert!(with_ids
        .conditions
        .iter()
        .any(|c| c.contains("metadata->>'type'")));

    let without_ids = mf.build_sql(false, 2);
    assert_eq!(without_ids.conditions.len(), 4);
}

#[test]
fn matches_and_sql_share_workspace_semantics() {
    let mf =
        MetadataFilter::from_tenant_workspace_type(Some("t1".into()), Some("ws1".into()), "chunk")
            .unwrap();

    assert!(mf.matches(&serde_json::json!({
        "tenant_id": "t1",
        "workspace_id": "ws1",
        "type": "chunk"
    })));
    assert!(!mf.matches(&serde_json::json!({
        "tenant_id": "t1",
        "workspace_id": "ws2",
        "type": "chunk"
    })));

    let sql = mf.build_sql(false, 2);
    assert_eq!(sql.conditions.len(), 3);
}
