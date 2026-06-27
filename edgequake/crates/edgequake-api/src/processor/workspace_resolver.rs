use super::*;

impl DocumentTaskProcessor {
    /// SPEC-032: Creates a new Pipeline instance configured with the workspace's
    /// LLM and embedding providers. Falls back to the default pipeline if:
    /// - No workspace_id provided
    /// - Workspace not found
    /// - Failed to create workspace-specific providers
    ///
    /// # WHY: Silent Fallback is Dangerous
    ///
    /// When this method falls back to `self.pipeline` (the server default, typically
    /// Ollama from auto-detection), documents get extracted with the WRONG provider.
    /// This produces confusing logs where Ollama appears even though the workspace
    /// is configured for OpenAI. Production code uses `get_workspace_pipeline_strict`
    /// instead, which fails the task explicitly.
    ///
    /// # WHY: This Method Still Exists
    ///
    /// Kept for backward compatibility in test/memory mode where strict workspace
    /// isolation isn't required. Production (PostgreSQL mode) always uses strict.
    pub(super) async fn get_workspace_pipeline(&self, workspace_id: Option<&str>) -> Arc<Pipeline> {
        info!(
            workspace_id = ?workspace_id,
            has_workspace_service = self.workspace_service.is_some(),
            has_models_config = self.models_config.is_some(),
            "[PIPELINE] SPEC-032: Getting pipeline for workspace"
        );

        let (workspace_service, _models_config): (&SharedWorkspaceService, &Arc<ModelsConfig>) =
            match (&self.workspace_service, &self.models_config) {
                (Some(ws), Some(mc)) => (ws, mc),
                _ => {
                    edgequake_observability::ErrorEvent::log_domain_warn(
                        "task_processor",
                        "workspace_pipeline_fallback",
                        "No workspace support configured, using default pipeline",
                        json!({ "spec": "SPEC-032" }),
                    );
                    return Arc::clone(&self.pipeline);
                }
            };

        let Some(workspace_id) = workspace_id.map(str::trim).filter(|id| !id.is_empty()) else {
            info!(
                workspace_id = ?workspace_id,
                "SPEC-032: No valid workspace_id, using default pipeline"
            );
            return Arc::clone(&self.pipeline);
        };

        let factory = crate::workspace_pipeline_factory::WorkspacePipelineFactory::new(
            Arc::clone(workspace_service),
            Arc::clone(&self.pipeline),
        );
        factory
            .resolve(
                workspace_id,
                crate::workspace_pipeline_factory::PipelineFallbackPolicy::LenientGlobal,
            )
            .await
            .unwrap_or_else(|_| Arc::clone(&self.pipeline))
    }

    /// Document-scoped pipeline with adaptive chunking and gleaning (SPEC-025 5.2/5.3).
    pub(super) async fn get_workspace_pipeline_for_ingestion(
        &self,
        workspace_id: Option<&str>,
        options: edgequake_pipeline::IngestionPipelineOptions,
        policy: crate::workspace_pipeline_factory::PipelineFallbackPolicy,
    ) -> Result<Arc<Pipeline>, String> {
        let (workspace_service, _models_config): (&SharedWorkspaceService, &Arc<ModelsConfig>) =
            match (&self.workspace_service, &self.models_config) {
                (Some(ws), Some(mc)) => (ws, mc),
                _ => {
                    return Err("OODA-16: No workspace support configured on processor".to_string());
                }
            };

        let Some(workspace_id) = workspace_id.map(str::trim).filter(|id| !id.is_empty()) else {
            return Err(format!(
                "OODA-16: Invalid workspace_id '{:?}' - must provide valid workspace ID in strict mode",
                workspace_id
            ));
        };

        let factory = crate::workspace_pipeline_factory::WorkspacePipelineFactory::new(
            Arc::clone(workspace_service),
            Arc::clone(&self.pipeline),
        );
        factory
            .resolve_for_ingestion(workspace_id, policy, options)
            .await
    }

    /// OODA-16: Strict variant that returns error instead of falling back.
    pub(super) async fn get_workspace_pipeline_strict(
        &self,
        workspace_id: Option<&str>,
    ) -> Result<Arc<Pipeline>, String> {
        info!(
            workspace_id = ?workspace_id,
            has_workspace_service = self.workspace_service.is_some(),
            has_models_config = self.models_config.is_some(),
            "[PIPELINE] OODA-16: Getting pipeline for workspace (STRICT mode)"
        );

        let (workspace_service, _models_config): (&SharedWorkspaceService, &Arc<ModelsConfig>) =
            match (&self.workspace_service, &self.models_config) {
                (Some(ws), Some(mc)) => (ws, mc),
                _ => {
                    return Err("OODA-16: No workspace support configured on processor".to_string());
                }
            };

        let Some(workspace_id) = workspace_id.map(str::trim).filter(|id| !id.is_empty()) else {
            return Err(format!(
                "OODA-16: Invalid workspace_id '{:?}' - must provide valid workspace ID in strict mode",
                workspace_id
            ));
        };

        let factory = crate::workspace_pipeline_factory::WorkspacePipelineFactory::new(
            Arc::clone(workspace_service),
            Arc::clone(&self.pipeline),
        );
        factory
            .resolve(
                workspace_id,
                crate::workspace_pipeline_factory::PipelineFallbackPolicy::Strict,
            )
            .await
    }

    /// Get workspace-specific vector storage using the registry.
    ///
    /// WHY: Different workspaces can have different embedding dimensions (e.g.,
    /// OpenAI 1536 vs Ollama/nomic 768). The registry creates per-workspace
    /// vector tables with the correct dimension.
    ///
    /// # OODA-223: Behavior depends on `strict_workspace_mode`
    ///
    /// - **Strict mode (production)**: Returns error if workspace storage cannot be obtained.
    /// - **Non-strict mode (tests/legacy)**: Falls back to default storage with warning.
    ///
    /// # Lesson Learned (OODA-223)
    ///
    /// Silent fallback to default storage caused data to be stored in the
    /// global table instead of workspace-specific tables, leading to "0 Sources"
    /// on queries because reads look in workspace tables.
    pub(super) async fn get_workspace_vector_storage_strict(
        &self,
        workspace_id: &str,
    ) -> Result<Arc<dyn VectorStorage>, String> {
        use edgequake_core::{
            resolve_workspace_vector_storage, WorkspaceVectorResolveInput,
            WorkspaceVectorResolvePolicy,
        };

        let policy = if self.strict_workspace_mode {
            WorkspaceVectorResolvePolicy::Strict
        } else {
            WorkspaceVectorResolvePolicy::AllowDefaultFallback
        };

        let fallback_dim = self.vector_storage.dimension();

        resolve_workspace_vector_storage(
            self.vector_registry.as_ref(),
            Arc::clone(&self.vector_storage),
            self.workspace_service.as_deref(),
            fallback_dim,
            WorkspaceVectorResolveInput::new(Some(workspace_id), "default"),
            policy,
        )
        .await
        .map_err(|e| e.to_string())
    }

    /// SPEC-032/OODA-198: Get provider lineage for a workspace.
    ///
    /// Returns the provider configuration that will be used for processing
    /// documents in this workspace. This enables lineage tracking by storing
    /// which providers were used for extraction.
    ///
    /// Returns default provider config if workspace not found.
    pub(super) async fn get_workspace_provider_lineage(
        &self,
        workspace_id: Option<&str>,
    ) -> ProviderLineage {
        use edgequake_core::types::{
            DEFAULT_EMBEDDING_DIMENSION, DEFAULT_EMBEDDING_MODEL, DEFAULT_EMBEDDING_PROVIDER,
            DEFAULT_LLM_MODEL, DEFAULT_LLM_PROVIDER,
        };

        // Default lineage (used when workspace not available)
        let default_lineage = ProviderLineage {
            extraction_provider: DEFAULT_LLM_PROVIDER.to_string(),
            extraction_model: DEFAULT_LLM_MODEL.to_string(),
            embedding_provider: DEFAULT_EMBEDDING_PROVIDER.to_string(),
            embedding_model: DEFAULT_EMBEDDING_MODEL.to_string(),
            embedding_dimension: DEFAULT_EMBEDDING_DIMENSION,
        };

        let Some(workspace_id) = workspace_id.map(str::trim).filter(|id| !id.is_empty()) else {
            return default_lineage;
        };

        let Some(workspace_uuid) = crate::middleware::resolve_workspace_uuid(Some(workspace_id))
        else {
            return default_lineage;
        };

        let workspace_service = match &self.workspace_service {
            Some(ws) => ws,
            None => return default_lineage,
        };

        match workspace_service.get_workspace(workspace_uuid).await {
            Ok(Some(ws)) => ProviderLineage {
                extraction_provider: ws.llm_provider.clone(),
                extraction_model: ws.llm_model.clone(),
                embedding_provider: ws.embedding_provider.clone(),
                embedding_model: ws.embedding_model.clone(),
                embedding_dimension: ws.embedding_dimension,
            },
            _ => default_lineage,
        }
    }
}
