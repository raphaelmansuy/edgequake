//! Unified workspace pipeline resolution (SPEC-017).
//!
//! Single source of truth for building workspace-scoped ingestion pipelines.
//! Replaces duplicated logic in `AppState::create_workspace_pipeline` and
//! `DocumentTaskProcessor::get_workspace_pipeline*`.

use std::sync::Arc;

use edgequake_core::WorkspaceService;
use edgequake_pipeline::{LLMExtractor, Pipeline};
use tracing::{error, info, warn};

use crate::safety_limits::{create_safe_embedding_provider, create_safe_llm_provider};

/// Policy when workspace provider resolution fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineFallbackPolicy {
    /// Return the global default pipeline (upload handlers, legacy tests).
    LenientGlobal,
    /// Fail with an explicit error (production task processor).
    Strict,
}

/// Builds workspace-scoped pipelines with explicit fallback semantics.
pub struct WorkspacePipelineFactory {
    workspace_service: Arc<dyn WorkspaceService>,
    global_pipeline: Arc<Pipeline>,
}

impl WorkspacePipelineFactory {
    pub fn new(
        workspace_service: Arc<dyn WorkspaceService>,
        global_pipeline: Arc<Pipeline>,
    ) -> Self {
        Self {
            workspace_service,
            global_pipeline,
        }
    }

    /// Resolve a pipeline for the given workspace ID string.
    pub async fn resolve(
        &self,
        workspace_id: &str,
        policy: PipelineFallbackPolicy,
    ) -> Result<Arc<Pipeline>, String> {
        let workspace_uuid = match crate::middleware::resolve_workspace_uuid(Some(workspace_id)) {
            Some(uuid) => uuid,
            None => {
                return self.handle_failure(
                    policy,
                    format!("Invalid workspace ID format '{}'", workspace_id),
                );
            }
        };

        let ws = match self.workspace_service.get_workspace(workspace_uuid).await {
            Ok(Some(ws)) => ws,
            Ok(None) => {
                return self
                    .handle_failure(policy, format!("Workspace '{}' not found", workspace_id));
            }
            Err(e) => {
                return self.handle_failure(
                    policy,
                    format!("Failed to lookup workspace '{}': {}", workspace_id, e),
                );
            }
        };

        let llm_provider = create_safe_llm_provider(&ws.llm_provider, &ws.llm_model);
        let embedding_provider = create_safe_embedding_provider(
            &ws.embedding_provider,
            &ws.embedding_model,
            ws.embedding_dimension,
        );

        match (llm_provider, embedding_provider) {
            (Ok(llm), Ok(embedding)) => {
                info!(
                    workspace_id = workspace_id,
                    llm_model = %ws.llm_full_id(),
                    embedding_model = %ws.embedding_full_id(),
                    "Resolved workspace-specific ingestion pipeline"
                );

                let entity_schema =
                    edgequake_pipeline::prompts::EntityExtractionSchema::from_workspace_metadata(
                        &ws.metadata,
                    );
                let extractor = Arc::new(LLMExtractor::new(llm).with_entity_schema(entity_schema));
                Ok(Arc::new(
                    Pipeline::default_pipeline()
                        .with_extractor(extractor)
                        .with_embedding_provider(embedding),
                ))
            }
            (Err(llm_err), Ok(_)) => {
                error!(
                    workspace_id = workspace_id,
                    error = %llm_err,
                    "Failed to create workspace LLM provider"
                );
                self.handle_failure(
                    policy,
                    format!(
                        "Failed to create LLM provider '{}' / '{}': {}",
                        ws.llm_provider, ws.llm_model, llm_err
                    ),
                )
            }
            (Ok(_), Err(embed_err)) => {
                error!(
                    workspace_id = workspace_id,
                    error = %embed_err,
                    "Failed to create workspace embedding provider"
                );
                self.handle_failure(
                    policy,
                    format!(
                        "Failed to create embedding provider '{}' / '{}': {}",
                        ws.embedding_provider, ws.embedding_model, embed_err
                    ),
                )
            }
            (Err(llm_err), Err(embed_err)) => {
                error!(
                    workspace_id = workspace_id,
                    llm_error = %llm_err,
                    embedding_error = %embed_err,
                    "Failed to create both workspace providers"
                );
                self.handle_failure(
                    policy,
                    format!(
                        "Failed to create LLM ({}) and embedding ({}) providers",
                        llm_err, embed_err
                    ),
                )
            }
        }
    }

    fn handle_failure(
        &self,
        policy: PipelineFallbackPolicy,
        message: String,
    ) -> Result<Arc<Pipeline>, String> {
        match policy {
            PipelineFallbackPolicy::Strict => Err(message),
            PipelineFallbackPolicy::LenientGlobal => {
                warn!(
                    error = %message,
                    "Using global default pipeline (lenient fallback policy)"
                );
                Ok(Arc::clone(&self.global_pipeline))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_core::types::{CreateWorkspaceRequest, Tenant};
    use edgequake_core::InMemoryWorkspaceService;
    use uuid::Uuid;

    fn test_factory() -> (WorkspacePipelineFactory, Arc<Pipeline>) {
        let global = Arc::new(Pipeline::default_pipeline());
        let ws = Arc::new(InMemoryWorkspaceService::new());
        let factory = WorkspacePipelineFactory::new(ws, Arc::clone(&global));
        (factory, global)
    }

    #[tokio::test]
    async fn strict_mode_fails_on_invalid_workspace_id() {
        let (factory, _) = test_factory();
        match factory
            .resolve("not-a-valid-uuid", PipelineFallbackPolicy::Strict)
            .await
        {
            Err(err) => assert!(err.contains("Invalid workspace ID")),
            Ok(_) => panic!("expected strict resolution to fail"),
        }
    }

    #[tokio::test]
    async fn lenient_mode_falls_back_on_invalid_workspace_id() {
        let (factory, global) = test_factory();
        let pipeline = factory
            .resolve("not-a-valid-uuid", PipelineFallbackPolicy::LenientGlobal)
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&pipeline, &global));
    }

    #[tokio::test]
    async fn strict_mode_fails_when_workspace_missing() {
        let (factory, _) = test_factory();
        let missing = Uuid::new_v4();
        match factory
            .resolve(&missing.to_string(), PipelineFallbackPolicy::Strict)
            .await
        {
            Err(err) => assert!(err.contains("not found")),
            Ok(_) => panic!("expected strict resolution to fail"),
        }
    }

    #[tokio::test]
    async fn resolves_workspace_with_mock_providers() {
        let global = Arc::new(Pipeline::default_pipeline());
        let ws_service = Arc::new(InMemoryWorkspaceService::new());
        let tenant = ws_service
            .create_tenant(Tenant::new("tenant", format!("tenant-{}", Uuid::new_v4())))
            .await
            .unwrap();

        let workspace = ws_service
            .create_workspace(
                tenant.tenant_id,
                CreateWorkspaceRequest {
                    name: "test-ws".to_string(),
                    slug: Some(format!("test-ws-{}", Uuid::new_v4())),
                    llm_provider: Some("mock".to_string()),
                    llm_model: Some("mock-model".to_string()),
                    embedding_provider: Some("mock".to_string()),
                    embedding_model: Some("mock-embedding".to_string()),
                    embedding_dimension: Some(1536),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let factory = WorkspacePipelineFactory::new(ws_service, Arc::clone(&global));
        let pipeline = factory
            .resolve(
                &workspace.workspace_id.to_string(),
                PipelineFallbackPolicy::Strict,
            )
            .await
            .expect("mock providers should resolve");
        assert!(!Arc::ptr_eq(&pipeline, &global));
    }
}
