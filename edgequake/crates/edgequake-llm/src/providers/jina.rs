//! Jina AI Embeddings provider implementation.
//!
//! This module provides integration with Jina AI's embedding API.
//!
//! # Environment Variables
//!
//! - `JINA_API_KEY`: Your Jina AI API key (required)
//!
//! # Example
//!
//! ```rust,ignore
//! use edgequake_llm::JinaProvider;
//!
//! let provider = JinaProvider::from_env()?;
//! let embeddings = provider.embed(vec!["Hello, world!".to_string()]).await?;
//! ```

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::LlmError;
use crate::traits::EmbeddingProvider;

/// Default Jina embedding model
const DEFAULT_JINA_EMBEDDING_MODEL: &str = "jina-embeddings-v3";

/// Default Jina API base URL
const DEFAULT_JINA_BASE_URL: &str = "https://api.jina.ai";

/// Jina AI Embeddings provider.
///
/// Supports Jina's embedding models including:
/// - `jina-embeddings-v4`: Multimodal multilingual (3.8B parameters)
/// - `jina-embeddings-v3`: Multilingual with task LoRA
/// - `jina-embeddings-v2-*`: Various specialized models
/// - `jina-clip-v2`: CLIP-style multimodal embeddings
#[derive(Debug, Clone)]
pub struct JinaProvider {
    client: Client,
    api_key: String,
    base_url: String,
    embedding_model: String,
    embedding_dimension: usize,
    /// Task type for embeddings (retrieval.query, retrieval.passage, etc.)
    task: Option<String>,
    /// Whether to normalize embeddings
    normalized: bool,
}

/// Builder for JinaProvider
#[derive(Debug, Clone)]
pub struct JinaProviderBuilder {
    api_key: Option<String>,
    base_url: String,
    embedding_model: String,
    embedding_dimension: usize,
    task: Option<String>,
    normalized: bool,
}

impl Default for JinaProviderBuilder {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: DEFAULT_JINA_BASE_URL.to_string(),
            embedding_model: DEFAULT_JINA_EMBEDDING_MODEL.to_string(),
            embedding_dimension: 1024, // jina-embeddings-v3 default
            task: None,
            normalized: true,
        }
    }
}

impl JinaProviderBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the API key
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set the base URL
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Set the embedding model
    pub fn embedding_model(mut self, model: impl Into<String>) -> Self {
        self.embedding_model = model.into();
        self
    }

    /// Set the embedding dimension
    pub fn embedding_dimension(mut self, dimension: usize) -> Self {
        self.embedding_dimension = dimension;
        self
    }

    /// Set the task type for embeddings
    ///
    /// Supported tasks:
    /// - `retrieval.query`: For query embeddings in search
    /// - `retrieval.passage`: For document/passage embeddings
    /// - `separation`: For clustering/separation tasks
    /// - `classification`: For classification tasks
    /// - `text-matching`: For semantic similarity
    pub fn task(mut self, task: impl Into<String>) -> Self {
        self.task = Some(task.into());
        self
    }

    /// Set whether to normalize embeddings (L2 normalization)
    pub fn normalized(mut self, normalized: bool) -> Self {
        self.normalized = normalized;
        self
    }

    /// Build the JinaProvider
    pub fn build(self) -> Result<JinaProvider, LlmError> {
        let api_key = self
            .api_key
            .ok_or_else(|| LlmError::ConfigError("JINA_API_KEY is required".to_string()))?;

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        Ok(JinaProvider {
            client,
            api_key,
            base_url: self.base_url,
            embedding_model: self.embedding_model,
            embedding_dimension: self.embedding_dimension,
            task: self.task,
            normalized: self.normalized,
        })
    }
}

impl JinaProvider {
    /// Create a new JinaProvider from environment variables.
    ///
    /// Required environment variables:
    /// - `JINA_API_KEY`: Your Jina AI API key
    ///
    /// Optional environment variables:
    /// - `JINA_BASE_URL`: Custom API base URL
    /// - `JINA_EMBEDDING_MODEL`: Model to use (default: jina-embeddings-v3)
    pub fn from_env() -> Result<Self, LlmError> {
        let api_key = std::env::var("JINA_API_KEY").map_err(|_| {
            LlmError::ConfigError("JINA_API_KEY environment variable is required".to_string())
        })?;

        let base_url =
            std::env::var("JINA_BASE_URL").unwrap_or_else(|_| DEFAULT_JINA_BASE_URL.to_string());

        let embedding_model = std::env::var("JINA_EMBEDDING_MODEL")
            .unwrap_or_else(|_| DEFAULT_JINA_EMBEDDING_MODEL.to_string());

        // Get dimension based on model
        let embedding_dimension = get_model_dimension(&embedding_model);

        JinaProviderBuilder::new()
            .api_key(api_key)
            .base_url(base_url)
            .embedding_model(embedding_model)
            .embedding_dimension(embedding_dimension)
            .build()
    }

    /// Create a new builder for JinaProvider
    pub fn builder() -> JinaProviderBuilder {
        JinaProviderBuilder::new()
    }
}

/// Get the default dimension for a Jina model
fn get_model_dimension(model: &str) -> usize {
    match model {
        "jina-embeddings-v4" => 1024,
        "jina-embeddings-v3" => 1024,
        "jina-embeddings-v2-base-en" => 768,
        "jina-embeddings-v2-small-en" => 512,
        "jina-embeddings-v2-base-de" => 768,
        "jina-embeddings-v2-base-zh" => 768,
        "jina-embeddings-v2-base-code" => 768,
        "jina-clip-v2" => 1024,
        "jina-clip-v1" => 768,
        _ => 1024, // Default
    }
}

// Request/Response structures for Jina API

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    normalized: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embedding_type: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    model: String,
    data: Vec<EmbeddingData>,
    usage: EmbeddingUsage,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    index: usize,
    embedding: Vec<f32>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct EmbeddingUsage {
    total_tokens: u32,
    #[serde(default)]
    prompt_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct JinaError {
    detail: String,
}

#[async_trait]
impl EmbeddingProvider for JinaProvider {
    fn name(&self) -> &str {
        "jina"
    }

    fn model(&self) -> &str {
        &self.embedding_model
    }

    fn dimension(&self) -> usize {
        self.embedding_dimension
    }

    fn max_tokens(&self) -> usize {
        // jina-embeddings-v3 supports 8192 tokens
        8192
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, LlmError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        debug!(
            "Jina embedding request: {} texts with model {}",
            texts.len(),
            self.embedding_model
        );

        let url = format!("{}/v1/embeddings", self.base_url);

        let request = EmbeddingRequest {
            model: self.embedding_model.clone(),
            input: texts.to_vec(),
            task: self.task.clone(),
            normalized: self.normalized,
            dimensions: None,
            embedding_type: Some("float".to_string()),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            if let Ok(error) = serde_json::from_str::<JinaError>(&error_text) {
                return Err(LlmError::ApiError(format!(
                    "Jina API error ({}): {}",
                    status, error.detail
                )));
            }
            return Err(LlmError::ApiError(format!(
                "Jina API error ({}): {}",
                status, error_text
            )));
        }

        let response: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ApiError(format!("Failed to parse response: {}", e)))?;

        // Sort by index and extract embeddings
        let mut embeddings: Vec<_> = response
            .data
            .into_iter()
            .map(|d| (d.index, d.embedding))
            .collect();
        embeddings.sort_by_key(|(i, _)| *i);

        let embeddings: Vec<Vec<f32>> = embeddings.into_iter().map(|(_, e)| e).collect();

        debug!(
            "Jina embedding response: {} embeddings of dimension {}",
            embeddings.len(),
            embeddings.first().map(|e| e.len()).unwrap_or(0)
        );

        Ok(embeddings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creation() {
        let result = JinaProviderBuilder::new()
            .api_key("test-key")
            .embedding_model("jina-embeddings-v3")
            .embedding_dimension(1024)
            .normalized(true)
            .build();

        assert!(result.is_ok());
        let provider = result.unwrap();
        assert_eq!(EmbeddingProvider::name(&provider), "jina");
        assert_eq!(EmbeddingProvider::model(&provider), "jina-embeddings-v3");
        assert_eq!(EmbeddingProvider::dimension(&provider), 1024);
    }

    #[test]
    fn test_builder_with_task() {
        let provider = JinaProviderBuilder::new()
            .api_key("test-key")
            .task("retrieval.query")
            .build()
            .unwrap();

        assert_eq!(provider.task, Some("retrieval.query".to_string()));
    }

    #[test]
    fn test_builder_missing_api_key() {
        let result = JinaProviderBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_model_dimensions() {
        assert_eq!(get_model_dimension("jina-embeddings-v4"), 1024);
        assert_eq!(get_model_dimension("jina-embeddings-v3"), 1024);
        assert_eq!(get_model_dimension("jina-embeddings-v2-base-en"), 768);
        assert_eq!(get_model_dimension("jina-embeddings-v2-small-en"), 512);
        assert_eq!(get_model_dimension("jina-clip-v2"), 1024);
        assert_eq!(get_model_dimension("unknown-model"), 1024);
    }

    #[tokio::test]
    async fn test_embed_empty_input() {
        let provider = JinaProviderBuilder::new()
            .api_key("test-key")
            .build()
            .unwrap();

        let result = provider.embed(&[]).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
