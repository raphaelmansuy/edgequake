//! VLM provider resolution for image ingest (SPEC-026 Phase 4).
//!
//! Priority (first principles — workspace SSOT before server env):
//!   1. Workspace `vision_llm_provider` / `vision_llm_model` (or `llm_roles.vlm`)
//!   2. Workspace main `llm_provider` / `llm_model`
//!   3. Server env vision defaults (`EDGEQUAKE_VISION_*`)
//!   4. Startup `vision_llm_provider` singleton
//!   5. Server default text LLM

use std::sync::Arc;

use edgequake_core::{resolve_role_llm, LlmRole, Workspace, WorkspaceService};
use edgequake_llm::traits::LLMProvider;
use uuid::Uuid;

use crate::safety_limits::{create_safe_llm_provider, create_safe_vision_provider};
use crate::state::AppState;
use crate::vision_env::{default_vision_model_for_provider, resolved_vision_provider_from_env};

/// Resolve vision provider/model from workspace (vision fields → main LLM fields).
pub fn resolve_workspace_vlm_config(ws: &Workspace) -> edgequake_core::ResolvedRoleLlm {
    resolve_role_llm(ws, LlmRole::Vlm)
}

async fn try_workspace_vlm(
    workspace_service: &Arc<dyn WorkspaceService>,
    workspace_id: Uuid,
) -> Option<Arc<dyn LLMProvider>> {
    let ws = workspace_service.get_workspace(workspace_id).await.ok()??;
    let role = resolve_workspace_vlm_config(&ws);
    create_safe_vision_provider(&role.provider, &role.model).ok()
}

async fn try_workspace_extract(
    workspace_service: &Arc<dyn WorkspaceService>,
    workspace_id: Uuid,
) -> Option<Arc<dyn LLMProvider>> {
    let ws = workspace_service.get_workspace(workspace_id).await.ok()??;
    let role = resolve_role_llm(&ws, LlmRole::Extract);
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

/// Resolve the vision-capable LLM for multimodal image describe-to-text.
pub async fn resolve_vlm_provider(
    state: &AppState,
    workspace_id: Option<Uuid>,
) -> Arc<dyn LLMProvider> {
    if let Some(ws_id) = workspace_id {
        if let Some(provider) = try_workspace_vlm(&state.workspace_service, ws_id).await {
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
        }
    }

    #[test]
    fn workspace_vlm_prefers_vision_fields_over_main_llm() {
        let ws = sample_workspace(HashMap::new());
        let cfg = resolve_workspace_vlm_config(&ws);
        assert_eq!(cfg.provider, "openai");
        assert_eq!(cfg.model, "gpt-4.1-mini");
    }

    #[test]
    fn workspace_vlm_falls_back_to_main_llm_when_vision_unset() {
        let mut ws = sample_workspace(HashMap::new());
        ws.vision_llm_provider = None;
        ws.vision_llm_model = None;
        let cfg = resolve_workspace_vlm_config(&ws);
        assert_eq!(cfg.provider, "ollama");
        assert_eq!(cfg.model, "gemma3:latest");
    }
}
