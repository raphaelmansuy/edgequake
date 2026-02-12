//! Azure OpenAI LLM provider implementation.
//!
//! Supports Azure OpenAI Service endpoints with API key or Entra ID authentication.
//!
//! # Environment Variables
//! - `AZURE_OPENAI_ENDPOINT`: Azure OpenAI endpoint (e.g., `https://myresource.openai.azure.com`)
//! - `AZURE_OPENAI_API_KEY`: API key for authentication
//! - `AZURE_OPENAI_DEPLOYMENT_NAME`: Deployment name for chat/completion model
//! - `AZURE_OPENAI_EMBEDDING_DEPLOYMENT_NAME`: Deployment name for embedding model
//! - `AZURE_OPENAI_API_VERSION`: API version (default: 2024-10-21)

use async_trait::async_trait;
use futures::stream::BoxStream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, instrument};

use crate::error::{LlmError, Result};
use crate::traits::{
    ChatMessage, ChatRole, CompletionOptions, EmbeddingProvider, LLMProvider, LLMResponse,
};

/// Default Azure OpenAI API version
const DEFAULT_API_VERSION: &str = "2024-10-21";

/// Azure OpenAI provider configuration
#[derive(Debug, Clone)]
pub struct AzureOpenAIProvider {
    client: Client,
    endpoint: String,
    api_key: String,
    deployment_name: String,
    embedding_deployment_name: String,
    api_version: String,
    max_context_length: usize,
    embedding_dimension: usize,
}

// ============================================================================
// Azure OpenAI Request/Response Types (same as OpenAI, but different endpoint)
// ============================================================================

/// Message format for Azure OpenAI
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AzureMessage {
    role: String,
    content: String,
}

/// Chat completion request
#[derive(Debug, Clone, Serialize)]
struct ChatCompletionRequest {
    messages: Vec<AzureMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

/// Chat completion response choice
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct ChatChoice {
    index: usize,
    message: AzureMessage,
    finish_reason: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Deserialize, Default)]
struct Usage {
    #[serde(default)]
    prompt_tokens: usize,
    #[serde(default)]
    completion_tokens: usize,
    #[serde(default)]
    total_tokens: usize,
}

/// Chat completion response
#[derive(Debug, Clone, Deserialize)]
struct ChatCompletionResponse {
    id: String,
    choices: Vec<ChatChoice>,
    #[serde(default)]
    usage: Usage,
    model: String,
}

/// Embedding request
#[derive(Debug, Clone, Serialize)]
struct EmbeddingRequest {
    input: Vec<String>,
}

/// Embedding data
#[derive(Debug, Clone, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

/// Embedding response
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
    #[serde(default)]
    usage: Usage,
}

/// Error response from Azure OpenAI
#[derive(Debug, Clone, Deserialize)]
struct AzureErrorResponse {
    error: AzureError,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct AzureError {
    code: Option<String>,
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
}

// ============================================================================
// AzureOpenAIProvider Implementation
// ============================================================================

impl AzureOpenAIProvider {
    /// Create a new Azure OpenAI provider.
    ///
    /// # Arguments
    /// * `endpoint` - Azure OpenAI endpoint (e.g., `https://myresource.openai.azure.com`)
    /// * `api_key` - API key for authentication
    /// * `deployment_name` - Deployment name for chat/completion model
    pub fn new(
        endpoint: impl Into<String>,
        api_key: impl Into<String>,
        deployment_name: impl Into<String>,
    ) -> Self {
        let deployment = deployment_name.into();
        Self {
            client: Client::new(),
            endpoint: endpoint.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            deployment_name: deployment.clone(),
            embedding_deployment_name: deployment,
            api_version: DEFAULT_API_VERSION.to_string(),
            max_context_length: 128_000, // Default for GPT-4o
            embedding_dimension: 1536,   // Default for text-embedding-ada-002
        }
    }

    /// Create a provider from environment variables.
    ///
    /// Reads from:
    /// - `AZURE_OPENAI_ENDPOINT`
    /// - `AZURE_OPENAI_API_KEY`
    /// - `AZURE_OPENAI_DEPLOYMENT_NAME`
    /// - `AZURE_OPENAI_EMBEDDING_DEPLOYMENT_NAME` (optional)
    /// - `AZURE_OPENAI_API_VERSION` (optional)
    pub fn from_env() -> Result<Self> {
        let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT")
            .map_err(|_| LlmError::ConfigError("AZURE_OPENAI_ENDPOINT not set".to_string()))?;

        let api_key = std::env::var("AZURE_OPENAI_API_KEY")
            .map_err(|_| LlmError::ConfigError("AZURE_OPENAI_API_KEY not set".to_string()))?;

        let deployment_name = std::env::var("AZURE_OPENAI_DEPLOYMENT_NAME").map_err(|_| {
            LlmError::ConfigError("AZURE_OPENAI_DEPLOYMENT_NAME not set".to_string())
        })?;

        let mut provider = Self::new(endpoint, api_key, deployment_name);

        if let Ok(embedding_deployment) = std::env::var("AZURE_OPENAI_EMBEDDING_DEPLOYMENT_NAME") {
            provider = provider.with_embedding_deployment(embedding_deployment);
        }

        if let Ok(api_version) = std::env::var("AZURE_OPENAI_API_VERSION") {
            provider = provider.with_api_version(api_version);
        }

        Ok(provider)
    }

    /// Set the embedding deployment name.
    pub fn with_embedding_deployment(mut self, deployment_name: impl Into<String>) -> Self {
        self.embedding_deployment_name = deployment_name.into();
        self
    }

    /// Set the API version.
    pub fn with_api_version(mut self, api_version: impl Into<String>) -> Self {
        self.api_version = api_version.into();
        self
    }

    /// Set the max context length.
    pub fn with_max_context_length(mut self, max_context_length: usize) -> Self {
        self.max_context_length = max_context_length;
        self
    }

    /// Set the embedding dimension.
    pub fn with_embedding_dimension(mut self, dimension: usize) -> Self {
        self.embedding_dimension = dimension;
        self
    }

    /// Build URL for a deployment operation.
    fn build_url(&self, deployment: &str, operation: &str) -> String {
        format!(
            "{}/openai/deployments/{}/{}?api-version={}",
            self.endpoint, deployment, operation, self.api_version
        )
    }

    /// Convert ChatMessage to Azure format.
    fn convert_messages(messages: &[ChatMessage]) -> Vec<AzureMessage> {
        messages
            .iter()
            .map(|msg| AzureMessage {
                role: match msg.role {
                    ChatRole::System => "system".to_string(),
                    ChatRole::User => "user".to_string(),
                    ChatRole::Assistant => "assistant".to_string(),
                    ChatRole::Function => "function".to_string(),
                },
                content: msg.content.clone(),
            })
            .collect()
    }

    /// Send a request and handle errors.
    async fn send_request<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        body: &impl Serialize,
    ) -> Result<T> {
        let response = self
            .client
            .post(url)
            .header("api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| LlmError::ApiError(format!("Request failed: {}", e)))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| LlmError::ApiError(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            if let Ok(error_response) = serde_json::from_str::<AzureErrorResponse>(&text) {
                return Err(LlmError::ApiError(format!(
                    "Azure OpenAI error: {}",
                    error_response.error.message
                )));
            }
            return Err(LlmError::ApiError(format!(
                "Azure OpenAI error ({}): {}",
                status, text
            )));
        }

        serde_json::from_str(&text).map_err(|e| {
            LlmError::ApiError(format!("Failed to parse response: {}. Body: {}", e, text))
        })
    }
}

#[async_trait]
impl LLMProvider for AzureOpenAIProvider {
    fn name(&self) -> &str {
        "azure-openai"
    }

    fn model(&self) -> &str {
        &self.deployment_name
    }

    fn max_context_length(&self) -> usize {
        self.max_context_length
    }

    #[instrument(skip(self, prompt), fields(deployment = %self.deployment_name))]
    async fn complete(&self, prompt: &str) -> Result<LLMResponse> {
        self.complete_with_options(prompt, &CompletionOptions::default())
            .await
    }

    #[instrument(skip(self, prompt, options), fields(deployment = %self.deployment_name))]
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

    #[instrument(skip(self, messages, options), fields(deployment = %self.deployment_name))]
    async fn chat(
        &self,
        messages: &[ChatMessage],
        options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse> {
        let azure_messages = Self::convert_messages(messages);
        let options = options.cloned().unwrap_or_default();

        let request = ChatCompletionRequest {
            messages: azure_messages,
            max_tokens: options.max_tokens,
            max_completion_tokens: None,
            temperature: options.temperature,
            top_p: options.top_p,
            stop: options.stop,
            frequency_penalty: options.frequency_penalty,
            presence_penalty: options.presence_penalty,
            stream: None,
        };

        let url = self.build_url(&self.deployment_name, "chat/completions");
        debug!("Sending request to Azure OpenAI: {}", url);

        let response: ChatCompletionResponse = self.send_request(&url, &request).await?;

        let choice = response
            .choices
            .first()
            .ok_or_else(|| LlmError::ApiError("No choices in response".to_string()))?;

        let mut metadata = HashMap::new();
        metadata.insert("response_id".to_string(), serde_json::json!(response.id));

        Ok(LLMResponse {
            content: choice.message.content.clone(),
            prompt_tokens: response.usage.prompt_tokens,
            completion_tokens: response.usage.completion_tokens,
            total_tokens: response.usage.total_tokens,
            model: response.model,
            finish_reason: choice.finish_reason.clone(),
            metadata,
        })
    }

    async fn stream(&self, prompt: &str) -> Result<BoxStream<'static, Result<String>>> {
        use futures::StreamExt;

        let messages = vec![ChatMessage::user(prompt)];
        let azure_messages = Self::convert_messages(&messages);

        let request = ChatCompletionRequest {
            messages: azure_messages,
            max_tokens: None,
            max_completion_tokens: None,
            temperature: None,
            top_p: None,
            stop: None,
            frequency_penalty: None,
            presence_penalty: None,
            stream: Some(true),
        };

        let url = self.build_url(&self.deployment_name, "chat/completions");

        let response = self
            .client
            .post(&url)
            .header("api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::ApiError(format!("Stream request failed: {}", e)))?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!("Stream error: {}", text)));
        }

        let stream = response.bytes_stream();

        let mapped_stream = stream.map(|result| {
            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    // Parse SSE format
                    let mut content = String::new();
                    for line in text.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                continue;
                            }
                            if let Ok(chunk) = serde_json::from_str::<serde_json::Value>(data) {
                                if let Some(delta_content) = chunk
                                    .get("choices")
                                    .and_then(|c| c.get(0))
                                    .and_then(|c| c.get("delta"))
                                    .and_then(|d| d.get("content"))
                                    .and_then(|c| c.as_str())
                                {
                                    content.push_str(delta_content);
                                }
                            }
                        }
                    }
                    Ok(content)
                }
                Err(e) => Err(LlmError::ApiError(format!("Stream error: {}", e))),
            }
        });

        Ok(mapped_stream.boxed())
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_json_mode(&self) -> bool {
        true
    }
}

#[async_trait]
impl EmbeddingProvider for AzureOpenAIProvider {
    fn name(&self) -> &str {
        "azure-openai"
    }

    fn model(&self) -> &str {
        &self.embedding_deployment_name
    }

    fn dimension(&self) -> usize {
        self.embedding_dimension
    }

    fn max_tokens(&self) -> usize {
        8191 // Azure OpenAI embedding models support 8191 tokens
    }

    #[instrument(skip(self, texts), fields(deployment = %self.embedding_deployment_name, count = texts.len()))]
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // WHY: Azure OpenAI (same as OpenAI) enforces a 2048 inputs per request limit.
        const MAX_EMBEDDING_BATCH_SIZE: usize = 2048;

        if texts.is_empty() {
            return Ok(Vec::new());
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
                "Splitting {} texts into {} batches of max {} for Azure OpenAI embedding API",
                total_texts, num_batches, MAX_EMBEDDING_BATCH_SIZE
            );
        }

        let url = self.build_url(&self.embedding_deployment_name, "embeddings");

        // Process in batches to respect API limits
        let mut all_embeddings: Vec<Vec<f32>> = Vec::with_capacity(total_texts);

        for (batch_idx, batch) in api_texts.chunks(MAX_EMBEDDING_BATCH_SIZE).enumerate() {
            if num_batches > 1 {
                debug!(
                    "Embedding batch {}/{}: {} texts",
                    batch_idx + 1,
                    num_batches,
                    batch.len()
                );
            }

            let request = EmbeddingRequest {
                input: batch.to_vec(),
            };

            debug!("Sending embedding request to Azure OpenAI: {}", url);

            let response: EmbeddingResponse = self.send_request(&url, &request).await?;

            // Sort by index to ensure correct ordering within batch
            let mut embeddings: Vec<_> = response.data.into_iter().collect();
            embeddings.sort_by_key(|e| e.index);
            all_embeddings.extend(embeddings.into_iter().map(|e| e.embedding));
        }

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
    fn test_provider_creation() {
        let provider = AzureOpenAIProvider::new(
            "https://myresource.openai.azure.com",
            "test-api-key",
            "gpt-4o",
        );

        assert_eq!(LLMProvider::name(&provider), "azure-openai");
        assert_eq!(LLMProvider::model(&provider), "gpt-4o");
        assert_eq!(provider.endpoint, "https://myresource.openai.azure.com");
    }

    #[test]
    fn test_provider_with_options() {
        let provider = AzureOpenAIProvider::new(
            "https://myresource.openai.azure.com/",
            "test-api-key",
            "gpt-4o",
        )
        .with_embedding_deployment("text-embedding-ada-002")
        .with_api_version("2024-06-01")
        .with_max_context_length(128_000)
        .with_embedding_dimension(1536);

        assert_eq!(
            EmbeddingProvider::model(&provider),
            "text-embedding-ada-002"
        );
        assert_eq!(provider.api_version, "2024-06-01");
        assert_eq!(provider.max_context_length(), 128_000);
        assert_eq!(provider.dimension(), 1536);
        // Trailing slash should be stripped
        assert_eq!(provider.endpoint, "https://myresource.openai.azure.com");
    }

    #[test]
    fn test_build_url() {
        let provider = AzureOpenAIProvider::new(
            "https://myresource.openai.azure.com",
            "test-api-key",
            "gpt-4o",
        );

        let url = provider.build_url("gpt-4o", "chat/completions");
        assert_eq!(
            url,
            "https://myresource.openai.azure.com/openai/deployments/gpt-4o/chat/completions?api-version=2024-10-21"
        );
    }

    #[test]
    fn test_message_conversion() {
        let messages = vec![
            ChatMessage::system("You are helpful"),
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there!"),
        ];

        let converted = AzureOpenAIProvider::convert_messages(&messages);

        assert_eq!(converted.len(), 3);
        assert_eq!(converted[0].role, "system");
        assert_eq!(converted[1].role, "user");
        assert_eq!(converted[2].role, "assistant");
    }
}
