//! Shared workspace model-config update logic (SPEC-013 / server-default reset).
//!
//! Clearing LLM/embedding overrides uses empty string (or `"none"`) — same contract as
//! vision LLM fields in [`UpdateWorkspaceRequest`].

use crate::types::Workspace;

/// True when the client intends to clear a workspace override (use server/env defaults).
pub fn is_clear_override_value(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized.is_empty() || normalized == "none"
}

/// Apply LLM model/provider update; empty values reset to [`Workspace::server_runtime_llm_config`].
pub fn apply_llm_config_update(
    workspace: &mut Workspace,
    llm_model: Option<String>,
    llm_provider: Option<String>,
) {
    let clear = llm_model
        .as_ref()
        .is_some_and(|v| is_clear_override_value(v))
        || llm_provider
            .as_ref()
            .is_some_and(|v| is_clear_override_value(v));

    if clear {
        let (model, provider) = Workspace::server_runtime_llm_config();
        workspace.llm_model = model;
        workspace.llm_provider = provider;
        workspace.metadata.remove("llm_model");
        workspace.metadata.remove("llm_provider");
        return;
    }

    if let Some(llm_model) = llm_model {
        workspace.llm_model = llm_model.clone();
        workspace
            .metadata
            .insert("llm_model".to_string(), serde_json::json!(llm_model));
    }
    if let Some(llm_provider) = llm_provider {
        workspace.llm_provider = llm_provider.clone();
        workspace
            .metadata
            .insert("llm_provider".to_string(), serde_json::json!(llm_provider));
    }
}

/// Apply embedding update; empty values reset to [`Workspace::server_runtime_embedding_config`].
pub fn apply_embedding_config_update(
    workspace: &mut Workspace,
    embedding_model: Option<String>,
    embedding_provider: Option<String>,
    embedding_dimension: Option<usize>,
) {
    let clear = embedding_model
        .as_ref()
        .is_some_and(|v| is_clear_override_value(v))
        || embedding_provider
            .as_ref()
            .is_some_and(|v| is_clear_override_value(v))
        || embedding_dimension == Some(0);

    if clear {
        let (model, provider, dimension) = Workspace::server_runtime_embedding_config();
        workspace.embedding_model = model;
        workspace.embedding_provider = provider;
        workspace.embedding_dimension = dimension;
        workspace.metadata.remove("embedding_model");
        workspace.metadata.remove("embedding_provider");
        workspace.metadata.remove("embedding_dimension");
        return;
    }

    if let Some(embedding_model) = embedding_model {
        workspace.embedding_model = embedding_model.clone();
        workspace.metadata.insert(
            "embedding_model".to_string(),
            serde_json::json!(embedding_model),
        );
    }
    if let Some(embedding_provider) = embedding_provider {
        workspace.embedding_provider = embedding_provider.clone();
        workspace.metadata.insert(
            "embedding_provider".to_string(),
            serde_json::json!(embedding_provider),
        );
    }
    if let Some(embedding_dimension) = embedding_dimension {
        workspace.embedding_dimension = embedding_dimension;
        workspace.metadata.insert(
            "embedding_dimension".to_string(),
            serde_json::json!(embedding_dimension),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    #[serial_test::serial]
    fn clear_llm_resets_to_env_defaults() {
        let key_runtime = "EDGEQUAKE_LLM_PROVIDER";
        let key_model = "EDGEQUAKE_LLM_MODEL";
        let key_default = "EDGEQUAKE_DEFAULT_LLM_PROVIDER";
        let prev_runtime = std::env::var(key_runtime).ok();
        let prev_model = std::env::var(key_model).ok();
        let prev_default = std::env::var(key_default).ok();

        std::env::remove_var(key_default);
        std::env::set_var(key_runtime, "ollama");
        std::env::set_var(key_model, "gemma4:latest");

        let mut ws = Workspace::new(Uuid::new_v4(), "t", "t");
        ws.llm_provider = "mock".to_string();
        ws.llm_model = "stale-model".to_string();
        ws.metadata
            .insert("llm_provider".to_string(), serde_json::json!("mock"));
        ws.metadata
            .insert("llm_model".to_string(), serde_json::json!("stale-model"));

        apply_llm_config_update(&mut ws, Some(String::new()), Some(String::new()));

        assert_eq!(ws.llm_provider, "ollama");
        assert_eq!(ws.llm_model, "gemma4:latest");
        assert!(!ws.metadata.contains_key("llm_provider"));
        assert!(!ws.metadata.contains_key("llm_model"));

        restore_env(key_default, prev_default);
        restore_env(key_runtime, prev_runtime);
        restore_env(key_model, prev_model);
    }

    #[test]
    #[serial_test::serial]
    fn clear_llm_prefers_runtime_provider_over_default_vars() {
        let key_default = "EDGEQUAKE_DEFAULT_LLM_PROVIDER";
        let key_runtime = "EDGEQUAKE_LLM_PROVIDER";
        let key_model = "EDGEQUAKE_LLM_MODEL";
        let prev_default = std::env::var(key_default).ok();
        let prev_runtime = std::env::var(key_runtime).ok();
        let prev_model = std::env::var(key_model).ok();

        std::env::set_var(key_default, "ollama");
        std::env::set_var(key_runtime, "mistral");
        std::env::set_var(key_model, "mistral-small-latest");

        let mut ws = Workspace::new(Uuid::new_v4(), "t", "t");
        ws.llm_provider = "mock".into();
        ws.llm_model = "stale".into();

        apply_llm_config_update(&mut ws, Some(String::new()), Some(String::new()));

        assert_eq!(ws.llm_provider, "mistral");
        assert_eq!(ws.llm_model, "mistral-small-latest");

        restore_env(key_default, prev_default);
        restore_env(key_runtime, prev_runtime);
        restore_env(key_model, prev_model);
    }

    fn restore_env(key: &str, value: Option<String>) {
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }
}
