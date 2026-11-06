//! VLM provider resolution for image ingest (SPEC-026 Phase 4 / SPEC-123).
//!
//! Priority (LAW-123-2 — Upload → Workspace vision → Tenant vision → Workspace LLM → Env):
//!   1. Workspace `vision_llm_provider` / `vision_llm_model` (or `llm_roles.vlm`)
//!   2. Tenant `default_vision_llm_*` (when workspace vision unset)
//!   3. Workspace main `llm_provider` / `llm_model`
//!   4. Server env vision defaults (`EDGEQUAKE_VISION_*`)
//!   5. Startup `vision_llm_provider` singleton
//!   6. Server default text LLM
//!
//! There is no separate “vision embedding” model — vision is VLM; embeddings are text vectors.

use std::sync::Arc;

use edgequake_core::{
    resolve_role_llm, resolve_vision_llm_choice, LlmRole, Workspace, WorkspaceService,
};
use edgequake_llm::traits::LLMProvider;
use uuid::Uuid;

use crate::safety_limits::{
    create_safe_llm_provider, create_safe_vision_provider, create_safe_vision_provider_for_pass_b,
};
use crate::state::AppState;
use crate::vision_env::{default_vision_model_for_provider, resolved_vision_provider_from_env};

/// Resolve the workspace VLM choice (SPEC-163 / SPEC-123).
///
/// One resolver for the pipeline and the API: `llm_roles.vlm` is canonical, then the legacy
/// vision fields, tenant vision, the workspace LLM, and env. The connection id comes from the
/// role only; it is `None` for legacy and inherited choices.
pub fn resolve_workspace_vlm_config(
    ws: &Workspace,
    tenant: Option<&edgequake_core::Tenant>,
) -> edgequake_core::ResolvedRoleLlm {
    let resolved = resolve_vision_llm_choice(None, None, Some(ws), tenant);
    edgequake_core::ResolvedRoleLlm {
        provider: resolved.provider,
        model: resolved.model,
        connection_id: resolve_role_llm(ws, LlmRole::Vlm).connection_id,
    }
}

async fn load_workspace_vlm(
    workspace_service: &Arc<dyn WorkspaceService>,
    workspace_id: Uuid,
) -> Option<edgequake_core::ResolvedRoleLlm> {
    let ws = workspace_service.get_workspace(workspace_id).await.ok()??;
    let tenant = workspace_service
        .get_tenant(ws.tenant_id)
        .await
        .ok()
        .flatten();
    Some(resolve_workspace_vlm_config(&ws, tenant.as_ref()))
}

async fn try_workspace_vlm(
    workspace_service: &Arc<dyn WorkspaceService>,
    workspace_id: Uuid,
) -> Option<Arc<dyn LLMProvider>> {
    let cfg = load_workspace_vlm(workspace_service, workspace_id).await?;
    if let Some(id) = cfg.connection_id.as_deref() {
        if let Some(connected) =
            crate::providers::connection_store::llm_from_ambient_connection(id, &cfg.model).await
        {
            return Some(connected);
        }
    }
    saved_fleet_or_safe(&cfg.provider, &cfg.model, false).await
}

async fn saved_fleet_or_safe(
    provider: &str,
    model: &str,
    pass_b: bool,
) -> Option<Arc<dyn LLMProvider>> {
    match crate::providers::connection_store::fleet_llm_from_ambient(provider, model).await {
        Ok(Some(provider)) => return Some(provider),
        Ok(None) => {}
        Err(error) => {
            tracing::warn!(
                %error,
                provider,
                "saved provider connection could not be opened"
            );
            return None;
        }
    }
    if pass_b {
        create_safe_vision_provider_for_pass_b(provider, model).ok()
    } else {
        create_safe_vision_provider(provider, model).ok()
    }
}

async fn try_workspace_vlm_pass_b(
    workspace_service: &Arc<dyn WorkspaceService>,
    workspace_id: Uuid,
) -> Option<Arc<dyn LLMProvider>> {
    let cfg = load_workspace_vlm(workspace_service, workspace_id).await?;
    if let Some(id) = cfg.connection_id.as_deref() {
        if let Some(connected) =
            crate::providers::connection_store::llm_from_ambient_connection(id, &cfg.model).await
        {
            return Some(connected);
        }
    }
    saved_fleet_or_safe(&cfg.provider, &cfg.model, true).await
}

async fn try_workspace_extract(
    workspace_service: &Arc<dyn WorkspaceService>,
    workspace_id: Uuid,
) -> Option<Arc<dyn LLMProvider>> {
    let ws = workspace_service.get_workspace(workspace_id).await.ok()??;
    let role = resolve_role_llm(&ws, LlmRole::Extract);
    if let Some(id) = role.connection_id.as_deref() {
        if let Some(connected) =
            crate::providers::connection_store::llm_from_ambient_connection(id, &role.model).await
        {
            return Some(connected);
        }
    }
    match crate::providers::connection_store::fleet_llm_from_ambient(&role.provider, &role.model)
        .await
    {
        Ok(Some(provider)) => return Some(provider),
        Ok(None) => {}
        Err(error) => {
            tracing::warn!(
                %error,
                provider = %role.provider,
                "saved extract connection could not be opened"
            );
            return None;
        }
    }
    create_safe_llm_provider(&role.provider, &role.model).ok()
}

/// Resolve Extract role LLM for table/equation textual analysis.
pub async fn resolve_extract_provider_for_workspace(
    workspace_service: Option<&Arc<dyn WorkspaceService>>,
    workspace_id: Uuid,
    fallback: Arc<dyn LLMProvider>,
) -> Arc<dyn LLMProvider> {
    if let Some(svc) = workspace_service {
        if let Some(provider) = try_workspace_extract(svc, workspace_id).await {
            tracing::info!(
                workspace_id = %workspace_id,
                "Multimodal extract using workspace Extract role"
            );
            return provider;
        }
    }
    fallback
}

/// Resolve VLM for background workers (workspace priority, no full AppState).
pub async fn resolve_vlm_provider_for_workspace(
    workspace_service: Option<&Arc<dyn WorkspaceService>>,
    workspace_id: Uuid,
    startup_vision: Option<Arc<dyn LLMProvider>>,
    fallback: Arc<dyn LLMProvider>,
) -> Arc<dyn LLMProvider> {
    if let Some(svc) = workspace_service {
        if let Some(provider) = try_workspace_vlm(svc, workspace_id).await {
            tracing::info!(
                workspace_id = %workspace_id,
                "VLM using workspace-configured provider"
            );
            return provider;
        }
    }

    let env_provider = resolved_vision_provider_from_env();
    let env_model = default_vision_model_for_provider(&env_provider);
    if let Ok(provider) = create_safe_vision_provider(&env_provider, &env_model) {
        return provider;
    }

    if let Some(provider) = startup_vision {
        return provider;
    }

    fallback
}

/// Resolve VLM for multimodal Pass B with shorter local per-call timeout.
///
/// SPEC-134 WP-1: every returned provider is wrapped in the image size guard
/// so oversized analyze payloads are re-encoded instead of rejected upstream.
pub async fn resolve_vlm_provider_for_pass_b(
    workspace_service: Option<&Arc<dyn WorkspaceService>>,
    workspace_id: Uuid,
    startup_vision: Option<Arc<dyn LLMProvider>>,
    fallback: Arc<dyn LLMProvider>,
) -> Arc<dyn LLMProvider> {
    let resolved = resolve_vlm_provider_for_pass_b_inner(
        workspace_service,
        workspace_id,
        startup_vision,
        fallback,
    )
    .await;
    edgequake_pdf::image_guard::ImageGuardProvider::wrap(resolved)
}

async fn resolve_vlm_provider_for_pass_b_inner(
    workspace_service: Option<&Arc<dyn WorkspaceService>>,
    workspace_id: Uuid,
    startup_vision: Option<Arc<dyn LLMProvider>>,
    fallback: Arc<dyn LLMProvider>,
) -> Arc<dyn LLMProvider> {
    if let Some(svc) = workspace_service {
        if let Some(provider) = try_workspace_vlm_pass_b(svc, workspace_id).await {
            tracing::info!(
                workspace_id = %workspace_id,
                "Pass B VLM using workspace-configured provider (Pass B timeout)"
            );
            return provider;
        }
    }

    let env_provider = resolved_vision_provider_from_env();
    let env_model = default_vision_model_for_provider(&env_provider);
    if let Ok(provider) = create_safe_vision_provider_for_pass_b(&env_provider, &env_model) {
        return provider;
    }

    if let Some(provider) = startup_vision {
        return provider;
    }

    fallback
}

/// Resolve the vision-capable LLM for multimodal image describe-to-text.
pub async fn resolve_vlm_provider(
    state: &AppState,
    workspace_id: Option<Uuid>,
) -> Arc<dyn LLMProvider> {
    if let Some(ws_id) = workspace_id {
        let provider = crate::providers::connection_store::scope_saved_connection_pool(
            crate::providers::connection_store::ambient_pool_from_state(state),
            try_workspace_vlm(&state.workspace_service, ws_id),
        )
        .await;
        if let Some(provider) = provider {
            tracing::info!(workspace_id = %ws_id, "VLM image ingest using workspace-configured provider");
            return provider;
        }
    }

    let env_provider = resolved_vision_provider_from_env();
    let env_model = default_vision_model_for_provider(&env_provider);
    if let Ok(provider) = create_safe_vision_provider(&env_provider, &env_model) {
        tracing::debug!(
            provider = %env_provider,
            model = %env_model,
            "VLM image ingest using env vision defaults"
        );
        return provider;
    }

    if let Some(ref provider) = state.query.vision_llm_provider {
        tracing::debug!("VLM image ingest using startup vision_llm_provider");
        return Arc::clone(provider);
    }

    tracing::warn!("VLM image ingest falling back to server default llm_provider");
    Arc::clone(&state.query.llm_provider)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_workspace(metadata: HashMap<String, serde_json::Value>) -> Workspace {
        Workspace {
            workspace_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            name: "test".into(),
            slug: "test".into(),
            description: None,
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metadata,
            llm_model: "gemma3:latest".into(),
            llm_provider: "ollama".into(),
            embedding_model: "embeddinggemma:latest".into(),
            embedding_provider: "ollama".into(),
            embedding_dimension: 768,
            vision_llm_model: Some("gpt-4.1-mini".into()),
            vision_llm_provider: Some("openai".into()),
            pdf_parser_backend: None,
            rag_profile: Default::default(),
            embedding_modalities: Default::default(),
            fts_regconfig: Default::default(),
            archived_at: None,
            archived_by: None,
        }
    }

    #[test]
    fn workspace_vlm_prefers_vision_fields_over_main_llm() {
        // A saved vision choice always has metadata keys (EC-175-22); struct fields alone do not count.
        let mut meta = HashMap::new();
        meta.insert("vision_llm_provider".into(), serde_json::json!("openai"));
        meta.insert("vision_llm_model".into(), serde_json::json!("gpt-4.1-mini"));
        let ws = sample_workspace(meta);
        let cfg = resolve_workspace_vlm_config(&ws, None);
        assert_eq!(cfg.provider, "openai");
        assert_eq!(cfg.model, "gpt-4.1-mini");
    }

    #[test]
    fn workspace_vlm_role_wins_over_legacy_vision_fields() {
        let mut meta = HashMap::new();
        meta.insert("vision_llm_provider".into(), serde_json::json!("openai"));
        meta.insert("vision_llm_model".into(), serde_json::json!("gpt-4.1-mini"));
        meta.insert(
            "llm_roles".into(),
            serde_json::json!({"vlm": {"provider": "mistral", "model": "pixtral-large-latest"}}),
        );
        let cfg = resolve_workspace_vlm_config(&sample_workspace(meta), None);
        assert_eq!(cfg.provider, "mistral");
        assert_eq!(cfg.model, "pixtral-large-latest");
        assert!(cfg.connection_id.is_none());
    }

    #[test]
    fn workspace_vlm_falls_back_to_main_llm_when_vision_unset() {
        let mut meta = HashMap::new();
        // LAW-123-8: workspace LLM fallback requires deliberate override metadata.
        meta.insert("llm_provider".into(), serde_json::json!("ollama"));
        meta.insert("llm_model".into(), serde_json::json!("gemma3:latest"));
        let mut ws = sample_workspace(meta);
        ws.vision_llm_provider = None;
        ws.vision_llm_model = None;
        let cfg = resolve_workspace_vlm_config(&ws, None);
        assert_eq!(cfg.provider, "ollama");
        assert_eq!(cfg.model, "gemma3:latest");
    }

    #[test]
    fn workspace_vlm_skips_painted_llm_without_metadata() {
        let mut ws = sample_workspace(HashMap::new());
        ws.vision_llm_provider = None;
        ws.vision_llm_model = None;
        // Painted llm_* without metadata must not win as workspace (LAW-123-8).
        let cfg = resolve_workspace_vlm_config(&ws, None);
        assert_ne!(
            (cfg.provider.as_str(), cfg.model.as_str()),
            ("ollama", "gemma3:latest"),
            "inherit-painted llm fields must not be treated as workspace VLM"
        );
    }
}
