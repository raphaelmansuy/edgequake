//! Shared constants and utility functions for Ollama emulation handlers.

use chrono::Utc;

/// Default model name for Ollama emulation.
pub(super) const OLLAMA_MODEL_NAME: &str = "edgequake";
/// Default model tag for Ollama emulation.
pub(super) const OLLAMA_MODEL_TAG: &str = "latest";
/// Default model size (placeholder).
pub(super) const OLLAMA_MODEL_SIZE: u64 = 7_000_000_000; // 7GB placeholder
/// Default model digest.
pub(super) const OLLAMA_MODEL_DIGEST: &str = "sha256:edgequake-rag-v1";
/// API version string.
pub(super) const OLLAMA_API_VERSION: &str = "0.9.3";

/// Estimate token count for a string (rough approximation: 1 token ≈ 4 chars).
pub(super) fn estimate_tokens(text: &str) -> u32 {
    (text.len() / 4) as u32
}

/// Get the current timestamp in ISO 8601 format.
pub(super) fn current_timestamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()
}

/// Get the model name for responses.
pub(super) fn model_name() -> String {
    format!("{}:{}", OLLAMA_MODEL_NAME, OLLAMA_MODEL_TAG)
}

/// Every compatibility query uses the same authenticated scope and request construction.
pub(super) fn scoped_request(
    query: &str,
    mode: crate::handlers::ollama_types::OllamaSearchMode,
    context_only: bool,
    system: Option<&str>,
    context: &crate::middleware::TenantContext,
    history: Vec<edgequake_query::ConversationMessage>,
) -> edgequake_query::QueryRequest {
    use crate::middleware::{
        default_tenant_uuid, default_workspace_uuid, resolve_tenant_uuid, resolve_workspace_uuid,
    };
    let mut request = edgequake_query::QueryRequest::new(query)
        .with_mode(
            mode.to_query_mode()
                .unwrap_or(edgequake_query::QueryMode::Hybrid),
        )
        .with_tenant_id(
            resolve_tenant_uuid(context.tenant_id.as_deref())
                .unwrap_or_else(default_tenant_uuid)
                .to_string(),
        )
        .with_workspace_id(
            resolve_workspace_uuid(context.workspace_id.as_deref())
                .unwrap_or_else(default_workspace_uuid)
                .to_string(),
        )
        .with_conversation_history(history);
    if context_only {
        request = request.context_only();
    }
    if let Some(system) = system {
        request = request.with_system_prompt(system);
    }
    request
}

pub(super) fn response_text(
    response: edgequake_query::QueryResponse,
    context_only: bool,
) -> String {
    if context_only {
        response.context.to_context_string()
    } else {
        response.answer
    }
}
