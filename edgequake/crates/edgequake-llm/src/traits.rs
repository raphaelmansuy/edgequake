//! LLM provider traits for text completion and embedding.
//!
//! # Implements
//!
//! @implements FEAT0006 (Vector Embedding Generation via EmbeddingProvider trait)
//! @implements FEAT0017 (Multi-Provider LLM Support via LLMProvider trait)
//! @implements FEAT0018 (Embedding Provider Abstraction - this file allocates both trait definitions)
//!
//! # Enforces
//!
//! - **BR0303**: Token usage tracked in [`LLMResponse`]
//! - **BR0010**: Embedding dimension validated by providers
//!
//! # WHY: Trait-Based Provider Abstraction
//!
//! Using traits instead of concrete types enables:
//! - **Testing**: MockProvider for unit tests (no API calls)
//! - **Flexibility**: Swap providers without code changes
//! - **Cost control**: Route to different providers based on request type
//! - **Resilience**: Fallback providers when primary is unavailable
//!
//! # Key Traits
//!
//! - [`LLMProvider`]: Text completion (chat, extraction prompts)
//! - [`EmbeddingProvider`]: Vector embedding generation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::Result;

use futures::stream::BoxStream;

/// Response from an LLM completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    /// The generated text content.
    pub content: String,

    /// Number of tokens in the prompt.
    pub prompt_tokens: usize,

    /// Number of tokens in the completion.
    pub completion_tokens: usize,

    /// Total tokens used.
    pub total_tokens: usize,

    /// Model used for the request.
    pub model: String,

    /// Finish reason (e.g., "stop", "length", "content_filter").
    pub finish_reason: Option<String>,

    /// Additional metadata from the provider.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl LLMResponse {
    /// Create a new LLM response.
    pub fn new(content: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            model: model.into(),
            finish_reason: None,
            metadata: HashMap::new(),
        }
    }

    /// Set token usage.
    pub fn with_usage(mut self, prompt: usize, completion: usize) -> Self {
        self.prompt_tokens = prompt;
        self.completion_tokens = completion;
        self.total_tokens = prompt + completion;
        self
    }

    /// Set finish reason.
    pub fn with_finish_reason(mut self, reason: impl Into<String>) -> Self {
        self.finish_reason = Some(reason.into());
        self
    }
}

/// Options for LLM completion requests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompletionOptions {
    /// Maximum number of tokens to generate.
    pub max_tokens: Option<usize>,

    /// Temperature for sampling (0.0 = deterministic, 1.0 = creative).
    pub temperature: Option<f32>,

    /// Top-p (nucleus) sampling.
    pub top_p: Option<f32>,

    /// Stop sequences.
    pub stop: Option<Vec<String>>,

    /// Frequency penalty.
    pub frequency_penalty: Option<f32>,

    /// Presence penalty.
    pub presence_penalty: Option<f32>,

    /// Response format (e.g., "json").
    pub response_format: Option<String>,

    /// System prompt to prepend.
    pub system_prompt: Option<String>,
}

impl CompletionOptions {
    /// Create options with a specific temperature.
    pub fn with_temperature(temperature: f32) -> Self {
        Self {
            temperature: Some(temperature),
            ..Default::default()
        }
    }

    /// Create options for JSON output.
    pub fn json_mode() -> Self {
        Self {
            response_format: Some("json_object".to_string()),
            ..Default::default()
        }
    }
}

/// Trait for LLM providers that can generate text completions.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get the name of this provider.
    fn name(&self) -> &str;

    /// Get the current model.
    fn model(&self) -> &str;

    /// Get the maximum context length for the model.
    fn max_context_length(&self) -> usize;

    /// Generate a completion for the given prompt.
    async fn complete(&self, prompt: &str) -> Result<LLMResponse>;

    /// Generate a completion with custom options.
    async fn complete_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> Result<LLMResponse>;

    /// Generate a chat completion with messages.
    async fn chat(
        &self,
        messages: &[ChatMessage],
        options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse>;

    /// Generate a streaming completion.
    async fn stream(&self, _prompt: &str) -> Result<BoxStream<'static, Result<String>>> {
        Err(crate::error::LlmError::NotSupported(
            "Streaming not supported".to_string(),
        ))
    }

    /// Generate a streaming completion with options.
    ///
    /// This method supports stop sequences and other completion options
    /// during streaming. Providers that don't override this will delegate
    /// to the basic `stream()` method (options are ignored).
    ///
    /// @implements SPEC-032: Ollama stop token handling (OODA 63)
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt to complete
    /// * `options` - Completion options including stop sequences
    ///
    /// # Default Implementation
    ///
    /// Delegates to `stream()` - stop tokens are ignored unless provider overrides.
    async fn stream_with_options(
        &self,
        prompt: &str,
        _options: &CompletionOptions,
    ) -> Result<BoxStream<'static, Result<String>>> {
        // Default: delegate to stream() - providers should override to use options
        self.stream(prompt).await
    }

    /// Attempt streaming with automatic fallback to non-streaming.
    ///
    /// This method first attempts to use streaming. If streaming fails
    /// (either because the provider doesn't support it or due to an error),
    /// it falls back to the non-streaming `complete()` method.
    ///
    /// @implements SPEC-032: LM Studio streaming fallback (Focus 8)
    ///
    /// # Returns
    ///
    /// - `StreamOrComplete::Stream(stream)` if streaming succeeds
    /// - `StreamOrComplete::Complete(response)` if fallback was used
    ///
    /// # Examples
    ///
    /// ```ignore
    /// match provider.stream_with_fallback(prompt).await? {
    ///     StreamOrComplete::Stream(stream) => {
    ///         // Handle streaming response
    ///     }
    ///     StreamOrComplete::Complete(response) => {
    ///         // Handle complete response (streaming wasn't available)
    ///     }
    /// }
    /// ```
    async fn stream_with_fallback(&self, prompt: &str) -> Result<StreamOrComplete> {
        // First check if streaming is supported at all
        if !self.supports_streaming() {
            // Fall back to non-streaming
            let response = self.complete(prompt).await?;
            return Ok(StreamOrComplete::Complete(response));
        }

        // Try streaming
        match self.stream(prompt).await {
            Ok(stream) => Ok(StreamOrComplete::Stream(stream)),
            Err(crate::error::LlmError::NotSupported(_)) => {
                // Streaming not supported, fall back
                let response = self.complete(prompt).await?;
                Ok(StreamOrComplete::Complete(response))
            }
            Err(e) => {
                // Check if it's a streaming-specific error we should fall back from
                let error_msg = e.to_string().to_lowercase();
                if error_msg.contains("stream")
                    || error_msg.contains("sse")
                    || error_msg.contains("not supported")
                {
                    // Fall back to non-streaming
                    tracing::warn!(
                        provider = self.name(),
                        error = %e,
                        "Streaming failed, falling back to non-streaming"
                    );
                    let response = self.complete(prompt).await?;
                    Ok(StreamOrComplete::Complete(response))
                } else {
                    // Propagate other errors
                    Err(e)
                }
            }
        }
    }

    /// Check if the model supports streaming.
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Check if the model supports JSON mode.
    fn supports_json_mode(&self) -> bool {
        false
    }
}

/// A message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role of the message sender.
    pub role: ChatRole,

    /// Content of the message.
    pub content: String,

    /// Optional name for the message sender.
    pub name: Option<String>,
}

impl ChatMessage {
    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: content.into(),
            name: None,
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
            name: None,
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
            name: None,
        }
    }
}

/// Role of a chat message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    /// System message for setting context.
    System,
    /// User input message.
    User,
    /// Assistant response message.
    Assistant,
    /// Function/tool result message.
    Function,
}

/// Trait for providers that can generate text embeddings.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Get the name of this provider.
    fn name(&self) -> &str;

    /// Get the embedding model.
    fn model(&self) -> &str;

    /// Get the dimension of the embeddings.
    fn dimension(&self) -> usize;

    /// Get the maximum number of tokens per input.
    fn max_tokens(&self) -> usize;

    /// Generate embeddings for a batch of texts.
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Generate embedding for a single text.
    async fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let results = self.embed(&[text.to_string()]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| crate::error::LlmError::Unknown("Empty embedding result".to_string()))
    }
}

/// Result type for streaming with fallback.
///
/// When a provider attempts streaming but it fails or is not supported,
/// this enum allows returning either a stream or a complete response.
///
/// @implements SPEC-032: LM Studio streaming fallback (Focus 8)
pub enum StreamOrComplete {
    /// Streaming is available, use the stream
    Stream(BoxStream<'static, Result<String>>),
    /// Streaming not available, use complete response
    Complete(LLMResponse),
}

impl std::fmt::Debug for StreamOrComplete {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamOrComplete::Stream(_) => write!(f, "StreamOrComplete::Stream(...)"),
            StreamOrComplete::Complete(response) => {
                write!(f, "StreamOrComplete::Complete({:?})", response)
            }
        }
    }
}

impl StreamOrComplete {
    /// Returns true if this is a stream
    pub fn is_stream(&self) -> bool {
        matches!(self, StreamOrComplete::Stream(_))
    }

    /// Returns true if this is a complete response
    pub fn is_complete(&self) -> bool {
        matches!(self, StreamOrComplete::Complete(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_response_builder() {
        let response = LLMResponse::new("Hello, world!", "gpt-4")
            .with_usage(10, 5)
            .with_finish_reason("stop");

        assert_eq!(response.content, "Hello, world!");
        assert_eq!(response.model, "gpt-4");
        assert_eq!(response.prompt_tokens, 10);
        assert_eq!(response.completion_tokens, 5);
        assert_eq!(response.total_tokens, 15);
        assert_eq!(response.finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_chat_message_constructors() {
        let system = ChatMessage::system("You are helpful");
        assert_eq!(system.role, ChatRole::System);

        let user = ChatMessage::user("Hello");
        assert_eq!(user.role, ChatRole::User);

        let assistant = ChatMessage::assistant("Hi there!");
        assert_eq!(assistant.role, ChatRole::Assistant);
    }
}
