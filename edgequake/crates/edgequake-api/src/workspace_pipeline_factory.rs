//! Unified workspace pipeline resolution (SPEC-017).
//!
//! Single source of truth for building workspace-scoped ingestion pipelines.
//! Replaces duplicated logic in `AppState::create_workspace_pipeline` and
//! `DocumentTaskProcessor::get_workspace_pipeline*`.

use std::sync::Arc;

use edgequake_core::WorkspaceService;
use edgequake_pipeline::extractor::decision::DecisionRuntime;
use edgequake_pipeline::{
    build_ingestion_pipeline, ExtractionMode, IngestionPipelineOptions, Pipeline,
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::workspace_pipeline_decision::{build_decision_pipeline, resolve_mode};

use crate::safety_limits::{
    create_safe_embedding_provider, create_safe_extraction_llm_provider, is_slow_local_provider,
};

/// Policy when workspace provider resolution fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineFallbackPolicy {
    /// Return the global default pipeline (upload handlers, legacy tests).
    LenientGlobal,
    /// Fail with an explicit error (production task processor).
    Strict,
}

/// SPEC-116: workspace chunking policy before document `chunk_options` (both modes).
fn with_workspace_chunking(
    options: IngestionPipelineOptions,
    ws: &edgequake_core::Workspace,
) -> IngestionPipelineOptions {
    match edgequake_pipeline::chunking_policy_from_metadata(&ws.metadata) {
        Some(policy) => options.with_chunking_policy(policy),
        None => options,
    }
}

/// Builds workspace-scoped pipelines with explicit fallback semantics.
pub struct WorkspacePipelineFactory {
    workspace_service: Arc<dyn WorkspaceService>,
    global_pipeline: Arc<Pipeline>,
    /// SPEC-160: present when the caller can run decision extraction.
    decision: Option<DecisionRuntime>,
    /// SPEC-160: lets a cancelled task stop decision questions in flight.
    cancel: CancellationToken,
    #[cfg(feature = "postgres")]
    pg_pool: Option<sqlx::PgPool>,
}

impl WorkspacePipelineFactory {
    pub fn new(
        workspace_service: Arc<dyn WorkspaceService>,
        global_pipeline: Arc<Pipeline>,
    ) -> Self {
        Self {
            workspace_service,
            global_pipeline,
            decision: None,
            cancel: CancellationToken::new(),
            #[cfg(feature = "postgres")]
            pg_pool: None,
        }
    }

    /// Enable the decision extraction mode (SPEC-160).
    pub fn with_decision(mut self, decision: Option<DecisionRuntime>) -> Self {
        self.decision = decision;
        self
    }

    /// Stop decision questions when this token is cancelled.
    pub fn with_cancellation(mut self, cancel: CancellationToken) -> Self {
        self.cancel = cancel;
        self
    }

    #[cfg(feature = "postgres")]
    pub fn with_pg_pool(mut self, pool: Option<sqlx::PgPool>) -> Self {
        self.pg_pool = pool;
        self
    }

    /// Resolve a pipeline for the given workspace ID string.
    pub async fn resolve(
        &self,
        workspace_id: &str,
        policy: PipelineFallbackPolicy,
    ) -> Result<Arc<Pipeline>, String> {
        self.resolve_for_ingestion(
            workspace_id,
            policy,
            IngestionPipelineOptions::from_document_size(0),
        )
        .await
    }

    /// Resolve a document-scoped pipeline (adaptive chunk + gleaning).
    pub async fn resolve_for_ingestion(
        &self,
        workspace_id: &str,
        policy: PipelineFallbackPolicy,
        options: IngestionPipelineOptions,
    ) -> Result<Arc<Pipeline>, String> {
        let workspace_uuid = match crate::middleware::resolve_workspace_uuid(Some(workspace_id)) {
            Some(uuid) => uuid,
            None => {
                return self.handle_failure(
                    policy,
                    format!("Invalid workspace ID format '{}'", workspace_id),
                    &options,
                );
            }
        };

        let ws = match self.workspace_service.get_workspace(workspace_uuid).await {
            Ok(Some(ws)) => ws,
            Ok(None) => {
                return self.handle_failure(
                    policy,
                    format!("Workspace '{}' not found", workspace_id),
                    &options,
                );
            }
            Err(e) => {
                return self.handle_failure(
                    policy,
                    format!("Failed to lookup workspace '{}': {}", workspace_id, e),
                    &options,
                );
            }
        };

        let tenant = self
            .workspace_service
            .get_tenant(ws.tenant_id)
            .await
            .ok()
            .flatten();

        // SPEC-160: resolve the mode first. A mode error never falls back (LAW-160-4).
        let resolved_mode = resolve_mode(&ws, &options)?;
        if resolved_mode.mode == ExtractionMode::Decision {
            let options = with_workspace_chunking(options, &ws);
            return build_decision_pipeline(
                self.decision.as_ref(),
                &ws,
                tenant.as_ref(),
                options,
                self.cancel.clone(),
            )
            .await;
        }

        // SPEC-086: EXTRACT≠QUERY — env pin beats workspace llm_roles.extract.
        let extract_role = edgequake_core::resolve_extract_role_llm(&ws);
        let llm_provider = {
            let mut connected: Option<Arc<dyn edgequake_llm::LLMProvider>> = None;
            #[cfg(feature = "postgres")]
            {
                if let (Some(raw), Some(pool)) =
                    (extract_role.connection_id.as_deref(), self.pg_pool.as_ref())
                {
                    if let Some(id) = crate::providers::connection_store::parse_connection_id(raw) {
                        if let Ok(spec) = crate::providers::connection_store::load_connection_spec(
                            pool,
                            id,
                            &extract_role.model,
                        )
                        .await
                        {
                            connected =
                                crate::providers::connection_factory::llm_from_connection(&spec)
                                    .ok();
                        }
                    }
                }
            }
            match connected {
                Some(p) => Ok(p),
                None => {
                    create_safe_extraction_llm_provider(&extract_role.provider, &extract_role.model)
                }
            }
        };
        // SPEC-123: embedding via SSOT (metadata gate + tenant), not painted DTO alone.
        let emb =
            edgequake_core::resolve_embedding_choice(None, None, None, Some(&ws), tenant.as_ref());
        let embedding_provider =
            create_safe_embedding_provider(&emb.provider, &emb.model, emb.dimension);

        match (llm_provider, embedding_provider) {
            (Ok(llm), Ok(embedding)) => {
                let options = options.with_llm_provider(&extract_role.provider);
                let tuned = edgequake_pipeline::PipelineConfig::from_env_for_provider(
                    &extract_role.provider,
                );
                let requested_concurrent = std::env::var("EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS")
                    .ok()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(tuned.max_concurrent_extractions);
                let (_, local_concurrency_clamped) =
                    edgequake_pipeline::apply_local_concurrency_safety_clamp(
                        &extract_role.provider,
                        requested_concurrent,
                    );
                info!(
                    workspace_id = workspace_id,
                    llm_model = %ws.llm_full_id(),
                    embedding_provider = %emb.provider,
                    embedding_model = %emb.model,
                    embedding_source = %emb.source.as_str(),
                    extract_provider = %extract_role.provider,
                    extract_model = %extract_role.model,
                    is_local = is_slow_local_provider(&extract_role.provider),
                    chunk_timeout_secs = tuned.chunk_extraction_timeout_secs,
                    max_concurrent_extractions = tuned.max_concurrent_extractions,
                    local_concurrency_clamped = local_concurrency_clamped,
                    ollama_context_length = %std::env::var("OLLAMA_CONTEXT_LENGTH")
                        .unwrap_or_else(|_| "(unset)".into()),
                    "Resolved workspace-specific ingestion pipeline"
                );

                // Adaptive routing guardrail: large local thinking models on big docs.
                if is_slow_local_provider(&extract_role.provider) {
                    let model_l = extract_role.model.to_ascii_lowercase();
                    let looks_heavy = model_l.contains("35b")
                        || model_l.contains("32b")
                        || model_l.contains("70b")
                        || model_l.contains("thinking")
                        || model_l.contains("reason");
                    if looks_heavy {
                        warn!(
                            workspace_id = workspace_id,
                            extract_model = %extract_role.model,
                            "Local heavy/thinking extract model — prefer gemma4/cloud for bulk PDFs \
                             (see docs/operations/local-extract-reliability.md)"
                        );
                    }
                }

                let entity_schema =
                    edgequake_pipeline::prompts::EntityExtractionSchema::from_workspace_metadata(
                        &ws.metadata,
                    );
                let ws_lang = edgequake_pipeline::extraction_language_from_metadata(&ws.metadata);
                let extraction_language =
                    edgequake_pipeline::resolve_extraction_language_from_env(ws_lang.as_deref());
                let language_source = if ws_lang
                    .as_deref()
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false)
                {
                    "workspace"
                } else if std::env::var(edgequake_pipeline::EXTRACTION_LANGUAGE_ENV)
                    .map(|v| !v.trim().is_empty())
                    .unwrap_or(false)
                {
                    "env"
                } else {
                    "default"
                };
                info!(
                    workspace_id = workspace_id,
                    extraction_language = %extraction_language,
                    extraction_language_source = language_source,
                    "Resolved extraction language for ingestion pipeline"
                );
                let extract_effort = crate::services::resolve_extract_reasoning_effort(
                    Some(&ws),
                    &extract_role.provider,
                    &extract_role.model,
                    None,
                    None,
                );
                info!(
                    workspace_id = workspace_id,
                    extract_provider = %extract_role.provider,
                    extract_model = %extract_role.model,
                    extract_reasoning_effort = extract_effort.as_deref().unwrap_or("(none)"),
                    "Resolved extract reasoning effort for ingestion pipeline"
                );
                // SPEC-116: workspace chunking policy before document chunk_options
                // (options may already carry doc overrides from prepare.rs).
                let options = with_workspace_chunking(options, &ws);
                // SPEC-117: document caps (already on options) > workspace > env
                let resolved = edgequake_pipeline::ExtractionCaps::resolve_for_ingestion(
                    &ws.metadata,
                    options.extraction_caps,
                );
                let options = options.with_extraction_caps(resolved);
                let options = options
                    .with_extraction_language(extraction_language)
                    .with_reasoning_effort(extract_effort);
                Ok(Arc::new(build_ingestion_pipeline(
                    llm,
                    embedding,
                    entity_schema,
                    options,
                )))
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
                    &options,
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
                    &options,
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
                    &options,
                )
            }
        }
    }

    fn handle_failure(
        &self,
        policy: PipelineFallbackPolicy,
        message: String,
        options: &IngestionPipelineOptions,
    ) -> Result<Arc<Pipeline>, String> {
        match policy {
            PipelineFallbackPolicy::Strict => Err(message),
            PipelineFallbackPolicy::LenientGlobal => {
                // SPEC-046: Semantic (V) chunking needs an embedder — refuse silent
                // Recursive fallback that would drop the caller's strategy.
                if options.chunk_strategy.requires_embeddings() {
                    return Err(format!(
                        "{message}; semantic chunking cannot use global fallback without workspace embedder"
                    ));
                }
                warn!(
                    error = %message,
                    chunk_strategy = options.chunk_strategy.as_str(),
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
    async fn lenient_semantic_refuses_fallback_without_embedder() {
        let (factory, _) = test_factory();
        let opts = IngestionPipelineOptions::from_document_size(1000)
            .with_chunk_strategy(edgequake_pipeline::ChunkStrategy::Semantic);
        match factory
            .resolve_for_ingestion(
                "not-a-valid-uuid",
                PipelineFallbackPolicy::LenientGlobal,
                opts,
            )
            .await
        {
            Ok(_) => panic!("semantic must not fall back to Recursive"),
            Err(err) => assert!(
                err.to_ascii_lowercase().contains("semantic"),
                "unexpected error: {err}"
            ),
        }
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
        // Isolate from developer shells where mock is healed away without this flag.
        unsafe {
            std::env::set_var("EDGEQUAKE_ALLOW_MOCK_PROVIDER", "1");
        }

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
