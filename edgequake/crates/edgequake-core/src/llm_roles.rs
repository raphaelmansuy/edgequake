//! LLM role resolution for workspace-scoped provider selection (SPEC-026 Phase 2 P-08).
//!
//! Mirrors LightRAG `llm_roles.py` fallback: role config → workspace default.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::Workspace;

/// LLM functional roles used during ingest and query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmRole {
    Extract,
    Query,
    Summary,
    /// Vision / multimodal analysis (LightRAG `vlm` role).
    Vlm,
}

impl LlmRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Extract => "extract",
            Self::Query => "query",
            Self::Summary => "summary",
            Self::Vlm => "vlm",
        }
    }
}

/// Per-role provider override stored in workspace metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoleLlmConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
}

/// Resolved provider + model for a role (always populated after fallback).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRoleLlm {
    pub provider: String,
    pub model: String,
}

/// Read role overrides from workspace metadata key `llm_roles`.
pub fn role_config_from_workspace(ws: &Workspace, role: LlmRole) -> Option<RoleLlmConfig> {
    let roles = ws.metadata.get("llm_roles")?;
    let role_obj = roles.get(role.as_str())?;
    serde_json::from_value(role_obj.clone()).ok()
}

/// Resolve provider/model with LightRAG-style fallback chain.
pub fn resolve_role_llm(ws: &Workspace, role: LlmRole) -> ResolvedRoleLlm {
    if let Some(cfg) = role_config_from_workspace(ws, role) {
        let provider = cfg
            .provider
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| default_provider_for_role(ws, role));
        let model = cfg
            .model
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| default_model_for_role(ws, role));
        return ResolvedRoleLlm { provider, model };
    }
    ResolvedRoleLlm {
        provider: default_provider_for_role(ws, role),
        model: default_model_for_role(ws, role),
    }
}

fn default_provider_for_role(ws: &Workspace, role: LlmRole) -> String {
    match role {
        LlmRole::Vlm => ws
            .vision_llm_provider
            .clone()
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| ws.llm_provider.clone()),
        _ => ws.llm_provider.clone(),
    }
}

fn default_model_for_role(ws: &Workspace, role: LlmRole) -> String {
    match role {
        LlmRole::Vlm => ws
            .vision_llm_model
            .clone()
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| ws.llm_model.clone()),
        _ => ws.llm_model.clone(),
    }
}

/// Parse full llm_roles map from metadata (for tests / API).
pub fn parse_llm_roles_map(
    metadata: &HashMap<String, serde_json::Value>,
) -> HashMap<String, RoleLlmConfig> {
    let Some(raw) = metadata.get("llm_roles") else {
        return HashMap::new();
    };
    raw.as_object()
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| {
                    serde_json::from_value(v.clone())
                        .ok()
                        .map(|cfg| (k.clone(), cfg))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

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
            vision_llm_model: None,
            vision_llm_provider: None,
            pdf_parser_backend: None,
        }
    }

    #[test]
    fn resolve_extract_role_uses_configured_provider() {
        let mut meta = HashMap::new();
        meta.insert(
            "llm_roles".into(),
            serde_json::json!({
                "extract": { "provider": "mock", "model": "mock-extract" }
            }),
        );
        let ws = sample_workspace(meta);
        let resolved = resolve_role_llm(&ws, LlmRole::Extract);
        assert_eq!(resolved.provider, "mock");
        assert_eq!(resolved.model, "mock-extract");
    }

    #[test]
    fn resolve_query_role_falls_back_to_workspace_default() {
        let ws = sample_workspace(HashMap::new());
        let resolved = resolve_role_llm(&ws, LlmRole::Query);
        assert_eq!(resolved.provider, "ollama");
        assert_eq!(resolved.model, "gemma3:latest");
    }

    #[test]
    fn resolve_vlm_falls_back_to_vision_fields() {
        let mut ws = sample_workspace(HashMap::new());
        ws.vision_llm_provider = Some("openai".into());
        ws.vision_llm_model = Some("gpt-4.1-mini".into());
        let resolved = resolve_role_llm(&ws, LlmRole::Vlm);
        assert_eq!(resolved.provider, "openai");
        assert_eq!(resolved.model, "gpt-4.1-mini");
    }

    #[test]
    fn resolve_vlm_role_prefers_llm_roles_vlm() {
        let mut meta = HashMap::new();
        meta.insert(
            "llm_roles".into(),
            serde_json::json!({
                "vlm": { "provider": "mock", "model": "mock-vlm" }
            }),
        );
        let ws = sample_workspace(meta);
        let resolved = resolve_role_llm(&ws, LlmRole::Vlm);
        assert_eq!(resolved.provider, "mock");
        assert_eq!(resolved.model, "mock-vlm");
    }
}
