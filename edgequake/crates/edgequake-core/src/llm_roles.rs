//! LLM role resolution for workspace-scoped provider selection (SPEC-026 Phase 2 P-08 /
//! SPEC-046 EQ-046-13).
//!
//! Mirrors LightRAG `llm_roles.py` fallback: role config → workspace default.
//!
//! # Recommended default matrix (First Principles)
//!
//! | Role | Capability need | Typical model class |
//! |------|-----------------|---------------------|
//! | **Extract** | Structured NER/RE, schema adherence | Stronger / slower |
//! | **Query** | Long noisy context → faithful answer | Long-context strong |
//! | **Summary** | Merge entity/relation descriptions | Mid / cheap |
//! | **Vlm** | Image/table/equation understanding | Vision model |
//! | **Keyword** | Dual-level hl/ll keywords (optional) | Small / fast |
//!
//! Configure via workspace metadata:
//! ```json
//! { "llm_roles": {
//!     "extract": { "provider": "openai", "model": "gpt-5-nano" },
//!     "query":   { "provider": "openai", "model": "gpt-5-nano" },
//!     "summary": { "provider": "ollama", "model": "gemma3:latest" },
//!     "vlm":     { "provider": "ollama", "model": "llava" },
//!     "keyword": { "provider": "ollama", "model": "gemma3:latest" }
//! }}
//! ```

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
    /// Dual-level keyword extraction at query time (SPEC-046 / LightRAG keyword role).
    Keyword,
}

impl LlmRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Extract => "extract",
            Self::Query => "query",
            Self::Summary => "summary",
            Self::Vlm => "vlm",
            Self::Keyword => "keyword",
        }
    }

    /// All roles for docs / iteration.
    pub fn all() -> &'static [LlmRole] {
        &[
            Self::Extract,
            Self::Query,
            Self::Summary,
            Self::Vlm,
            Self::Keyword,
        ]
    }
}

/// Per-role provider override stored in workspace metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RoleLlmConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    /// SPEC-109: optional reasoning effort for this role (`none`/`minimal`/`low`/…).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    /// SPEC-163: optional `provider_connections.id` for this role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
}

/// Resolved provider + model for a role (always populated after fallback).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRoleLlm {
    pub provider: String,
    pub model: String,
    /// SPEC-163: connection used for URL/key when set.
    pub connection_id: Option<String>,
}

/// SPEC-109: resolved reasoning effort with explainability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedReasoningEffort {
    /// Pre-clamp desired string (may be unset for Auto / floor).
    pub desired: Option<String>,
    /// Value to put on `CompletionOptions.reasoning_effort` (post-clamp).
    pub effective: Option<String>,
    /// Which hierarchy layer supplied `desired` (or `compiled_default`).
    pub source: String,
    /// True when effective differs from desired due to capability clamp.
    pub clamped: bool,
}

/// Roles that default to lowest supported effort (structured / high-volume).
pub fn role_uses_structured_effort_floor(role: LlmRole) -> bool {
    matches!(
        role,
        LlmRole::Extract | LlmRole::Summary | LlmRole::Keyword | LlmRole::Vlm
    )
}

/// Workspace-level fallback effort from metadata key `default_reasoning_effort`.
pub fn workspace_default_reasoning_effort(ws: &Workspace) -> Option<String> {
    ws.metadata
        .get("default_reasoning_effort")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn env_reasoning_effort_for_role(role: LlmRole) -> Option<String> {
    let role_key = match role {
        LlmRole::Extract => "EDGEQUAKE_EXTRACT_REASONING_EFFORT",
        LlmRole::Query => "EDGEQUAKE_QUERY_REASONING_EFFORT",
        LlmRole::Summary => "EDGEQUAKE_SUMMARY_REASONING_EFFORT",
        LlmRole::Vlm => "EDGEQUAKE_VLM_REASONING_EFFORT",
        LlmRole::Keyword => "EDGEQUAKE_KEYWORD_REASONING_EFFORT",
    };
    non_empty_env(role_key).or_else(|| non_empty_env("EDGEQUAKE_REASONING_EFFORT"))
}

/// Resolve + clamp reasoning effort (SPEC-109 LAW-R6).
///
/// Hierarchy: request → llm_roles.{role} → workspace default → tenant →
/// server role map → server default → env → compiled (structured floor / query omit).
#[allow(clippy::too_many_arguments)] // SPEC-109 cascade needs role + provider + layered overrides
pub fn resolve_role_reasoning_effort(
    role: LlmRole,
    provider: &str,
    model: &str,
    ws: &Workspace,
    request_override: Option<&str>,
    tenant_default: Option<&str>,
    server_effort: Option<&str>,
    server_by_role: Option<&str>,
) -> ResolvedReasoningEffort {
    let mut source = "compiled_default".to_string();
    let mut desired: Option<String> = None;

    if let Some(v) = request_override.map(str::trim).filter(|s| !s.is_empty()) {
        desired = Some(v.to_string());
        source = "request".into();
    }
    if desired.is_none() {
        if let Some(cfg) = role_config_from_workspace(ws, role) {
            if let Some(v) = cfg
                .reasoning_effort
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                desired = Some(v.to_string());
                source = "workspace.llm_roles".into();
            }
        }
    }
    if desired.is_none() {
        if let Some(v) = workspace_default_reasoning_effort(ws) {
            desired = Some(v);
            source = "workspace.default_reasoning_effort".into();
        }
    }
    if desired.is_none() {
        if let Some(v) = tenant_default.map(str::trim).filter(|s| !s.is_empty()) {
            desired = Some(v.to_string());
            source = "tenant".into();
        }
    }
    if desired.is_none() {
        if let Some(v) = server_by_role.map(str::trim).filter(|s| !s.is_empty()) {
            desired = Some(v.to_string());
            source = "server.reasoning_by_role".into();
        }
    }
    if desired.is_none() {
        if let Some(v) = server_effort.map(str::trim).filter(|s| !s.is_empty()) {
            desired = Some(v.to_string());
            source = "server.reasoning_effort".into();
        }
    }
    if desired.is_none() {
        if let Some(v) = env_reasoning_effort_for_role(role) {
            desired = Some(v);
            source = "env".into();
        }
    }

    let effective = if let Some(ref d) = desired {
        let clamped = edgequake_llm::clamp_reasoning_effort(provider, model, Some(d.as_str()));
        // Ollama has no static effort ladder — pass through so extract can send think:false.
        clamped.or_else(|| {
            let p = provider.to_ascii_lowercase();
            if p.contains("ollama") || p.contains("lmstudio") || p.contains("lm-studio") {
                Some(d.trim().to_ascii_lowercase())
            } else {
                None
            }
        })
    } else if role_uses_structured_effort_floor(role) {
        edgequake_llm::lowest_for_structured_output(provider, model).or_else(|| {
            let p = provider.to_ascii_lowercase();
            if p.contains("ollama") || p.contains("lmstudio") || p.contains("lm-studio") {
                // Structured extract must not use Ollama Auto (think:true).
                Some("none".to_string())
            } else {
                None
            }
        })
    } else {
        None // query/chat Auto
    };

    let clamped = match (&desired, &effective) {
        (Some(d), Some(e)) => !d.eq_ignore_ascii_case(e),
        (Some(_), None) => true,
        (None, Some(_)) if role_uses_structured_effort_floor(role) => false,
        _ => false,
    };

    ResolvedReasoningEffort {
        desired,
        effective,
        source,
        clamped,
    }
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
        return ResolvedRoleLlm {
            provider,
            model,
            connection_id: cfg.connection_id.filter(|s| !s.is_empty()),
        };
    }
    ResolvedRoleLlm {
        provider: default_provider_for_role(ws, role),
        model: default_model_for_role(ws, role),
        connection_id: None,
    }
}

fn default_provider_for_role(ws: &Workspace, role: LlmRole) -> String {
    match role {
        LlmRole::Vlm => ws
            .vision_llm_provider
            .clone()
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| ws.llm_provider.clone()),
        // Keyword/Summary default to workspace LLM provider (cheap local OK)
        LlmRole::Keyword | LlmRole::Summary | LlmRole::Extract | LlmRole::Query => {
            ws.llm_provider.clone()
        }
    }
}

fn default_model_for_role(ws: &Workspace, role: LlmRole) -> String {
    match role {
        LlmRole::Vlm => ws
            .vision_llm_model
            .clone()
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| ws.llm_model.clone()),
        LlmRole::Keyword | LlmRole::Summary | LlmRole::Extract | LlmRole::Query => {
            ws.llm_model.clone()
        }
    }
}

/// Human-readable capability hint for ops / docs (SPEC-046 EQ-046-13).
pub fn role_capability_hint(role: LlmRole) -> &'static str {
    match role {
        LlmRole::Extract => "strong structured extraction (NER/RE)",
        LlmRole::Query => "long-context faithful generation",
        LlmRole::Summary => "cheap merge/summarization of entity descriptions",
        LlmRole::Vlm => "vision / table / equation understanding",
        LlmRole::Keyword => "fast dual-level keyword extraction",
    }
}

/// Process-env KEYWORD role (LightRAG `KEYWORD_LLM_*` law).
///
/// Env (either MODEL or PROVIDER non-empty enables the override):
/// - `EDGEQUAKE_KEYWORD_LLM_MODEL` — required for a usable pin (empty = unset)
/// - `EDGEQUAKE_KEYWORD_LLM_PROVIDER` — optional; falls back to `EDGEQUAKE_LLM_PROVIDER`
///   then `"mistral"` when only the model is set
///
/// Precedence at query time: **env → workspace `llm_roles.keyword` → Query LLM**.
pub fn env_keyword_role_llm() -> Option<ResolvedRoleLlm> {
    env_role_llm(
        "EDGEQUAKE_KEYWORD_LLM_PROVIDER",
        "EDGEQUAKE_KEYWORD_LLM_MODEL",
        /* mistral_default */ "ministral-3b-latest",
    )
}

/// Process-env EXTRACT role (LightRAG EXTRACT≠QUERY / SPEC-086 Idea F).
///
/// Env (either MODEL or PROVIDER non-empty enables the override):
/// - `EDGEQUAKE_EXTRACT_LLM_MODEL` — schema-strong extract model (empty = unset)
/// - `EDGEQUAKE_EXTRACT_LLM_PROVIDER` — optional; falls back to `EDGEQUAKE_LLM_PROVIDER`
///   then `"mistral"` when only the model is set
///
/// Precedence at ingest: **env → workspace `llm_roles.extract` → workspace LLM**.
/// Acc QUERY pin stays on workspace `llm_model` (unchanged).
pub fn env_extract_role_llm() -> Option<ResolvedRoleLlm> {
    env_role_llm(
        "EDGEQUAKE_EXTRACT_LLM_PROVIDER",
        "EDGEQUAKE_EXTRACT_LLM_MODEL",
        /* mistral_default */ "mistral-medium-latest",
    )
}

/// Resolve Extract role with env-first precedence (SPEC-086 Phase B).
pub fn resolve_extract_role_llm(ws: &Workspace) -> ResolvedRoleLlm {
    env_extract_role_llm().unwrap_or_else(|| resolve_role_llm(ws, LlmRole::Extract))
}

fn env_role_llm(
    provider_key: &str,
    model_key: &str,
    mistral_default_model: &str,
) -> Option<ResolvedRoleLlm> {
    let model = non_empty_env(model_key);
    let provider = non_empty_env(provider_key);
    match (provider, model) {
        (None, None) => None,
        (Some(provider), Some(model)) => Some(ResolvedRoleLlm {
            provider,
            model,
            connection_id: None,
        }),
        (None, Some(model)) => {
            let provider =
                non_empty_env("EDGEQUAKE_LLM_PROVIDER").unwrap_or_else(|| "mistral".to_string());
            Some(ResolvedRoleLlm {
                provider,
                model,
                connection_id: None,
            })
        }
        (Some(provider), None) => {
            let model = match provider.to_ascii_lowercase().as_str() {
                "mistral" => mistral_default_model.to_string(),
                "openai" => "gpt-5.4-nano".to_string(),
                "ollama" => "gemma3:latest".to_string(),
                other => other.to_string(),
            };
            Some(ResolvedRoleLlm {
                provider,
                model,
                connection_id: None,
            })
        }
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
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
    use serial_test::serial;
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
    fn connection_id_is_preserved_on_role() {
        let mut meta = HashMap::new();
        meta.insert(
            "llm_roles".into(),
            serde_json::json!({
                "query": {
                    "provider": "openai_chat",
                    "model": "fake-chat",
                    "connection_id": "11111111-1111-1111-1111-111111111111"
                }
            }),
        );
        let ws = sample_workspace(meta);
        let resolved = resolve_role_llm(&ws, LlmRole::Query);
        assert_eq!(
            resolved.connection_id.as_deref(),
            Some("11111111-1111-1111-1111-111111111111")
        );
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

    #[test]
    fn keyword_and_summary_roles_resolve_and_have_hints() {
        let mut meta = HashMap::new();
        meta.insert(
            "llm_roles".into(),
            serde_json::json!({
                "keyword": { "provider": "ollama", "model": "gemma3:latest" },
                "summary": { "provider": "mock", "model": "mock-summary" }
            }),
        );
        let ws = sample_workspace(meta);
        let kw = resolve_role_llm(&ws, LlmRole::Keyword);
        assert_eq!(kw.model, "gemma3:latest");
        let sum = resolve_role_llm(&ws, LlmRole::Summary);
        assert_eq!(sum.provider, "mock");
        assert!(!role_capability_hint(LlmRole::Keyword).is_empty());
        assert_eq!(LlmRole::all().len(), 5);
    }

    #[test]
    #[serial]
    fn env_keyword_role_unset_when_empty() {
        // Isolate from ambient Acc/dev env (serialize — env is process-global).
        std::env::remove_var("EDGEQUAKE_KEYWORD_LLM_PROVIDER");
        std::env::remove_var("EDGEQUAKE_KEYWORD_LLM_MODEL");
        assert!(env_keyword_role_llm().is_none());
        std::env::set_var("EDGEQUAKE_KEYWORD_LLM_MODEL", "ministral-3b-latest");
        let role = env_keyword_role_llm().expect("model-only pin");
        assert_eq!(role.model, "ministral-3b-latest");
        assert!(!role.provider.is_empty());
        std::env::remove_var("EDGEQUAKE_KEYWORD_LLM_MODEL");
        std::env::set_var("EDGEQUAKE_KEYWORD_LLM_PROVIDER", "mistral");
        std::env::set_var("EDGEQUAKE_KEYWORD_LLM_MODEL", "ministral-3b-latest");
        let role = env_keyword_role_llm().expect("full pin");
        assert_eq!(role.provider, "mistral");
        assert_eq!(role.model, "ministral-3b-latest");
        std::env::remove_var("EDGEQUAKE_KEYWORD_LLM_PROVIDER");
        std::env::remove_var("EDGEQUAKE_KEYWORD_LLM_MODEL");
    }

    #[test]
    #[serial]
    fn env_extract_role_and_resolve_precedence() {
        std::env::remove_var("EDGEQUAKE_EXTRACT_LLM_PROVIDER");
        std::env::remove_var("EDGEQUAKE_EXTRACT_LLM_MODEL");
        assert!(env_extract_role_llm().is_none());

        let mut meta = HashMap::new();
        meta.insert(
            "llm_roles".into(),
            serde_json::json!({
                "extract": { "provider": "mistral", "model": "ministral-8b-latest" }
            }),
        );
        let ws = sample_workspace(meta);
        let from_ws = resolve_extract_role_llm(&ws);
        assert_eq!(from_ws.model, "ministral-8b-latest");

        std::env::set_var("EDGEQUAKE_EXTRACT_LLM_MODEL", "mistral-medium-latest");
        let from_env = resolve_extract_role_llm(&ws);
        assert_eq!(from_env.model, "mistral-medium-latest");
        assert_eq!(from_env.provider, "mistral");
        std::env::remove_var("EDGEQUAKE_EXTRACT_LLM_MODEL");
        std::env::remove_var("EDGEQUAKE_EXTRACT_LLM_PROVIDER");
    }

    #[test]
    #[serial]
    fn resolve_extract_effort_uses_structured_floor_for_gpt5_mini() {
        for key in [
            "EDGEQUAKE_EXTRACT_REASONING_EFFORT",
            "EDGEQUAKE_REASONING_EFFORT",
        ] {
            std::env::remove_var(key);
        }
        let ws = sample_workspace(HashMap::new());
        let resolved = resolve_role_reasoning_effort(
            LlmRole::Extract,
            "openai",
            "gpt-5-mini",
            &ws,
            None,
            None,
            None,
            None,
        );
        assert_eq!(resolved.effective.as_deref(), Some("minimal"));
        assert_eq!(resolved.source, "compiled_default");
    }

    #[test]
    #[serial]
    fn resolve_extract_effort_ollama_qwen_floors_to_none() {
        for key in [
            "EDGEQUAKE_EXTRACT_REASONING_EFFORT",
            "EDGEQUAKE_REASONING_EFFORT",
        ] {
            std::env::remove_var(key);
        }
        let ws = sample_workspace(HashMap::new());
        let resolved = resolve_role_reasoning_effort(
            LlmRole::Extract,
            "ollama",
            "qwen3.6:35b-a3b",
            &ws,
            None,
            None,
            None,
            None,
        );
        assert_eq!(resolved.effective.as_deref(), Some("none"));
        assert_eq!(resolved.source, "compiled_default");
    }

    #[test]
    fn resolve_query_effort_omits_by_default() {
        let ws = sample_workspace(HashMap::new());
        let resolved = resolve_role_reasoning_effort(
            LlmRole::Query,
            "openai",
            "gpt-5-mini",
            &ws,
            None,
            None,
            None,
            None,
        );
        assert!(resolved.effective.is_none());
        assert!(resolved.desired.is_none());
    }

    #[test]
    fn resolve_request_override_wins_and_clamps() {
        let mut meta = HashMap::new();
        meta.insert(
            "llm_roles".into(),
            serde_json::json!({
                "query": { "provider": "openai", "model": "gpt-5-mini", "reasoning_effort": "medium" }
            }),
        );
        let ws = sample_workspace(meta);
        let resolved = resolve_role_reasoning_effort(
            LlmRole::Query,
            "openai",
            "gpt-5-mini",
            &ws,
            Some("none"),
            None,
            None,
            None,
        );
        assert_eq!(resolved.desired.as_deref(), Some("none"));
        assert_eq!(resolved.effective.as_deref(), Some("minimal"));
        assert!(resolved.clamped);
        assert_eq!(resolved.source, "request");
    }
}
