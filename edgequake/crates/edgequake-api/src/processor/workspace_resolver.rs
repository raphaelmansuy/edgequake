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
        use crate::safety_limits::{create_safe_embedding_provider, create_safe_llm_provider};

        info!(
            workspace_id = ?workspace_id,
            has_workspace_service = self.workspace_service.is_some(),
            has_models_config = self.models_config.is_some(),
            "[PIPELINE] SPEC-032: Getting pipeline for workspace"
        );

        // If no workspace support configured, use default pipeline
        let (workspace_service, _models_config): (&SharedWorkspaceService, &Arc<ModelsConfig>) =
            match (&self.workspace_service, &self.models_config) {
                (Some(ws), Some(mc)) => (ws, mc),
                _ => {
                    warn!("SPEC-032: No workspace support configured, using default pipeline");
                    return Arc::clone(&self.pipeline);
                }
            };

        // If no workspace_id provided, use default pipeline
        let workspace_id = match workspace_id {
            Some(id) if !id.is_empty() && id != "default" => id,
            _ => {
                info!(
                    workspace_id = ?workspace_id,
                    "SPEC-032: No valid workspace_id, using default pipeline"
                );
                return Arc::clone(&self.pipeline);
            }
        };

        // Parse workspace_id to UUID
        let workspace_uuid = match uuid::Uuid::parse_str(workspace_id) {
            Ok(uuid) => uuid,
            Err(e) => {
                warn!(
                    workspace_id = workspace_id,
                    error = %e,
                    "Invalid workspace ID format, using default pipeline"
                );
                return Arc::clone(&self.pipeline);
            }
        };

        // Look up workspace configuration
        match workspace_service.get_workspace(workspace_uuid).await {
            Ok(Some(ws)) => {
                // Try to create workspace-specific LLM provider with safety limits
                // @implements OODA-189: Explicit error logging for provider failures
                // @implements FEAT0780: Safety limits for LLM calls (DocumentTaskProcessor)
                let llm_provider_result = create_safe_llm_provider(&ws.llm_provider, &ws.llm_model);

                // Try to create workspace-specific embedding provider with safety limits
                let embedding_provider_result = create_safe_embedding_provider(
                    &ws.embedding_provider,
                    &ws.embedding_model,
                    ws.embedding_dimension,
                );

                // Check for provider creation failures and log explicit errors
                match (&llm_provider_result, &embedding_provider_result) {
                    (Ok(llm), Ok(embedding)) => {
                        // SUCCESS: Both providers created
                        info!(
                            workspace_id = workspace_id,
                            llm_provider = %ws.llm_provider,
                            llm_model = %ws.llm_model,
                            embedding_provider = %ws.embedding_provider,
                            embedding_model = %ws.embedding_model,
                            "[PIPELINE] SPEC-032: Using workspace-specific providers for document processing"
                        );

                        let extractor = Arc::new(LLMExtractor::new(Arc::clone(llm)));
                        return Arc::new(
                            Pipeline::default_pipeline()
                                .with_extractor(extractor)
                                .with_embedding_provider(Arc::clone(embedding)),
                        );
                    }
                    (Err(llm_err), Ok(_)) => {
                        // LLM provider failed - this is a CRITICAL issue
                        error!(
                            workspace_id = workspace_id,
                            llm_provider = %ws.llm_provider,
                            llm_model = %ws.llm_model,
                            error = %llm_err,
                            "CRITICAL: Failed to create workspace LLM provider. \
                             Document extraction will use DEFAULT provider instead of workspace config. \
                             This may result in unexpected extraction results."
                        );
                    }
                    (Ok(_), Err(embed_err)) => {
                        // Embedding provider failed - this is a CRITICAL issue
                        error!(
                            workspace_id = workspace_id,
                            embedding_provider = %ws.embedding_provider,
                            embedding_model = %ws.embedding_model,
                            error = %embed_err,
                            "CRITICAL: Failed to create workspace embedding provider. \
                             Document embeddings will use DEFAULT provider instead of workspace config. \
                             This may result in dimension mismatches or unexpected query results."
                        );
                    }
                    (Err(llm_err), Err(embed_err)) => {
                        // Both providers failed - this is a CRITICAL issue
                        error!(
                            workspace_id = workspace_id,
                            llm_provider = %ws.llm_provider,
                            llm_model = %ws.llm_model,
                            llm_error = %llm_err,
                            embedding_provider = %ws.embedding_provider,
                            embedding_model = %ws.embedding_model,
                            embedding_error = %embed_err,
                            "CRITICAL: Failed to create BOTH workspace providers. \
                             Document processing will use DEFAULT pipeline instead of workspace config. \
                             Check API keys and provider configuration."
                        );
                    }
                }

                // Fallback to default pipeline (but with explicit ERROR logging above)
                warn!(
                    workspace_id = workspace_id,
                    llm_config = %ws.llm_full_id(),
                    embedding_config = %ws.embedding_full_id(),
                    "Falling back to default pipeline due to provider creation failure. \
                     WHY: This means document extraction will use the SERVER DEFAULT provider (likely Ollama) \
                     instead of the workspace-configured provider. Check API keys and provider config."
                );
            }
            Ok(None) => {
                warn!(
                    workspace_id = workspace_id,
                    "Workspace not found, using default pipeline"
                );
            }
            Err(e) => {
                warn!(
                    workspace_id = workspace_id,
                    error = %e,
                    "Failed to lookup workspace, using default pipeline"
                );
            }
        }

        Arc::clone(&self.pipeline)
    }

    /// OODA-16: Strict variant that returns error instead of falling back.
    ///
    /// WHY: In production, silent fallback to default pipeline causes data to be
    /// processed with wrong providers (e.g., Ollama 768-dim instead of OpenAI 1536-dim).
    /// This strict method ensures tasks fail clearly when workspace providers can't be created.
    pub(super) async fn get_workspace_pipeline_strict(
        &self,
        workspace_id: Option<&str>,
    ) -> Result<Arc<Pipeline>, String> {
        use crate::safety_limits::{create_safe_embedding_provider, create_safe_llm_provider};

        info!(
            workspace_id = ?workspace_id,
            has_workspace_service = self.workspace_service.is_some(),
            has_models_config = self.models_config.is_some(),
            "[PIPELINE] OODA-16: Getting pipeline for workspace (STRICT mode)"
        );

        // If no workspace support configured, fail explicitly
        let (workspace_service, _models_config): (&SharedWorkspaceService, &Arc<ModelsConfig>) =
            match (&self.workspace_service, &self.models_config) {
                (Some(ws), Some(mc)) => (ws, mc),
                _ => {
                    return Err("OODA-16: No workspace support configured on processor".to_string());
                }
            };

        // If no workspace_id provided, fail explicitly
        let workspace_id = match workspace_id {
            Some(id) if !id.is_empty() && id != "default" => id,
            _ => {
                return Err(format!(
                    "OODA-16: Invalid workspace_id '{:?}' - must provide valid workspace ID in strict mode",
                    workspace_id
                ));
            }
        };

        // Parse workspace_id to UUID
        let workspace_uuid = uuid::Uuid::parse_str(workspace_id).map_err(|e| {
            format!(
                "OODA-16: Invalid workspace ID format '{}': {}",
                workspace_id, e
            )
        })?;

        // Look up workspace configuration
        let ws = workspace_service
            .get_workspace(workspace_uuid)
            .await
            .map_err(|e| {
                format!(
                    "OODA-16: Failed to lookup workspace '{}': {}",
                    workspace_id, e
                )
            })?
            .ok_or_else(|| {
                format!(
                    "OODA-16: Workspace '{}' not found in database",
                    workspace_id
                )
            })?;

        // Create workspace-specific LLM provider - FAIL on error
        let llm_provider =
            create_safe_llm_provider(&ws.llm_provider, &ws.llm_model).map_err(|e| {
                format!(
                    "OODA-16: Failed to create LLM provider '{}' with model '{}': {}. \
                     Check if OPENAI_API_KEY is set for OpenAI providers.",
                    ws.llm_provider, ws.llm_model, e
                )
            })?;

        // Create workspace-specific embedding provider - FAIL on error
        let embedding_provider = create_safe_embedding_provider(
            &ws.embedding_provider,
            &ws.embedding_model,
            ws.embedding_dimension,
        )
        .map_err(|e| {
            format!(
                "OODA-16: Failed to create embedding provider '{}' with model '{}': {}. \
                 Check if OPENAI_API_KEY is set for OpenAI providers.",
                ws.embedding_provider, ws.embedding_model, e
            )
        })?;

        // SUCCESS: Both providers created
        info!(
            workspace_id = workspace_id,
            llm_provider = %ws.llm_provider,
            llm_model = %ws.llm_model,
            embedding_provider = %ws.embedding_provider,
            embedding_model = %ws.embedding_model,
            "[PIPELINE] OODA-16: Successfully created workspace-specific providers (STRICT mode)"
        );

        let extractor = Arc::new(LLMExtractor::new(Arc::clone(&llm_provider)));
        Ok(Arc::new(
            Pipeline::default_pipeline()
                .with_extractor(extractor)
                .with_embedding_provider(Arc::clone(&embedding_provider)),
        ))
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

        // Handle empty/default workspace IDs
        if workspace_id.is_empty() || workspace_id == "default" {
            if allow_fallback {
                warn!(
                    workspace_id = %workspace_id,
                    strict_mode = self.strict_workspace_mode,
                    "Empty/default workspace ID - using default storage (non-strict mode)"
                );
                return Ok(Arc::clone(&self.vector_storage));
            }
            error!(
                workspace_id = %workspace_id,
                "CRITICAL INGESTION ERROR: Cannot use 'default' workspace for document ingestion. \
                 Data must be stored in workspace-specific tables."
            );
            return Err("Cannot ingest documents without a valid workspace ID. \
                 Please ensure workspace context is properly set."
                .to_string());
        }

        // Parse workspace UUID
        let workspace_uuid = match uuid::Uuid::parse_str(workspace_id) {
            Ok(uuid) => uuid,
            Err(e) => {
                if allow_fallback {
                    warn!(
                        workspace_id = %workspace_id,
                        error = %e,
                        strict_mode = self.strict_workspace_mode,
                        "Invalid workspace ID format - using default storage (non-strict mode)"
                    );
                    return Ok(Arc::clone(&self.vector_storage));
                }
                error!(
                    workspace_id = %workspace_id,
                    error = %e,
                    "CRITICAL INGESTION ERROR: Invalid workspace ID format"
                );
                return Err(format!(
                    "Invalid workspace ID format '{}': {}",
                    workspace_id, e
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

        let workspace_id = match workspace_id {
            Some(id) if !id.is_empty() && id != "default" => id,
            _ => return default_lineage,
        };

        let workspace_uuid = match uuid::Uuid::parse_str(workspace_id) {
            Ok(uuid) => uuid,
            Err(_) => return default_lineage,
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
