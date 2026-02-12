//! Gemini LLM provider implementation.
//!
//! Supports both Google AI Gemini API and VertexAI endpoints.
//!
//! # Environment Variables
//! - `GEMINI_API_KEY`: API key for Google AI Gemini API
//! - `GOOGLE_APPLICATION_CREDENTIALS`: Path to service account JSON for VertexAI
//! - `GOOGLE_CLOUD_PROJECT`: GCP project ID for VertexAI
//! - `GOOGLE_CLOUD_REGION`: GCP region for VertexAI (default: us-central1)

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

/// Gemini API endpoints
const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Default models
const DEFAULT_GEMINI_MODEL: &str = "gemini-2.0-flash";
const DEFAULT_EMBEDDING_MODEL: &str = "text-embedding-004";

/// Gemini provider configuration
#[derive(Debug, Clone)]
pub enum GeminiEndpoint {
    /// Google AI Gemini API (uses API key)
    GoogleAI { api_key: String },
    /// VertexAI endpoint (uses OAuth2/service account)
    VertexAI {
        project_id: String,
        region: String,
        access_token: String,
    },
}

/// Gemini LLM provider.
#[derive(Debug, Clone)]
pub struct GeminiProvider {
    client: Client,
    endpoint: GeminiEndpoint,
    model: String,
    embedding_model: String,
    max_context_length: usize,
    embedding_dimension: usize,
}

// ============================================================================
// Gemini API Request/Response Types
// ============================================================================

/// Content part for Gemini API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    pub text: String,
}

/// Content for Gemini API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    pub parts: Vec<Part>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// Generation config for Gemini API
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,
}

/// Safety setting for Gemini API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetySetting {
    pub category: String,
    pub threshold: String,
}

/// Request body for generateContent
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateContentRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    safety_settings: Option<Vec<SafetySetting>>,
}

/// Candidate from Gemini response
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: Content,
    finish_reason: Option<String>,
    #[serde(default)]
    safety_ratings: Vec<serde_json::Value>,
}

/// Usage metadata from Gemini response
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct UsageMetadata {
    #[serde(default)]
    prompt_token_count: usize,
    #[serde(default)]
    candidates_token_count: usize,
    #[serde(default)]
    total_token_count: usize,
}

/// Response from generateContent
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerateContentResponse {
    candidates: Option<Vec<Candidate>>,
    usage_metadata: Option<UsageMetadata>,
}

/// Request body for embedContent
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmbedContentRequest {
    content: Content,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_dimensionality: Option<usize>,
}

/// Request body for batchEmbedContents
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BatchEmbedContentsRequest {
    requests: Vec<EmbedContentRequest>,
}

/// Embedding values from Gemini response
#[derive(Debug, Clone, Deserialize)]
struct EmbeddingValues {
    values: Vec<f32>,
}

/// Response from embedContent
#[derive(Debug, Clone, Deserialize)]
struct EmbedContentResponse {
    embedding: EmbeddingValues,
}

/// Response from batchEmbedContents
#[derive(Debug, Clone, Deserialize)]
struct BatchEmbedContentsResponse {
    embeddings: Vec<EmbeddingValues>,
}

/// Error response from Gemini API
#[derive(Debug, Clone, Deserialize)]
struct GeminiErrorResponse {
    error: GeminiError,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct GeminiError {
    code: i32,
    message: String,
    status: String,
}

// ============================================================================
// GeminiProvider Implementation
// ============================================================================

impl GeminiProvider {
    /// Create a new Gemini provider using Google AI API key.
    ///
    /// # Arguments
    /// * `api_key` - Google AI API key (from <https://aistudio.google.com/app/apikey>)
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            endpoint: GeminiEndpoint::GoogleAI {
                api_key: api_key.into(),
            },
            model: DEFAULT_GEMINI_MODEL.to_string(),
            embedding_model: DEFAULT_EMBEDDING_MODEL.to_string(),
            max_context_length: 1_000_000, // Gemini 2.0 supports up to 1M tokens
            embedding_dimension: 768,      // text-embedding-004 default
        }
    }

    /// Create a provider from environment variables.
    ///
    /// Checks for `GEMINI_API_KEY` first, then falls back to VertexAI credentials.
    pub fn from_env() -> Result<Self> {
        // Try Google AI API key first
        if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
            return Ok(Self::new(api_key));
        }

        // Try VertexAI credentials
        if let (Ok(project_id), Ok(access_token)) = (
            std::env::var("GOOGLE_CLOUD_PROJECT"),
            std::env::var("GOOGLE_ACCESS_TOKEN"),
        ) {
            let region =
                std::env::var("GOOGLE_CLOUD_REGION").unwrap_or_else(|_| "us-central1".to_string());
            return Ok(Self::vertex_ai(project_id, region, access_token));
        }

        Err(LlmError::ConfigError(
            "No Gemini credentials found. Set GEMINI_API_KEY or GOOGLE_CLOUD_PROJECT + GOOGLE_ACCESS_TOKEN".to_string()
        ))
    }

    /// Create a VertexAI provider.
    ///
    /// # Arguments
    /// * `project_id` - GCP project ID
    /// * `region` - GCP region (e.g., "us-central1")
    /// * `access_token` - OAuth2 access token
    pub fn vertex_ai(
        project_id: impl Into<String>,
        region: impl Into<String>,
        access_token: impl Into<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            endpoint: GeminiEndpoint::VertexAI {
                project_id: project_id.into(),
                region: region.into(),
                access_token: access_token.into(),
            },
            model: DEFAULT_GEMINI_MODEL.to_string(),
            embedding_model: DEFAULT_EMBEDDING_MODEL.to_string(),
            max_context_length: 1_000_000,
            embedding_dimension: 768,
        }
    }

    /// Set the model to use.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        let model_name = model.into();
        self.max_context_length = Self::context_length_for_model(&model_name);
        self.model = model_name;
        self
    }

    /// Set the embedding model to use.
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        let model_name = model.into();
        self.embedding_dimension = Self::dimension_for_model(&model_name);
        self.embedding_model = model_name;
        self
    }

    /// Get context length for a given model.
    pub fn context_length_for_model(model: &str) -> usize {
        match model {
            m if m.contains("gemini-2.5") => 1_000_000,
            m if m.contains("gemini-2.0") => 1_000_000,
            m if m.contains("gemini-1.5-pro") => 2_000_000,
            m if m.contains("gemini-1.5-flash") => 1_000_000,
            m if m.contains("gemini-1.0") => 32_000,
            _ => 128_000, // Conservative default
        }
    }

    /// Get embedding dimension for a given model.
    pub fn dimension_for_model(model: &str) -> usize {
        match model {
            m if m.contains("text-embedding-004") => 768,
            m if m.contains("text-embedding-005") => 768,
            m if m.contains("embedding-001") => 768,
            m if m.contains("text-multilingual-embedding-002") => 768,
            _ => 768, // Default dimension
        }
    }

    /// Build the URL for a Gemini API endpoint.
    fn build_url(&self, model: &str, action: &str) -> String {
        match &self.endpoint {
            GeminiEndpoint::GoogleAI { api_key } => {
                format!(
                    "{}/models/{}:{}?key={}",
                    GEMINI_API_BASE, model, action, api_key
                )
            }
            GeminiEndpoint::VertexAI {
                project_id, region, ..
            } => {
                format!(
                    "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:{}",
                    region, project_id, region, model, action
                )
            }
        }
    }

    /// Get authorization headers for the request.
    fn auth_headers(&self) -> Vec<(&'static str, String)> {
        match &self.endpoint {
            GeminiEndpoint::GoogleAI { .. } => {
                // API key is in URL for Google AI
                vec![]
            }
            GeminiEndpoint::VertexAI { access_token, .. } => {
                vec![("Authorization", format!("Bearer {}", access_token))]
            }
        }
    }

    /// Convert ChatMessage to Gemini Content format.
    fn convert_messages(messages: &[ChatMessage]) -> (Option<Content>, Vec<Content>) {
        let mut system_instruction = None;
        let mut contents = Vec::new();

        for msg in messages {
            match msg.role {
                ChatRole::System => {
                    // Gemini uses system_instruction field for system prompts
                    system_instruction = Some(Content {
                        parts: vec![Part {
                            text: msg.content.clone(),
                        }],
                        role: None,
                    });
                }
                ChatRole::User => {
                    contents.push(Content {
                        parts: vec![Part {
                            text: msg.content.clone(),
                        }],
                        role: Some("user".to_string()),
                    });
                }
                ChatRole::Assistant => {
                    contents.push(Content {
                        parts: vec![Part {
                            text: msg.content.clone(),
                        }],
                        role: Some("model".to_string()),
                    });
                }
                ChatRole::Function => {
                    // Handle function messages as user messages with context
                    contents.push(Content {
                        parts: vec![Part {
                            text: msg.content.clone(),
                        }],
                        role: Some("user".to_string()),
                    });
                }
            }
        }

        (system_instruction, contents)
    }

    /// Send a request and handle errors.
    async fn send_request<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        body: &impl Serialize,
    ) -> Result<T> {
        let mut request = self.client.post(url).json(body);

        for (key, value) in self.auth_headers() {
            request = request.header(key, value);
        }

        let response = request
            .send()
            .await
            .map_err(|e| LlmError::ApiError(format!("Request failed: {}", e)))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| LlmError::ApiError(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            // Try to parse error response
            if let Ok(error_response) = serde_json::from_str::<GeminiErrorResponse>(&text) {
                return Err(LlmError::ApiError(format!(
                    "Gemini API error ({}): {}",
                    error_response.error.code, error_response.error.message
                )));
            }
            return Err(LlmError::ApiError(format!(
                "Gemini API error ({}): {}",
                status, text
            )));
        }

        serde_json::from_str(&text).map_err(|e| {
            LlmError::ApiError(format!("Failed to parse response: {}. Body: {}", e, text))
        })
    }
}

#[async_trait]
impl LLMProvider for GeminiProvider {
    fn name(&self) -> &str {
        match &self.endpoint {
            GeminiEndpoint::GoogleAI { .. } => "gemini",
            GeminiEndpoint::VertexAI { .. } => "vertex-ai",
        }
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn max_context_length(&self) -> usize {
        self.max_context_length
    }

    #[instrument(skip(self, prompt), fields(model = %self.model))]
    async fn complete(&self, prompt: &str) -> Result<LLMResponse> {
        self.complete_with_options(prompt, &CompletionOptions::default())
            .await
    }

    #[instrument(skip(self, prompt, options), fields(model = %self.model))]
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

    #[instrument(skip(self, messages, options), fields(model = %self.model))]
    async fn chat(
        &self,
        messages: &[ChatMessage],
        options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse> {
        let (system_instruction, contents) = Self::convert_messages(messages);

        if contents.is_empty() {
            return Err(LlmError::InvalidRequest(
                "No user messages provided".to_string(),
            ));
        }

        let options = options.cloned().unwrap_or_default();

        // Build generation config
        let mut generation_config = GenerationConfig::default();

        if let Some(max_tokens) = options.max_tokens {
            generation_config.max_output_tokens = Some(max_tokens);
        }
        if let Some(temp) = options.temperature {
            generation_config.temperature = Some(temp);
        }
        if let Some(top_p) = options.top_p {
            generation_config.top_p = Some(top_p);
        }
        if let Some(stop) = options.stop {
            generation_config.stop_sequences = Some(stop);
        }
        if options.response_format.as_deref() == Some("json_object") {
            generation_config.response_mime_type = Some("application/json".to_string());
        }

        let request = GenerateContentRequest {
            contents,
            generation_config: Some(generation_config),
            system_instruction,
            safety_settings: None,
        };

        let url = self.build_url(&self.model, "generateContent");
        debug!("Sending request to Gemini: {}", url);

        let response: GenerateContentResponse = self.send_request(&url, &request).await?;

        // Extract content from response
        let candidates = response
            .candidates
            .ok_or_else(|| LlmError::ApiError("No candidates in response".to_string()))?;

        let candidate = candidates
            .first()
            .ok_or_else(|| LlmError::ApiError("Empty candidates array".to_string()))?;

        let content = candidate
            .content
            .parts
            .iter()
            .map(|p| p.text.clone())
            .collect::<Vec<_>>()
            .join("");

        let usage = response.usage_metadata.unwrap_or_default();

        let mut metadata = HashMap::new();
        if !candidate.safety_ratings.is_empty() {
            metadata.insert(
                "safety_ratings".to_string(),
                serde_json::json!(candidate.safety_ratings),
            );
        }

        Ok(LLMResponse {
            content,
            prompt_tokens: usage.prompt_token_count,
            completion_tokens: usage.candidates_token_count,
            total_tokens: usage.total_token_count,
            model: self.model.clone(),
            finish_reason: candidate.finish_reason.clone(),
            metadata,
        })
    }

    async fn stream(&self, prompt: &str) -> Result<BoxStream<'static, Result<String>>> {
        // Delegate to stream_with_options with default options
        self.stream_with_options(prompt, &CompletionOptions::default())
            .await
    }

    /// Stream with options including stop sequences.
    ///
    /// @implements SPEC-032: Gemini stop token handling (OODA 63)
    async fn stream_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> Result<BoxStream<'static, Result<String>>> {
        use futures::StreamExt;

        let messages = vec![ChatMessage::user(prompt)];
        let (system_instruction, contents) = Self::convert_messages(&messages);

        // Build generation config with options
        let generation_config = if options.temperature.is_some()
            || options.max_tokens.is_some()
            || options.stop.is_some()
        {
            Some(GenerationConfig {
                temperature: options.temperature,
                top_p: options.top_p,
                top_k: None,
                max_output_tokens: options.max_tokens,
                stop_sequences: options.stop.clone(),
                response_mime_type: None,
            })
        } else {
            None
        };

        let request = GenerateContentRequest {
            contents,
            generation_config,
            system_instruction,
            safety_settings: None,
        };

        let url = self.build_url(&self.model, "streamGenerateContent");

        let mut req = self.client.post(&url).json(&request);
        for (key, value) in self.auth_headers() {
            req = req.header(key, value);
        }

        let response = req
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
                    // Parse SSE format - Gemini streams JSON objects
                    // Each chunk is a GenerateContentResponse
                    if let Ok(chunk) = serde_json::from_str::<GenerateContentResponse>(&text) {
                        if let Some(candidates) = chunk.candidates {
                            if let Some(candidate) = candidates.first() {
                                let content: String = candidate
                                    .content
                                    .parts
                                    .iter()
                                    .map(|p| p.text.clone())
                                    .collect();
                                return Ok(content);
                            }
                        }
                    }
                    Ok(String::new())
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
        // Gemini 1.5+ supports JSON mode
        self.model.contains("gemini-1.5") || self.model.contains("gemini-2")
    }
}

#[async_trait]
impl EmbeddingProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
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
        2048 // Gemini embedding models max tokens
    }

    #[instrument(skip(self, texts), fields(model = %self.embedding_model, count = texts.len()))]
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // WHY: Gemini batch API has a limit of 100 requests per batchEmbedContents call.
        // Large documents can produce thousands of entities, so we must sub-batch.
        const MAX_EMBEDDING_BATCH_SIZE: usize = 100;

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
                "Splitting {} texts into {} batches of max {} for Gemini embedding API",
                total_texts, num_batches, MAX_EMBEDDING_BATCH_SIZE
            );
        }

        let mut api_result: Vec<Vec<f32>> = Vec::with_capacity(total_texts);

        if api_texts.len() == 1 {
            // Single text - use embedContent
            let request = EmbedContentRequest {
                content: Content {
                    parts: vec![Part {
                        text: api_texts[0].clone(),
                    }],
                    role: None,
                },
                task_type: Some("RETRIEVAL_DOCUMENT".to_string()),
                title: None,
                output_dimensionality: None,
            };

            let url = self.build_url(&self.embedding_model, "embedContent");
            debug!("Sending embedding request to Gemini: {}", url);

            let response: EmbedContentResponse = self.send_request(&url, &request).await?;
            api_result.push(response.embedding.values);
        } else {
            // Multiple texts - use batch endpoint, split into sub-batches
            for (batch_idx, batch) in api_texts.chunks(MAX_EMBEDDING_BATCH_SIZE).enumerate() {
                if num_batches > 1 {
                    debug!(
                        "Gemini embedding batch {}/{}: {} texts",
                        batch_idx + 1,
                        num_batches,
                        batch.len()
                    );
                }
                let batch_result = self.embed_batch(batch).await?;
                api_result.extend(batch_result);
            }
        }

        // Map embeddings back to original indices
        let mut result = vec![vec![0.0; self.embedding_dimension]; texts.len()];
        for ((orig_idx, _), embedding) in valid_texts.iter().zip(api_result) {
            result[*orig_idx] = embedding;
        }

        Ok(result)
    }

    async fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let results = self.embed(&[text.to_string()]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| LlmError::Unknown("Empty embedding result".to_string()))
    }
}

impl GeminiProvider {
    /// Embed multiple texts using batch endpoint.
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let requests: Vec<EmbedContentRequest> = texts
            .iter()
            .map(|text| EmbedContentRequest {
                content: Content {
                    parts: vec![Part { text: text.clone() }],
                    role: None,
                },
                task_type: Some("RETRIEVAL_DOCUMENT".to_string()),
                title: None,
                output_dimensionality: None,
            })
            .collect();

        let batch_request = BatchEmbedContentsRequest { requests };

        let url = self.build_url(&self.embedding_model, "batchEmbedContents");
        debug!("Sending batch embedding request to Gemini: {}", url);

        let response: BatchEmbedContentsResponse = self.send_request(&url, &batch_request).await?;

        Ok(response.embeddings.into_iter().map(|e| e.values).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_length_detection() {
        assert_eq!(
            GeminiProvider::context_length_for_model("gemini-2.0-flash"),
            1_000_000
        );
        assert_eq!(
            GeminiProvider::context_length_for_model("gemini-1.5-pro"),
            2_000_000
        );
        assert_eq!(
            GeminiProvider::context_length_for_model("gemini-1.0-pro"),
            32_000
        );
    }

    #[test]
    fn test_embedding_dimension_detection() {
        assert_eq!(
            GeminiProvider::dimension_for_model("text-embedding-004"),
            768
        );
        assert_eq!(
            GeminiProvider::dimension_for_model("text-embedding-005"),
            768
        );
    }

    #[test]
    fn test_provider_builder() {
        let provider = GeminiProvider::new("test-key")
            .with_model("gemini-1.5-pro")
            .with_embedding_model("text-embedding-004");

        assert_eq!(LLMProvider::model(&provider), "gemini-1.5-pro");
        assert_eq!(provider.dimension(), 768);
        assert_eq!(provider.max_context_length(), 2_000_000);
    }

    #[test]
    fn test_vertex_ai_provider() {
        let provider = GeminiProvider::vertex_ai("my-project", "us-central1", "test-token");
        assert_eq!(LLMProvider::name(&provider), "vertex-ai");
    }

    #[test]
    fn test_message_conversion() {
        let messages = vec![
            ChatMessage::system("You are helpful"),
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there!"),
        ];

        let (system, contents) = GeminiProvider::convert_messages(&messages);

        assert!(system.is_some());
        assert_eq!(system.unwrap().parts[0].text, "You are helpful");
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0].role.as_deref(), Some("user"));
        assert_eq!(contents[1].role.as_deref(), Some("model"));
    }

    #[test]
    fn test_build_url_google_ai() {
        let provider = GeminiProvider::new("test-api-key");
        let url = provider.build_url("gemini-2.0-flash", "generateContent");

        assert!(url.contains("generativelanguage.googleapis.com"));
        assert!(url.contains("gemini-2.0-flash"));
        assert!(url.contains("key=test-api-key"));
    }

    #[test]
    fn test_build_url_vertex_ai() {
        let provider = GeminiProvider::vertex_ai("my-project", "us-central1", "token");
        let url = provider.build_url("gemini-2.0-flash", "generateContent");

        assert!(url.contains("aiplatform.googleapis.com"));
        assert!(url.contains("my-project"));
        assert!(url.contains("us-central1"));
    }
}
