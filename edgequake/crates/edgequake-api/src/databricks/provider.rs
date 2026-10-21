//! Databricks Foundation Model serving as an OpenAI-compatible provider.
//!
//! `edgequake-llm` 0.10.9 rejects `EDGEQUAKE_LLM_PROVIDER=databricks`. Apps pass
//! that name plus `system.ai.*` model ids. Serving endpoints are
//! `databricks-*` under `{DATABRICKS_HOST}/serving-endpoints`. The workspace
//! access token is fetched per call (cached) so a one-hour OIDC token does not
//! stick in the provider for the life of the process.

use std::sync::Arc;

use async_trait::async_trait;
use edgequake_llm::{
    ChatMessage, CompletionOptions, EmbeddingProvider, LLMProvider, LlmError, OpenAIProvider,
    Result as LlmResult, ToolChoice, ToolDefinition,
};
use tracing::info;

use super::lakebase::{workspace_access_token, workspace_host, DbUrlError};

/// `true` when the process LLM provider is Databricks serving.
pub fn llm_provider_selected() -> bool {
    std::env::var("EDGEQUAKE_LLM_PROVIDER")
        .map(|value| value.trim().eq_ignore_ascii_case("databricks"))
        .unwrap_or(false)
}

/// Chat and embedding providers for `EDGEQUAKE_LLM_PROVIDER=databricks`.
pub fn providers_from_env() -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>), String> {
    let provider = Arc::new(DatabricksProvider::from_env()?);
    info!(
        chat_model = %provider.chat_model,
        embedding_model = %provider.embedding_model,
        dimension = provider.dimension,
        "Databricks serving provider ready"
    );
    let embedding: Arc<dyn EmbeddingProvider> = provider.clone();
    let chat: Arc<dyn LLMProvider> = provider;
    Ok((chat, embedding))
}

/// Embedding provider when `EDGEQUAKE_EMBEDDING_PROVIDER=databricks`.
pub fn embedding_provider_from_env() -> Result<Arc<dyn EmbeddingProvider>, String> {
    let provider = Arc::new(DatabricksProvider::from_env()?);
    Ok(provider)
}

/// Map a workspace model id onto a serving-endpoint name.
///
/// `system.ai.llama-4-maverick` is the catalog id. The endpoint that answers
/// `POST /serving-endpoints/chat/completions` is `databricks-llama-4-maverick`.
pub fn serving_endpoint_name(model: &str) -> String {
    let model = model.trim();
    match model.strip_prefix("system.ai.") {
        Some(rest) if !rest.is_empty() => format!("databricks-{rest}"),
        _ => model.to_string(),
    }
}

/// Embedding width. `EDGEQUAKE_EMBEDDING_DIMENSION` wins; otherwise 1024
/// (`system.ai.gte-large-en`).
pub fn embedding_dimension(raw: Option<&str>) -> usize {
    raw.and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|dimension| *dimension > 0)
        .unwrap_or(1024)
}

struct DatabricksProvider {
    chat_model: String,
    embedding_model: String,
    dimension: usize,
}

impl DatabricksProvider {
    fn from_env() -> Result<Self, String> {
        let _ = workspace_host().map_err(|error| error.to_string())?;
        Ok(Self {
            chat_model: serving_endpoint_name(
                &nonempty("EDGEQUAKE_LLM_MODEL")
                    .unwrap_or_else(|| "databricks-llama-4-maverick".into()),
            ),
            embedding_model: serving_endpoint_name(
                &nonempty("EDGEQUAKE_EMBEDDING_MODEL")
                    .unwrap_or_else(|| "databricks-gte-large-en".into()),
            ),
            dimension: embedding_dimension(nonempty("EDGEQUAKE_EMBEDDING_DIMENSION").as_deref()),
        })
    }

    async fn openai(&self, embedding: bool) -> LlmResult<OpenAIProvider> {
        let token = workspace_access_token().await.map_err(config_error)?;
        let host = workspace_host().map_err(config_error)?;
        let base = format!("{host}/serving-endpoints");
        let provider = OpenAIProvider::compatible(token, base);
        if embedding {
            Ok(provider.with_embedding_model(&self.embedding_model))
        } else {
            Ok(provider.with_model(&self.chat_model))
        }
    }
}

fn config_error(error: DbUrlError) -> LlmError {
    LlmError::ConfigError(error.to_string())
}

fn nonempty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[async_trait]
impl LLMProvider for DatabricksProvider {
    fn name(&self) -> &str {
        "databricks"
    }

    fn model(&self) -> &str {
        &self.chat_model
    }

    fn max_context_length(&self) -> usize {
        128_000
    }

    fn supports_json_mode(&self) -> bool {
        true
    }

    async fn complete(&self, prompt: &str) -> LlmResult<edgequake_llm::LLMResponse> {
        self.openai(false).await?.complete(prompt).await
    }

    async fn complete_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> LlmResult<edgequake_llm::LLMResponse> {
        self.openai(false)
            .await?
            .complete_with_options(prompt, options)
            .await
    }

    async fn chat(
        &self,
        messages: &[ChatMessage],
        options: Option<&CompletionOptions>,
    ) -> LlmResult<edgequake_llm::LLMResponse> {
        self.openai(false).await?.chat(messages, options).await
    }

    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        tool_choice: Option<ToolChoice>,
        options: Option<&CompletionOptions>,
    ) -> LlmResult<edgequake_llm::LLMResponse> {
        self.openai(false)
            .await?
            .chat_with_tools(messages, tools, tool_choice, options)
            .await
    }
}

#[async_trait]
impl EmbeddingProvider for DatabricksProvider {
    fn name(&self) -> &str {
        "databricks"
    }

    fn model(&self) -> &str {
        &self.embedding_model
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn max_tokens(&self) -> usize {
        8192
    }

    async fn embed(&self, texts: &[String]) -> LlmResult<Vec<Vec<f32>>> {
        self.openai(true).await?.embed(texts).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_ai_models_map_to_serving_endpoints() {
        assert_eq!(
            serving_endpoint_name("system.ai.llama-4-maverick"),
            "databricks-llama-4-maverick"
        );
        assert_eq!(
            serving_endpoint_name("system.ai.gte-large-en"),
            "databricks-gte-large-en"
        );
        assert_eq!(
            serving_endpoint_name("databricks-llama-4-maverick"),
            "databricks-llama-4-maverick"
        );
        assert_eq!(serving_endpoint_name("  system.ai.x  "), "databricks-x");
        assert_eq!(serving_endpoint_name("system.ai."), "system.ai.");
    }

    #[test]
    fn embedding_dimension_prefers_positive_env() {
        assert_eq!(embedding_dimension(Some("1024")), 1024);
        assert_eq!(embedding_dimension(Some("0")), 1024);
        assert_eq!(embedding_dimension(Some("nope")), 1024);
        assert_eq!(embedding_dimension(None), 1024);
    }
}
