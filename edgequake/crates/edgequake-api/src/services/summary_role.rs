//! Shared Summary-role LLM resolution for merge summarization (SPEC-046 EQ-046-13).
//!
//! Prefer workspace `llm_roles.summary` when configured; otherwise fall back to
//! the caller's Extract/default provider. Single place for text_insert + injection.

use std::sync::Arc;

use edgequake_core::{resolve_role_llm, role_config_from_workspace, LlmRole, Workspace};
use edgequake_llm::traits::LLMProvider;

/// Resolve merge-summarizer LLM: Summary role override, else `fallback`.
pub fn resolve_summary_llm_or_fallback(
    workspace: Option<&Workspace>,
    fallback: Arc<dyn LLMProvider>,
    create: impl FnOnce(&str, &str) -> Result<Arc<dyn LLMProvider>, String>,
) -> Result<Arc<dyn LLMProvider>, String> {
    let Some(ws) = workspace else {
        return Ok(fallback);
    };
    if role_config_from_workspace(ws, LlmRole::Summary).is_none() {
        return Ok(fallback);
    }
    let role = resolve_role_llm(ws, LlmRole::Summary);
    create(&role.provider, &role.model)
        .map_err(|err| format!("credential_missing: {err}. Fix this under Workspace → Models."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_core::Workspace;
    use edgequake_llm::MockProvider;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn ws_with_summary() -> Workspace {
        let mut metadata = HashMap::new();
        metadata.insert(
            "llm_roles".into(),
            serde_json::json!({
                "summary": { "provider": "mock", "model": "mock-summary" }
            }),
        );
        Workspace {
            workspace_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            name: "s".into(),
            slug: "s".into(),
            description: None,
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metadata,
            llm_model: "base".into(),
            llm_provider: "mock".into(),
            embedding_model: "mock-embedding".into(),
            embedding_provider: "mock".into(),
            embedding_dimension: 8,
            vision_llm_model: None,
            vision_llm_provider: None,
            pdf_parser_backend: None,
            rag_profile: Default::default(),
            embedding_modalities: Default::default(),
            fts_regconfig: Default::default(),
        }
    }

    #[test]
    fn prefers_summary_role_when_configured() {
        let fallback = Arc::new(MockProvider::new()) as Arc<dyn LLMProvider>;
        let ws = ws_with_summary();
        let mut used_summary = false;
        let _ = resolve_summary_llm_or_fallback(Some(&ws), fallback, |p, m| {
            used_summary = p == "mock" && m == "mock-summary";
            Ok(Arc::new(MockProvider::new()) as Arc<dyn LLMProvider>)
        })
        .expect("summary role");
        assert!(used_summary);
    }

    #[test]
    fn falls_back_when_summary_unset() {
        let fallback = Arc::new(MockProvider::new()) as Arc<dyn LLMProvider>;
        let mut ws = ws_with_summary();
        ws.metadata.clear();
        let mut create_called = false;
        let out = resolve_summary_llm_or_fallback(Some(&ws), fallback.clone(), |_, _| {
            create_called = true;
            Ok(Arc::new(MockProvider::new()) as Arc<dyn LLMProvider>)
        })
        .expect("unset summary inherits");
        assert!(!create_called);
        assert!(Arc::ptr_eq(&out, &fallback));
    }
}
