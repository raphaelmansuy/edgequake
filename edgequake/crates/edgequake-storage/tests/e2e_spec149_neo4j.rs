#![cfg(feature = "neo4j")]

#[path = "common/tenant_measurements.rs"]
mod tenant_measurements;
use edgequake_storage::{
    GraphObjectPayload, Neo4jClient, Neo4jConfig, Neo4jEdgeRevision, Neo4jEntityRevision,
};
use edgequake_storage_contracts::{
    AccessScope, EdgeDirection, GraphEdgeKey, GraphNodeKey, IncidentEdgesRequest, ScopedGraphRead,
    TenantId, WorkspaceId,
};
use serde_json::Map;
use uuid::Uuid;

fn client_or_skip() -> Option<Neo4jClient> {
    let require = std::env::var("EDGEQUAKE_REQUIRE_NEO4J")
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes"));
    let Some(url) = std::env::var("NEO4J_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        assert!(
            !require,
            "EDGEQUAKE_REQUIRE_NEO4J is set but NEO4J_URL is missing"
        );
        eprintln!("skipping Neo4j integration test: NEO4J_URL is not set");
        return None;
    };
    let user = std::env::var("EDGEQUAKE_NEO4J_USER").unwrap_or_else(|_| "neo4j".into());
    let password =
        std::env::var("EDGEQUAKE_NEO4J_PASSWORD").unwrap_or_else(|_| "provider-access-test".into());
    let database = std::env::var("EDGEQUAKE_NEO4J_DATABASE").unwrap_or_else(|_| "neo4j".into());
    let config = Neo4jConfig::new(url, database, user, password).expect("valid Neo4j test config");
    Some(Neo4jClient::new(config).expect("Neo4j HTTP client"))
}

#[tokio::test]
async fn immutable_edge_as_node_round_trip_and_exact_delete() {
    let Some(client) = client_or_skip() else {
        return;
    };
    client
        .provision_schema()
        .await
        .expect("provision Neo4j test schema");

    let scope = AccessScope::new(
        TenantId::new(Uuid::new_v4()),
        WorkspaceId::new(Uuid::new_v4()),
    );
    let node = GraphNodeKey::new(Uuid::new_v4());
    let entity = Neo4jEntityRevision {
        scope,
        logical_id: node,
        physical_id: Uuid::new_v4(),
        revision: 1,
        digest: [1; 32],
        contribution_manifest: vec![Uuid::new_v4()],
        properties: Map::new(),
    };
    let edge = Neo4jEdgeRevision {
        scope,
        logical_id: GraphEdgeKey::new(Uuid::new_v4()),
        physical_id: Uuid::new_v4(),
        source: node,
        target: node,
        relationship_type: "SELF_TEST".into(),
        direction: EdgeDirection::Directed,
        revision: 1,
        digest: [2; 32],
        contribution_manifest: vec![Uuid::new_v4()],
        properties: Map::new(),
    };

    client
        .upsert_graph_revisions(std::slice::from_ref(&entity), std::slice::from_ref(&edge))
        .await
        .expect("write immutable graph revisions");
    client
        .upsert_graph_revisions(std::slice::from_ref(&entity), std::slice::from_ref(&edge))
        .await
        .expect("same digest must replay");

    let page = client
        .incident_edges(&IncidentEdgesRequest {
            scope,
            node_ids: vec![node],
            cursor: None,
            limit: 10,
        })
        .await
        .expect("read incident edges");
    assert_eq!(page.items.len(), 1, "self-loop is one logical edge");
    assert_eq!(page.items[0].key.into_uuid(), edge.physical_id);

    for foreign_scope in [
        AccessScope::new(TenantId::new(Uuid::new_v4()), scope.workspace()),
        AccessScope::new(scope.tenant(), WorkspaceId::new(Uuid::new_v4())),
    ] {
        let foreign_page = client
            .incident_edges(&IncidentEdgesRequest {
                scope: foreign_scope,
                node_ids: vec![node],
                cursor: None,
                limit: 10,
            })
            .await
            .unwrap();
        assert!(foreign_page.items.is_empty());
        assert!(client
            .revision_digests(&foreign_scope, &[entity.physical_id, edge.physical_id])
            .await
            .unwrap()
            .is_empty());
        assert_eq!(
            client
                .delete_edge_revisions(&foreign_scope, &[edge.physical_id])
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            client
                .delete_entity_revisions(&foreign_scope, &[entity.physical_id])
                .await
                .unwrap(),
            0
        );
    }
    let mut forged = entity.clone();
    forged.scope = AccessScope::new(
        TenantId::new(Uuid::new_v4()),
        WorkspaceId::new(Uuid::new_v4()),
    );
    assert!(client.upsert_graph_revisions(&[forged], &[]).await.is_err());
    assert_eq!(
        client
            .incident_edges(&IncidentEdgesRequest {
                scope,
                node_ids: vec![node],
                cursor: None,
                limit: 10
            })
            .await
            .unwrap()
            .items
            .len(),
        1
    );

    let digests = client
        .revision_digests(&scope, &[entity.physical_id, edge.physical_id])
        .await
        .unwrap();
    assert_eq!(digests.len(), 2);
    let mut samples = Vec::new();
    for i in 0..26 {
        let start = std::time::Instant::now();
        let page = client
            .incident_edges(&IncidentEdgesRequest {
                scope,
                node_ids: vec![node],
                cursor: None,
                limit: 10,
            })
            .await
            .unwrap();
        assert_eq!(page.items.len(), 1);
        if i >= 5 {
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    tenant_measurements::record("neo4j", 2, samples);
    let deleted = client
        .delete_edge_revisions(&scope, &[edge.physical_id])
        .await
        .expect("delete exact edge revision");
    assert_eq!(deleted, 1);
    client
        .delete_entity_revisions(&scope, &[entity.physical_id])
        .await
        .expect("delete exact entity revision");
}

#[test]
fn graph_payload_wire_shape_is_versionable_json() {
    let payload = GraphObjectPayload::Entity {
        logical_id: GraphNodeKey::new(Uuid::from_u128(1)),
        properties: Map::new(),
    };
    let encoded = serde_json::to_value(payload).unwrap();
    assert_eq!(encoded["kind"], "entity");
}
