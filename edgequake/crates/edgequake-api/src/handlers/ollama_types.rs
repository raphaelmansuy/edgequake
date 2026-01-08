//! DTOs for Ollama emulation API endpoints.
//!
//! This module contains all data transfer objects used in Ollama-compatible API endpoints,
//! including chat, generate, and model listing.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ============================================================================
// Query Mode Enum
// ============================================================================

/// Query mode for Ollama API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OllamaSearchMode {
    Naive,
    Local,
    Global,
    Hybrid,
    Mix,
    Bypass,
    Context,
}

impl OllamaSearchMode {
    /// Parse query prefix to determine search mode.
    pub fn from_query(query: &str) -> (String, Self, bool) {
        let prefixes = [
            ("/localcontext ", Self::Local, true),
            ("/globalcontext ", Self::Global, true),
            ("/naivecontext ", Self::Naive, true),
            ("/hybridcontext ", Self::Hybrid, true),
            ("/mixcontext ", Self::Mix, true),
            ("/context ", Self::Mix, true),
            ("/local ", Self::Local, false),
            ("/global ", Self::Global, false),
            ("/naive ", Self::Naive, false),
            ("/hybrid ", Self::Hybrid, false),
            ("/mix ", Self::Mix, false),
            ("/bypass ", Self::Bypass, false),
        ];

        for (prefix, mode, context_only) in prefixes {
            if let Some(rest) = query.strip_prefix(prefix) {
                return (rest.to_string(), mode, context_only);
            }
        }

        (query.to_string(), Self::Hybrid, false)
    }

    /// Convert to EdgeQuake QueryMode.
    pub fn to_query_mode(&self) -> Option<edgequake_query::QueryMode> {
        match self {
            Self::Naive => Some(edgequake_query::QueryMode::Naive),
            Self::Local => Some(edgequake_query::QueryMode::Local),
            Self::Global => Some(edgequake_query::QueryMode::Global),
            Self::Hybrid => Some(edgequake_query::QueryMode::Hybrid),
            Self::Mix => Some(edgequake_query::QueryMode::Mix),
            Self::Bypass => None, // Bypass goes directly to LLM
            Self::Context => Some(edgequake_query::QueryMode::Mix),
        }
    }
}

// ============================================================================
// Message and Request DTOs
// ============================================================================

/// Ollama message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OllamaMessage {
    /// Role of the message sender (user, assistant, system).
    pub role: String,

    /// Content of the message.
    pub content: String,

    /// Optional images (base64 encoded, for multimodal models).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

/// Ollama chat request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OllamaChatRequest {
    /// Model name (ignored, EdgeQuake handles all queries).
    pub model: String,

    /// Conversation messages.
    pub messages: Vec<OllamaMessage>,

    /// Whether to stream the response.
    #[serde(default = "ollama_default_stream")]
    pub stream: bool,

    /// System prompt override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// Model options (temperature, top_p, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
}

/// Default stream value for Ollama operations.
pub fn ollama_default_stream() -> bool {
    true
}

/// Ollama chat response (non-streaming).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OllamaChatResponse {
    /// Model name.
    pub model: String,

    /// Creation timestamp.
    pub created_at: String,

    /// Assistant's response message.
    pub message: OllamaMessage,

    /// Whether the response is complete.
    pub done: bool,

    /// Reason for completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_reason: Option<String>,

    /// Total duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,

    /// Load duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u64>,

    /// Prompt evaluation count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u32>,

    /// Prompt evaluation duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u64>,

    /// Response evaluation count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u32>,

    /// Response evaluation duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u64>,
}

/// Ollama generate request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OllamaGenerateRequest {
    /// Model name (ignored, EdgeQuake handles all queries).
    pub model: String,

    /// The prompt to generate a response for.
    pub prompt: String,

    /// Whether to stream the response.
    #[serde(default)]
    pub stream: bool,

    /// System prompt override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// Model options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
}

/// Ollama generate response (non-streaming).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OllamaGenerateResponse {
    /// Model name.
    pub model: String,

    /// Creation timestamp.
    pub created_at: String,

    /// Generated response text.
    pub response: String,

    /// Whether the response is complete.
    pub done: bool,

    /// Reason for completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_reason: Option<String>,

    /// Context tokens (for continuation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<i32>>,

    /// Total duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,

    /// Load duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u64>,

    /// Prompt evaluation count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u32>,

    /// Prompt evaluation duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u64>,

    /// Response evaluation count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u32>,

    /// Response evaluation duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u64>,
}

// ============================================================================
// Model Information DTOs
// ============================================================================

/// Ollama version response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OllamaVersionResponse {
    /// API version.
    pub version: String,
}

/// Ollama model details.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OllamaModelDetails {
    /// Parent model name.
    pub parent_model: String,

    /// Model format.
    pub format: String,

    /// Model family.
    pub family: String,

    /// Model families.
    pub families: Vec<String>,

    /// Parameter size.
    pub parameter_size: String,

    /// Quantization level.
    pub quantization_level: String,
}

/// Ollama model information.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OllamaModel {
    /// Model name.
    pub name: String,

    /// Model identifier.
    pub model: String,

    /// Model size in bytes.
    pub size: u64,

    /// Model digest.
    pub digest: String,

    /// Modification timestamp.
    pub modified_at: String,

    /// Model details.
    pub details: OllamaModelDetails,
}

/// Ollama tags response (list models).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OllamaTagsResponse {
    /// Available models.
    pub models: Vec<OllamaModel>,
}

/// Ollama running model details.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OllamaRunningModel {
    /// Model name.
    pub name: String,

    /// Model identifier.
    pub model: String,

    /// Model size in bytes.
    pub size: u64,

    /// Model digest.
    pub digest: String,

    /// Model details.
    pub details: OllamaModelDetails,

    /// Expiration timestamp.
    pub expires_at: String,

    /// VRAM usage in bytes.
    pub size_vram: u64,
}

/// Ollama ps response (running models).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OllamaPsResponse {
    /// Running models.
    pub models: Vec<OllamaRunningModel>,
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_search_mode_from_query() {
        let (query, mode, context) = OllamaSearchMode::from_query("/local what is AI?");
        assert_eq!(query, "what is AI?");
        assert_eq!(mode, OllamaSearchMode::Local);
        assert!(!context);

        let (query, mode, context) = OllamaSearchMode::from_query("/localcontext show me entities");
        assert_eq!(query, "show me entities");
        assert_eq!(mode, OllamaSearchMode::Local);
        assert!(context);

        let (query, mode, _) = OllamaSearchMode::from_query("no prefix query");
        assert_eq!(query, "no prefix query");
        assert_eq!(mode, OllamaSearchMode::Hybrid);
    }

    #[test]
    fn test_ollama_message_serialization() {
        let msg = OllamaMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
            images: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_ollama_chat_request_serialization() {
        let req = OllamaChatRequest {
            model: "edgequake".to_string(),
            messages: vec![],
            stream: true,
            system: Some("You are helpful".to_string()),
            options: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("edgequake"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_ollama_chat_response_serialization() {
        let response = OllamaChatResponse {
            model: "edgequake".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            message: OllamaMessage {
                role: "assistant".to_string(),
                content: "Hello!".to_string(),
                images: None,
            },
            done: true,
            done_reason: Some("stop".to_string()),
            total_duration: Some(1000),
            load_duration: None,
            prompt_eval_count: Some(5),
            prompt_eval_duration: None,
            eval_count: Some(2),
            eval_duration: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("assistant"));
        assert!(json.contains("Hello!"));
    }

    #[test]
    fn test_ollama_generate_request_serialization() {
        let req = OllamaGenerateRequest {
            model: "edgequake".to_string(),
            prompt: "Tell me about AI".to_string(),
            stream: false,
            system: None,
            options: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("Tell me about AI"));
        assert!(json.contains("false"));
    }

    #[test]
    fn test_ollama_generate_response_serialization() {
        let response = OllamaGenerateResponse {
            model: "edgequake".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            response: "AI is artificial intelligence".to_string(),
            done: true,
            done_reason: Some("stop".to_string()),
            context: None,
            total_duration: Some(2000),
            load_duration: None,
            prompt_eval_count: Some(10),
            prompt_eval_duration: None,
            eval_count: Some(5),
            eval_duration: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("AI is artificial intelligence"));
    }

    #[test]
    fn test_ollama_version_response_serialization() {
        let response = OllamaVersionResponse {
            version: "0.9.3".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("0.9.3"));
    }

    #[test]
    fn test_ollama_model_details_serialization() {
        let details = OllamaModelDetails {
            parent_model: "llama".to_string(),
            format: "gguf".to_string(),
            family: "llama".to_string(),
            families: vec!["llama".to_string()],
            parameter_size: "7B".to_string(),
            quantization_level: "Q4_0".to_string(),
        };

        let json = serde_json::to_string(&details).unwrap();
        assert!(json.contains("llama"));
        assert!(json.contains("7B"));
    }

    #[test]
    fn test_ollama_model_serialization() {
        let model = OllamaModel {
            name: "edgequake:latest".to_string(),
            model: "edgequake".to_string(),
            size: 7_000_000_000,
            digest: "sha256:test".to_string(),
            modified_at: "2024-01-01T00:00:00Z".to_string(),
            details: OllamaModelDetails {
                parent_model: "llama".to_string(),
                format: "gguf".to_string(),
                family: "llama".to_string(),
                families: vec![],
                parameter_size: "7B".to_string(),
                quantization_level: "Q4_0".to_string(),
            },
        };

        let json = serde_json::to_string(&model).unwrap();
        assert!(json.contains("edgequake"));
        assert!(json.contains("7000000000"));
    }

    #[test]
    fn test_ollama_tags_response_serialization() {
        let response = OllamaTagsResponse { models: vec![] };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("models"));
    }

    #[test]
    fn test_ollama_running_model_serialization() {
        let model = OllamaRunningModel {
            name: "edgequake:latest".to_string(),
            model: "edgequake".to_string(),
            size: 7_000_000_000,
            digest: "sha256:test".to_string(),
            details: OllamaModelDetails {
                parent_model: "llama".to_string(),
                format: "gguf".to_string(),
                family: "llama".to_string(),
                families: vec![],
                parameter_size: "7B".to_string(),
                quantization_level: "Q4_0".to_string(),
            },
            expires_at: "2024-01-01T01:00:00Z".to_string(),
            size_vram: 4_000_000_000,
        };

        let json = serde_json::to_string(&model).unwrap();
        assert!(json.contains("edgequake"));
        assert!(json.contains("4000000000"));
    }

    #[test]
    fn test_ollama_ps_response_serialization() {
        let response = OllamaPsResponse { models: vec![] };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("models"));
    }
}
