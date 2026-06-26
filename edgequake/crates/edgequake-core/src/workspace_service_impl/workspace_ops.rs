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
use super::helpers::{apply_entity_types_metadata, apply_entity_types_strict_metadata};
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
        if let Some(pdf_parser_backend) = request.pdf_parser_backend {
            workspace.pdf_parser_backend = Some(pdf_parser_backend);
            workspace.metadata.insert(
                "pdf_parser_backend".to_string(),
                serde_json::json!(pdf_parser_backend.as_str()),
            );
        }

        // SPEC-085: Apply entity type configuration from request
        // Normalize: uppercase, underscored, deduplicated, max 50 types
        apply_entity_types_metadata(&mut workspace.metadata, request.entity_types);
        apply_entity_types_strict_metadata(&mut workspace.metadata, request.entity_types_strict);

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

        Ok(row.map(|r| r.into_workspace()))
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

        Ok(row.map(|r| r.into_workspace()))
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
        // SPEC-032 / SPEC-013: LLM + embedding updates (empty string = server/env default)
        crate::workspace_model_update::apply_llm_config_update(
            &mut workspace,
            request.llm_model,
            request.llm_provider,
        );
        crate::workspace_model_update::apply_embedding_config_update(
            &mut workspace,
            request.embedding_model,
            request.embedding_provider,
            request.embedding_dimension,
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
                    "Invalid pdf_parser_backend '{}'. Expected 'vision', 'edgeparse', or 'none'",
                    pdf_parser_backend
                )));
            }
        }
        apply_entity_types_metadata(&mut workspace.metadata, request.entity_types);
        apply_entity_types_strict_metadata(&mut workspace.metadata, request.entity_types_strict);
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
        let rows: Vec<WorkspaceRow> = sqlx::query_as(
            r#"
            SELECT workspace_id, tenant_id, name, slug, description, is_active, metadata, created_at, updated_at
            FROM workspaces
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to list workspaces: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into_workspace()).collect())
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

        // WHY scalar subqueries: Single round-trip to database, efficient counting.
        // Each subquery uses indexed workspace_id for O(log n) performance.
        // OODA-13: Implements real-time metrics per mission requirement.
        #[derive(sqlx::FromRow)]
        struct StatsRow {
            document_count: i64,
            chunk_count: i64,
            entity_count: i64,
            relationship_count: i64,
            embedding_count: i64,
            storage_bytes: i64,
        }

        let stats: StatsRow = sqlx::query_as(
            r#"
            SELECT 
                (SELECT COUNT(*) FROM documents WHERE workspace_id = $1) as document_count,
                (SELECT COUNT(*) FROM chunks WHERE workspace_id = $1) as chunk_count,
                (SELECT COUNT(*) FROM entities WHERE workspace_id = $1) as entity_count,
                (SELECT COUNT(*) FROM relationships WHERE workspace_id = $1) as relationship_count,
                (SELECT COUNT(*) FROM chunks WHERE workspace_id = $1) as embedding_count,
                (SELECT COALESCE(SUM(file_size_bytes), 0)::BIGINT FROM documents WHERE workspace_id = $1) as storage_bytes
            "#,
        )
        .bind(workspace_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to get workspace stats: {}", e)))?;

        Ok(WorkspaceStats {
            workspace_id,
            document_count: stats.document_count as usize,
            entity_count: stats.entity_count as usize,
            relationship_count: stats.relationship_count as usize,
            chunk_count: stats.chunk_count as usize,
            embedding_count: stats.embedding_count as usize,
            storage_bytes: stats.storage_bytes as usize,
        })
    }
}
