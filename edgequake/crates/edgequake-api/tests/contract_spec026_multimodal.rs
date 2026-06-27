//! SPEC-026 Phase 4 — multimodal ingest contract tests.

use edgequake_api::services::{
    image_analysis_to_markdown, parse_image_analysis_json, ImageAnalysisResult,
    MultimodalProcessOptions, IMAGE_TYPE_FALLBACK,
};
use edgequake_core::{resolve_role_llm, LlmRole};
use std::collections::HashMap;
use uuid::Uuid;

fn sample_workspace(metadata: HashMap<String, serde_json::Value>) -> edgequake_core::Workspace {
    edgequake_core::Workspace {
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
fn image_analysis_json_parses_lightrag_schema() {
    let raw = r#"{"name":"sarah_chen_profile","type":"Photo","description":"Dr. Sarah Chen leads EdgeQuake."}"#;
    let parsed = parse_image_analysis_json(raw).unwrap();
    assert_eq!(parsed.name, "sarah_chen_profile");
    assert_eq!(parsed.image_type, "Photo");
}

#[test]
fn image_analysis_invalid_type_falls_back_to_other() {
    let raw = r#"{"name":"x","type":"Alien","description":"y"}"#;
    let parsed = parse_image_analysis_json(raw).unwrap();
    assert_eq!(parsed.image_type, IMAGE_TYPE_FALLBACK);
}

#[test]
fn vision_markdown_includes_name_heading() {
    let analysis = ImageAnalysisResult {
        name: "sarah_chen".into(),
        image_type: "Photo".into(),
        description: "Research lead.".into(),
    };
    let md = image_analysis_to_markdown(&analysis);
    assert!(md.contains("# sarah chen"));
}

#[test]
fn multimodal_process_options_parse_ite() {
    let opts = MultimodalProcessOptions::from_option_str("ite");
    assert!(opts.images && opts.tables && opts.equations);
}

#[test]
fn resolve_vlm_role_prefers_llm_roles_vlm() {
    let mut meta = HashMap::new();
    meta.insert(
        "llm_roles".into(),
        serde_json::json!({ "vlm": { "provider": "mock", "model": "mock-vlm" } }),
    );
    let ws = sample_workspace(meta);
    let resolved = resolve_role_llm(&ws, LlmRole::Vlm);
    assert_eq!(resolved.provider, "mock");
    assert_eq!(resolved.model, "mock-vlm");
}

#[test]
fn resolve_vlm_falls_back_to_vision_fields() {
    let ws = sample_workspace(HashMap::new());
    let resolved = resolve_role_llm(&ws, LlmRole::Vlm);
    assert_eq!(resolved.provider, "openai");
    assert_eq!(resolved.model, "gpt-4.1-mini");
}

#[test]
fn resolve_vlm_falls_back_to_workspace_main_llm() {
    let mut ws = sample_workspace(HashMap::new());
    ws.vision_llm_provider = None;
    ws.vision_llm_model = None;
    let resolved = resolve_role_llm(&ws, LlmRole::Vlm);
    assert_eq!(resolved.provider, "ollama");
    assert_eq!(resolved.model, "gemma3:latest");
}
