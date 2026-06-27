//! Shared vision provider / model resolution from environment (DRY SSOT).
//!
//! Used by PDF upload, image ingest, and startup `vision_llm_provider` wiring.

/// Read an env var, treating empty strings as unset.
pub fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

/// Server-default vision provider (matches `PdfUploadOptions` resolution chain).
pub fn resolved_vision_provider_from_env() -> String {
    non_empty_env("EDGEQUAKE_VISION_PROVIDER")
        .or_else(|| non_empty_env("EDGEQUAKE_VISION_LLM_PROVIDER"))
        .or_else(|| non_empty_env("EDGEQUAKE_DEFAULT_LLM_PROVIDER"))
        .or_else(|| non_empty_env("EDGEQUAKE_LLM_PROVIDER"))
        .unwrap_or_else(|| "ollama".to_string())
}

/// Return a vision model compatible with the resolved provider.
pub fn default_vision_model_for_provider(provider: &str) -> String {
    use crate::safety_limits::is_model_provider_mismatch;

    let candidates = [
        non_empty_env("EDGEQUAKE_VISION_MODEL"),
        non_empty_env("EDGEQUAKE_VISION_LLM_MODEL"),
        non_empty_env("EDGEQUAKE_DEFAULT_LLM_MODEL"),
        non_empty_env("EDGEQUAKE_LLM_MODEL"),
    ];

    for candidate in candidates.into_iter().flatten() {
        if !is_model_provider_mismatch(provider, &candidate) {
            return candidate;
        }
        tracing::warn!(
            provider,
            model = %candidate,
            "Skipping incompatible vision model from env — model '{}' cannot run on provider '{}'",
            candidate,
            provider,
        );
    }

    match provider {
        "openai" => "gpt-4.1-nano".to_string(),
        "anthropic" => "claude-sonnet-4-20250514".to_string(),
        "mistral" => "pixtral-large-latest".to_string(),
        _ => "gemma4:latest".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_openai_vision_model_is_compatible() {
        let model = default_vision_model_for_provider("openai");
        assert!(!model.contains("gemma"));
    }
}
