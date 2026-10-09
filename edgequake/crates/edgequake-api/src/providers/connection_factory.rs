//! Construct LLM/embedding providers from an explicit Connection (no env mutation).
//!
//! Uses public constructors from `edgequake-llm` 0.10.9.

use std::sync::Arc;

use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_llm::{AnthropicProvider, OpenAIProvider, ProviderFactory};

use crate::error::ApiError;

pub struct ConnectionSpec {
    pub shape: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub embedding_model: Option<String>,
    pub embedding_dimension: Option<usize>,
}

pub fn llm_from_connection(spec: &ConnectionSpec) -> Result<Arc<dyn LLMProvider>, ApiError> {
    let shape = spec.shape.to_ascii_lowercase();
    let key = spec.api_key.clone().unwrap_or_default();
    match shape.as_str() {
        "openai_chat" | "openai" | "openai-compatible" | "openai_compatible" | "omlx"
        | "mlx-lm" | "mlx_lm" | "llamacpp" | "vllm-mlx" | "lmstudio" => Ok(Arc::new(
            OpenAIProvider::compatible(key, spec.base_url.clone()).with_model(&spec.model),
        )),
        "anthropic_messages" | "anthropic" | "claude" => Ok(Arc::new(
            AnthropicProvider::new(key)
                .with_base_url(&spec.base_url)
                .with_model(&spec.model),
        )),
        other => ProviderFactory::create_llm_provider(other, &spec.model)
            .map_err(|e| ApiError::BadRequest(e.to_string())),
    }
}

pub fn embedding_from_connection(
    spec: &ConnectionSpec,
) -> Result<Arc<dyn EmbeddingProvider>, ApiError> {
    let shape = spec.shape.to_ascii_lowercase();
    let key = spec.api_key.clone().unwrap_or_default();
    let model = spec
        .embedding_model
        .clone()
        .unwrap_or_else(|| spec.model.clone());
    match shape.as_str() {
        "openai_chat" | "openai" | "openai-compatible" | "openai_compatible" | "omlx"
        | "mlx-lm" | "mlx_lm" | "llamacpp" | "vllm-mlx" | "lmstudio" => Ok(Arc::new(
            OpenAIProvider::compatible(key, spec.base_url.clone()).with_embedding_model(&model),
        )),
        other => {
            let dim = spec.embedding_dimension.unwrap_or(768);
            ProviderFactory::create_embedding_provider(other, &model, dim)
                .map_err(|e| ApiError::BadRequest(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_compatible_constructs() {
        let spec = ConnectionSpec {
            shape: "openai_chat".into(),
            base_url: "http://127.0.0.1:9050".into(),
            api_key: Some("k".into()),
            model: "default".into(),
            embedding_model: None,
            embedding_dimension: None,
        };
        assert!(llm_from_connection(&spec).is_ok());
        assert!(embedding_from_connection(&spec).is_ok());
    }
}
