#[cfg(feature = "postgres")]
use edgequake_pdf::PdfParserBackend;
#[cfg(feature = "postgres")]
use uuid::Uuid;

#[cfg(feature = "postgres")]
use crate::{
    error::{Error, Result},
    types::{CreateWorkspaceRequest, UpdateWorkspaceRequest, Workspace, WorkspaceStats},
};

#[cfg(feature = "postgres")]
use super::helpers::{
    apply_chunking_metadata, apply_default_reasoning_effort_metadata, apply_entity_types_metadata,
    apply_entity_types_strict_metadata, apply_extraction_language_metadata,
    apply_kg_schema_preset_metadata, apply_llm_roles_metadata, apply_relation_edges_metadata,
    apply_relation_types_metadata, apply_relation_types_strict_metadata,
    apply_vision_extract_metadata,
};
#[cfg(feature = "postgres")]
use super::rows::WorkspaceRow;
#[cfg(feature = "postgres")]
use super::WorkspaceServiceImpl;

#[cfg(feature = "postgres")]
impl WorkspaceServiceImpl {
    // ============ Workspace Operations ============

    pub(super) async fn pg_create_workspace(
        &self,
        tenant_id: Uuid,
        request: CreateWorkspaceRequest,
    ) -> Result<Workspace> {
        // Check tenant exists and get max workspaces from metadata
        let tenant = self
            .pg_get_tenant(tenant_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("Tenant {} not found", tenant_id)))?;

        // Check workspace limit
        let current_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM workspaces WHERE tenant_id = $1")
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| Error::internal(format!("Failed to count workspaces: {}", e)))?;

        if current_count as usize >= tenant.max_workspaces {
            return Err(Error::validation(format!(
                "Tenant has reached maximum workspace limit ({})",
                tenant.max_workspaces
            )));
        }

        let slug = request
            .slug
            .unwrap_or_else(|| Self::generate_slug(&request.name));

        // Check slug uniqueness within tenant
        let existing: Option<(Uuid,)> = sqlx::query_as(
            "SELECT workspace_id FROM workspaces WHERE tenant_id = $1 AND slug = $2",
        )
        .bind(tenant_id)
        .bind(&slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to check workspace slug: {}", e)))?;

        if existing.is_some() {
            return Err(Error::validation(format!(
                "Workspace with slug '{}' already exists in this tenant",
                slug
            )));
        }

        let mut workspace = Workspace::new(tenant_id, &request.name, &slug);
        if let Some(desc) = request.description {
            workspace = workspace.with_description(desc);
        }
        if let Some(max_docs) = request.max_documents {
            workspace = workspace.with_max_documents(max_docs);
        }

        // SPEC-032: Apply LLM configuration from request
        // Uses auto-detection for provider if not specified
        if let Some(model) = request.llm_model {
            workspace = workspace.with_llm_model(&model);
            // Explicit provider overrides auto-detection
            if let Some(provider) = request.llm_provider {
                workspace = workspace.with_llm_provider(&provider);
            }
        } else if let Some(provider) = request.llm_provider {
            // Provider specified without model - use default model for provider
            workspace = workspace.with_llm_provider(&provider);
        }

        // SPEC-032: Apply embedding configuration from request
        // Uses auto-detection for provider/dimension if not specified
        if let Some(model) = request.embedding_model {
            workspace = workspace.with_embedding_model(&model);
            // Auto-detect provider if not specified
            if let Some(provider) = request.embedding_provider {
                workspace = workspace.with_embedding_provider(&provider);
            } else {
                let detected = Workspace::detect_provider_from_model(&model);
                workspace = workspace.with_embedding_provider(detected);
            }
            // Auto-detect dimension if not specified
            if let Some(dim) = request.embedding_dimension {
                workspace = workspace.with_embedding_dimension(dim);
            } else {
                let detected = Workspace::detect_dimension_from_model(&model);
                workspace = workspace.with_embedding_dimension(detected);
            }
        }

        // SPEC-041: Apply vision LLM configuration from request
        if let Some(vision_model) = request.vision_llm_model {
            if !vision_model.is_empty() {
                if let Some(provider) = request.vision_llm_provider {
                    workspace.vision_llm_provider = Some(provider.clone());
                    workspace.metadata.insert(
                        "vision_llm_provider".to_string(),
                        serde_json::json!(provider),
                    );
                } else {
                    let detected = Workspace::detect_provider_from_model(&vision_model);
                    workspace.vision_llm_provider = Some(detected.clone().to_string());
                    workspace.metadata.insert(
                        "vision_llm_provider".to_string(),
                        serde_json::json!(detected),
                    );
                }
                workspace.vision_llm_model = Some(vision_model.clone());
                workspace.metadata.insert(
                    "vision_llm_model".to_string(),
                    serde_json::json!(vision_model),
                );
            }
        } else if let Some(provider) = request.vision_llm_provider {
            workspace.vision_llm_provider = Some(provider.clone());
            workspace.metadata.insert(
                "vision_llm_provider".to_string(),
                serde_json::json!(provider),
            );
        }

        // SPEC-109: seed workspace default reasoning effort from tenant when unset
        if !workspace.metadata.contains_key("default_reasoning_effort") {
            if let Some(ref effort) = tenant.default_reasoning_effort {
                if !effort.trim().is_empty() {
                    workspace.metadata.insert(
                        "default_reasoning_effort".to_string(),
                        serde_json::json!(effort),
                    );
                }
            }
        }

        // Persist vision by default when omitted so env edgeparse cannot silently
        // override a freshly created workspace.
        let pdf_parser_backend = request
            .pdf_parser_backend
            .unwrap_or(PdfParserBackend::Vision);
        workspace.pdf_parser_backend = Some(pdf_parser_backend);
        workspace.metadata.insert(
            "pdf_parser_backend".to_string(),
            serde_json::json!(pdf_parser_backend.as_str()),
        );

        // SPEC-109: optional create-time override (else tenant seed above)
        apply_default_reasoning_effort_metadata(
            &mut workspace.metadata,
            request.default_reasoning_effort,
        );
        apply_llm_roles_metadata(&mut workspace.metadata, request.llm_roles);

        // SPEC-085: Apply entity type configuration from request
        // Normalize: uppercase, underscored, deduplicated, max 50 types
        apply_entity_types_metadata(&mut workspace.metadata, request.entity_types);
        apply_entity_types_strict_metadata(&mut workspace.metadata, request.entity_types_strict);
        // SPEC-114: relation types + schema preset
        apply_relation_types_metadata(&mut workspace.metadata, request.relation_types);
        apply_relation_types_strict_metadata(
            &mut workspace.metadata,
            request.relation_types_strict,
        );
        apply_kg_schema_preset_metadata(&mut workspace.metadata, request.kg_schema_preset)
            .map_err(Error::validation)?;
        // SPEC-114b: typed edges (after type lists so allow-lists are current)
        apply_relation_edges_metadata(&mut workspace.metadata, request.relation_edges);
        // SPEC-096: Workspace extraction language (future ingestions only)
        apply_extraction_language_metadata(&mut workspace.metadata, request.extraction_language)
            .map_err(Error::validation)?;
        // SPEC-116: Workspace chunking policy (future ingestions only)
        apply_chunking_metadata(
            &mut workspace.metadata,
            request.chunking_mode,
            request.chunk_token_size,
            request.chunk_overlap_token_size,
        )
        .map_err(Error::validation)?;
        // SPEC-117: Workspace extract budget (future ingestions only)
        crate::extract_budget_metadata::apply_extract_budget_metadata(
            &mut workspace.metadata,
            request.extract_budget_mode,
            request.extract_max_entities,
            request.extract_max_records,
        )
        .map_err(Error::validation)?;
        // SPEC-160: extraction mode + decision settings (future ingestions only)
        crate::decision_metadata::apply_extraction_mode_fields(
            &mut workspace.metadata,
            &crate::decision_metadata::ExtractionModeFields {
                extraction_mode: request.extraction_mode.as_deref(),
                decision_gate_preset: request.decision_gate_preset.as_deref(),
                decision_model: request.decision_model.as_deref(),
                decision_pack_size: request.decision_pack_size,
                decision_enabled: request.decision_enabled,
            },
        )
        .map_err(Error::validation)?;
        // SPEC-015V: Vision extract toggles + prompts
        apply_vision_extract_metadata(
            &mut workspace.metadata,
            request.vision_extract_images,
            request.vision_extract_charts,
            request.vision_extract_figures,
            request.vision_page_system_prompt,
            request.vision_image_system_prompt,
            request.vision_chart_system_prompt,
            request.vision_figure_system_prompt,
        )
        .map_err(Error::validation)?;
        // SPEC-102: entity type color overrides for graph visualization
        crate::entity_type_colors::apply_entity_type_colors_metadata(
            &mut workspace.metadata,
            request.entity_type_colors,
        )
        .map_err(Error::validation)?;

        sqlx::query(
            r#"
            INSERT INTO workspaces (workspace_id, tenant_id, name, slug, description, is_active, metadata, settings, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, '{}'::jsonb, $8, $9)
            "#,
        )
        .bind(workspace.workspace_id)
        .bind(workspace.tenant_id)
        .bind(&workspace.name)
        .bind(&workspace.slug)
        .bind(&workspace.description)
        .bind(workspace.is_active)
        // SPEC-032/SPEC-040: Store LLM, embedding, and vision config in metadata
        .bind({
            let mut metadata = workspace.metadata.clone();
            // LLM configuration
            metadata.insert("llm_model".to_string(), serde_json::Value::String(workspace.llm_model.clone()));
            metadata.insert("llm_provider".to_string(), serde_json::Value::String(workspace.llm_provider.clone()));
            // Embedding configuration
            metadata.insert("embedding_model".to_string(), serde_json::Value::String(workspace.embedding_model.clone()));
            metadata.insert("embedding_provider".to_string(), serde_json::Value::String(workspace.embedding_provider.clone()));
            metadata.insert("embedding_dimension".to_string(), serde_json::Value::Number(workspace.embedding_dimension.into()));
            // SPEC-041: Vision LLM configuration (already set in workspace.metadata above)
            if let Some(pdf_parser_backend) = workspace.pdf_parser_backend {
                metadata.insert(
                    "pdf_parser_backend".to_string(),
                    serde_json::Value::String(pdf_parser_backend.as_str().to_string()),
                );
            }
            serde_json::json!(metadata)
        })
        .bind(workspace.created_at)
        .bind(workspace.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to create workspace: {}", e)))?;

        tracing::info!(
            workspace_id = %workspace.workspace_id,
            tenant_id = %tenant_id,
            slug = %slug,
            llm_model = %workspace.llm_full_id(),
            embedding_model = %workspace.embedding_full_id(),
            "Created workspace in PostgreSQL"
        );

        Ok(workspace)
    }

    pub(super) async fn pg_insert_workspace(&self, workspace: Workspace) -> Result<Workspace> {
        // Validate tenant exists
        if self.pg_get_tenant(workspace.tenant_id).await?.is_none() {
            return Err(Error::not_found(format!(
                "Tenant {} not found",
                workspace.tenant_id
            )));
        }

        sqlx::query(
            r#"
            INSERT INTO workspaces (workspace_id, tenant_id, name, slug, description, is_active, metadata, settings, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, '{}'::jsonb, $8, $9)
            ON CONFLICT (workspace_id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                is_active = EXCLUDED.is_active,
                metadata = EXCLUDED.metadata,
                updated_at = NOW()
            "#,
        )
        .bind(workspace.workspace_id)
        .bind(workspace.tenant_id)
        .bind(&workspace.name)
        .bind(&workspace.slug)
        .bind(&workspace.description)
        .bind(workspace.is_active)
        .bind(serde_json::json!(workspace.metadata))
        .bind(workspace.created_at)
        .bind(workspace.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to insert workspace: {}", e)))?;

        tracing::info!(
            workspace_id = %workspace.workspace_id,
            tenant_id = %workspace.tenant_id,
            "Inserted workspace in PostgreSQL"
        );

        Ok(workspace)
    }

    pub(super) async fn pg_get_workspace(&self, workspace_id: Uuid) -> Result<Option<Workspace>> {
        let row: Option<WorkspaceRow> = sqlx::query_as(
            r#"
            SELECT workspace_id, tenant_id, name, slug, description, is_active, metadata, created_at, updated_at
            FROM workspaces
            WHERE workspace_id = $1
            "#,
        )
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to get workspace: {}", e)))?;

        match row {
            None => Ok(None),
            Some(r) => {
                let mut workspace = r.into_workspace();
                let tenant = self.pg_get_tenant(workspace.tenant_id).await?;
                crate::workspace_model_update::resolve_inherited_model_fields(
                    &mut workspace,
                    tenant.as_ref(),
                );
                Ok(Some(workspace))
            }
        }
    }

    pub(super) async fn pg_get_workspace_by_slug(
        &self,
        tenant_id: Uuid,
        slug: &str,
    ) -> Result<Option<Workspace>> {
        let row: Option<WorkspaceRow> = sqlx::query_as(
            r#"
            SELECT workspace_id, tenant_id, name, slug, description, is_active, metadata, created_at, updated_at
            FROM workspaces
            WHERE tenant_id = $1 AND slug = $2
            "#,
        )
        .bind(tenant_id)
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to get workspace by slug: {}", e)))?;

        match row {
            None => Ok(None),
            Some(r) => {
                let mut workspace = r.into_workspace();
                let tenant = self.pg_get_tenant(tenant_id).await?;
                crate::workspace_model_update::resolve_inherited_model_fields(
                    &mut workspace,
                    tenant.as_ref(),
                );
                Ok(Some(workspace))
            }
        }
    }

    pub(super) async fn pg_update_workspace(
        &self,
        workspace_id: Uuid,
        request: UpdateWorkspaceRequest,
    ) -> Result<Workspace> {
        // First get the existing workspace
        let mut workspace = self
            .pg_get_workspace(workspace_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("Workspace {} not found", workspace_id)))?;

        // Apply updates
        if let Some(name) = request.name {
            workspace.name = name;
        }
        if let Some(desc) = request.description {
            workspace.description = Some(desc);
        }
        if let Some(is_active) = request.is_active {
            workspace.is_active = is_active;
        }
        if let Some(max_docs) = request.max_documents {
            workspace
                .metadata
                .insert("max_documents".to_string(), serde_json::json!(max_docs));
        }
        // SPEC-032 / SPEC-013: LLM + embedding updates (empty string = tenant → env default)
        let tenant = self.pg_get_tenant(workspace.tenant_id).await?;
        let tenant_llm = tenant.as_ref().map(|t| {
            (
                t.default_llm_provider.as_str(),
                t.default_llm_model.as_str(),
            )
        });
        let tenant_emb = tenant.as_ref().map(|t| {
            (
                t.default_embedding_provider.as_str(),
                t.default_embedding_model.as_str(),
                t.default_embedding_dimension,
            )
        });
        crate::workspace_model_update::apply_llm_config_update_with_tenant(
            &mut workspace,
            request.llm_model,
            request.llm_provider,
            tenant_llm,
        );
        crate::workspace_model_update::apply_embedding_config_update_with_tenant(
            &mut workspace,
            request.embedding_model,
            request.embedding_provider,
            request.embedding_dimension,
            tenant_emb,
        );
        // SPEC-040: Vision LLM configuration updates
        if let Some(vision_provider) = request.vision_llm_provider {
            if vision_provider.is_empty() || vision_provider == "none" {
                workspace.vision_llm_provider = None;
                workspace.metadata.remove("vision_llm_provider");
            } else {
                workspace.metadata.insert(
                    "vision_llm_provider".to_string(),
                    serde_json::json!(vision_provider.clone()),
                );
                workspace.vision_llm_provider = Some(vision_provider);
            }
        }
        if let Some(vision_model) = request.vision_llm_model {
            if vision_model.is_empty() || vision_model == "none" {
                workspace.vision_llm_model = None;
                workspace.metadata.remove("vision_llm_model");
            } else {
                workspace.metadata.insert(
                    "vision_llm_model".to_string(),
                    serde_json::json!(vision_model.clone()),
                );
                workspace.vision_llm_model = Some(vision_model);
            }
        }
        if let Some(pdf_parser_backend) = request.pdf_parser_backend {
            let normalized_backend = pdf_parser_backend.trim().to_ascii_lowercase();
            if normalized_backend.is_empty() || normalized_backend == "none" {
                workspace.pdf_parser_backend = None;
                workspace.metadata.remove("pdf_parser_backend");
            } else if let Some(parsed_backend) = PdfParserBackend::from_env_str(&normalized_backend)
            {
                workspace.pdf_parser_backend = Some(parsed_backend);
                workspace.metadata.insert(
                    "pdf_parser_backend".to_string(),
                    serde_json::json!(parsed_backend.as_str()),
                );
            } else {
                return Err(Error::validation(format!(
                    "Invalid pdf_parser_backend '{}'. Expected 'vision', 'edgeparse', 'edgeparse-ocr', 'auto', or 'none'",
                    pdf_parser_backend
                )));
            }
        }
        apply_entity_types_metadata(&mut workspace.metadata, request.entity_types);
        apply_entity_types_strict_metadata(&mut workspace.metadata, request.entity_types_strict);
        apply_relation_types_metadata(&mut workspace.metadata, request.relation_types);
        apply_relation_types_strict_metadata(
            &mut workspace.metadata,
            request.relation_types_strict,
        );
        apply_kg_schema_preset_metadata(&mut workspace.metadata, request.kg_schema_preset)
            .map_err(Error::validation)?;
        apply_relation_edges_metadata(&mut workspace.metadata, request.relation_edges);
        apply_extraction_language_metadata(&mut workspace.metadata, request.extraction_language)
            .map_err(Error::validation)?;
        apply_chunking_metadata(
            &mut workspace.metadata,
            request.chunking_mode,
            request.chunk_token_size,
            request.chunk_overlap_token_size,
        )
        .map_err(Error::validation)?;
        crate::extract_budget_metadata::apply_extract_budget_metadata(
            &mut workspace.metadata,
            request.extract_budget_mode,
            request.extract_max_entities,
            request.extract_max_records,
        )
        .map_err(Error::validation)?;
        // SPEC-160: extraction mode + decision settings (future ingestions only)
        crate::decision_metadata::apply_extraction_mode_fields(
            &mut workspace.metadata,
            &crate::decision_metadata::ExtractionModeFields {
                extraction_mode: request.extraction_mode.as_deref(),
                decision_gate_preset: request.decision_gate_preset.as_deref(),
                decision_model: request.decision_model.as_deref(),
                decision_pack_size: request.decision_pack_size,
                decision_enabled: request.decision_enabled,
            },
        )
        .map_err(Error::validation)?;
        apply_vision_extract_metadata(
            &mut workspace.metadata,
            request.vision_extract_images,
            request.vision_extract_charts,
            request.vision_extract_figures,
            request.vision_page_system_prompt,
            request.vision_image_system_prompt,
            request.vision_chart_system_prompt,
            request.vision_figure_system_prompt,
        )
        .map_err(Error::validation)?;
        crate::entity_type_colors::apply_entity_type_colors_metadata(
            &mut workspace.metadata,
            request.entity_type_colors,
        )
        .map_err(Error::validation)?;
        apply_default_reasoning_effort_metadata(
            &mut workspace.metadata,
            request.default_reasoning_effort,
        );
        apply_llm_roles_metadata(&mut workspace.metadata, request.llm_roles);
        workspace.updated_at = chrono::Utc::now();

        // Store all config in metadata JSONB column (database schema uses metadata, not separate columns)
        sqlx::query(
            r#"
            UPDATE workspaces 
            SET name = $2, description = $3, is_active = $4, metadata = $5,
                updated_at = NOW()
            WHERE workspace_id = $1
            "#,
        )
        .bind(workspace.workspace_id)
        .bind(&workspace.name)
        .bind(&workspace.description)
        .bind(workspace.is_active)
        .bind(serde_json::json!(workspace.metadata))
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to update workspace: {}", e)))?;

        crate::workspace_model_update::resolve_inherited_model_fields(
            &mut workspace,
            tenant.as_ref(),
        );
        Ok(workspace)
    }

    pub(super) async fn pg_delete_workspace(&self, workspace_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM workspaces WHERE workspace_id = $1")
            .bind(workspace_id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("Failed to delete workspace: {}", e)))?;

        tracing::info!(workspace_id = %workspace_id, "Deleted workspace from PostgreSQL");
        Ok(())
    }

    pub(super) async fn pg_list_workspaces(&self, tenant_id: Uuid) -> Result<Vec<Workspace>> {
        self.pg_list_workspaces_page(tenant_id, i64::MAX as usize, 0)
            .await
    }

    pub(super) async fn pg_list_workspaces_page(
        &self,
        tenant_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Workspace>> {
        let rows: Vec<WorkspaceRow> = sqlx::query_as(
            r#"
            SELECT workspace_id, tenant_id, name, slug, description, is_active, metadata, created_at, updated_at
            FROM workspaces
            WHERE tenant_id = $1
            ORDER BY created_at DESC, workspace_id DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to list workspaces: {}", e)))?;

        let tenant = self.pg_get_tenant(tenant_id).await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let mut workspace = r.into_workspace();
                crate::workspace_model_update::resolve_inherited_model_fields(
                    &mut workspace,
                    tenant.as_ref(),
                );
                workspace
            })
            .collect())
    }

    pub(super) async fn pg_count_workspaces(&self, tenant_id: Uuid) -> Result<usize> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspaces WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("Failed to count workspaces: {}", e)))?;
        Ok(count as usize)
    }

    pub(super) async fn pg_get_workspace_stats(
        &self,
        workspace_id: Uuid,
    ) -> Result<WorkspaceStats> {
        // Verify workspace exists
        let _ = self
            .pg_get_workspace(workspace_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("Workspace {} not found", workspace_id)))?;

        // SPEC-091 F-091-11 / Pre-W1: do NOT read the unpopulated `chunks` spine.
        // LAW-D4 interim projections (distinct facts, labeled as such):
        //   chunk_count     ← SUM(documents.chunk_count)  (writer-maintained denorm)
        //   embedding_count ← COUNT on shared vector table by workspace_id
        // After Wave 1 these re-point to `chunks` / `chunk_serving_state`.
        #[derive(sqlx::FromRow)]
        struct StatsRow {
            document_count: i64,
            chunk_count: i64,
            entity_count: i64,
            relationship_count: i64,
            storage_bytes: i64,
        }

        let stats: StatsRow = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM public.documents WHERE workspace_id = $1) AS document_count,
                (SELECT COALESCE(SUM(chunk_count), 0)::BIGINT FROM public.documents WHERE workspace_id = $1) AS chunk_count,
                (SELECT COUNT(*) FROM entities WHERE workspace_id = $1) AS entity_count,
                (SELECT COUNT(*) FROM relationships WHERE workspace_id = $1) AS relationship_count,
                (SELECT COALESCE(SUM(file_size_bytes), 0)::BIGINT FROM public.documents WHERE workspace_id = $1) AS storage_bytes
            "#,
        )
        .bind(workspace_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to get workspace stats: {}", e)))?;

        // The migration gate guarantees typed embedding homes before serve.
        // Database failures must not be reported as a valid zero count.
        let embedding_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)::BIGINT
            FROM public.chunk_embeddings ce
            INNER JOIN public.chunks c ON c.id = ce.chunk_id
            WHERE c.workspace_id = $1
            "#,
        )
        .bind(workspace_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to get workspace embedding count: {e}")))?;

        Ok(WorkspaceStats {
            workspace_id,
            document_count: stats.document_count as usize,
            entity_count: stats.entity_count as usize,
            relationship_count: stats.relationship_count as usize,
            chunk_count: stats.chunk_count as usize,
            embedding_count: embedding_count as usize,
            storage_bytes: stats.storage_bytes as usize,
        })
    }
}
