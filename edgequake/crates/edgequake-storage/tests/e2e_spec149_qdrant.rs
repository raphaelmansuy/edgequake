#![cfg(feature = "qdrant")]

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
    client.upsert_points(&model, &points).await.unwrap();

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

    client.delete_points(&[key_a.into_uuid()]).await.unwrap();
    let after_delete = client
        .search(&VectorSearchRequest {
            scope: AccessScope::new(TenantId::new(tenant), WorkspaceId::new(workspace_a)),
            model,
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
