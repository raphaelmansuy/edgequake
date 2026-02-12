//! OpenAI provider implementation.
//!
//! Supports OpenAI and OpenAI-compatible APIs (Ollama, LM Studio, etc.)

use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    types::embeddings::{CreateEmbeddingRequestArgs, EmbeddingInput},
    Client,
};
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, info};

use crate::error::{LlmError, Result};
use crate::traits::{
    ChatMessage, ChatRole, CompletionOptions, EmbeddingProvider, LLMProvider, LLMResponse,
};

/// OpenAI provider for text completion and embeddings.
pub struct OpenAIProvider {
    client: Client<OpenAIConfig>,
    model: String,
    embedding_model: String,
    max_context_length: usize,
    embedding_dimension: usize,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        let config = OpenAIConfig::new().with_api_key(api_key);
        Self::with_config(config)
    }

    /// Create a provider with custom configuration.
    pub fn with_config(config: OpenAIConfig) -> Self {
        Self {
            client: Client::with_config(config),
            model: "gpt-4.1-nano".to_string(), // ✅ Default to gpt-4.1-nano (cost-effective, high-quality)
            embedding_model: "text-embedding-3-small".to_string(),
            max_context_length: 128000,
            embedding_dimension: 1536,
        }
    }

    /// Create a provider for an OpenAI-compatible API.
    pub fn compatible(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_url);
        Self::with_config(config)
    }

    /// Set the completion model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self.max_context_length = Self::context_length_for_model(&self.model);
        self
    }

    /// Set the embedding model.
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.embedding_model = model.into();
        self.embedding_dimension = Self::dimension_for_model(&self.embedding_model);
        self
    }

    /// Set the embedding model with a specific dimension.
    ///
    /// Use this to override auto-detection when workspace configuration specifies a dimension.
    /// @implements SPEC-032: Respect workspace embedding dimension configuration
    pub fn with_embedding_model_and_dimension(
        mut self,
        model: impl Into<String>,
        dimension: usize,
    ) -> Self {
        self.embedding_model = model.into();
        self.embedding_dimension = dimension;
        self
    }

    /// Get the context length for a model.
    fn context_length_for_model(model: &str) -> usize {
        match model {
            m if m.contains("gpt-4.1") => 128000, // ✅ gpt-4.1-nano/mini/full support
            m if m.contains("gpt-4o") => 128000,
            m if m.contains("gpt-4-turbo") => 128000,
            m if m.contains("gpt-4-32k") => 32768,
            m if m.contains("gpt-4") => 8192,
            m if m.contains("gpt-3.5-turbo-16k") => 16384,
            m if m.contains("gpt-3.5") => 4096,
            m if m.contains("o1") || m.contains("o3") => 200000,
            _ => 8192, // Conservative default
        }
    }

    /// Get the embedding dimension for a model.
    fn dimension_for_model(model: &str) -> usize {
        match model {
            m if m.contains("text-embedding-3-large") => 3072,
            m if m.contains("text-embedding-3-small") => 1536,
            m if m.contains("text-embedding-ada") => 1536,
            _ => 1536, // Default
        }
    }

    /// Convert chat messages to OpenAI format.
    fn convert_messages(messages: &[ChatMessage]) -> Result<Vec<ChatCompletionRequestMessage>> {
        messages
            .iter()
            .map(|msg| {
                match msg.role {
                    ChatRole::System => ChatCompletionRequestSystemMessageArgs::default()
                        .content(msg.content.as_str())
                        .build()
                        .map(Into::into)
                        .map_err(|e| LlmError::InvalidRequest(e.to_string())),
                    ChatRole::User => ChatCompletionRequestUserMessageArgs::default()
                        .content(msg.content.as_str())
                        .build()
                        .map(Into::into)
                        .map_err(|e| LlmError::InvalidRequest(e.to_string())),
                    ChatRole::Assistant => ChatCompletionRequestAssistantMessageArgs::default()
                        .content(msg.content.as_str())
                        .build()
                        .map(Into::into)
                        .map_err(|e| LlmError::InvalidRequest(e.to_string())),
                    ChatRole::Function => {
                        // Function messages are handled as user messages in simplified API
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(msg.content.as_str())
                            .build()
                            .map(Into::into)
                            .map_err(|e| LlmError::InvalidRequest(e.to_string()))
                    }
                }
            })
            .collect()
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
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
        let openai_messages = Self::convert_messages(messages)?;
        let options = options.cloned().unwrap_or_default();

        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder.model(&self.model).messages(openai_messages);

        if let Some(max_tokens) = options.max_tokens {
            request_builder.max_completion_tokens(max_tokens as u32);
        }

        if let Some(temp) = options.temperature {
            request_builder.temperature(temp);
        }

        if let Some(top_p) = options.top_p {
            request_builder.top_p(top_p);
        }

        if let Some(stop) = options.stop {
            request_builder.stop(stop);
        }

        if let Some(freq_penalty) = options.frequency_penalty {
            request_builder.frequency_penalty(freq_penalty);
        }

        if let Some(pres_penalty) = options.presence_penalty {
            request_builder.presence_penalty(pres_penalty);
        }

        let request = request_builder
            .build()
            .map_err(|e| LlmError::InvalidRequest(e.to_string()))?;

        let response = self.client.chat().create(request).await?;

        // Debug logging for token tracking
        debug!(
            "OpenAI response - usage: {:?}, model: {}",
            response.usage, response.model
        );

        let choice = response
            .choices
            .first()
            .ok_or_else(|| LlmError::ApiError("No choices in response".to_string()))?;

        let content = choice.message.content.clone().unwrap_or_default();

        let usage = response.usage.clone().unwrap_or_default();

        // Log extracted token counts
        debug!(
            "OpenAI token usage - prompt: {}, completion: {}, total: {}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );

        let mut metadata = HashMap::new();
        metadata.insert("response_id".to_string(), serde_json::json!(response.id));

        Ok(LLMResponse {
            content,
            prompt_tokens: usage.prompt_tokens as usize,
            completion_tokens: usage.completion_tokens as usize,
            total_tokens: usage.total_tokens as usize,
            model: response.model,
            finish_reason: choice.finish_reason.map(|r| format!("{:?}", r)),
            metadata,
        })
    }

    async fn stream(
        &self,
        prompt: &str,
    ) -> Result<futures::stream::BoxStream<'static, Result<String>>> {
        // Delegate to stream_with_options with default options
        self.stream_with_options(prompt, &CompletionOptions::default())
            .await
    }

    /// Stream with options including stop sequences.
    ///
    /// @implements SPEC-032: OpenAI stop token handling (OODA 63)
    async fn stream_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> Result<futures::stream::BoxStream<'static, Result<String>>> {
        use futures::StreamExt;

        let request = ChatCompletionRequestUserMessageArgs::default()
            .content(prompt)
            .build()
            .map(Into::into)
            .map_err(|e| LlmError::InvalidRequest(e.to_string()))?;

        let mut builder = CreateChatCompletionRequestArgs::default();
        builder
            .model(&self.model)
            .messages(vec![request])
            .stream(true);

        // Apply optional parameters
        if let Some(temp) = options.temperature {
            builder.temperature(temp);
        }
        if let Some(max_tokens) = options.max_tokens {
            builder.max_tokens(max_tokens as u32);
        }
        if let Some(ref stop) = options.stop {
            builder.stop(stop.clone());
        }

        let request = builder
            .build()
            .map_err(|e| LlmError::InvalidRequest(e.to_string()))?;

        let stream = self.client.chat().create_stream(request).await?;

        let mapped_stream = stream.map(|res| match res {
            Ok(response) => {
                let content = response
                    .choices
                    .first()
                    .and_then(|c| c.delta.content.clone())
                    .unwrap_or_default();
                Ok(content)
            }
            Err(e) => Err(LlmError::ApiError(e.to_string())),
        });

        Ok(mapped_stream.boxed())
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_json_mode(&self) -> bool {
        self.model.contains("gpt-4") || self.model.contains("gpt-3.5-turbo")
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    /// Returns the embedding model name (not completion model).
    ///
    /// Note: `model` field refers to completion, `embedding_model` is for embeddings.
    #[allow(clippy::misnamed_getters)]
    fn model(&self) -> &str {
        &self.embedding_model
    }

    fn dimension(&self) -> usize {
        self.embedding_dimension
    }

    fn max_tokens(&self) -> usize {
        8191 // OpenAI embedding models support 8191 tokens
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // WHY: OpenAI embedding API enforces a hard limit of 2048 inputs per request.
        // Large documents can produce thousands of entities (e.g. 8932 for a 268KB doc),
        // which causes "$.input is invalid" when sent as a single batch.
        // We split into sub-batches of MAX_EMBEDDING_BATCH_SIZE to stay within limits.
        const MAX_EMBEDDING_BATCH_SIZE: usize = 2048;

        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Filter out empty/whitespace-only strings - OpenAI API rejects these
        // Track which indices have valid text for mapping results back
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

        // Extract just the valid text strings for the API call
        let api_texts: Vec<String> = valid_texts
            .iter()
            .map(|(_, text)| (*text).clone())
            .collect();

        let total_texts = api_texts.len();
        let num_batches = total_texts.div_ceil(MAX_EMBEDDING_BATCH_SIZE);

        if num_batches > 1 {
            info!(
                "Splitting {} texts into {} batches of max {} for OpenAI embedding API",
                total_texts, num_batches, MAX_EMBEDDING_BATCH_SIZE
            );
        }

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

            let input = EmbeddingInput::StringArray(batch.to_vec());

            let request = CreateEmbeddingRequestArgs::default()
                .model(&self.embedding_model)
                .input(input)
                .build()
                .map_err(|e| LlmError::InvalidRequest(e.to_string()))?;

            let response = self.client.embeddings().create(request).await?;

            // Sort by index to ensure correct ordering within batch
            let mut batch_data = response.data;
            batch_data.sort_by_key(|e| e.index);

            all_embeddings.extend(batch_data.into_iter().map(|e| e.embedding));
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
    fn test_context_length_detection() {
        assert_eq!(OpenAIProvider::context_length_for_model("gpt-4o"), 128000);
        assert_eq!(OpenAIProvider::context_length_for_model("gpt-4"), 8192);
        assert_eq!(
            OpenAIProvider::context_length_for_model("gpt-3.5-turbo"),
            4096
        );
    }

    #[test]
    fn test_embedding_dimension_detection() {
        assert_eq!(
            OpenAIProvider::dimension_for_model("text-embedding-3-large"),
            3072
        );
        assert_eq!(
            OpenAIProvider::dimension_for_model("text-embedding-3-small"),
            1536
        );
    }

    #[test]
    fn test_provider_builder() {
        let provider = OpenAIProvider::new("test-key")
            .with_model("gpt-4")
            .with_embedding_model("text-embedding-3-large");

        assert_eq!(LLMProvider::model(&provider), "gpt-4");
        assert_eq!(provider.dimension(), 3072);
    }

    #[test]
    fn test_message_conversion() {
        let messages = vec![
            ChatMessage::system("You are helpful"),
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there!"),
        ];

        let converted = OpenAIProvider::convert_messages(&messages).unwrap();
        assert_eq!(converted.len(), 3);
    }
}
