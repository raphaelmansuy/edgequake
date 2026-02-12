//! LM Studio provider implementation.
//!
//! This module provides integration with LM Studio's local OpenAI-compatible API.
//! LM Studio runs local models and exposes them via an OpenAI-compatible HTTP API.
//!
//! # Default Configuration
//!
//! - Base URL: `http://localhost:1234`
//! - Default model: `gemma2-9b-it` (chat), `nomic-embed-text-v1.5` (embeddings, 768 dimensions)
//!
//! # Environment Variables
//!
//! - `LMSTUDIO_HOST`: LM Studio server URL (default: http://localhost:1234)
//! - `LMSTUDIO_MODEL`: Default chat model
//! - `LMSTUDIO_EMBEDDING_MODEL`: Default embedding model
//! - `LMSTUDIO_EMBEDDING_DIM`: Embedding dimension (default: 768)
//!
//! # Streaming Support
//!
//! LM Studio supports OpenAI-compatible streaming via Server-Sent Events (SSE).
//! The `stream()` method returns a stream of content chunks.
//! If streaming fails, the caller can fall back to non-streaming mode.
//!
//! # Example
//!
//! ```rust,ignore
//! use edgequake_llm::LMStudioProvider;
//!
//! // Connect to local LM Studio with defaults
//! let provider = LMStudioProvider::from_env()?;
//!
//! // Or specify custom settings
//! let provider = LMStudioProvider::builder()
//!     .host("http://localhost:1234")
//!     .model("mistral-7b-instruct")
//!     .embedding_model("nomic-embed-text-v1.5")
//!     .build()?;
//! ```

use async_trait::async_trait;
use futures::stream::BoxStream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

use crate::error::{LlmError, Result};
use crate::traits::{
    ChatMessage, ChatRole, CompletionOptions, EmbeddingProvider, LLMProvider, LLMResponse,
};

/// Default LM Studio host URL
const DEFAULT_LMSTUDIO_HOST: &str = "http://localhost:1234";

/// Default LM Studio chat model
const DEFAULT_LMSTUDIO_MODEL: &str = "gemma2-9b-it";

/// Default LM Studio embedding model
const DEFAULT_LMSTUDIO_EMBEDDING_MODEL: &str = "nomic-embed-text-v1.5";

/// Default embedding dimension for nomic-embed-text-v1.5
const DEFAULT_LMSTUDIO_EMBEDDING_DIM: usize = 768;

/// LM Studio LLM and embedding provider.
///
/// Provides integration with locally running LM Studio instance.
/// Uses OpenAI-compatible API format.
#[derive(Debug, Clone)]
pub struct LMStudioProvider {
    client: Client,
    host: String,
    model: String,
    embedding_model: String,
    max_context_length: usize,
    embedding_dimension: usize,
}

/// Builder for LMStudioProvider
#[derive(Debug, Clone)]
pub struct LMStudioProviderBuilder {
    host: String,
    model: String,
    embedding_model: String,
    max_context_length: usize,
    embedding_dimension: usize,
}

impl Default for LMStudioProviderBuilder {
    fn default() -> Self {
        Self {
            host: DEFAULT_LMSTUDIO_HOST.to_string(),
            model: DEFAULT_LMSTUDIO_MODEL.to_string(),
            embedding_model: DEFAULT_LMSTUDIO_EMBEDDING_MODEL.to_string(),
            max_context_length: 8192,
            embedding_dimension: DEFAULT_LMSTUDIO_EMBEDDING_DIM,
        }
    }
}

impl LMStudioProviderBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the LM Studio host URL
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Set the chat model
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the embedding model
    pub fn embedding_model(mut self, model: impl Into<String>) -> Self {
        self.embedding_model = model.into();
        self
    }

    /// Set the maximum context length
    pub fn max_context_length(mut self, length: usize) -> Self {
        self.max_context_length = length;
        self
    }

    /// Set the embedding dimension
    pub fn embedding_dimension(mut self, dimension: usize) -> Self {
        self.embedding_dimension = dimension;
        self
    }

    /// Build the LMStudioProvider
    pub fn build(self) -> Result<LMStudioProvider> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // Longer timeout for local models
            .build()
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        Ok(LMStudioProvider {
            client,
            host: self.host,
            model: self.model,
            embedding_model: self.embedding_model,
            max_context_length: self.max_context_length,
            embedding_dimension: self.embedding_dimension,
        })
    }
}

impl LMStudioProvider {
    /// Create a new LMStudioProvider from environment variables.
    ///
    /// Environment variables:
    /// - `LMSTUDIO_HOST`: Server URL (default: http://localhost:1234)
    /// - `LMSTUDIO_MODEL`: Chat model (default: gemma2-9b-it)
    /// - `LMSTUDIO_EMBEDDING_MODEL`: Embedding model (default: nomic-embed-text-v1.5)
    /// - `LMSTUDIO_EMBEDDING_DIM`: Embedding dimension (default: 768)
    pub fn from_env() -> Result<Self> {
        let host =
            std::env::var("LMSTUDIO_HOST").unwrap_or_else(|_| DEFAULT_LMSTUDIO_HOST.to_string());

        let model =
            std::env::var("LMSTUDIO_MODEL").unwrap_or_else(|_| DEFAULT_LMSTUDIO_MODEL.to_string());

        let embedding_model = std::env::var("LMSTUDIO_EMBEDDING_MODEL")
            .unwrap_or_else(|_| DEFAULT_LMSTUDIO_EMBEDDING_MODEL.to_string());

        let embedding_dimension = std::env::var("LMSTUDIO_EMBEDDING_DIM")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_LMSTUDIO_EMBEDDING_DIM);

        LMStudioProviderBuilder::new()
            .host(host)
            .model(model)
            .embedding_model(embedding_model)
            .embedding_dimension(embedding_dimension)
            .build()
    }

    /// Create a new builder for LMStudioProvider
    pub fn builder() -> LMStudioProviderBuilder {
        LMStudioProviderBuilder::new()
    }

    /// Create with default settings (localhost:1234)
    pub fn default_local() -> Result<Self> {
        LMStudioProviderBuilder::new().build()
    }

    /// Get the API base URL with /v1 suffix
    fn api_base(&self) -> String {
        if self.host.ends_with("/v1") {
            self.host.clone()
        } else {
            format!("{}/v1", self.host)
        }
    }

    /// Check if LM Studio server is available.
    ///
    /// Makes a lightweight request to check if the server is responding.
    /// Used for health checks and capability detection.
    pub async fn is_available(&self) -> bool {
        let url = format!("{}/models", self.api_base());
        match self.client.get(&url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    /// Get available models from LM Studio.
    ///
    /// Queries the /v1/models endpoint to get the list of loaded models.
    /// Returns empty vec if the server is not available or no models are loaded.
    pub async fn available_models(&self) -> Vec<String> {
        let url = format!("{}/models", self.api_base());
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                #[derive(Deserialize)]
                struct ModelsResponse {
                    data: Vec<ModelInfo>,
                }
                #[derive(Deserialize)]
                struct ModelInfo {
                    id: String,
                }

                match response.json::<ModelsResponse>().await {
                    Ok(models) => models.data.into_iter().map(|m| m.id).collect(),
                    Err(_) => vec![],
                }
            }
            _ => vec![],
        }
    }
}

// OpenAI-compatible API request/response structures

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessageRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i32>,
    /// Stop sequences - generation stops when any of these are encountered.
    /// @implements SPEC-032: LMStudio stop token handling (OODA 63)
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct ChatMessageRequest {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
    usage: Option<UsageInfo>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChatMessageResponse {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct UsageInfo {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

// API error handling

#[derive(Debug, Deserialize)]
struct ApiError {
    error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ApiErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
}

// Streaming response structures (OpenAI-compatible SSE)

/// Delta content in a streaming chunk
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
struct StreamDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    role: Option<String>,
}

/// A choice in a streaming chunk
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StreamChoice {
    delta: StreamDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

/// Streaming response chunk (OpenAI-compatible format)
#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[async_trait]
impl LLMProvider for LMStudioProvider {
    fn name(&self) -> &str {
        "lmstudio"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn max_context_length(&self) -> usize {
        self.max_context_length
    }

    async fn complete(&self, prompt: &str) -> Result<LLMResponse> {
        let messages = vec![ChatMessage {
            role: ChatRole::User,
            content: prompt.to_string(),
            name: None,
        }];
        self.chat(&messages, None).await
    }

    async fn complete_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> Result<LLMResponse> {
        let messages = vec![ChatMessage {
            role: ChatRole::User,
            content: prompt.to_string(),
            name: None,
        }];
        self.chat(&messages, Some(options)).await
    }

    async fn chat(
        &self,
        messages: &[ChatMessage],
        options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse> {
        let api_messages: Vec<ChatMessageRequest> = messages
            .iter()
            .map(|m| ChatMessageRequest {
                role: match m.role {
                    ChatRole::System => "system".to_string(),
                    ChatRole::User => "user".to_string(),
                    ChatRole::Assistant => "assistant".to_string(),
                    ChatRole::Function => "function".to_string(),
                },
                content: m.content.clone(),
            })
            .collect();

        let opts = options.cloned().unwrap_or_default();
        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages: api_messages,
            temperature: opts.temperature,
            max_tokens: opts.max_tokens.map(|t| t as i32),
            stop: opts.stop.clone(),
            stream: false,
        };

        let url = format!("{}/chat/completions", self.api_base());

        debug!(
            provider = "lmstudio",
            model = %self.model,
            url = %url,
            message_count = messages.len(),
            "Sending chat completion request"
        );

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(format!("LM Studio request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Try to parse as API error
            if let Ok(api_error) = serde_json::from_str::<ApiError>(&error_text) {
                return Err(LlmError::ApiError(format!(
                    "LM Studio API error ({}): {}",
                    status, api_error.error.message
                )));
            }

            return Err(LlmError::ApiError(format!(
                "LM Studio API error ({}): {}",
                status, error_text
            )));
        }

        let completion: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Failed to parse response: {}", e)))?;

        let content = completion
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let (prompt_tokens, completion_tokens) = completion
            .usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((0, 0));

        debug!(
            provider = "lmstudio",
            prompt_tokens = prompt_tokens,
            completion_tokens = completion_tokens,
            content_length = content.len(),
            "Received chat completion response"
        );

        Ok(LLMResponse {
            content,
            prompt_tokens,
            completion_tokens,
            model: self.model.clone(),
            total_tokens: prompt_tokens + completion_tokens,
            finish_reason: completion
                .choices
                .first()
                .and_then(|c| c.finish_reason.clone()),
            metadata: HashMap::new(),
        })
    }

    /// Stream a completion response from LM Studio.
    ///
    /// LM Studio uses OpenAI-compatible SSE (Server-Sent Events) streaming.
    /// Each chunk contains a delta with partial content.
    ///
    /// If streaming fails, the caller should fall back to non-streaming mode
    /// using the `chat()` method.
    async fn stream(&self, prompt: &str) -> Result<BoxStream<'static, Result<String>>> {
        // Delegate to stream_with_options with default options
        self.stream_with_options(prompt, &CompletionOptions::default())
            .await
    }

    /// Stream with options including stop sequences.
    ///
    /// @implements SPEC-032: LMStudio stop token handling (OODA 63)
    async fn stream_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> Result<BoxStream<'static, Result<String>>> {
        use futures::StreamExt;

        debug!(
            provider = "lmstudio",
            model = %self.model,
            stop_sequences = ?options.stop,
            "Starting streaming request with options"
        );

        let url = format!("{}/chat/completions", self.api_base());

        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages: vec![ChatMessageRequest {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: options.temperature,
            max_tokens: options.max_tokens.map(|t| t as i32),
            stop: options.stop.clone(),
            stream: true,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                LlmError::NetworkError(format!("LM Studio stream request failed: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!(
                "LM Studio streaming API error ({}): {}",
                status, error_text
            )));
        }

        let stream = response.bytes_stream();

        // Parse SSE stream - each line is "data: {json}" or "data: [DONE]"
        let mapped_stream = stream.map(|chunk_result| {
            match chunk_result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let mut content = String::new();

                    for line in text.lines() {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        // Skip non-data lines (like "event:" or comments)
                        if !line.starts_with("data:") {
                            continue;
                        }
                        // Extract JSON after "data: "
                        let json_str = line.strip_prefix("data:").unwrap_or(line).trim();

                        // Check for stream end marker
                        if json_str == "[DONE]" {
                            continue;
                        }

                        // Parse the JSON chunk
                        if let Ok(chunk) = serde_json::from_str::<StreamChunk>(json_str) {
                            if let Some(choice) = chunk.choices.first() {
                                if let Some(delta_content) = &choice.delta.content {
                                    content.push_str(delta_content);
                                }
                            }
                        }
                    }
                    Ok(content)
                }
                Err(e) => Err(LlmError::NetworkError(e.to_string())),
            }
        });

        Ok(mapped_stream.boxed())
    }

    /// LM Studio supports streaming via OpenAI-compatible SSE.
    fn supports_streaming(&self) -> bool {
        true
    }

    /// LM Studio supports JSON mode via OpenAI-compatible response_format.
    fn supports_json_mode(&self) -> bool {
        true
    }
}

#[async_trait]
impl EmbeddingProvider for LMStudioProvider {
    fn name(&self) -> &str {
        "lmstudio"
    }

    // WHY: Clippy false positive - EmbeddingProvider::model() should return
    // embedding_model (not self.model which is the LLM model).
    // The struct has separate fields for LLM (model) and embedding (embedding_model).
    #[allow(clippy::wrong_self_convention)]
    #[allow(clippy::misnamed_getters)]
    fn model(&self) -> &str {
        &self.embedding_model
    }

    fn dimension(&self) -> usize {
        self.embedding_dimension
    }

    fn max_tokens(&self) -> usize {
        8192 // Default max tokens for embedding models
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // WHY: LM Studio (OpenAI-compatible) has batch limits. Use 2048 as safe default.
        const MAX_EMBEDDING_BATCH_SIZE: usize = 2048;

        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Filter out empty/whitespace-only strings - APIs reject these
        let valid_texts: Vec<(usize, &String)> = texts
            .iter()
            .enumerate()
            .filter(|(_, text)| !text.trim().is_empty())
            .collect();

        // If all texts are empty/whitespace, return zero vectors
        if valid_texts.is_empty() {
            debug!(
                "All {} input texts are empty or whitespace-only, returning zero vectors",
                texts.len()
            );
            return Ok(vec![vec![0.0; self.embedding_dimension]; texts.len()]);
        }

        // Extract just the valid texts
        let api_texts: Vec<String> = valid_texts
            .iter()
            .map(|(_, text)| (*text).clone())
            .collect();

        let total_texts = api_texts.len();
        let num_batches = total_texts.div_ceil(MAX_EMBEDDING_BATCH_SIZE);

        if num_batches > 1 {
            info!(
                "Splitting {} texts into {} batches of max {} for LM Studio embedding API",
                total_texts, num_batches, MAX_EMBEDDING_BATCH_SIZE
            );
        }

        let url = format!("{}/embeddings", self.api_base());

        // Process in batches to respect API limits
        let mut all_embeddings: Vec<Vec<f32>> = Vec::with_capacity(total_texts);

        for (batch_idx, batch) in api_texts.chunks(MAX_EMBEDDING_BATCH_SIZE).enumerate() {
            if num_batches > 1 {
                debug!(
                    provider = "lmstudio",
                    model = %self.embedding_model,
                    batch = batch_idx + 1,
                    total_batches = num_batches,
                    text_count = batch.len(),
                    "Sending embedding batch"
                );
            } else {
                debug!(
                    provider = "lmstudio",
                    model = %self.embedding_model,
                    url = %url,
                    text_count = batch.len(),
                    "Sending embedding request"
                );
            }

            let request = EmbeddingRequest {
                model: self.embedding_model.clone(),
                input: batch.to_vec(),
            };

            let response = self
                .client
                .post(&url)
                .json(&request)
                .send()
                .await
                .map_err(|e| {
                    LlmError::NetworkError(format!("LM Studio embedding request failed: {}", e))
                })?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());

                // Try to parse as API error
                if let Ok(api_error) = serde_json::from_str::<ApiError>(&error_text) {
                    return Err(LlmError::ApiError(format!(
                        "LM Studio embedding API error ({}): {}",
                        status, api_error.error.message
                    )));
                }

                return Err(LlmError::ApiError(format!(
                    "LM Studio embedding API error ({}): {}",
                    status, error_text
                )));
            }

            let embedding_response: EmbeddingResponse = response.json().await.map_err(|e| {
                LlmError::NetworkError(format!("Failed to parse embedding response: {}", e))
            })?;

            all_embeddings.extend(embedding_response.data.into_iter().map(|d| d.embedding));
        }

        debug!(
            provider = "lmstudio",
            embedding_count = all_embeddings.len(),
            dimension = all_embeddings
                .first()
                .map(|e: &Vec<f32>| e.len())
                .unwrap_or(0),
            "Received embeddings"
        );

        // Map embeddings back to original indices
        let mut result = vec![vec![0.0; self.embedding_dimension]; texts.len()];
        for ((orig_idx, _), embedding) in valid_texts.iter().zip(all_embeddings) {
            result[*orig_idx] = embedding;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let builder = LMStudioProviderBuilder::new();
        assert_eq!(builder.host, DEFAULT_LMSTUDIO_HOST);
        assert_eq!(builder.model, DEFAULT_LMSTUDIO_MODEL);
        assert_eq!(builder.embedding_model, DEFAULT_LMSTUDIO_EMBEDDING_MODEL);
        assert_eq!(builder.embedding_dimension, DEFAULT_LMSTUDIO_EMBEDDING_DIM);
    }

    #[test]
    fn test_builder_custom() {
        let builder = LMStudioProviderBuilder::new()
            .host("http://custom:8080")
            .model("custom-model")
            .embedding_model("custom-embed")
            .embedding_dimension(1024);

        assert_eq!(builder.host, "http://custom:8080");
        assert_eq!(builder.model, "custom-model");
        assert_eq!(builder.embedding_model, "custom-embed");
        assert_eq!(builder.embedding_dimension, 1024);
    }

    #[test]
    fn test_provider_build() {
        use crate::traits::{EmbeddingProvider, LLMProvider};

        let provider = LMStudioProviderBuilder::new().build().unwrap();
        assert_eq!(LLMProvider::name(&provider), "lmstudio");
        assert_eq!(LLMProvider::model(&provider), DEFAULT_LMSTUDIO_MODEL);
        assert_eq!(
            EmbeddingProvider::dimension(&provider),
            DEFAULT_LMSTUDIO_EMBEDDING_DIM
        );
    }

    #[test]
    fn test_api_base_with_v1() {
        let provider = LMStudioProviderBuilder::new()
            .host("http://localhost:1234/v1")
            .build()
            .unwrap();
        assert_eq!(provider.api_base(), "http://localhost:1234/v1");
    }

    #[test]
    fn test_api_base_without_v1() {
        let provider = LMStudioProviderBuilder::new()
            .host("http://localhost:1234")
            .build()
            .unwrap();
        assert_eq!(provider.api_base(), "http://localhost:1234/v1");
    }

    #[test]
    fn test_from_env_defaults() {
        // Clean environment
        std::env::remove_var("LMSTUDIO_HOST");
        std::env::remove_var("LMSTUDIO_MODEL");
        std::env::remove_var("LMSTUDIO_EMBEDDING_MODEL");
        std::env::remove_var("LMSTUDIO_EMBEDDING_DIM");

        let provider = LMStudioProvider::from_env().unwrap();
        assert_eq!(provider.host, DEFAULT_LMSTUDIO_HOST);
        assert_eq!(provider.model, DEFAULT_LMSTUDIO_MODEL);
    }

    #[test]
    fn test_from_env_custom() {
        std::env::set_var("LMSTUDIO_HOST", "http://custom:9999");
        std::env::set_var("LMSTUDIO_MODEL", "test-model");
        std::env::set_var("LMSTUDIO_EMBEDDING_MODEL", "test-embed");
        std::env::set_var("LMSTUDIO_EMBEDDING_DIM", "512");

        let provider = LMStudioProvider::from_env().unwrap();
        assert_eq!(provider.host, "http://custom:9999");
        assert_eq!(provider.model, "test-model");
        assert_eq!(provider.embedding_model, "test-embed");
        assert_eq!(provider.embedding_dimension, 512);

        // Clean up
        std::env::remove_var("LMSTUDIO_HOST");
        std::env::remove_var("LMSTUDIO_MODEL");
        std::env::remove_var("LMSTUDIO_EMBEDDING_MODEL");
        std::env::remove_var("LMSTUDIO_EMBEDDING_DIM");
    }
}
