//! SPEC-026 Phase 2 — LLM role E2E (extract vs query resolution).

mod common;

use edgequake_api::providers::{LlmResolutionRequest, WorkspaceProviderResolver};
use edgequake_core::{resolve_role_llm, LlmRole, Workspace};
use edgequake_pipeline::{build_ingestion_pipeline, IngestionPipelineOptions};
use std::collections::HashMap;
use uuid::Uuid;

fn workspace_with_roles() -> Workspace {
    let mut metadata = HashMap::new();
    metadata.insert(
        "llm_roles".into(),
        serde_json::json!({
            "extract": { "provider": "mock", "model": "mock-extract" },
            "query": { "provider": "mock", "model": "mock-query" }
        }),
    );
    Workspace {
        workspace_id: Uuid::parse_str(common::TEST_WORKSPACE_ID).unwrap(),
        tenant_id: Uuid::parse_str(common::TEST_TENANT_ID).unwrap(),
        name: "roles".into(),
        slug: "roles".into(),
        description: None,
        is_active: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        metadata,
        llm_model: "gemma3:latest".into(),
        llm_provider: "ollama".into(),
        embedding_model: "mock-embedding".into(),
        embedding_provider: "mock".into(),
        embedding_dimension: 1536,
        vision_llm_model: None,
        vision_llm_provider: None,
        pdf_parser_backend: None,
    }
}

#[test]
fn workspace_extract_role_used_in_ingest_pipeline() {
    let ws = workspace_with_roles();
    let extract = resolve_role_llm(&ws, LlmRole::Extract);
    assert_eq!(extract.provider, "mock");
    assert_eq!(extract.model, "mock-extract");

    let llm =
        edgequake_llm::ProviderFactory::create_llm_provider(&extract.provider, &extract.model)
            .expect("mock extract provider");
    let embedding =
        edgequake_llm::ProviderFactory::create_embedding_provider("mock", "mock-embedding", 1536)
            .expect("mock embedding");
    let pipeline = build_ingestion_pipeline(
        llm,
        embedding,
        edgequake_pipeline::prompts::EntityExtractionSchema::server_default(),
        IngestionPipelineOptions::from_document_size(0),
    );
    assert_eq!(pipeline.config().chunk_strategy.as_str(), "recursive");
}

#[tokio::test]
async fn workspace_query_role_used_in_query_resolution() {
    let ws = workspace_with_roles();
    let state = edgequake_api::AppState::new_memory(None::<String>);
    let resolver = WorkspaceProviderResolver::new(state.workspace_service.clone());

    let resolved = resolver
        .resolve_llm_provider_with_workspace(
            Some(&ws),
            &LlmResolutionRequest {
                provider: None,
                model: None,
                extra_headers: None,
            },
        )
        .expect("resolve")
        .expect("query role provider");

    assert_eq!(resolved.provider_name, "mock");
    assert_eq!(resolved.model_name, "mock-query");
}
