//! In-memory implementation of [`WorkspaceService`](super::WorkspaceService) for testing.

use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{
    CreateWorkspaceRequest, Membership, MembershipRole, MetricsSnapshot, MetricsTriggerType,
    Tenant, TenantContext, TenantPlan, UpdateWorkspaceRequest, Workspace, WorkspaceStats,
};

use super::{UpdateTenantQuotaResult, WorkspaceService};

/// SPEC-096: apply extraction_language to in-memory workspace metadata.
fn apply_in_memory_extraction_language(
    metadata: &mut HashMap<String, serde_json::Value>,
    language: Option<String>,
) -> Result<()> {
    let Some(raw) = language else {
        return Ok(());
    };
    if edgequake_pipeline::is_extraction_language_clear(&raw) {
        metadata.remove("extraction_language");
        return Ok(());
    }
    match edgequake_pipeline::canonicalize_extraction_language(&raw) {
        Some(canonical) => {
            metadata.insert(
                "extraction_language".to_string(),
                serde_json::json!(canonical),
            );
            Ok(())
        }
        None => Err(Error::validation(format!(
            "Unsupported extraction_language '{}'. Allowed values: {}",
            raw.trim(),
            edgequake_pipeline::SUPPORTED_LANGUAGES.join(", ")
        ))),
    }
}

/// In-memory implementation of WorkspaceService for testing.
pub struct InMemoryWorkspaceService {
    tenants: RwLock<HashMap<Uuid, Tenant>>,
    workspaces: RwLock<HashMap<Uuid, Workspace>>,
    memberships: RwLock<HashMap<Uuid, Membership>>,
    /// Server-wide default max_workspaces for new tenants (SPEC-0001).
    server_default_max_workspaces: RwLock<usize>,
}

impl InMemoryWorkspaceService {
    /// Create a new in-memory workspace service.
    pub fn new() -> Self {
        Self {
            tenants: RwLock::new(HashMap::new()),
            workspaces: RwLock::new(HashMap::new()),
            memberships: RwLock::new(HashMap::new()),
            server_default_max_workspaces: RwLock::new(100),
        }
    }

    /// Create with a default tenant for testing.
    pub async fn with_default_tenant() -> Self {
        let service = Self::new();

        let tenant = Tenant::new("Default Tenant", "default").with_plan(TenantPlan::Pro);

        service.create_tenant(tenant).await.ok();

        service
    }

    fn generate_slug(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }

    async fn seed_default_workspace_inner(&self) {
        let tenant_id = uuid::Uuid::from_u128(2);
        let workspace_id = uuid::Uuid::from_u128(3);

        // Ensure the default tenant exists (idempotent).
        if self.get_tenant(tenant_id).await.ok().flatten().is_none() {
            let tenant = Tenant::new("Default Tenant", "default").with_plan(TenantPlan::Pro);
            // Force the canonical tenant id.
            let mut tenant = tenant;
            tenant.tenant_id = tenant_id;
            self.tenants.write().await.insert(tenant_id, tenant);
        }

        if self
            .get_workspace(workspace_id)
            .await
            .ok()
            .flatten()
            .is_none()
        {
            let now = chrono::Utc::now();
            let (llm_model, llm_provider) = Workspace::default_llm_config();
            let (embedding_model, embedding_provider, embedding_dimension) =
                Workspace::default_embedding_config();
            let ws = Workspace {
                workspace_id,
                tenant_id,
                name: "Default Workspace".to_string(),
                slug: "default".to_string(),
                description: None,
                is_active: true,
                created_at: now,
                updated_at: now,
                metadata: HashMap::new(),
                llm_model,
                llm_provider,
                embedding_model,
                embedding_provider,
                embedding_dimension,
                vision_llm_provider: None,
                vision_llm_model: None,
                pdf_parser_backend: Some(edgequake_pdf::PdfParserBackend::Vision),
            };
            self.workspaces.write().await.insert(workspace_id, ws);
        }
    }
}

impl Default for InMemoryWorkspaceService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkspaceService for InMemoryWorkspaceService {
    /// Seed the built-in default tenant + workspace at their canonical UUIDs.
    /// See `seed_default_workspace_inner` for the rationale (P-G2b).
    async fn seed_default_workspace(&self) {
        self.seed_default_workspace_inner().await
    }

    async fn create_tenant(&self, tenant: Tenant) -> Result<Tenant> {
        let mut tenants = self.tenants.write().await;

        // SPEC-104 A+: same natural-key policy as PostgreSQL path (LAW-I3 / EC-11).
        // Same slug + same name → idempotent get-or-create; different name → Conflict.
        if let Some(existing) = tenants.values().find(|t| t.slug == tenant.slug).cloned() {
            if existing.name.trim() != tenant.name.trim() {
                return Err(Error::conflict(format!(
                    "Tenant slug '{}' already exists (tenant_id={})",
                    existing.slug, existing.tenant_id
                )));
            }
            tracing::info!(
                tenant_id = %existing.tenant_id,
                slug = %existing.slug,
                "Tenant slug already existed — returning existing (SPEC-104)"
            );
            return Ok(existing);
        }

        tenants.insert(tenant.tenant_id, tenant.clone());
        tracing::info!(tenant_id = %tenant.tenant_id, "Created tenant");
        Ok(tenant)
    }

    async fn get_tenant(&self, tenant_id: Uuid) -> Result<Option<Tenant>> {
        let tenants = self.tenants.read().await;
        Ok(tenants.get(&tenant_id).cloned())
    }

    async fn get_tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>> {
        let tenants = self.tenants.read().await;
        Ok(tenants.values().find(|t| t.slug == slug).cloned())
    }

    async fn update_tenant(&self, tenant: Tenant) -> Result<Tenant> {
        let mut tenants = self.tenants.write().await;

        if !tenants.contains_key(&tenant.tenant_id) {
            return Err(Error::not_found(format!(
                "Tenant {} not found",
                tenant.tenant_id
            )));
        }

        tenants.insert(tenant.tenant_id, tenant.clone());
        Ok(tenant)
    }

    async fn delete_tenant(&self, tenant_id: Uuid) -> Result<()> {
        let mut tenants = self.tenants.write().await;
        let mut workspaces = self.workspaces.write().await;
        let mut memberships = self.memberships.write().await;

        tenants.remove(&tenant_id);

        // Remove all workspaces for this tenant
        workspaces.retain(|_, ws| ws.tenant_id != tenant_id);

        // Remove all memberships for this tenant
        memberships.retain(|_, m| m.tenant_id != tenant_id);

        tracing::info!(tenant_id = %tenant_id, "Deleted tenant and all workspaces");
        Ok(())
    }

    async fn list_tenants(&self, limit: usize, offset: usize) -> Result<Vec<Tenant>> {
        let tenants = self.tenants.read().await;
        let mut rows: Vec<Tenant> = tenants.values().cloned().collect();
        rows.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then(b.tenant_id.cmp(&a.tenant_id))
        });
        Ok(rows.into_iter().skip(offset).take(limit).collect())
    }

    async fn count_tenants(&self) -> Result<usize> {
        Ok(self.tenants.read().await.len())
    }

    async fn create_workspace(
        &self,
        tenant_id: Uuid,
        request: CreateWorkspaceRequest,
    ) -> Result<Workspace> {
        // Check tenant exists
        {
            let tenants = self.tenants.read().await;
            let tenant = tenants
                .get(&tenant_id)
                .ok_or_else(|| Error::not_found(format!("Tenant {} not found", tenant_id)))?;

            // Check workspace limit
            let workspaces = self.workspaces.read().await;
            let current_count = workspaces
                .values()
                .filter(|ws| ws.tenant_id == tenant_id)
                .count();

            if current_count >= tenant.max_workspaces {
                return Err(Error::validation(format!(
                    "Tenant has reached maximum workspace limit ({})",
                    tenant.max_workspaces
                )));
            }
        }

        let slug = request
            .slug
            .unwrap_or_else(|| Self::generate_slug(&request.name));

        // Check slug uniqueness within tenant
        {
            let workspaces = self.workspaces.read().await;
            if workspaces
                .values()
                .any(|ws| ws.tenant_id == tenant_id && ws.slug == slug)
            {
                return Err(Error::validation(format!(
                    "Workspace with slug '{}' already exists in this tenant",
                    slug
                )));
            }
        }

        let mut workspace = Workspace::new(tenant_id, &request.name, &slug);

        if let Some(desc) = request.description {
            workspace = workspace.with_description(desc);
        }

        if let Some(max_docs) = request.max_documents {
            workspace = workspace.with_max_documents(max_docs);
        }

        // SPEC-032: Apply LLM/embedding from request and stamp metadata overrides.
        // Without metadata keys, get_workspace → resolve_inherited_model_fields
        // would overwrite struct fields with tenant/server defaults (classifier / mock
        // create tests + "Use tenant defaults" honesty).
        let llm_requested = request.llm_model.is_some() || request.llm_provider.is_some();
        let emb_requested = request.embedding_model.is_some()
            || request.embedding_provider.is_some()
            || request.embedding_dimension.is_some();
        if let Some(model) = request.llm_model {
            workspace = workspace.with_llm_model(&model);
            if let Some(provider) = request.llm_provider {
                workspace = workspace.with_llm_provider(&provider);
            }
        } else if let Some(provider) = request.llm_provider {
            workspace = workspace.with_llm_provider(&provider);
        }
        if let Some(model) = request.embedding_model {
            workspace = workspace.with_embedding_model(&model);
            if let Some(provider) = request.embedding_provider {
                workspace = workspace.with_embedding_provider(&provider);
            } else {
                let detected = Workspace::detect_provider_from_model(&model);
                workspace = workspace.with_embedding_provider(detected);
            }
            if let Some(dim) = request.embedding_dimension {
                workspace = workspace.with_embedding_dimension(dim);
            } else {
                let detected = Workspace::detect_dimension_from_model(&model);
                workspace = workspace.with_embedding_dimension(detected);
            }
        } else if let Some(provider) = request.embedding_provider {
            workspace = workspace.with_embedding_provider(&provider);
            if let Some(dim) = request.embedding_dimension {
                workspace = workspace.with_embedding_dimension(dim);
            }
        } else if let Some(dim) = request.embedding_dimension {
            workspace = workspace.with_embedding_dimension(dim);
        }
        if llm_requested {
            workspace.metadata.insert(
                "llm_model".to_string(),
                serde_json::json!(workspace.llm_model.clone()),
            );
            workspace.metadata.insert(
                "llm_provider".to_string(),
                serde_json::json!(workspace.llm_provider.clone()),
            );
        }
        if emb_requested {
            workspace.metadata.insert(
                "embedding_model".to_string(),
                serde_json::json!(workspace.embedding_model.clone()),
            );
            workspace.metadata.insert(
                "embedding_provider".to_string(),
                serde_json::json!(workspace.embedding_provider.clone()),
            );
            workspace.metadata.insert(
                "embedding_dimension".to_string(),
                serde_json::json!(workspace.embedding_dimension),
            );
        }

        // Persist vision by default when omitted (mirrors postgres create path).
        let pdf_parser_backend = request
            .pdf_parser_backend
            .unwrap_or(edgequake_pdf::PdfParserBackend::Vision);
        workspace.pdf_parser_backend = Some(pdf_parser_backend);
        workspace.metadata.insert(
            "pdf_parser_backend".to_string(),
            serde_json::json!(pdf_parser_backend.as_str()),
        );

        if let Some(model) = request.vision_llm_model {
            workspace.vision_llm_model = Some(model.clone());
            workspace
                .metadata
                .insert("vision_llm_model".to_string(), serde_json::json!(model));
        }
        if let Some(provider) = request.vision_llm_provider {
            workspace.vision_llm_provider = Some(provider.clone());
            workspace.metadata.insert(
                "vision_llm_provider".to_string(),
                serde_json::json!(provider),
            );
        }

        // SPEC-085 / SPEC-114: type allow-lists on create
        crate::type_list::apply_type_list_metadata(
            &mut workspace.metadata,
            "entity_types",
            request.entity_types,
        );
        crate::type_list::apply_type_list_strict_metadata(
            &mut workspace.metadata,
            "entity_types_strict",
            request.entity_types_strict,
        );
        crate::type_list::apply_type_list_metadata(
            &mut workspace.metadata,
            "relation_types",
            request.relation_types,
        );
        crate::type_list::apply_type_list_strict_metadata(
            &mut workspace.metadata,
            "relation_types_strict",
            request.relation_types_strict,
        );
        crate::type_list::apply_kg_schema_preset_metadata(
            &mut workspace.metadata,
            request.kg_schema_preset,
        )
        .map_err(Error::validation)?;
        crate::type_list::apply_relation_edges_metadata(
            &mut workspace.metadata,
            request.relation_edges,
        );
        // SPEC-096: extraction language
        apply_in_memory_extraction_language(&mut workspace.metadata, request.extraction_language)?;
        crate::chunking_metadata::apply_chunking_metadata(
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
        edgequake_pdf::VisionExtractConfig::apply_to_metadata(
            &mut workspace.metadata,
            &edgequake_pdf::VisionExtractOverlay {
                extract_images: request.vision_extract_images,
                extract_charts: request.vision_extract_charts,
                extract_figures: request.vision_extract_figures,
                page_system_prompt: request.vision_page_system_prompt,
                image_system_prompt: request.vision_image_system_prompt,
                chart_system_prompt: request.vision_chart_system_prompt,
                figure_system_prompt: request.vision_figure_system_prompt,
            },
        )
        .map_err(Error::validation)?;
        // SPEC-102: entity type colors
        crate::entity_type_colors::apply_entity_type_colors_metadata(
            &mut workspace.metadata,
            request.entity_type_colors,
        )
        .map_err(Error::validation)?;
        if let Some(effort) = request.default_reasoning_effort {
            let trimmed = effort.trim();
            if trimmed.is_empty()
                || trimmed.eq_ignore_ascii_case("none")
                || trimmed.eq_ignore_ascii_case("auto")
            {
                workspace.metadata.remove("default_reasoning_effort");
            } else {
                workspace.metadata.insert(
                    "default_reasoning_effort".to_string(),
                    serde_json::json!(trimmed),
                );
            }
        }
        if let Some(roles) = request.llm_roles {
            if roles.is_null() || roles.as_object().map(|o| o.is_empty()).unwrap_or(false) {
                workspace.metadata.remove("llm_roles");
            } else {
                workspace.metadata.insert("llm_roles".to_string(), roles);
            }
        }

        let mut workspaces = self.workspaces.write().await;
        workspaces.insert(workspace.workspace_id, workspace.clone());

        tracing::info!(
            workspace_id = %workspace.workspace_id,
            tenant_id = %tenant_id,
            "Created workspace"
        );

        Ok(workspace)
    }

    async fn insert_workspace(&self, workspace: Workspace) -> Result<Workspace> {
        // Validate tenant exists
        {
            let tenants = self.tenants.read().await;
            if !tenants.contains_key(&workspace.tenant_id) {
                return Err(Error::not_found(format!(
                    "Tenant {} not found",
                    workspace.tenant_id
                )));
            }
        }

        // Check slug uniqueness within tenant
        {
            let workspaces = self.workspaces.read().await;
            if workspaces.values().any(|ws| {
                ws.tenant_id == workspace.tenant_id
                    && ws.slug == workspace.slug
                    && ws.workspace_id != workspace.workspace_id
            }) {
                return Err(Error::validation(format!(
                    "Workspace with slug '{}' already exists in this tenant",
                    workspace.slug
                )));
            }
        }

        let mut workspaces = self.workspaces.write().await;
        workspaces.insert(workspace.workspace_id, workspace.clone());

        tracing::info!(
            workspace_id = %workspace.workspace_id,
            tenant_id = %workspace.tenant_id,
            "Inserted workspace with specific ID"
        );

        Ok(workspace)
    }

    async fn get_workspace(&self, workspace_id: Uuid) -> Result<Option<Workspace>> {
        let mut workspace = {
            let workspaces = self.workspaces.read().await;
            match workspaces.get(&workspace_id).cloned() {
                Some(ws) => ws,
                None => return Ok(None),
            }
        };
        let tenants = self.tenants.read().await;
        let tenant = tenants.get(&workspace.tenant_id);
        crate::workspace_model_update::resolve_inherited_model_fields(&mut workspace, tenant);
        Ok(Some(workspace))
    }

    async fn get_workspace_by_slug(
        &self,
        tenant_id: Uuid,
        slug: &str,
    ) -> Result<Option<Workspace>> {
        let mut workspace = {
            let workspaces = self.workspaces.read().await;
            match workspaces
                .values()
                .find(|ws| ws.tenant_id == tenant_id && ws.slug == slug)
                .cloned()
            {
                Some(ws) => ws,
                None => return Ok(None),
            }
        };
        let tenants = self.tenants.read().await;
        let tenant = tenants.get(&tenant_id);
        crate::workspace_model_update::resolve_inherited_model_fields(&mut workspace, tenant);
        Ok(Some(workspace))
    }

    async fn update_workspace(
        &self,
        workspace_id: Uuid,
        request: UpdateWorkspaceRequest,
    ) -> Result<Workspace> {
        let tenant = {
            let workspaces = self.workspaces.read().await;
            let ws = workspaces
                .get(&workspace_id)
                .ok_or_else(|| Error::not_found(format!("Workspace {} not found", workspace_id)))?;
            let tenants = self.tenants.read().await;
            tenants.get(&ws.tenant_id).cloned()
        };

        let mut workspaces = self.workspaces.write().await;

        let workspace = workspaces
            .get_mut(&workspace_id)
            .ok_or_else(|| Error::not_found(format!("Workspace {} not found", workspace_id)))?;

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
            workspace,
            request.llm_model,
            request.llm_provider,
            tenant_llm,
        );
        crate::workspace_model_update::apply_embedding_config_update_with_tenant(
            workspace,
            request.embedding_model,
            request.embedding_provider,
            request.embedding_dimension,
            tenant_emb,
        );

        // SPEC-085 / SPEC-114 / GitHub #216: type allow-list updates
        crate::type_list::apply_type_list_metadata(
            &mut workspace.metadata,
            "entity_types",
            request.entity_types,
        );
        crate::type_list::apply_type_list_strict_metadata(
            &mut workspace.metadata,
            "entity_types_strict",
            request.entity_types_strict,
        );
        crate::type_list::apply_type_list_metadata(
            &mut workspace.metadata,
            "relation_types",
            request.relation_types,
        );
        crate::type_list::apply_type_list_strict_metadata(
            &mut workspace.metadata,
            "relation_types_strict",
            request.relation_types_strict,
        );
        crate::type_list::apply_kg_schema_preset_metadata(
            &mut workspace.metadata,
            request.kg_schema_preset,
        )
        .map_err(Error::validation)?;
        crate::type_list::apply_relation_edges_metadata(
            &mut workspace.metadata,
            request.relation_edges,
        );
        apply_in_memory_extraction_language(&mut workspace.metadata, request.extraction_language)?;
        crate::chunking_metadata::apply_chunking_metadata(
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
        edgequake_pdf::VisionExtractConfig::apply_to_metadata(
            &mut workspace.metadata,
            &edgequake_pdf::VisionExtractOverlay {
                extract_images: request.vision_extract_images,
                extract_charts: request.vision_extract_charts,
                extract_figures: request.vision_extract_figures,
                page_system_prompt: request.vision_page_system_prompt,
                image_system_prompt: request.vision_image_system_prompt,
                chart_system_prompt: request.vision_chart_system_prompt,
                figure_system_prompt: request.vision_figure_system_prompt,
            },
        )
        .map_err(Error::validation)?;
        crate::entity_type_colors::apply_entity_type_colors_metadata(
            &mut workspace.metadata,
            request.entity_type_colors,
        )
        .map_err(Error::validation)?;
        if let Some(effort) = request.default_reasoning_effort {
            let trimmed = effort.trim();
            if trimmed.is_empty()
                || trimmed.eq_ignore_ascii_case("none")
                || trimmed.eq_ignore_ascii_case("auto")
            {
                workspace.metadata.remove("default_reasoning_effort");
            } else {
                workspace.metadata.insert(
                    "default_reasoning_effort".to_string(),
                    serde_json::json!(trimmed),
                );
            }
        }
        if let Some(roles) = request.llm_roles {
            if roles.is_null() || roles.as_object().map(|o| o.is_empty()).unwrap_or(false) {
                workspace.metadata.remove("llm_roles");
            } else {
                workspace.metadata.insert("llm_roles".to_string(), roles);
            }
        }

        // SPEC-123 / SPEC-038: PDF parser override (mirrors postgres workspace_ops).
        if let Some(pdf_parser_backend) = request.pdf_parser_backend {
            let normalized_backend = pdf_parser_backend.trim().to_ascii_lowercase();
            if normalized_backend.is_empty() || normalized_backend == "none" {
                workspace.pdf_parser_backend = None;
                workspace.metadata.remove("pdf_parser_backend");
            } else if let Some(parsed_backend) =
                edgequake_pdf::PdfParserBackend::from_env_str(&normalized_backend)
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

        if let Some(provider) = request.vision_llm_provider {
            let trimmed = provider.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
                workspace.vision_llm_provider = None;
                workspace.metadata.remove("vision_llm_provider");
            } else {
                workspace.vision_llm_provider = Some(trimmed.to_string());
                workspace.metadata.insert(
                    "vision_llm_provider".to_string(),
                    serde_json::json!(trimmed),
                );
            }
        }
        if let Some(model) = request.vision_llm_model {
            let trimmed = model.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
                workspace.vision_llm_model = None;
                workspace.metadata.remove("vision_llm_model");
            } else {
                workspace.vision_llm_model = Some(trimmed.to_string());
                workspace
                    .metadata
                    .insert("vision_llm_model".to_string(), serde_json::json!(trimmed));
            }
        }

        workspace.updated_at = chrono::Utc::now();

        crate::workspace_model_update::resolve_inherited_model_fields(workspace, tenant.as_ref());
        Ok(workspace.clone())
    }

    async fn delete_workspace(&self, workspace_id: Uuid) -> Result<()> {
        let mut workspaces = self.workspaces.write().await;
        let mut memberships = self.memberships.write().await;

        workspaces.remove(&workspace_id);
        memberships.retain(|_, m| m.workspace_id != Some(workspace_id));

        tracing::info!(workspace_id = %workspace_id, "Deleted workspace");
        Ok(())
    }

    async fn list_workspaces(&self, tenant_id: Uuid) -> Result<Vec<Workspace>> {
        self.list_workspaces_page(tenant_id, usize::MAX, 0).await
    }

    async fn list_workspaces_page(
        &self,
        tenant_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Workspace>> {
        let tenants = self.tenants.read().await;
        let tenant = tenants.get(&tenant_id).cloned();
        let workspaces = self.workspaces.read().await;
        let mut rows: Vec<Workspace> = workspaces
            .values()
            .filter(|ws| ws.tenant_id == tenant_id)
            .cloned()
            .collect();
        rows.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then(b.workspace_id.cmp(&a.workspace_id))
        });
        Ok(rows
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|mut ws| {
                crate::workspace_model_update::resolve_inherited_model_fields(
                    &mut ws,
                    tenant.as_ref(),
                );
                ws
            })
            .collect())
    }

    async fn count_workspaces(&self, tenant_id: Uuid) -> Result<usize> {
        let workspaces = self.workspaces.read().await;
        Ok(workspaces
            .values()
            .filter(|ws| ws.tenant_id == tenant_id)
            .count())
    }

    async fn get_workspace_stats(&self, workspace_id: Uuid) -> Result<WorkspaceStats> {
        // WHY zeros: In-memory implementation is a stub for single-tenant mode.
        // Real metrics require storage adapters which are not available here.
        // TODO: Accept storage adapters in constructor for real-time counting.
        Ok(WorkspaceStats {
            workspace_id,
            document_count: 0,
            entity_count: 0,
            relationship_count: 0,
            chunk_count: 0,
            embedding_count: 0,
            storage_bytes: 0,
        })
    }

    async fn record_metrics_snapshot(
        &self,
        workspace_id: Uuid,
        trigger_type: MetricsTriggerType,
    ) -> Result<MetricsSnapshot> {
        // WHY stub: In-memory implementation doesn't persist history.
        // Returns a snapshot with current (zero) stats for testing compatibility.
        // OODA-20: Real implementation is in PostgresWorkspaceService.
        Ok(MetricsSnapshot {
            id: Uuid::new_v4(),
            workspace_id,
            recorded_at: chrono::Utc::now(),
            trigger_type,
            document_count: 0,
            chunk_count: 0,
            entity_count: 0,
            relationship_count: 0,
            embedding_count: 0,
            storage_bytes: 0,
        })
    }

    async fn get_metrics_history(
        &self,
        _workspace_id: Uuid,
        _limit: usize,
        _offset: usize,
    ) -> Result<Vec<MetricsSnapshot>> {
        // WHY empty: In-memory implementation doesn't persist history.
        // OODA-22: Real implementation is in PostgresWorkspaceService.
        Ok(Vec::new())
    }

    async fn add_membership(&self, membership: Membership) -> Result<Membership> {
        let mut memberships = self.memberships.write().await;

        // Check for existing membership
        let exists = memberships.values().any(|m| {
            m.user_id == membership.user_id
                && m.tenant_id == membership.tenant_id
                && m.workspace_id == membership.workspace_id
        });

        if exists {
            return Err(Error::validation("Membership already exists"));
        }

        memberships.insert(membership.membership_id, membership.clone());

        tracing::info!(
            membership_id = %membership.membership_id,
            user_id = %membership.user_id,
            tenant_id = %membership.tenant_id,
            "Added membership"
        );

        Ok(membership)
    }

    async fn get_user_memberships(&self, user_id: Uuid) -> Result<Vec<Membership>> {
        let memberships = self.memberships.read().await;
        Ok(memberships
            .values()
            .filter(|m| m.user_id == user_id && m.is_active)
            .cloned()
            .collect())
    }

    async fn get_tenant_memberships(&self, tenant_id: Uuid) -> Result<Vec<Membership>> {
        let memberships = self.memberships.read().await;
        Ok(memberships
            .values()
            .filter(|m| m.tenant_id == tenant_id && m.is_active)
            .cloned()
            .collect())
    }

    async fn update_membership_role(
        &self,
        membership_id: Uuid,
        role: MembershipRole,
    ) -> Result<Membership> {
        let mut memberships = self.memberships.write().await;

        let membership = memberships
            .get_mut(&membership_id)
            .ok_or_else(|| Error::not_found(format!("Membership {} not found", membership_id)))?;

        membership.role = role;

        Ok(membership.clone())
    }

    async fn remove_membership(&self, membership_id: Uuid) -> Result<()> {
        let mut memberships = self.memberships.write().await;
        memberships.remove(&membership_id);
        Ok(())
    }

    async fn check_tenant_access(&self, user_id: Uuid, tenant_id: Uuid) -> Result<bool> {
        if !self
            .get_tenant(tenant_id)
            .await?
            .is_some_and(|t| t.is_active)
        {
            return Ok(false);
        }
        let memberships = self.memberships.read().await;
        Ok(memberships
            .values()
            .any(|m| m.user_id == user_id && m.tenant_id == tenant_id && m.is_active))
    }

    async fn check_workspace_access(&self, user_id: Uuid, workspace_id: Uuid) -> Result<bool> {
        let Some(workspace) = self.get_workspace(workspace_id).await? else {
            return Ok(false);
        };
        if !workspace.is_active
            || !self
                .get_tenant(workspace.tenant_id)
                .await?
                .is_some_and(|t| t.is_active)
        {
            return Ok(false);
        }

        let memberships = self.memberships.read().await;
        Ok(memberships.values().any(|m| {
            m.user_id == user_id
                && m.tenant_id == workspace.tenant_id
                && m.is_active
                && m.can_access_workspace(&workspace_id)
        }))
    }

    async fn get_user_role(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Option<MembershipRole>> {
        let memberships = self.memberships.read().await;
        Ok(memberships
            .values()
            .filter(|m| {
                m.user_id == user_id
                    && m.tenant_id == tenant_id
                    && m.is_active
                    && m.workspace_id.is_none()
            })
            .map(|m| m.role)
            .max_by_key(MembershipRole::level))
    }

    async fn build_context(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        workspace_id: Option<Uuid>,
    ) -> Result<TenantContext> {
        if !self
            .get_tenant(tenant_id)
            .await?
            .is_some_and(|t| t.is_active)
        {
            return Err(Error::validation("Tenant is inactive or missing"));
        }
        // Check access
        if !self.check_tenant_access(user_id, tenant_id).await? {
            return Err(Error::validation(format!(
                "User {} does not have access to tenant {}",
                user_id, tenant_id
            )));
        }

        // If workspace specified, check access
        if let Some(ws_id) = workspace_id {
            if !self
                .get_workspace(ws_id)
                .await?
                .is_some_and(|w| w.is_active && w.tenant_id == tenant_id)
            {
                return Err(Error::validation("Workspace is inactive or outside tenant"));
            }
            if !self.check_workspace_access(user_id, ws_id).await? {
                return Err(Error::validation(format!(
                    "User {} does not have access to workspace {}",
                    user_id, ws_id
                )));
            }
        }

        let role = self
            .get_user_memberships(user_id)
            .await?
            .into_iter()
            .filter(|m| m.applies_to(tenant_id, workspace_id))
            .map(|m| m.role)
            .max_by_key(MembershipRole::level)
            .ok_or_else(|| Error::validation("No active membership for scope"))?;

        let mut ctx = TenantContext::new(tenant_id);
        if let Some(ws_id) = workspace_id {
            ctx = ctx.with_workspace(ws_id);
        }
        ctx = ctx.with_user(user_id, role);

        Ok(ctx)
    }

    // ============ Quota Operations (SPEC-0001) ============

    async fn update_tenant_quota(
        &self,
        tenant_id: Uuid,
        new_max_workspaces: usize,
    ) -> Result<UpdateTenantQuotaResult> {
        // Validation V1: must be positive
        if new_max_workspaces == 0 {
            return Err(Error::validation("max_workspaces must be positive"));
        }
        // Validation V3: sanity limit
        if new_max_workspaces > 10_000 {
            return Err(Error::validation(
                "max_workspaces exceeds sanity limit (10000)",
            ));
        }

        let mut tenants = self.tenants.write().await;
        let tenant = tenants
            .get_mut(&tenant_id)
            .ok_or_else(|| Error::not_found(format!("Tenant {} not found", tenant_id)))?;

        let previous = tenant.max_workspaces;

        // Count workspaces under write lock to avoid TOCTOU
        let current_count = {
            let workspaces = self.workspaces.read().await;
            workspaces
                .values()
                .filter(|ws| ws.tenant_id == tenant_id)
                .count()
        };

        // Validation V2: cannot go below current usage
        if new_max_workspaces < current_count {
            return Err(Error::validation(format!(
                "Cannot reduce below current workspace count ({})",
                current_count
            )));
        }

        tenant.max_workspaces = new_max_workspaces;
        tenant.updated_at = chrono::Utc::now();

        tracing::info!(
            tenant_id = %tenant_id,
            previous = previous,
            new = new_max_workspaces,
            current_count = current_count,
            "SPEC-0001: Updated tenant quota"
        );

        Ok(UpdateTenantQuotaResult {
            tenant_id,
            max_workspaces: new_max_workspaces,
            previous_max_workspaces: previous,
            current_workspace_count: current_count,
        })
    }

    async fn get_server_default_max_workspaces(&self) -> Result<usize> {
        // Check env var override first
        if let Ok(val) = std::env::var("EDGEQUAKE_DEFAULT_MAX_WORKSPACES") {
            if let Ok(n) = val.parse::<usize>() {
                return Ok(n);
            }
        }
        Ok(*self.server_default_max_workspaces.read().await)
    }

    async fn set_server_default_max_workspaces(&self, value: usize) -> Result<usize> {
        if value == 0 {
            return Err(Error::validation("default_max_workspaces must be positive"));
        }
        if value > 10_000 {
            return Err(Error::validation(
                "default_max_workspaces exceeds sanity limit (10000)",
            ));
        }
        *self.server_default_max_workspaces.write().await = value;
        tracing::info!(
            value = value,
            "SPEC-0001: Updated server default max_workspaces"
        );
        Ok(value)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_tenant() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("Test Tenant", "test-tenant").with_plan(TenantPlan::Basic);

        let created = service.create_tenant(tenant).await.unwrap();
        assert_eq!(created.name, "Test Tenant");
        assert_eq!(created.slug, "test-tenant");
        assert_eq!(created.plan, TenantPlan::Basic);
    }

    #[tokio::test]
    async fn test_create_workspace() {
        let service = InMemoryWorkspaceService::new();

        // Create tenant first
        let tenant = Tenant::new("Test Tenant", "test");
        let tenant = service.create_tenant(tenant).await.unwrap();

        // Create workspace
        let request = CreateWorkspaceRequest {
            name: "My Knowledge Base".to_string(),
            slug: Some("my-kb".to_string()),
            description: Some("Test KB".to_string()),
            max_documents: Some(1000),
            llm_model: None,
            llm_provider: None,
            embedding_model: None,
            embedding_provider: None,
            embedding_dimension: None,
            vision_llm_model: None,
            vision_llm_provider: None,
            pdf_parser_backend: None,
            entity_types: None,
            entity_types_strict: None,
            extraction_language: None,
            chunking_mode: None,
            chunk_token_size: None,
            chunk_overlap_token_size: None,
            extract_budget_mode: None,
            extract_max_entities: None,
            extract_max_records: None,
            extraction_mode: None,
            decision_gate_preset: None,
            decision_model: None,
            decision_pack_size: None,
            decision_enabled: None,
            entity_type_colors: None,
            relation_types: None,
            relation_types_strict: None,
            kg_schema_preset: None,
            relation_edges: None,
            default_reasoning_effort: None,
            llm_roles: None,
            vision_extract_images: None,
            vision_extract_charts: None,
            vision_extract_figures: None,
            vision_page_system_prompt: None,
            vision_image_system_prompt: None,
            vision_chart_system_prompt: None,
            vision_figure_system_prompt: None,
        };

        let workspace = service
            .create_workspace(tenant.tenant_id, request)
            .await
            .unwrap();
        assert_eq!(workspace.name, "My Knowledge Base");
        assert_eq!(workspace.slug, "my-kb");
        assert_eq!(workspace.max_documents(), Some(1000));
        assert_eq!(
            workspace.pdf_parser_backend,
            Some(edgequake_pdf::PdfParserBackend::Vision),
            "omitted pdf_parser_backend must persist vision"
        );
        assert_eq!(
            workspace
                .metadata
                .get("pdf_parser_backend")
                .and_then(|v| v.as_str()),
            Some("vision")
        );
    }

    /// Explicit create overrides must survive get_workspace inheritance.
    #[tokio::test]
    async fn create_workspace_stamps_llm_metadata_for_get() {
        let service = InMemoryWorkspaceService::new();
        let tenant = service
            .create_tenant(Tenant::new("Test Tenant", "test-meta"))
            .await
            .unwrap();
        let created = service
            .create_workspace(
                tenant.tenant_id,
                CreateWorkspaceRequest {
                    name: "Explicit".to_string(),
                    slug: Some(format!("explicit-{}", Uuid::new_v4())),
                    llm_provider: Some("ollama".to_string()),
                    llm_model: Some("gemma3:latest".to_string()),
                    embedding_provider: Some("mock".to_string()),
                    embedding_model: Some("mock-emb".to_string()),
                    embedding_dimension: Some(8),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let got = service
            .get_workspace(created.workspace_id)
            .await
            .unwrap()
            .expect("workspace");
        assert_eq!(got.llm_provider, "ollama");
        assert_eq!(got.llm_model, "gemma3:latest");
        assert_eq!(got.embedding_provider, "mock");
        assert_eq!(got.embedding_model, "mock-emb");
        assert_eq!(got.embedding_dimension, 8);
    }

    #[tokio::test]
    async fn test_create_workspace_explicit_edgeparse_overrides_default() {
        let service = InMemoryWorkspaceService::new();
        let tenant = service
            .create_tenant(Tenant::new("Test Tenant", "test-edgeparse"))
            .await
            .unwrap();

        let request = CreateWorkspaceRequest {
            name: "EdgeParse WS".to_string(),
            slug: Some("edgeparse-ws".to_string()),
            pdf_parser_backend: Some(edgequake_pdf::PdfParserBackend::EdgeParse),
            ..Default::default()
        };
        let workspace = service
            .create_workspace(tenant.tenant_id, request)
            .await
            .unwrap();
        assert_eq!(
            workspace.pdf_parser_backend,
            Some(edgequake_pdf::PdfParserBackend::EdgeParse)
        );
    }

    #[tokio::test]
    async fn test_seed_default_workspace_persists_vision() {
        let service = InMemoryWorkspaceService::new();
        service.seed_default_workspace().await;
        let workspace = service
            .get_workspace(Uuid::from_u128(3))
            .await
            .unwrap()
            .expect("seeded default workspace");
        assert_eq!(
            workspace.pdf_parser_backend,
            Some(edgequake_pdf::PdfParserBackend::Vision)
        );
    }

    #[tokio::test]
    async fn test_workspace_limit() {
        let service = InMemoryWorkspaceService::new();

        // Create tenant with limit of 2 workspaces
        let mut tenant = Tenant::new("Limited Tenant", "limited");
        tenant.max_workspaces = 2;
        let tenant = service.create_tenant(tenant).await.unwrap();

        // Create 2 workspaces (should succeed)
        for i in 0..2 {
            let request = CreateWorkspaceRequest {
                name: format!("Workspace {}", i),
                slug: Some(format!("ws-{}", i)),
                description: None,
                max_documents: None,
                llm_model: None,
                llm_provider: None,
                embedding_model: None,
                embedding_provider: None,
                embedding_dimension: None,
                vision_llm_model: None,
                vision_llm_provider: None,
                pdf_parser_backend: None,
                entity_types: None,
                entity_types_strict: None,
                extraction_language: None,
                chunking_mode: None,
                chunk_token_size: None,
                chunk_overlap_token_size: None,
                extract_budget_mode: None,
                extract_max_entities: None,
                extract_max_records: None,
                extraction_mode: None,
                decision_gate_preset: None,
                decision_model: None,
                decision_pack_size: None,
                decision_enabled: None,
                entity_type_colors: None,
                relation_types: None,
                relation_types_strict: None,
                kg_schema_preset: None,
                relation_edges: None,
                default_reasoning_effort: None,
                llm_roles: None,
                vision_extract_images: None,
                vision_extract_charts: None,
                vision_extract_figures: None,
                vision_page_system_prompt: None,
                vision_image_system_prompt: None,
                vision_chart_system_prompt: None,
                vision_figure_system_prompt: None,
            };
            service
                .create_workspace(tenant.tenant_id, request)
                .await
                .unwrap();
        }

        // Third workspace should fail
        let request = CreateWorkspaceRequest {
            name: "Workspace 3".to_string(),
            slug: Some("ws-3".to_string()),
            description: None,
            max_documents: None,
            llm_model: None,
            llm_provider: None,
            embedding_model: None,
            embedding_provider: None,
            embedding_dimension: None,
            vision_llm_model: None,
            vision_llm_provider: None,
            pdf_parser_backend: None,
            entity_types: None,
            entity_types_strict: None,
            extraction_language: None,
            chunking_mode: None,
            chunk_token_size: None,
            chunk_overlap_token_size: None,
            extract_budget_mode: None,
            extract_max_entities: None,
            extract_max_records: None,
            extraction_mode: None,
            decision_gate_preset: None,
            decision_model: None,
            decision_pack_size: None,
            decision_enabled: None,
            entity_type_colors: None,
            relation_types: None,
            relation_types_strict: None,
            kg_schema_preset: None,
            relation_edges: None,
            default_reasoning_effort: None,
            llm_roles: None,
            vision_extract_images: None,
            vision_extract_charts: None,
            vision_extract_figures: None,
            vision_page_system_prompt: None,
            vision_image_system_prompt: None,
            vision_chart_system_prompt: None,
            vision_figure_system_prompt: None,
        };
        let result = service.create_workspace(tenant.tenant_id, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_membership_access() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("Test", "test");
        let tenant = service.create_tenant(tenant).await.unwrap();

        let user_id = Uuid::new_v4();

        // No access initially
        assert!(!service
            .check_tenant_access(user_id, tenant.tenant_id)
            .await
            .unwrap());

        // Add membership
        let membership = Membership::new(user_id, tenant.tenant_id, MembershipRole::Member);
        service.add_membership(membership).await.unwrap();

        // Now has access
        assert!(service
            .check_tenant_access(user_id, tenant.tenant_id)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn scoped_context_rejects_tenant_escalation_and_foreign_workspace() {
        let service = InMemoryWorkspaceService::new();
        let tenant = service
            .create_tenant(Tenant::new("Scope tenant", "scope-tenant"))
            .await
            .unwrap();
        let foreign = service
            .create_tenant(Tenant::new("Foreign tenant", "foreign-tenant"))
            .await
            .unwrap();
        let workspace = service
            .create_workspace(
                tenant.tenant_id,
                CreateWorkspaceRequest {
                    name: "Scoped workspace".into(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let user = Uuid::new_v4();
        service
            .add_membership(
                Membership::new(user, tenant.tenant_id, MembershipRole::Admin)
                    .for_workspace(workspace.workspace_id),
            )
            .await
            .unwrap();
        service
            .add_membership(Membership::new(
                user,
                foreign.tenant_id,
                MembershipRole::Readonly,
            ))
            .await
            .unwrap();
        assert!(service
            .build_context(user, tenant.tenant_id, None)
            .await
            .is_err());
        assert!(service
            .build_context(user, foreign.tenant_id, Some(workspace.workspace_id))
            .await
            .is_err());
        assert_eq!(
            service
                .build_context(user, tenant.tenant_id, Some(workspace.workspace_id))
                .await
                .unwrap()
                .role,
            Some(MembershipRole::Admin)
        );
    }

    #[tokio::test]
    async fn test_build_context() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("Test", "test");
        let tenant = service.create_tenant(tenant).await.unwrap();

        let user_id = Uuid::new_v4();

        // Without membership, should fail
        let result = service.build_context(user_id, tenant.tenant_id, None).await;
        assert!(result.is_err());

        // Add membership
        let membership = Membership::new(user_id, tenant.tenant_id, MembershipRole::Admin);
        service.add_membership(membership).await.unwrap();

        // Now should succeed
        let ctx = service
            .build_context(user_id, tenant.tenant_id, None)
            .await
            .unwrap();
        assert!(ctx.is_valid());
        assert_eq!(ctx.tenant_id, Some(tenant.tenant_id));
        assert!(ctx.can_write());
    }
}
