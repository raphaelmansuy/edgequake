//! SPEC-026 Phase 4 — VLM LLM role resolution contract tests.

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
        rag_profile: Default::default(),
        embedding_modalities: Default::default(),
        fts_regconfig: Default::default(),
        archived_at: None,
        archived_by: None,
    }
}

#[test]
fn resolve_vlm_falls_back_to_vision_fields() {
    let mut workspace = ws(HashMap::new());
    workspace.vision_llm_provider = Some("openai".into());
    workspace.vision_llm_model = Some("gpt-4.1-mini".into());
    let resolved = resolve_role_llm(&workspace, LlmRole::Vlm);
    assert_eq!(resolved.provider, "openai");
    assert_eq!(resolved.model, "gpt-4.1-mini");
}

/// SPEC-047 / 025: stronger vision ablation pins Medium for VLM while query LLM stays Small.
#[test]
fn resolve_vlm_prefers_mistral_medium_3_5_vision_pin() {
    let mut workspace = ws(HashMap::new());
    workspace.llm_provider = "mistral".into();
    workspace.llm_model = "mistral-small-latest".into();
    workspace.vision_llm_provider = Some("mistral".into());
    workspace.vision_llm_model = Some("mistral-medium-3-5".into());
    let vlm = resolve_role_llm(&workspace, LlmRole::Vlm);
    assert_eq!(vlm.provider, "mistral");
    assert_eq!(vlm.model, "mistral-medium-3-5");
    let query_llm = resolve_role_llm(&workspace, LlmRole::Query);
    assert_eq!(query_llm.model, "mistral-small-latest");
}

#[test]
fn resolve_vlm_falls_back_to_default_llm() {
    let resolved = resolve_role_llm(&ws(HashMap::new()), LlmRole::Vlm);
    assert_eq!(resolved.provider, "ollama");
    assert_eq!(resolved.model, "gemma3:latest");
}

#[test]
fn resolve_vlm_role_prefers_llm_roles_vlm() {
    let mut meta = HashMap::new();
    meta.insert(
        "llm_roles".into(),
        serde_json::json!({"vlm": {"provider": "mock", "model": "mock-vlm"}}),
    );
    let resolved = resolve_role_llm(&ws(meta), LlmRole::Vlm);
    assert_eq!(resolved.provider, "mock");
    assert_eq!(resolved.model, "mock-vlm");
}
