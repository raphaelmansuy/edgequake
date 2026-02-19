use async_trait::async_trait;
use edgequake_llm::{
    ChatMessage, ChatRole, CompletionOptions, EmbeddingProvider, LLMProvider, LLMResponse,
    LlmError, Result,
};
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Native Gemini provider using Google's Generative Language API.
pub struct GeminiProvider {
    client: Client,
    model: String,
    api_key: String,
}

impl GeminiProvider {
    /// Create a new Gemini provider.
    pub fn new(model: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            model,
            api_key,
        }
    }

    /// Get the API base URL for a specific method.
    fn api_url(&self, method: &str) -> String {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:{}",
            self.model, method
        )
    }
}

#[derive(Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GeminiGenerateContentRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Deserialize)]
struct GeminiGenerateContentResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContentResponse,
}

#[derive(Deserialize)]
struct GeminiContentResponse {
    parts: Vec<GeminiPartResponse>,
}

#[derive(Deserialize)]
struct GeminiPartResponse {
    text: String,
}

#[derive(Serialize)]
struct GeminiEmbedContentRequest {
    content: GeminiContent,
}

#[derive(Deserialize)]
struct GeminiEmbedContentResponse {
    embedding: GeminiEmbedding,
}

#[derive(Deserialize)]
struct GeminiEmbedding {
    values: Vec<f32>,
}

#[async_trait]
impl LLMProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn max_context_length(&self) -> usize {
        // Gemini models usually have very large context windows
        128000
    }

    async fn complete(&self, prompt: &str) -> Result<LLMResponse> {
        let request = GeminiGenerateContentRequest {
            contents: vec![GeminiContent {
                role: "user".to_string(),
                parts: vec![GeminiPart {
                    text: prompt.to_string(),
                }],
            }],
        };

        let response = self
            .client
            .post(format!(
                "{}?key={}",
                self.api_url("generateContent"),
                self.api_key
            ))
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::ApiError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!(
                "Gemini API error: {}",
                error_text
            )));
        }

        let gemini_response: GeminiGenerateContentResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ApiError(format!("Failed to parse Gemini response: {}", e)))?;

        let text = gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| LlmError::ApiError("Gemini returned empty response".to_string()))?;

        Ok(LLMResponse {
            content: text,
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            cache_hit_tokens: Some(0),
            finish_reason: None,
            model: self.model.clone(),
            tool_calls: Vec::new(),
            metadata: std::collections::HashMap::new(),
            thinking_tokens: Some(0),
            thinking_content: None,
        })
    }

    async fn complete_with_options(
        &self,
        prompt: &str,
        _options: &CompletionOptions,
    ) -> Result<LLMResponse> {
        // For now, ignoring options to keeps it simple, but could map temperature etc.
        self.complete(prompt).await
    }

    async fn chat(
        &self,
        messages: &[ChatMessage],
        _options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse> {
        let contents: Vec<GeminiContent> = messages
            .iter()
            .map(|m| GeminiContent {
                role: match m.role {
                    ChatRole::User => "user".to_string(),
                    _ => "model".to_string(),
                },
                parts: vec![GeminiPart {
                    text: m.content.clone(),
                }],
            })
            .collect();

        let request = GeminiGenerateContentRequest { contents };

        let response = self
            .client
            .post(format!(
                "{}?key={}",
                self.api_url("generateContent"),
                self.api_key
            ))
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::ApiError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!(
                "Gemini API error: {}",
                error_text
            )));
        }

        let gemini_response: GeminiGenerateContentResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ApiError(format!("Failed to parse Gemini response: {}", e)))?;

        let text = gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| LlmError::ApiError("Gemini returned empty response".to_string()))?;

        Ok(LLMResponse {
            content: text,
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            cache_hit_tokens: Some(0),
            finish_reason: None,
            model: self.model.clone(),
            tool_calls: Vec::new(),
            metadata: std::collections::HashMap::new(),
            thinking_tokens: Some(0),
            thinking_content: None,
        })
    }

    async fn stream(&self, prompt: &str) -> Result<BoxStream<'static, Result<String>>> {
        // For now, fall back to complete() and return it as a stream
        // Full streaming support requires SSE handling which is more complex
        match self.complete(prompt).await {
            Ok(response) => {
                let content = response.content;
                let stream = futures::stream::once(async move { Ok(content) }).boxed();
                Ok(stream)
            }
            Err(e) => Err(e),
        }
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}

#[async_trait]
impl EmbeddingProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn dimension(&self) -> usize {
        // gemini-embedding-001 has a default output dimension of 3072
        if self.model.contains("embedding") {
            3072
        } else {
            1536 // Fallback
        }
    }

    fn max_tokens(&self) -> usize {
        2048
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::new();

        for text in texts {
            let request = GeminiEmbedContentRequest {
                content: GeminiContent {
                    role: "user".to_string(),
                    parts: vec![GeminiPart { text: text.clone() }],
                },
            };

            let response = self
                .client
                .post(format!(
                    "{}?key={}",
                    self.api_url("embedContent"),
                    self.api_key
                ))
                .json(&request)
                .send()
                .await
                .map_err(|e| LlmError::ApiError(e.to_string()))?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(LlmError::ApiError(format!(
                    "Gemini Embedding API error: {}",
                    error_text
                )));
            }

            let gemini_response: GeminiEmbedContentResponse =
                response.json().await.map_err(|e| {
                    LlmError::ApiError(format!("Failed to parse Gemini embedding response: {}", e))
                })?;

            results.push(gemini_response.embedding.values);
        }

        Ok(results)
    }
}
