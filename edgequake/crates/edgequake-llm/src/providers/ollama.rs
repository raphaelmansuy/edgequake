//! Ollama provider implementation.
//!
//! This module provides integration with Ollama's local LLM API.
//! Ollama provides an OpenAI-compatible API, so this provider wraps
//! the functionality with Ollama-specific defaults.
//!
//! # Default Configuration
//!
//! - Base URL: `http://localhost:11434`
//! - Default model: `gemma3:12b` (chat), `embeddinggemma:latest` (embeddings, 768 dimensions)
//!
//! # Environment Variables
//!
//! - `OLLAMA_HOST`: Ollama server URL (default: http://localhost:11434)
//! - `OLLAMA_MODEL`: Default chat model
//! - `OLLAMA_EMBEDDING_MODEL`: Default embedding model
//!
//! # Example
//!
//! ```rust,ignore
//! use edgequake_llm::OllamaProvider;
//!
//! // Connect to local Ollama with defaults
//! let provider = OllamaProvider::from_env()?;
//!
//! // Or specify custom settings
//! let provider = OllamaProvider::builder()
//!     .host("http://localhost:11434")
//!     .model("mistral")
//!     .embedding_model("nomic-embed-text")
//!     .build()?;
//! ```

use async_trait::async_trait;
use futures::stream::BoxStream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::{LlmError, Result};
use crate::traits::{
    ChatMessage, ChatRole, CompletionOptions, EmbeddingProvider, LLMProvider, LLMResponse,
};

/// Default Ollama host URL
const DEFAULT_OLLAMA_HOST: &str = "http://localhost:11434";

/// Default Ollama chat model
const DEFAULT_OLLAMA_MODEL: &str = "gemma3:12b";

/// Default Ollama embedding model
const DEFAULT_OLLAMA_EMBEDDING_MODEL: &str = "embeddinggemma:latest";

/// Ollama LLM and embedding provider.
///
/// Provides integration with locally running Ollama instance.
/// Supports both chat completion and text embeddings.
#[derive(Debug, Clone)]
pub struct OllamaProvider {
    client: Client,
    host: String,
    model: String,
    embedding_model: String,
    max_context_length: usize,
    embedding_dimension: usize,
    /// WHY: Ollama embedding models have smaller context windows than LLMs.
    /// nomic-embed-text and embeddinggemma both have 2048 token max.
    /// Without this, API returns 400 "input length exceeds context length".
    /// OODA-09: Added to prevent embedding context overflow errors.
    embedding_max_tokens: usize,
}

/// Builder for OllamaProvider
#[derive(Debug, Clone)]
pub struct OllamaProviderBuilder {
    host: String,
    model: String,
    embedding_model: String,
    max_context_length: usize,
    embedding_dimension: usize,
    embedding_max_tokens: usize,
}

impl Default for OllamaProviderBuilder {
    fn default() -> Self {
        Self {
            host: DEFAULT_OLLAMA_HOST.to_string(),
            model: DEFAULT_OLLAMA_MODEL.to_string(),
            embedding_model: DEFAULT_OLLAMA_EMBEDDING_MODEL.to_string(),
            max_context_length: 8192,
            embedding_dimension: 768, // embeddinggemma:latest default (VERIFIED via Ollama API)
            // WHY 2000: Leave 48 token buffer for safety margin.
            // nomic-embed-text: 2048, embeddinggemma: 2048, mxbai-embed-large: 512
            embedding_max_tokens: 2000,
        }
    }
}

impl OllamaProviderBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the Ollama host URL
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

    /// Set the maximum tokens for embedding model context.
    /// WHY: Ollama embedding models have smaller context than LLMs (typically 2048).
    pub fn embedding_max_tokens(mut self, tokens: usize) -> Self {
        self.embedding_max_tokens = tokens;
        self
    }

    /// Build the OllamaProvider
    pub fn build(self) -> Result<OllamaProvider> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // Longer timeout for local models
            .build()
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        Ok(OllamaProvider {
            client,
            host: self.host,
            model: self.model,
            embedding_model: self.embedding_model,
            max_context_length: self.max_context_length,
            embedding_dimension: self.embedding_dimension,
            embedding_max_tokens: self.embedding_max_tokens,
        })
    }
}

impl OllamaProvider {
    /// Create a new OllamaProvider from environment variables.
    ///
    /// Environment variables:
    /// - `OLLAMA_HOST`: Server URL (default: http://localhost:11434)
    /// - `OLLAMA_MODEL`: Chat model (default: llama3)
    /// - `OLLAMA_EMBEDDING_MODEL`: Embedding model (default: nomic-embed-text)
    pub fn from_env() -> Result<Self> {
        let host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_OLLAMA_HOST.to_string());

        let model =
            std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_OLLAMA_MODEL.to_string());

        let embedding_model = std::env::var("OLLAMA_EMBEDDING_MODEL")
            .unwrap_or_else(|_| DEFAULT_OLLAMA_EMBEDDING_MODEL.to_string());

        OllamaProviderBuilder::new()
            .host(host)
            .model(model)
            .embedding_model(embedding_model)
            .build()
    }

    /// Create a new builder for OllamaProvider
    pub fn builder() -> OllamaProviderBuilder {
        OllamaProviderBuilder::new()
    }

    /// Create with default settings (localhost:11434)
    pub fn default_local() -> Result<Self> {
        OllamaProviderBuilder::new().build()
    }
}

// Ollama API request/response structures

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ChatOptions>,
}

#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ChatResponse {
    model: String,
    message: ResponseMessage,
    done: bool,
    #[serde(default)]
    total_duration: u64,
    #[serde(default)]
    prompt_eval_count: u32,
    #[serde(default)]
    eval_count: u32,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    #[allow(dead_code)]
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    #[serde(default)]
    message: Option<ResponseMessage>,
    #[allow(dead_code)]
    done: bool,
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    embeddings: Vec<Vec<f32>>,
}

impl OllamaProvider {
    fn convert_role(role: &ChatRole) -> &'static str {
        match role {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
            ChatRole::Function => "user", // Ollama doesn't have function role
        }
    }

    fn convert_messages(messages: &[ChatMessage]) -> Vec<OllamaMessage> {
        messages
            .iter()
            .map(|msg| OllamaMessage {
                role: Self::convert_role(&msg.role).to_string(),
                content: msg.content.clone(),
            })
            .collect()
    }

    /// Truncate text to fit within token limit for embedding models.
    ///
    /// WHY: Ollama embedding models have smaller context windows than LLMs.
    /// nomic-embed-text and embeddinggemma both have ~2048 token max.
    /// Sending longer text causes 400 "input length exceeds context length".
    ///
    /// OODA-09: Added to prevent embedding API failures with large chunks.
    /// Uses ~4 chars per token approximation (standard for English text).
    fn truncate_for_embedding(&self, text: &str) -> String {
        // Approximate tokens as chars / 4 (conservative estimate for English)
        let max_chars = self.embedding_max_tokens * 4;

        if text.len() <= max_chars {
            return text.to_string();
        }

        // Log warning when truncating
        tracing::warn!(
            original_len = text.len(),
            truncated_to = max_chars,
            model = %self.embedding_model,
            "Truncating text for embedding (exceeds model context)"
        );

        // Truncate on char boundary
        let truncated: String = text.chars().take(max_chars).collect();
        truncated
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn max_context_length(&self) -> usize {
        self.max_context_length
    }

    async fn complete(&self, prompt: &str) -> Result<LLMResponse> {
        self.complete_with_options(prompt, &CompletionOptions::default())
            .await
    }

    async fn complete_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> Result<LLMResponse> {
        let mut messages = Vec::new();

        if let Some(system) = &options.system_prompt {
            messages.push(ChatMessage::system(system));
        }
        messages.push(ChatMessage::user(prompt));

        self.chat(&messages, Some(options)).await
    }

    async fn chat(
        &self,
        messages: &[ChatMessage],
        options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse> {
        debug!(
            "Ollama chat request: {} messages to model {}",
            messages.len(),
            self.model
        );

        let url = format!("{}/api/chat", self.host);
        let opts = options.cloned().unwrap_or_default();

        let chat_options = ChatOptions {
            temperature: opts.temperature,
            num_predict: opts.max_tokens.map(|t| t as i32),
            stop: opts.stop.clone(),
        };

        let request = ChatRequest {
            model: self.model.clone(),
            messages: Self::convert_messages(messages),
            stream: false,
            options: Some(chat_options),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!(
                "Ollama API error ({}): {}",
                status, error_text
            )));
        }

        let response: ChatResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ApiError(format!("Failed to parse response: {}", e)))?;

        Ok(
            LLMResponse::new(response.message.content, response.model).with_usage(
                response.prompt_eval_count as usize,
                response.eval_count as usize,
            ),
        )
    }

    async fn stream(&self, prompt: &str) -> Result<BoxStream<'static, Result<String>>> {
        // Delegate to stream_with_options with default options
        self.stream_with_options(prompt, &CompletionOptions::default())
            .await
    }

    /// Stream with options including stop sequences.
    ///
    /// @implements SPEC-032: Ollama stop token handling (OODA 63)
    ///
    /// This method properly passes stop sequences to Ollama's streaming API,
    /// ensuring that generation stops when configured stop tokens are encountered.
    async fn stream_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> Result<BoxStream<'static, Result<String>>> {
        use futures::StreamExt;

        debug!(
            "Ollama stream request: prompt to model {}, stop_sequences={:?}",
            self.model, options.stop
        );

        let url = format!("{}/api/chat", self.host);

        let chat_options = ChatOptions {
            temperature: options.temperature,
            num_predict: options.max_tokens.map(|t| t as i32),
            stop: options.stop.clone(),
        };

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![OllamaMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            stream: true,
            options: Some(chat_options),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!(
                "Ollama API error ({}): {}",
                status, error_text
            )));
        }

        let stream = response.bytes_stream();

        let mapped_stream = stream.map(|chunk_result| {
            match chunk_result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    // Parse NDJSON - each line is a separate JSON object
                    let mut content = String::new();
                    for line in text.lines() {
                        if line.is_empty() {
                            continue;
                        }
                        if let Ok(chunk) = serde_json::from_str::<StreamChunk>(line) {
                            if let Some(msg) = chunk.message {
                                content.push_str(&msg.content);
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

    fn supports_streaming(&self) -> bool {
        true
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    /// Returns the embedding model name (not completion model).
    #[allow(clippy::misnamed_getters)]
    fn model(&self) -> &str {
        &self.embedding_model
    }

    fn dimension(&self) -> usize {
        self.embedding_dimension
    }

    fn max_tokens(&self) -> usize {
        // WHY: Return actual embedding model limit, not LLM context.
        // nomic-embed-text: 2048, embeddinggemma: 2048
        // OODA-09: Fixed to return correct embedding context limit.
        self.embedding_max_tokens
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // OODA-09: Truncate texts to fit embedding model's context window.
        // WHY: Ollama returns 400 "input length exceeds context length" if
        // any single input exceeds the model's limit (typically 2048 tokens).
        let truncated_texts: Vec<String> = texts
            .iter()
            .map(|t| self.truncate_for_embedding(t))
            .collect();

        debug!(
            "Ollama embedding request: {} texts with model {} (max_tokens={})",
            truncated_texts.len(),
            self.embedding_model,
            self.embedding_max_tokens
        );

        let url = format!("{}/api/embed", self.host);

        let request = EmbeddingRequest {
            model: self.embedding_model.clone(),
            input: truncated_texts,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!(
                "Ollama API error ({}): {}",
                status, error_text
            )));
        }

        let response: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ApiError(format!("Failed to parse response: {}", e)))?;

        debug!(
            "Ollama embedding response: {} embeddings",
            response.embeddings.len()
        );

        Ok(response.embeddings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creation() {
        let provider = OllamaProviderBuilder::new()
            .host("http://localhost:11434")
            .model("mistral")
            .embedding_model("nomic-embed-text")
            .build()
            .unwrap();

        assert_eq!(LLMProvider::name(&provider), "ollama");
        assert_eq!(LLMProvider::model(&provider), "mistral");
        assert_eq!(EmbeddingProvider::model(&provider), "nomic-embed-text");
    }

    #[test]
    fn test_default_builder() {
        let provider = OllamaProviderBuilder::new().build().unwrap();

        assert_eq!(LLMProvider::name(&provider), "ollama");
        assert_eq!(LLMProvider::model(&provider), "gemma3:12b");
        assert_eq!(EmbeddingProvider::model(&provider), "embeddinggemma:latest");
        assert_eq!(provider.max_context_length(), 8192);
    }

    #[test]
    fn test_message_conversion() {
        let messages = vec![
            ChatMessage::system("You are helpful"),
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there!"),
        ];

        let converted = OllamaProvider::convert_messages(&messages);

        assert_eq!(converted.len(), 3);
        assert_eq!(converted[0].role, "system");
        assert_eq!(converted[1].role, "user");
        assert_eq!(converted[2].role, "assistant");
    }

    #[tokio::test]
    async fn test_embed_empty_input() {
        let provider = OllamaProviderBuilder::new().build().unwrap();
        let result = provider.embed(&[]).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
