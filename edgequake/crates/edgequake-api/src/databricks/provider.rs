//! MLflow AI Gateway chat and embeddings.
//!
//! Base URL: `{DATABRICKS_HOST}/ai-gateway/mlflow/v1`.
//! The client posts `{base}/chat/completions` and `{base}/embeddings`.

use std::sync::Arc;

use async_trait::async_trait;
use edgequake_llm::error::{LlmError, Result};
use edgequake_llm::traits::{
    ChatMessage, CompletionOptions, EmbeddingProvider, LLMProvider, LLMResponse, StreamChunk,
};
use futures::stream::BoxStream;
use serde_json::{json, Value};

use super::auth::DatabricksBearer;
use super::mlflow_content::answer_and_thinking;

const DEFAULT_CHAT_MODEL: &str = "system.ai.llama-4-maverick";
const DEFAULT_EMBED_MODEL: &str = "system.ai.gte-large-en";
const DEFAULT_EMBED_DIM: usize = 1024;

#[derive(Clone)]
pub struct DatabricksMLflowProvider {
    inner: Arc<Inner>,
}

struct Inner {
    bearer: DatabricksBearer,
    chat_model: String,
    embed_model: String,
    dimension: usize,
    omit_temperature: bool,
}

impl DatabricksMLflowProvider {
    pub fn from_env() -> std::result::Result<Self, String> {
        let bearer = DatabricksBearer::from_env()?;
        let chat_model = first_nonempty(&["EDGEQUAKE_LLM_MODEL", "DATABRICKS_LLM_MODEL"])
            .unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string());
        let embed_model =
            first_nonempty(&["EDGEQUAKE_EMBEDDING_MODEL", "DATABRICKS_EMBEDDING_MODEL"])
                .unwrap_or_else(|| DEFAULT_EMBED_MODEL.to_string());
        let dimension = std::env::var("EDGEQUAKE_EMBEDDING_DIMENSION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_EMBED_DIM);
        let omit_temperature = !matches!(
            std::env::var("EDGEQUAKE_LLM_OMIT_TEMPERATURE")
                .ok()
                .as_deref(),
            Some("0") | Some("false") | Some("FALSE")
        );
        Ok(Self {
            inner: Arc::new(Inner {
                bearer,
                chat_model,
                embed_model,
                dimension,
                omit_temperature,
            }),
        })
    }

    pub fn with_chat_model(&self, model: &str) -> Self {
        self.retarget(Some(model), None, None)
    }

    pub fn with_embedding(&self, model: &str, dimension: usize) -> Self {
        self.retarget(None, Some(model), Some(dimension))
    }

    fn retarget(
        &self,
        chat_model: Option<&str>,
        embed_model: Option<&str>,
        dimension: Option<usize>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                bearer: self.inner.bearer.clone(),
                chat_model: chat_model
                    .unwrap_or(self.inner.chat_model.as_str())
                    .to_string(),
                embed_model: embed_model
                    .unwrap_or(self.inner.embed_model.as_str())
                    .to_string(),
                dimension: dimension.unwrap_or(self.inner.dimension),
                omit_temperature: self.inner.omit_temperature,
            }),
        }
    }

    fn gateway_base(&self) -> String {
        if let Ok(url) = std::env::var("EDGEQUAKE_MLFLOW_BASE_URL") {
            let url = url.trim().trim_end_matches('/').to_string();
            if !url.is_empty() {
                return url;
            }
        }
        format!("{}/ai-gateway/mlflow/v1", self.inner.bearer.host())
    }

    async fn chat_request(
        &self,
        messages: &[ChatMessage],
        options: Option<&CompletionOptions>,
        stream: bool,
    ) -> Result<reqwest::Response> {
        let token = self
            .inner
            .bearer
            .access_token()
            .await
            .map_err(LlmError::AuthError)?;
        let mut body = json!({
            "model": self.inner.chat_model,
            "messages": messages.iter().map(message_json).collect::<Vec<_>>(),
        });
        if stream {
            body["stream"] = json!(true);
        }
        body["max_tokens"] = json!(capped_max_tokens(options));
        if let Some(options) = options {
            if !self.inner.omit_temperature {
                if let Some(temperature) = options.temperature {
                    body["temperature"] = json!(temperature);
                }
            }
            if options.response_format.as_deref() == Some("json_object") {
                body["response_format"] = json!({"type": "json_object"});
            }
        }
        let url = format!("{}/chat/completions", self.gateway_base());
        let response = self
            .inner
            .bearer
            .client()
            .post(url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(error_from_status(response).await);
        }
        Ok(response)
    }
}

pub fn is_databricks_provider(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "databricks" | "mlflow" | "databricks-mlflow"
    )
}

pub fn databricks_llm_enabled() -> bool {
    std::env::var("EDGEQUAKE_LLM_PROVIDER")
        .ok()
        .map(|name| is_databricks_provider(&name))
        .unwrap_or(false)
}

/// Gateway models in this workspace reject output budgets above 8192.
fn capped_max_tokens(options: Option<&CompletionOptions>) -> usize {
    let cap = std::env::var("EDGEQUAKE_DATABRICKS_MAX_OUTPUT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8192);
    let requested = options.and_then(|options| options.max_tokens);
    requested.unwrap_or(cap).min(cap).max(1)
}

fn first_nonempty(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        std::env::var(name).ok().and_then(|v| {
            let v = v.trim().to_string();
            if v.is_empty() {
                None
            } else {
                Some(v)
            }
        })
    })
}

fn message_json(message: &ChatMessage) -> Value {
    let mut value = json!({
        "role": message.role.as_str(),
        "content": message.content,
    });
    if let Some(images) = &message.images {
        let mut parts = vec![json!({"type": "text", "text": message.content})];
        for image in images {
            parts.push(json!({
                "type": "image_url",
                "image_url": {"url": image.to_data_uri()}
            }));
        }
        value["content"] = Value::Array(parts);
    }
    value
}

fn response_from_json(body: &Value, fallback_model: &str) -> LLMResponse {
    let message = body
        .pointer("/choices/0/message/content")
        .cloned()
        .unwrap_or(Value::Null);
    let (answer, thinking) = answer_and_thinking(&message);
    let model = body
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or(fallback_model);
    let mut response = LLMResponse::new(answer, model);
    if let Some(thinking) = thinking {
        response.thinking_content = Some(thinking);
    }
    let prompt = body
        .pointer("/usage/prompt_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let completion = body
        .pointer("/usage/completion_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    response = response.with_usage(prompt, completion);
    if let Some(cached) = body
        .pointer("/usage/cache_read_input_tokens")
        .and_then(Value::as_u64)
    {
        response = response.with_cache_hit_tokens(cached as usize);
    }
    if let Some(reason) = body
        .pointer("/choices/0/finish_reason")
        .and_then(Value::as_str)
    {
        response = response.with_finish_reason(reason);
    }
    response
}

async fn error_from_status(response: reqwest::Response) -> LlmError {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    let message = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| {
            v.pointer("/error/message")
                .or_else(|| v.get("message"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| truncate(&text, 240));
    if status.as_u16() == 401 || status.as_u16() == 403 {
        LlmError::AuthError(format!("{status}: {message}"))
    } else if status.as_u16() == 404 {
        LlmError::ModelNotFound(message)
    } else if status.as_u16() == 429 {
        LlmError::RateLimited(message)
    } else {
        LlmError::ApiError(format!("{status}: {message}"))
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        text.chars().take(max).collect()
    }
}

/// Pull complete SSE events out of a growing buffer.
pub fn drain_sse(buffer: &mut String) -> Vec<String> {
    let mut events = Vec::new();
    while let Some(index) = buffer.find("\n\n") {
        let raw = buffer[..index].to_string();
        buffer.replace_range(..index + 2, "");
        for line in raw.lines() {
            let line = line.trim();
            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if !data.is_empty() && data != "[DONE]" {
                    events.push(data.to_string());
                }
            }
        }
    }
    events
}

fn chunks_from_sse_data(data: &str) -> Vec<StreamChunk> {
    let Ok(value) = serde_json::from_str::<Value>(data) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let delta = value.pointer("/choices/0/delta/content");
    if let Some(delta) = delta {
        let (text, thinking) = answer_and_thinking(delta);
        if !text.is_empty() {
            out.push(StreamChunk::Content(text));
        }
        if let Some(thinking) = thinking {
            out.push(StreamChunk::ThinkingContent {
                text: thinking,
                tokens_used: None,
                budget_total: None,
            });
        }
    }
    if let Some(reason) = value
        .pointer("/choices/0/finish_reason")
        .and_then(Value::as_str)
    {
        out.push(StreamChunk::Finished {
            reason: reason.to_string(),
            ttft_ms: None,
            usage: None,
        });
    }
    out
}

#[async_trait]
impl LLMProvider for DatabricksMLflowProvider {
    fn name(&self) -> &str {
        "databricks"
    }

    fn model(&self) -> &str {
        &self.inner.chat_model
    }

    fn max_context_length(&self) -> usize {
        128_000
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_json_mode(&self) -> bool {
        true
    }

    async fn complete(&self, prompt: &str) -> Result<LLMResponse> {
        self.chat(&[ChatMessage::user(prompt)], None).await
    }

    async fn complete_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> Result<LLMResponse> {
        self.chat(&[ChatMessage::user(prompt)], Some(options)).await
    }

    async fn chat(
        &self,
        messages: &[ChatMessage],
        options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse> {
        let response = self.chat_request(messages, options, false).await?;
        let body: Value = response
            .json()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;
        Ok(response_from_json(&body, &self.inner.chat_model))
    }

    async fn stream(&self, prompt: &str) -> Result<BoxStream<'static, Result<String>>> {
        let response = self
            .chat_request(&[ChatMessage::user(prompt)], None, true)
            .await?;
        let byte_stream = response.bytes_stream();
        let mapped = futures::stream::unfold(
            (byte_stream, String::new()),
            |(mut stream, mut buffer)| async move {
                use futures::StreamExt;
                loop {
                    if let Some(event) = drain_sse(&mut buffer).into_iter().next() {
                        let text = chunks_from_sse_data(&event).into_iter().find_map(|chunk| {
                            if let StreamChunk::Content(text) = chunk {
                                Some(text)
                            } else {
                                None
                            }
                        });
                        if let Some(text) = text {
                            return Some((Ok(text), (stream, buffer)));
                        }
                        continue;
                    }
                    match stream.next().await {
                        Some(Ok(bytes)) => {
                            buffer.push_str(&String::from_utf8_lossy(&bytes));
                        }
                        Some(Err(error)) => {
                            return Some((
                                Err(LlmError::NetworkError(error.to_string())),
                                (stream, buffer),
                            ));
                        }
                        None => return None,
                    }
                }
            },
        );
        Ok(Box::pin(mapped))
    }
}

#[async_trait]
impl EmbeddingProvider for DatabricksMLflowProvider {
    fn name(&self) -> &str {
        "databricks"
    }

    fn model(&self) -> &str {
        &self.inner.embed_model
    }

    fn dimension(&self) -> usize {
        self.inner.dimension
    }

    fn max_tokens(&self) -> usize {
        8192
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let token = self
            .inner
            .bearer
            .access_token()
            .await
            .map_err(LlmError::AuthError)?;
        let url = format!("{}/embeddings", self.gateway_base());
        let response = self
            .inner
            .bearer
            .client()
            .post(url)
            .bearer_auth(token)
            .json(&json!({
                "model": self.inner.embed_model,
                "input": texts,
            }))
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(error_from_status(response).await);
        }
        let body: Value = response
            .json()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;
        let data = body
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| LlmError::ApiError("embeddings response has no data".into()))?;
        let mut vectors = Vec::with_capacity(data.len());
        for item in data {
            let values = item
                .get("embedding")
                .and_then(Value::as_array)
                .ok_or_else(|| LlmError::ApiError("embedding item has no vector".into()))?;
            let vector: Vec<f32> = values
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();
            vectors.push(vector);
        }
        if let Some(vector) = vectors.first() {
            if vector.len() != self.inner.dimension {
                return Err(LlmError::InvalidRequest(format!(
                    "embedding dimension {} does not match configured {}",
                    vector.len(),
                    self.inner.dimension
                )));
            }
        }
        Ok(vectors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_budget_is_capped_at_the_gateway_limit() {
        let options = CompletionOptions {
            max_tokens: Some(16384),
            ..CompletionOptions::default()
        };
        assert_eq!(capped_max_tokens(Some(&options)), 8192);
        assert_eq!(capped_max_tokens(None), 8192);
    }

    #[test]
    fn sse_drain_keeps_a_partial_event() {
        let mut buffer = "data: {\"a\":1}\n\ndata: {\"b\"".to_string();
        let events = drain_sse(&mut buffer);
        assert_eq!(events, vec!["{\"a\":1}"]);
        assert!(buffer.contains("{\"b\""));
    }

    #[test]
    fn sse_delta_string_is_content() {
        let chunks = chunks_from_sse_data(
            r#"{"choices":[{"delta":{"content":"pon"},"finish_reason":null}]}"#,
        );
        assert!(matches!(chunks.as_slice(), [StreamChunk::Content(text)] if text == "pon"));
    }

    #[tokio::test]
    async fn chat_and_embed_against_a_fake_gateway() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let _guard = crate::databricks::env_lock();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(async move {
                    let mut buf = vec![0_u8; 8192];
                    let n = stream.read(&mut buf).await.unwrap_or(0);
                    let request = String::from_utf8_lossy(&buf[..n]);
                    let body = if request.contains("/embeddings") {
                        br#"{"data":[{"embedding":[0.25,0.5],"index":0}],"model":"system.ai.gte-large-en"}"#.to_vec()
                    } else {
                        br#"{"model":"system.ai.llama-4-maverick","choices":[{"message":{"role":"assistant","content":[{"type":"text","text":"pong"}]},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":1}}"#.to_vec()
                    };
                    let header = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(header.as_bytes()).await;
                    let _ = stream.write_all(&body).await;
                });
            }
        });

        std::env::set_var("DATABRICKS_HOST", format!("http://{addr}"));
        std::env::set_var("DATABRICKS_TOKEN", "probe-token");
        std::env::remove_var("DATABRICKS_CLIENT_ID");
        std::env::remove_var("DATABRICKS_CLIENT_SECRET");
        std::env::set_var("EDGEQUAKE_EMBEDDING_DIMENSION", "2");
        std::env::remove_var("EDGEQUAKE_MLFLOW_BASE_URL");
        let provider = DatabricksMLflowProvider::from_env().expect("provider");
        let response = provider.complete("ping").await.expect("chat");
        assert_eq!(response.content, "pong");
        assert_eq!(response.prompt_tokens, 3);
        let vectors = provider.embed(&["hello".into()]).await.expect("embed");
        assert_eq!(vectors, vec![vec![0.25, 0.5]]);
        std::env::remove_var("DATABRICKS_HOST");
        std::env::remove_var("DATABRICKS_TOKEN");
        std::env::remove_var("EDGEQUAKE_EMBEDDING_DIMENSION");
    }
}
