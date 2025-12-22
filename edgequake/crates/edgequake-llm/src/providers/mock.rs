//! Mock LLM and Embedding provider for testing.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::Result;
use crate::traits::{EmbeddingProvider, LLMProvider, LLMResponse};

/// Mock LLM provider for testing.
#[derive(Debug, Clone)]
pub struct MockProvider {
    responses: Arc<Mutex<Vec<String>>>,
    embeddings: Arc<Mutex<Vec<Vec<f32>>>>,
}

impl MockProvider {
    /// Create a new mock provider with default responses.
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
            embeddings: Arc::new(Mutex::new(vec![
                vec![0.1; 1536], // Default 1536-dim embedding
            ])),
        }
    }

    /// Add a response to the queue.
    pub async fn add_response(&self, response: impl Into<String>) {
        self.responses.lock().await.push(response.into());
    }

    /// Add an embedding to the queue.
    pub async fn add_embedding(&self, embedding: Vec<f32>) {
        self.embeddings.lock().await.push(embedding);
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LLMProvider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn model(&self) -> &str {
        "mock-model"
    }

    fn max_context_length(&self) -> usize {
        4096
    }

    async fn complete(&self, _prompt: &str) -> Result<LLMResponse> {
        let mut responses = self.responses.lock().await;
        let content = if responses.is_empty() {
            "Mock response".to_string()
        } else {
            responses.remove(0)
        };

        Ok(LLMResponse::new(content, "mock-model"))
    }

    async fn complete_with_options(
        &self,
        prompt: &str,
        _options: &crate::traits::CompletionOptions,
    ) -> Result<LLMResponse> {
        self.complete(prompt).await
    }

    async fn chat(
        &self,
        _messages: &[crate::traits::ChatMessage],
        _options: Option<&crate::traits::CompletionOptions>,
    ) -> Result<LLMResponse> {
        self.complete("").await
    }

    async fn stream(&self, prompt: &str) -> Result<futures::stream::BoxStream<'static, Result<String>>> {
        use futures::StreamExt;
        let response = self.complete(prompt).await?;
        let stream = futures::stream::iter(vec![Ok(response.content)]);
        Ok(stream.boxed())
    }
}

#[async_trait]
impl EmbeddingProvider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn model(&self) -> &str {
        "mock-embedding"
    }

    fn dimension(&self) -> usize {
        1536
    }

    fn max_tokens(&self) -> usize {
        512
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for _ in texts {
            let mut embeddings = self.embeddings.lock().await;
            let emb = if embeddings.is_empty() {
                vec![0.1; 1536]
            } else {
                embeddings.remove(0)
            };
            results.push(emb);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider() {
        let provider = MockProvider::new();
        provider.add_response("This is a mock response.").await;
        
        // Test LLM
        let response = provider.complete("test").await.unwrap();
        assert_eq!(response.content, "This is a mock response.");
        
        // Test embedding
        let embedding = provider.embed_one("test").await.unwrap();
        assert_eq!(embedding.len(), 1536);
    }

    #[tokio::test]
    async fn test_custom_responses() {
        let provider = MockProvider::new();
        provider.add_response("Custom response").await;
        
        let response = provider.complete("test").await.unwrap();
        assert_eq!(response.content, "Custom response");
    }
}
