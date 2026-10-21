//! Databricks AI Gateway as the chat and embedding provider.
//!
//! `edgequake-llm` 0.10.9 rejects `EDGEQUAKE_LLM_PROVIDER=databricks` and sends
//! `max_completion_tokens`. AI Gateway (`{DATABRICKS_HOST}/ai-gateway/mlflow/v1`)
//! accepts Unity Catalog model ids (`system.ai.*`) and `max_tokens`. A rejected
//! `max_completion_tokens` comes back as a 101-byte `{error_code, message}`
//! object, which an OpenAI error parser reports as `missing field 'error'`.
//! The workspace access token is fetched per call (cached) so a one-hour OIDC
//! token does not stick in the provider for the life of the process.

use std::sync::Arc;

use async_trait::async_trait;
use edgequake_llm::{
    ChatMessage, CompletionOptions, EmbeddingProvider, LLMProvider, LlmError, OpenAIProvider,
    Result as LlmResult,
};
use futures::stream::{BoxStream, StreamExt};
use tracing::info;

use super::gateway::{self, ai_gateway_v1_base, model_service_name};
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

/// Chat provider for a caller-selected model (vision OCR, extraction).
///
/// The process-wide provider uses `EDGEQUAKE_LLM_MODEL`. PDF vision and entity
/// extraction pass the workspace model into `ProviderFactory`, which rejects
/// the name `databricks`. This builds the same serving client for that model.
pub fn llm_for_model(model: &str) -> Result<Arc<dyn LLMProvider>, String> {
    let mut provider = DatabricksProvider::from_env()?;
    provider.chat_model = resolved_chat_model(model, &provider.chat_model);
    Ok(Arc::new(provider))
}

/// Empty request keeps the fallback. Both sides are Unity Catalog service names.
pub fn resolved_chat_model(requested: &str, fallback: &str) -> String {
    let requested = requested.trim();
    if requested.is_empty() {
        model_service_name(fallback)
    } else {
        model_service_name(requested)
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
            chat_model: model_service_name(
                &nonempty("EDGEQUAKE_LLM_MODEL")
                    .unwrap_or_else(|| "system.ai.llama-4-maverick".into()),
            ),
            embedding_model: model_service_name(
                &nonempty("EDGEQUAKE_EMBEDDING_MODEL")
                    .unwrap_or_else(|| "system.ai.gte-large-en".into()),
            ),
            dimension: embedding_dimension(nonempty("EDGEQUAKE_EMBEDDING_DIMENSION").as_deref()),
        })
    }

    async fn embeddings_client(&self) -> LlmResult<OpenAIProvider> {
        let token = workspace_access_token().await.map_err(config_error)?;
        let host = workspace_host().map_err(config_error)?;
        Ok(OpenAIProvider::compatible(token, ai_gateway_v1_base(&host))
            .with_embedding_model(&self.embedding_model))
    }

    async fn post_chat_learning_cap(
        &self,
        mut body: serde_json::Value,
    ) -> LlmResult<reqwest::Response> {
        apply_remembered_output_cap(&mut body);
        let response = self.post_chat(&body).await?;
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status().as_u16();
        let text = response.text().await.map_err(|error| {
            LlmError::NetworkError(format!("Failed to read AI Gateway response: {error}"))
        })?;
        if let Some(cap) = gateway::output_token_cap(&text) {
            remember_output_cap(cap);
            let requested = body
                .get("max_tokens")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as usize;
            if requested > cap {
                body["max_tokens"] = serde_json::json!(cap);
                let retry = self.post_chat(&body).await?;
                if retry.status().is_success() {
                    return Ok(retry);
                }
                let status = retry.status().as_u16();
                let text = retry.text().await.map_err(|error| {
                    LlmError::NetworkError(format!("Failed to read AI Gateway response: {error}"))
                })?;
                return Err(gateway::error_from_body(status, &text));
            }
        }
        Err(gateway::error_from_body(status, &text))
    }

    async fn post_chat(&self, body: &serde_json::Value) -> LlmResult<reqwest::Response> {
        let token = workspace_access_token().await.map_err(config_error)?;
        let host = workspace_host().map_err(config_error)?;
        let url = format!("{}/chat/completions", ai_gateway_v1_base(&host));
        http_client()
            .post(url)
            .bearer_auth(token)
            .json(body)
            .send()
            .await
            .map_err(|error| LlmError::NetworkError(format!("AI Gateway request failed: {error}")))
    }
}

static OUTPUT_TOKEN_CAP: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn apply_remembered_output_cap(body: &mut serde_json::Value) {
    let cap = OUTPUT_TOKEN_CAP.load(std::sync::atomic::Ordering::Relaxed);
    if cap == 0 {
        return;
    }
    let Some(requested) = body.get("max_tokens").and_then(serde_json::Value::as_u64) else {
        return;
    };
    if requested as usize > cap {
        body["max_tokens"] = serde_json::json!(cap);
    }
}

fn remember_output_cap(cap: usize) {
    if cap > 0 {
        OUTPUT_TOKEN_CAP.store(cap, std::sync::atomic::Ordering::Relaxed);
    }
}

fn http_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

async fn chat_response(response: reqwest::Response) -> LlmResult<String> {
    let status = response.status();
    let body = response.text().await.map_err(|error| {
        LlmError::NetworkError(format!("Failed to read AI Gateway response: {error}"))
    })?;
    if !status.is_success() {
        return Err(gateway::error_from_body(status.as_u16(), &body));
    }
    Ok(body)
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

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn complete(&self, prompt: &str) -> LlmResult<edgequake_llm::LLMResponse> {
        self.complete_with_options(prompt, &CompletionOptions::default())
            .await
    }

    async fn complete_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> LlmResult<edgequake_llm::LLMResponse> {
        let mut messages = Vec::new();
        if let Some(system) = &options.system_prompt {
            messages.push(ChatMessage::system(system.clone()));
        }
        messages.push(ChatMessage::user(prompt));
        self.chat(&messages, Some(options)).await
    }

    async fn chat(
        &self,
        messages: &[ChatMessage],
        options: Option<&CompletionOptions>,
    ) -> LlmResult<edgequake_llm::LLMResponse> {
        let options = options.cloned().unwrap_or_default();
        let body = gateway::chat_request(&self.chat_model, messages, &options)?;
        let response = self.post_chat_learning_cap(body).await?;
        let text = chat_response(response).await?;
        gateway::parse_chat_response(&text)
    }

    async fn stream(&self, prompt: &str) -> LlmResult<BoxStream<'static, LlmResult<String>>> {
        let mut body = gateway::chat_request(
            &self.chat_model,
            &[ChatMessage::user(prompt)],
            &CompletionOptions::default(),
        )?;
        body["stream"] = serde_json::json!(true);
        let response = self.post_chat_learning_cap(body).await?;
        let incoming = Box::pin(response.bytes_stream());
        let stream = futures::stream::unfold(
            (incoming, String::new()),
            |(mut incoming, mut buf)| async move {
                loop {
                    if let Some(line) = next_sse_line(&mut buf) {
                        match gateway::content_deltas_from_sse(&line) {
                            Ok(deltas) if deltas.is_empty() => continue,
                            Ok(deltas) => return Some((Ok(deltas.concat()), (incoming, buf))),
                            Err(error) => return Some((Err(error), (incoming, buf))),
                        }
                    }
                    match incoming.as_mut().next().await {
                        Some(Ok(bytes)) => buf.push_str(&String::from_utf8_lossy(&bytes)),
                        Some(Err(error)) => {
                            return Some((
                                Err(LlmError::NetworkError(format!(
                                    "AI Gateway stream failed: {error}"
                                ))),
                                (incoming, buf),
                            ));
                        }
                        None => {
                            if buf.trim().is_empty() {
                                return None;
                            }
                            let line = std::mem::take(&mut buf);
                            match gateway::content_deltas_from_sse(&line) {
                                Ok(deltas) => {
                                    if let Some(text) = deltas.into_iter().next() {
                                        return Some((Ok(text), (incoming, buf)));
                                    }
                                }
                                Err(error) => return Some((Err(error), (incoming, buf))),
                            }
                            return None;
                        }
                    }
                }
            },
        );
        Ok(Box::pin(stream))
    }
}

fn next_sse_line(buf: &mut String) -> Option<String> {
    let end = buf.find('\n')?;
    let line: String = buf.drain(..=end).collect();
    Some(line)
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
        self.embeddings_client().await?.embed(texts).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requested_vision_model_keeps_the_unity_catalog_name() {
        assert_eq!(
            resolved_chat_model("system.ai.llama-4-maverick", "databricks-gte-large-en"),
            "system.ai.llama-4-maverick"
        );
        assert_eq!(
            resolved_chat_model("  ", "databricks-llama-4-maverick"),
            "system.ai.llama-4-maverick"
        );
    }

    #[test]
    fn embedding_dimension_prefers_positive_env() {
        assert_eq!(embedding_dimension(Some("1024")), 1024);
        assert_eq!(embedding_dimension(Some("0")), 1024);
        assert_eq!(embedding_dimension(Some("nope")), 1024);
        assert_eq!(embedding_dimension(None), 1024);
    }
}
