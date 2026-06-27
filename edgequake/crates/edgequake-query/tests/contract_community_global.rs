//! SPEC-023 I6 — community global expansion uses index-time labels (default-on).

use std::collections::HashMap;
use std::sync::Arc;

use edgequake_query::community_global;
use edgequake_query::QueryEngineConfig;
use edgequake_storage::traits::GraphStorage;
use edgequake_storage::{community_features_enabled, GraphReadView, MemoryGraphStorage};
use serde_json::json;

#[tokio::test]
async fn contract_community_global_expands_co_community_entities() {
    assert!(
        community_features_enabled(),
        "community features must be enabled by default"
    );

    let graph: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("comm-global"));
    graph.initialize().await.unwrap();

    let mut seed_props = HashMap::new();
    seed_props.insert("community_id".to_string(), json!(1_u64));
    seed_props.insert("entity_type".to_string(), json!("ORG"));
    graph.upsert_node("SEED_ORG", seed_props).await.unwrap();

    let mut sibling_props = HashMap::new();
    sibling_props.insert("community_id".to_string(), json!(1_u64));
    sibling_props.insert("entity_type".to_string(), json!("ORG"));
    graph
        .upsert_node("SIBLING_ORG", sibling_props)
        .await
        .unwrap();
    graph
        .upsert_edge("SEED_ORG", "SIBLING_ORG", HashMap::new())
        .await
        .unwrap();

    let mut other_props = HashMap::new();
    other_props.insert("community_id".to_string(), json!(2_u64));
    other_props.insert("entity_type".to_string(), json!("ORG"));
    graph.upsert_node("OTHER_ORG", other_props).await.unwrap();

    let config = QueryEngineConfig::default();
    let mut context = edgequake_query::QueryContext::new();
    let mut entity_ids = vec!["SEED_ORG".to_string()];

    community_global::expand_global_context_with_communities(
        &config,
        &mut context,
        &mut entity_ids,
        GraphReadView::new(graph.as_ref()),
        None,
        None,
    )
    .await
    .expect("community expansion");

    let names: Vec<_> = context.entities.iter().map(|e| e.name.as_str()).collect();
    assert!(
        names.contains(&"SIBLING_ORG"),
        "must expand with co-community entity, got {names:?}"
    );
    assert!(
        !names.contains(&"OTHER_ORG"),
        "must not expand into unrelated community, got {names:?}"
    );
}

#[test]
fn contract_community_global_default_and_env_opt_out() {
    std::env::remove_var("EDGEQUAKE_COMMUNITY_GLOBAL");
    assert!(
        community_features_enabled(),
        "community features must be enabled by default"
    );

    std::env::set_var("EDGEQUAKE_COMMUNITY_GLOBAL", "false");
    assert!(
        !community_features_enabled(),
        "EDGEQUAKE_COMMUNITY_GLOBAL=false must disable community features"
    );
    std::env::remove_var("EDGEQUAKE_COMMUNITY_GLOBAL");
}
