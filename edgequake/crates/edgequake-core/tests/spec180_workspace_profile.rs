//! SPEC-180: workspace profile at the service boundary (in-memory twin, EC-180-59).

use edgequake_core::rag_profile::{codes, EmbeddingModalities, FtsRegconfig, RagProfile};
use edgequake_core::workspace_service::InMemoryWorkspaceService;
use edgequake_core::{
    CreateWorkspaceRequest, Error, Tenant, UpdateWorkspaceRequest, Workspace, WorkspaceService,
};
use uuid::Uuid;

async fn tenant(service: &InMemoryWorkspaceService) -> Uuid {
    service
        .create_tenant(Tenant::new("T180", format!("t180-{}", Uuid::new_v4())))
        .await
        .unwrap()
        .tenant_id
}

fn named(name: &str) -> CreateWorkspaceRequest {
    CreateWorkspaceRequest::new(name)
}

async fn create(
    service: &InMemoryWorkspaceService,
    t: Uuid,
    req: CreateWorkspaceRequest,
) -> Workspace {
    service.create_workspace(t, req).await.unwrap()
}

#[tokio::test]
async fn t_180_create_default_graph() {
    let service = InMemoryWorkspaceService::new();
    let t = tenant(&service).await;
    let ws = create(&service, t, named("plain")).await;
    assert_eq!(ws.rag_profile, RagProfile::Graph);
    assert_eq!(ws.embedding_modalities, EmbeddingModalities::Text);
    assert_eq!(ws.fts_regconfig, FtsRegconfig::English);
    assert!(ws.capabilities().extracts_graph);
}

#[tokio::test]
async fn create_normal_with_regconfig_round_trips() {
    let service = InMemoryWorkspaceService::new();
    let t = tenant(&service).await;
    let req = CreateWorkspaceRequest {
        fts_regconfig: Some(FtsRegconfig::French),
        ..named("fr").with_rag_profile(RagProfile::Normal)
    };
    let ws = create(&service, t, req).await;
    let got = service
        .get_workspace(ws.workspace_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.rag_profile, RagProfile::Normal);
    assert_eq!(got.fts_regconfig, FtsRegconfig::French);
    assert!(!got.capabilities().extracts_graph);
    assert!(got.capabilities().writes_bm25);
}

#[tokio::test]
async fn t_180_modalities_graph_is_refused() {
    let service = InMemoryWorkspaceService::new();
    let t = tenant(&service).await;
    let req = named("gi").with_embedding_modalities(EmbeddingModalities::TextImage);
    let err = service.create_workspace(t, req).await.unwrap_err();
    assert!(
        matches!(err, Error::Validation(ref m) if m.starts_with(codes::MODALITIES_REQUIRE_NORMAL))
    );
}

#[tokio::test]
async fn normal_text_image_is_accepted() {
    let service = InMemoryWorkspaceService::new();
    let t = tenant(&service).await;
    let req = named("ni")
        .with_rag_profile(RagProfile::Normal)
        .with_embedding_modalities(EmbeddingModalities::TextImage);
    let ws = create(&service, t, req).await;
    assert!(ws.capabilities().embeds_images);
}

#[tokio::test]
async fn t_180_memory_immutable_rejects_every_profile_field_even_unchanged() {
    let service = InMemoryWorkspaceService::new();
    let t = tenant(&service).await;
    let ws = create(
        &service,
        t,
        named("imm").with_rag_profile(RagProfile::Normal),
    )
    .await;
    let cases = [
        (
            UpdateWorkspaceRequest {
                rag_profile: Some("normal".into()),
                ..Default::default()
            },
            codes::RAG_PROFILE_IMMUTABLE,
        ),
        (
            UpdateWorkspaceRequest {
                rag_profile: Some("graph".into()),
                ..Default::default()
            },
            codes::RAG_PROFILE_IMMUTABLE,
        ),
        (
            UpdateWorkspaceRequest {
                embedding_modalities: Some("text".into()),
                ..Default::default()
            },
            codes::EMBEDDING_MODALITIES_IMMUTABLE,
        ),
        (
            UpdateWorkspaceRequest {
                fts_regconfig: Some("english".into()),
                ..Default::default()
            },
            codes::FTS_REGCONFIG_IMMUTABLE,
        ),
    ];
    for (req, code) in cases {
        let err = service
            .update_workspace(ws.workspace_id, req)
            .await
            .unwrap_err();
        assert!(
            matches!(err, Error::Conflict(ref m) if m.starts_with(code)),
            "expected {code}, got {err:?}"
        );
    }
    let still = service
        .get_workspace(ws.workspace_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(still.rag_profile, RagProfile::Normal);
}

#[tokio::test]
async fn t_180_update_name_ok_keeps_profile() {
    let service = InMemoryWorkspaceService::new();
    let t = tenant(&service).await;
    let ws = create(
        &service,
        t,
        named("before").with_rag_profile(RagProfile::Normal),
    )
    .await;
    let updated = service
        .update_workspace(
            ws.workspace_id,
            UpdateWorkspaceRequest {
                name: Some("after".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.name, "after");
    assert_eq!(updated.rag_profile, RagProfile::Normal);
}
