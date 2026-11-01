//! SPEC-123: single Workspace → WorkspaceResponse mapper (DRY / SOLID).
//!
//! Resolution (LLM / embedding / vision) is a separate concern from persistence paint:
//! this mapper attaches honest `resolved_*` + `*_resolution_source` via
//! [`edgequake_core::model_resolution`].

use edgequake_core::{
    resolve_embedding_choice, resolve_llm_choice, resolve_vision_llm_choice, Tenant, Workspace,
};

use super::responses::WorkspaceResponse;

/// Flag an update whose resolved embedding stack differs from the one before it (SPEC-178).
///
/// A different model or dimension means stored vectors came from another embedder, so the
/// update response says a rebuild is required and reports the previous dimension.
pub fn mark_embedding_rebuild(before: &WorkspaceResponse, after: &mut WorkspaceResponse) {
    let changed = before.resolved_embedding_provider != after.resolved_embedding_provider
        || before.resolved_embedding_model != after.resolved_embedding_model
        || before.resolved_embedding_dimension != after.resolved_embedding_dimension;
    after.embedding_rebuild_required = changed;
    after.previous_embedding_dimension = if changed {
        before.resolved_embedding_dimension
    } else {
        None
    };
}

/// Convert domain workspace (+ optional tenant) to API DTO with SPEC-123 provenance.
pub fn workspace_to_response(workspace: &Workspace, tenant: Option<&Tenant>) -> WorkspaceResponse {
    let llm = resolve_llm_choice(None, None, Some(workspace), tenant);
    let emb = resolve_embedding_choice(None, None, None, Some(workspace), tenant);
    let vision = resolve_vision_llm_choice(None, None, Some(workspace), tenant);

    // Inherited groups are not stored on the workspace (LAW-123-8), so the raw LLM and
    // embedding fields show the effective values. They then agree with `resolved_*`.
    let mut workspace = workspace.clone();
    workspace.llm_provider = llm.provider.clone();
    workspace.llm_model = llm.model.clone();
    workspace.embedding_provider = emb.provider.clone();
    workspace.embedding_model = emb.model.clone();
    workspace.embedding_dimension = emb.dimension;
    let workspace = &workspace;

    WorkspaceResponse {
        id: workspace.workspace_id,
        tenant_id: workspace.tenant_id,
        name: workspace.name.clone(),
        slug: workspace.slug.clone(),
        description: workspace.description.clone(),
        is_active: workspace.is_active,
        archived_at: workspace.archived_at.map(|t| t.to_rfc3339()),
        max_documents: workspace.max_documents(),
        llm_model: workspace.llm_model.clone(),
        llm_provider: workspace.llm_provider.clone(),
        llm_full_id: workspace.llm_full_id(),
        resolved_llm_provider: Some(llm.provider.clone()),
        resolved_llm_model: Some(llm.model.clone()),
        llm_resolution_source: Some(llm.source.as_str().to_string()),
        embedding_model: workspace.embedding_model.clone(),
        embedding_provider: workspace.embedding_provider.clone(),
        embedding_dimension: workspace.embedding_dimension,
        embedding_full_id: workspace.embedding_full_id(),
        resolved_embedding_provider: Some(emb.provider.clone()),
        resolved_embedding_model: Some(emb.model.clone()),
        resolved_embedding_dimension: Some(emb.dimension),
        embedding_resolution_source: Some(emb.source.as_str().to_string()),
        embedding_rebuild_required: false,
        previous_embedding_dimension: None,
        vision_llm_provider: workspace.vision_llm_provider.clone(),
        vision_llm_model: workspace.vision_llm_model.clone(),
        resolved_vision_llm_provider: Some(vision.provider.clone()),
        resolved_vision_llm_model: Some(vision.model.clone()),
        vision_llm_resolution_source: Some(vision.source.as_str().to_string()),
        pdf_parser_backend: workspace
            .pdf_parser_backend
            .map(|backend| backend.as_str().to_string()),
        entity_types: workspace
            .metadata
            .get("entity_types")
            .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok()),
        entity_types_strict: workspace
            .metadata
            .get("entity_types_strict")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        extraction_language: workspace
            .metadata
            .get("extraction_language")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        chunking_mode: workspace
            .metadata
            .get("chunking_mode")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        chunk_token_size: workspace
            .metadata
            .get("chunk_token_size")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        chunk_overlap_token_size: workspace
            .metadata
            .get("chunk_overlap_token_size")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        extract_budget_mode: workspace
            .metadata
            .get("extract_max_entities")
            .map(|_| "custom".to_string()),
        extract_max_entities: workspace
            .metadata
            .get("extract_max_entities")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        extract_max_records: workspace
            .metadata
            .get("extract_max_records")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        extraction: super::ExtractionModeView::from_metadata(&workspace.metadata),
        profile: super::RagProfileView::from_workspace(workspace),
        entity_type_colors: workspace.metadata.get("entity_type_colors").and_then(|v| {
            serde_json::from_value::<std::collections::HashMap<String, String>>(v.clone()).ok()
        }),
        relation_types: workspace
            .metadata
            .get("relation_types")
            .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok()),
        relation_types_strict: workspace
            .metadata
            .get("relation_types_strict")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        kg_schema_preset: workspace
            .metadata
            .get("kg_schema_preset")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        relation_edges: workspace.metadata.get("relation_edges").and_then(|v| {
            serde_json::from_value::<Vec<super::RelationEdgeDto>>(v.clone())
                .ok()
                .filter(|e| !e.is_empty())
        }),
        default_reasoning_effort: workspace
            .metadata
            .get("default_reasoning_effort")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        llm_roles: workspace.metadata.get("llm_roles").cloned(),
        vision_extract_images: workspace
            .metadata
            .get(edgequake_pdf::META_EXTRACT_IMAGES)
            .and_then(|v| v.as_bool()),
        vision_extract_charts: workspace
            .metadata
            .get(edgequake_pdf::META_EXTRACT_CHARTS)
            .and_then(|v| v.as_bool()),
        vision_extract_figures: workspace
            .metadata
            .get(edgequake_pdf::META_EXTRACT_FIGURES)
            .and_then(|v| v.as_bool()),
        vision_page_system_prompt: workspace
            .metadata
            .get(edgequake_pdf::META_PAGE_SYSTEM_PROMPT)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        vision_image_system_prompt: workspace
            .metadata
            .get(edgequake_pdf::META_IMAGE_SYSTEM_PROMPT)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        vision_chart_system_prompt: workspace
            .metadata
            .get(edgequake_pdf::META_CHART_SYSTEM_PROMPT)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        vision_figure_system_prompt: workspace
            .metadata
            .get(edgequake_pdf::META_FIGURE_SYSTEM_PROMPT)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        stats: None,
        created_at: workspace.created_at.to_rfc3339(),
        updated_at: workspace.updated_at.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn response() -> WorkspaceResponse {
        let ws = Workspace::new(Uuid::nil(), "ws", "ws");
        workspace_to_response(&ws, None)
    }

    #[test]
    fn unchanged_embedding_stack_needs_no_rebuild() {
        let before = response();
        let mut after = response();
        mark_embedding_rebuild(&before, &mut after);
        assert!(!after.embedding_rebuild_required);
        assert_eq!(after.previous_embedding_dimension, None);
    }

    #[test]
    fn changed_dimension_requires_rebuild_and_reports_previous() {
        let before = response();
        let mut after = response();
        after.resolved_embedding_dimension = Some(768);
        mark_embedding_rebuild(&before, &mut after);
        assert!(after.embedding_rebuild_required);
        assert_eq!(
            after.previous_embedding_dimension,
            before.resolved_embedding_dimension
        );
    }

    #[test]
    fn changed_model_with_same_dimension_still_requires_rebuild() {
        let before = response();
        let mut after = response();
        after.resolved_embedding_model = Some("another-model".to_string());
        mark_embedding_rebuild(&before, &mut after);
        assert!(after.embedding_rebuild_required);
    }
}
