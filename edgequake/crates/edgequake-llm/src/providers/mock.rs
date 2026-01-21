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

/// Mock LLM provider that does NOT support streaming.
///
/// Used to test the `stream_with_fallback()` fallback path.
#[derive(Debug, Clone)]
pub struct NonStreamingMockProvider {
    response: String,
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

    async fn stream(
        &self,
        prompt: &str,
    ) -> Result<futures::stream::BoxStream<'static, Result<String>>> {
        use futures::StreamExt;
        let response = self.complete(prompt).await?;
        let stream = futures::stream::iter(vec![Ok(response.content)]);
        Ok(stream.boxed())
    }

    fn supports_streaming(&self) -> bool {
        true
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

// ============================================================================
// NonStreamingMockProvider - for testing streaming fallback
// ============================================================================

impl NonStreamingMockProvider {
    /// Create a new non-streaming mock provider with a fixed response.
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
        }
    }
}

#[async_trait]
impl LLMProvider for NonStreamingMockProvider {
    fn name(&self) -> &str {
        "non-streaming-mock"
    }

    fn model(&self) -> &str {
        "non-streaming-mock-model"
    }

    fn max_context_length(&self) -> usize {
        4096
    }

    async fn complete(&self, _prompt: &str) -> Result<LLMResponse> {
        Ok(LLMResponse::new(
            self.response.clone(),
            "non-streaming-mock-model",
        ))
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

    // Note: We don't implement stream() - it uses the default NotSupported error

    fn supports_streaming(&self) -> bool {
        false // Explicitly does not support streaming
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

    #[tokio::test]
    async fn test_stream_with_fallback_uses_stream_when_supported() {
        use crate::traits::StreamOrComplete;
        use futures::StreamExt;

        let provider = MockProvider::new();
        provider.add_response("Streamed response").await;

        // MockProvider supports streaming, so we should get a stream
        let result = provider.stream_with_fallback("test").await.unwrap();

        match result {
            StreamOrComplete::Stream(mut stream) => {
                let first_chunk = stream.next().await;
                assert!(first_chunk.is_some());
                let content = first_chunk.unwrap().unwrap();
                assert_eq!(content, "Streamed response");
            }
            StreamOrComplete::Complete(_) => {
                panic!("Expected stream but got complete response");
            }
        }
    }

    #[tokio::test]
    async fn test_stream_with_fallback_falls_back_when_not_supported() {
        use crate::traits::StreamOrComplete;

        let provider = NonStreamingMockProvider::new("Fallback response");

        // NonStreamingMockProvider does NOT support streaming
        assert!(!provider.supports_streaming());

        // stream_with_fallback should use complete() instead
        let result = provider.stream_with_fallback("test").await.unwrap();

        match result {
            StreamOrComplete::Complete(response) => {
                assert_eq!(response.content, "Fallback response");
                assert_eq!(response.model, "non-streaming-mock-model");
            }
            StreamOrComplete::Stream(_) => {
                panic!("Expected complete response but got stream");
            }
        }
    }

    #[tokio::test]
    async fn test_non_streaming_mock_provider() {
        let provider = NonStreamingMockProvider::new("Fixed response");

        assert_eq!(provider.name(), "non-streaming-mock");
        assert!(!provider.supports_streaming());

        let response = provider.complete("test").await.unwrap();
        assert_eq!(response.content, "Fixed response");
    }
}
