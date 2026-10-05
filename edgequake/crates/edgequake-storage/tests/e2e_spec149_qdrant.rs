#![cfg(feature = "qdrant")]

#[path = "common/tenant_measurements.rs"]
mod tenant_measurements;
use chrono::{Duration, Utc};
use edgequake_storage::{
    contracts::{
        AccessScope, DocumentId, EmbeddingKey, ScopedVectorSearch, TenantId, VectorModelDescriptor,
        VectorSearchMode, VectorSearchRequest, WorkspaceId,
    },
    drop_qdrant_binding, provision_qdrant_binding, QdrantClient, QdrantPointPayload,
    QdrantVectorPoint,
};
use uuid::Uuid;

fn qdrant_url() -> Option<String> {
    std::env::var("QDRANT_URL")
        .ok()
        .or_else(|| std::env::var("EDGEQUAKE_VECTOR_URL").ok())
        .filter(|value| !value.trim().is_empty())
}

fn require_qdrant() -> bool {
    std::env::var("EDGEQUAKE_REQUIRE_QDRANT")
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "on"))
}

#[tokio::test]
async fn provider_access_qdrant_upsert_scope_search_and_delete() {
    let Some(url) = qdrant_url() else {
        assert!(
            !require_qdrant(),
            "EDGEQUAKE_REQUIRE_QDRANT is set but QDRANT_URL/EDGEQUAKE_VECTOR_URL is absent"
        );
        return;
    };

    let binding_id = Uuid::new_v4();
    let client = QdrantClient::from_env_url(url, binding_id).unwrap();
    let model = VectorModelDescriptor {
        provider: "spec149".into(),
        model: "fixture-3d".into(),
        version: "r1".into(),
        dimensions: 3,
        metric: "cosine".into(),
    };
    provision_qdrant_binding(&client, &model).await.unwrap();

    let tenant = Uuid::new_v4();
    let workspace_a = Uuid::new_v4();
    let workspace_b = Uuid::new_v4();
    let document_a = DocumentId::new(Uuid::new_v4());
    let document_b = DocumentId::new(Uuid::new_v4());
    let key_a = EmbeddingKey::new(Uuid::new_v4());
    let key_b = EmbeddingKey::new(Uuid::new_v4());
    let points = [
        point(
            key_a,
            tenant,
            workspace_a,
            document_a,
            Uuid::new_v4(),
            vec![1.0, 0.0, 0.0],
        ),
        point(
            key_b,
            tenant,
            workspace_b,
            document_b,
            Uuid::new_v4(),
            vec![1.0, 0.0, 0.0],
        ),
    ];
    let scope_a = AccessScope::new(TenantId::new(tenant), WorkspaceId::new(workspace_a));
    let scope_b = AccessScope::new(TenantId::new(tenant), WorkspaceId::new(workspace_b));
    client
        .upsert_points(&scope_a, &model, &points[..1])
        .await
        .unwrap();
    client
        .upsert_points(&scope_b, &model, &points[1..])
        .await
        .unwrap();
    assert!(client
        .upsert_points(&scope_a, &model, &points[1..])
        .await
        .is_err());
    // Delete with a foreign ID must leave that scope's point intact.
    client
        .delete_points(&scope_a, &[key_b.into_uuid()])
        .await
        .unwrap();

    let page = client
        .search(&VectorSearchRequest {
            scope: AccessScope::new(TenantId::new(tenant), WorkspaceId::new(workspace_a)),
            model: model.clone(),
            family: "chunk".into(),
            embedding: vec![1.0, 0.0, 0.0],
            top_k: 10,
            document_ids: None,
            modalities: None,
            filter_ids: None,
            threshold: None,
            search_mode: VectorSearchMode::Exact,
            deadline: Utc::now() + Duration::seconds(5),
            scan_budget: 100,
        })
        .await
        .unwrap();

    assert_eq!(page.hits.len(), 1);
    assert_eq!(page.hits[0].key, key_a);

    client
        .delete_points(&scope_a, &[key_a.into_uuid()])
        .await
        .unwrap();
    let after_delete = client
        .search(&VectorSearchRequest {
            scope: AccessScope::new(TenantId::new(tenant), WorkspaceId::new(workspace_a)),
            model: model.clone(),
            family: "chunk".into(),
            embedding: vec![1.0, 0.0, 0.0],
            top_k: 10,
            document_ids: None,
            modalities: None,
            filter_ids: None,
            threshold: None,
            search_mode: VectorSearchMode::Exact,
            deadline: Utc::now() + Duration::seconds(5),
            scan_budget: 100,
        })
        .await
        .unwrap();
    assert!(after_delete.hits.is_empty());

    let mut foreign_request = VectorSearchRequest {
        scope: scope_b,
        model: model.clone(),
        family: "chunk".into(),
        embedding: vec![1.0, 0.0, 0.0],
        top_k: 10,
        document_ids: None,
        modalities: None,
        filter_ids: None,
        threshold: None,
        search_mode: VectorSearchMode::Exact,
        deadline: Utc::now() + Duration::seconds(5),
        scan_budget: 100,
    };
    assert_eq!(
        client.search(&foreign_request).await.unwrap().hits[0].key,
        key_b
    );
    foreign_request.scope =
        AccessScope::new(TenantId::new(Uuid::new_v4()), WorkspaceId::new(workspace_b));
    assert!(client
        .search(&foreign_request)
        .await
        .unwrap()
        .hits
        .is_empty());
    // Reusing a foreign revision UUID creates a distinct physical point, never an overwrite.
    let mut collision = points[0].clone();
    collision.key = key_b;
    client
        .upsert_points(&scope_a, &model, &[collision])
        .await
        .unwrap();
    foreign_request.scope = scope_b;
    assert_eq!(
        client.search(&foreign_request).await.unwrap().hits[0].subject_id,
        points[1].payload.subject_id
    );
    foreign_request.model.version = "unpublished-model".into();
    assert!(client
        .search(&foreign_request)
        .await
        .unwrap()
        .hits
        .is_empty());
    foreign_request.model = model.clone();
    // Shared collection: 1,000 own rows, plus the foreign workspace row above.
    let fixture = (0..1000)
        .map(|_| {
            point(
                EmbeddingKey::new(Uuid::new_v4()),
                tenant,
                workspace_a,
                document_a,
                Uuid::new_v4(),
                vec![1.0, 0.0, 0.0],
            )
        })
        .collect::<Vec<_>>();
    client
        .upsert_points(&scope_a, &model, &fixture)
        .await
        .unwrap();
    foreign_request.scope = scope_a;
    let mut samples = Vec::new();
    for i in 0..26 {
        foreign_request.deadline = Utc::now() + Duration::seconds(5);
        let start = std::time::Instant::now();
        let page = client.search(&foreign_request).await.unwrap();
        assert_eq!(page.hits.len(), 10);
        assert!(page
            .hits
            .iter()
            .all(|h| h.subject_id != points[1].payload.subject_id));
        if i >= 5 {
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    tenant_measurements::record("qdrant", 1002, samples);
    drop_qdrant_binding(&client).await.unwrap();
}

fn point(
    key: EmbeddingKey,
    tenant_id: Uuid,
    workspace_id: Uuid,
    document_id: DocumentId,
    subject_id: Uuid,
    vector: Vec<f32>,
) -> QdrantVectorPoint {
    QdrantVectorPoint {
        key,
        vector,
        payload: QdrantPointPayload {
            tenant_id,
            workspace_id,
            family: "chunk".into(),
            subject_id,
            document_id,
            contribution_ref: format!("document:{document_id}"),
            model_revision: "r1".into(),
            content_revision: 1,
            manifest_id: Uuid::new_v4(),
            digest: format!("digest:{key}"),
            modality: Some("text".into()),
        },
    }
}
