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
        use edgequake_storage::traits::WorkspaceVectorConfig;

        // OODA-223: Check if we should allow fallback
        let allow_fallback = !self.strict_workspace_mode;

        let workspace_id = workspace_id.trim();
        if workspace_id.is_empty() {
            if allow_fallback {
                warn!(
                    workspace_id = %workspace_id,
                    strict_mode = self.strict_workspace_mode,
                    "Empty workspace ID - using default storage (non-strict mode)"
                );
                return Ok(Arc::clone(&self.vector_storage));
            }
            error!(
                workspace_id = %workspace_id,
                "CRITICAL INGESTION ERROR: Cannot ingest documents without a workspace ID"
            );
            return Err("Cannot ingest documents without a valid workspace ID. \
                 Please ensure workspace context is properly set."
                .to_string());
        }

        // Parse workspace UUID, preserving the legacy `default` alias.
        let workspace_uuid = match crate::middleware::resolve_workspace_uuid(Some(workspace_id)) {
            Some(uuid) => uuid,
            None => {
                if allow_fallback {
                    warn!(
                        workspace_id = %workspace_id,
                        strict_mode = self.strict_workspace_mode,
                        "Invalid workspace ID format - using default storage (non-strict mode)"
                    );
                    return Ok(Arc::clone(&self.vector_storage));
                }
                error!(
                    workspace_id = %workspace_id,
                    "CRITICAL INGESTION ERROR: Invalid workspace ID format"
                );
                return Err(format!(
                    "Invalid workspace ID format '{}': could not resolve to a UUID",
                    workspace_id
                ));
            }
        };

        // Check if we already have this workspace's vector storage cached
        if let Some(storage) = self.vector_registry.get(&workspace_uuid).await {
            return Ok(storage);
        }

        // Look up workspace to get embedding dimension
        let workspace_service = match &self.workspace_service {
            Some(ws) => ws,
            None => {
                if allow_fallback {
                    warn!(
                        workspace_id = %workspace_id,
                        strict_mode = self.strict_workspace_mode,
                        "No workspace service - using default storage (non-strict mode)"
                    );
                    return Ok(Arc::clone(&self.vector_storage));
                }
                error!(
                    workspace_id = %workspace_id,
                    "CRITICAL INGESTION ERROR: No workspace service available"
                );
                return Err(
                    "Workspace service not configured. Cannot verify workspace exists.".to_string(),
                );
            }
        };

        match workspace_service.get_workspace(workspace_uuid).await {
            Ok(Some(ws)) => {
                // Create workspace-specific vector storage with correct dimension
                let config = WorkspaceVectorConfig {
                    workspace_id: workspace_uuid,
                    dimension: ws.embedding_dimension,
                    namespace: "default".to_string(),
                };

                match self.vector_registry.get_or_create(config).await {
                    Ok(storage) => {
                        info!(
                            workspace_id = %workspace_id,
                            dimension = ws.embedding_dimension,
                            strict_mode = self.strict_workspace_mode,
                            "Using workspace-specific vector storage"
                        );
                        Ok(storage)
                    }
                    Err(e) => {
                        if allow_fallback {
                            warn!(
                                workspace_id = %workspace_id,
                                error = %e,
                                strict_mode = self.strict_workspace_mode,
                                "Failed to create workspace storage - using default (non-strict mode)"
                            );
                            return Ok(Arc::clone(&self.vector_storage));
                        }
                        error!(
                            workspace_id = %workspace_id,
                            error = %e,
                            "CRITICAL INGESTION ERROR: Failed to create workspace vector storage"
                        );
                        Err(format!(
                            "Failed to create vector storage for workspace '{}': {}",
                            workspace_id, e
                        ))
                    }
                }
            }
            Ok(None) => {
                if allow_fallback {
                    warn!(
                        workspace_id = %workspace_id,
                        strict_mode = self.strict_workspace_mode,
                        "Workspace not found - using default storage (non-strict mode)"
                    );
                    return Ok(Arc::clone(&self.vector_storage));
                }
                error!(
                    workspace_id = %workspace_id,
                    "CRITICAL INGESTION ERROR: Workspace not found"
                );
                Err(format!(
                    "Workspace '{}' not found. Cannot ingest documents into non-existent workspace.",
                    workspace_id
                ))
            }
            Err(e) => {
                if allow_fallback {
                    warn!(
                        workspace_id = %workspace_id,
                        error = %e,
                        strict_mode = self.strict_workspace_mode,
                        "Failed to lookup workspace - using default storage (non-strict mode)"
                    );
                    return Ok(Arc::clone(&self.vector_storage));
                }
                error!(
                    workspace_id = %workspace_id,
                    error = %e,
                    "CRITICAL INGESTION ERROR: Failed to lookup workspace"
                );
                Err(format!(
                    "Failed to lookup workspace '{}': {}",
                    workspace_id, e
                ))
            }
        }
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
