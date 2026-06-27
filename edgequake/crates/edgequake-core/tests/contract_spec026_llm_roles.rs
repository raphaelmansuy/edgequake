//! SPEC-026 Phase 2 — LLM role resolution contract tests.

use edgequake_core::{resolve_role_llm, LlmRole, Workspace};
use std::collections::HashMap;
use uuid::Uuid;

fn ws(metadata: HashMap<String, serde_json::Value>) -> Workspace {
    Workspace {
        workspace_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        name: "t".into(),
        slug: "t".into(),
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
        serde_json::json!({"extract": {"provider": "mock", "model": "mock-extract"}}),
    );
    let resolved = resolve_role_llm(&ws(meta), LlmRole::Extract);
    assert_eq!(resolved.provider, "mock");
    assert_eq!(resolved.model, "mock-extract");
}

#[test]
fn resolve_query_role_falls_back_to_workspace_default() {
    let resolved = resolve_role_llm(&ws(HashMap::new()), LlmRole::Query);
    assert_eq!(resolved.provider, "ollama");
    assert_eq!(resolved.model, "gemma3:latest");
}

#[test]
fn resolve_summary_role_for_merge() {
    let mut meta = HashMap::new();
    meta.insert(
        "llm_roles".into(),
        serde_json::json!({"summary": {"provider": "openai", "model": "gpt-5-nano"}}),
    );
    let resolved = resolve_role_llm(&ws(meta), LlmRole::Summary);
    assert_eq!(resolved.provider, "openai");
}

#[test]
fn role_priority_matches_lightrag_semantics() {
    let mut meta = HashMap::new();
    meta.insert(
        "llm_roles".into(),
        serde_json::json!({"extract": {"provider": "", "model": "only-model"}}),
    );
    let resolved = resolve_role_llm(&ws(meta), LlmRole::Extract);
    assert_eq!(resolved.provider, "ollama");
    assert_eq!(resolved.model, "only-model");
}
