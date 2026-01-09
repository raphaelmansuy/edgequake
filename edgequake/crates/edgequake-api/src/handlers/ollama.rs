//! Ollama Emulation API handlers.
//!
//! This module provides Ollama-compatible API endpoints, allowing EdgeQuake
//! to act as a drop-in replacement for Ollama. This enables integration with
//! tools like OpenWebUI that expect Ollama's API format.
//!
//! ## Implements
//!
//! - **FEAT0600**: Ollama API compatibility layer
//! - **FEAT0601**: Model listing endpoint (/api/tags)
//! - **FEAT0602**: Chat completion with streaming (/api/chat)
//! - **FEAT0603**: Text generation endpoint (/api/generate)
//!
//! ## Use Cases
//!
//! - **UC2200**: User connects OpenWebUI to EdgeQuake as Ollama backend
//! - **UC2201**: User lists available models via Ollama-compatible API
//! - **UC2202**: User sends chat completion with query mode prefix
//! - **UC2203**: User generates text with streaming response
//!
//! ## Enforces
//!
//! - **BR0600**: API responses must match Ollama's JSON schema
//! - **BR0601**: Query mode prefixes must be stripped from user messages
//! - **BR0602**: Streaming format must be newline-delimited JSON
//!
//! ## Endpoints
//!
//! - `GET /api/tags` - List available models
//! - `GET /api/version` - Get version information
//! - `GET /api/ps` - List running models
//! - `POST /api/generate` - Generate completion (text-only)
//! - `POST /api/chat` - Chat completion with conversation history
//!
//! ## Query Mode Prefixes
//!
//! Chat messages can include a prefix to select the query mode:
//!
//! - `/local` - Use local (entity-centric) query mode
//! - `/global` - Use global (relationship-centric) query mode
//! - `/naive` - Use naive (chunk-only) query mode
//! - `/hybrid` - Use hybrid query mode (default)
//! - `/mix` - Use mix query mode (combines local + naive)
//! - `/bypass` - Bypass RAG, send directly to LLM

use axum::{
    body::Body,
    extract::State,
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use std::time::Instant;
use tokio_stream::wrappers::ReceiverStream;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use edgequake_query::{QueryMode, QueryRequest as EngineQueryRequest};

// Re-export DTOs for backward compatibility
pub use crate::handlers::ollama_types::{
    ollama_default_stream, OllamaChatRequest, OllamaChatResponse, OllamaGenerateRequest,
    OllamaGenerateResponse, OllamaMessage, OllamaModel, OllamaModelDetails, OllamaPsResponse,
    OllamaRunningModel, OllamaSearchMode, OllamaTagsResponse, OllamaVersionResponse,
};

/// Default model name for Ollama emulation.
const OLLAMA_MODEL_NAME: &str = "edgequake";
/// Default model tag for Ollama emulation.
const OLLAMA_MODEL_TAG: &str = "latest";
/// Default model size (placeholder).
const OLLAMA_MODEL_SIZE: u64 = 7_000_000_000; // 7GB placeholder
/// Default model digest.
const OLLAMA_MODEL_DIGEST: &str = "sha256:edgequake-rag-v1";
/// API version string.
const OLLAMA_API_VERSION: &str = "0.9.3";

// ============================================================================
// Utility Functions
// ============================================================================

/// Estimate token count for a string (rough approximation).
fn estimate_tokens(text: &str) -> u32 {
    // Rough estimate: 1 token ≈ 4 characters
    (text.len() / 4) as u32
}

/// Get the current timestamp in ISO 8601 format.
fn current_timestamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()
}

/// Get the model name for responses.
fn model_name() -> String {
    format!("{}:{}", OLLAMA_MODEL_NAME, OLLAMA_MODEL_TAG)
}

// ============================================================================
// Handlers
// ============================================================================

/// Get Ollama API version.
#[utoipa::path(
    get,
    path = "/api/version",
    tag = "Ollama Emulation",
    responses(
        (status = 200, description = "Version information", body = OllamaVersionResponse)
    )
)]
pub async fn ollama_version() -> Json<OllamaVersionResponse> {
    Json(OllamaVersionResponse {
        version: OLLAMA_API_VERSION.to_string(),
    })
}

/// List available models (Ollama tags endpoint).
#[utoipa::path(
    get,
    path = "/api/tags",
    tag = "Ollama Emulation",
    responses(
        (status = 200, description = "List of available models", body = OllamaTagsResponse)
    )
)]
pub async fn ollama_tags() -> Json<OllamaTagsResponse> {
    let model = OllamaModel {
        name: model_name(),
        model: model_name(),
        size: OLLAMA_MODEL_SIZE,
        digest: OLLAMA_MODEL_DIGEST.to_string(),
        modified_at: current_timestamp(),
        details: OllamaModelDetails {
            parent_model: String::new(),
            format: "gguf".to_string(),
            family: OLLAMA_MODEL_NAME.to_string(),
            families: vec![OLLAMA_MODEL_NAME.to_string()],
            parameter_size: "7B".to_string(),
            quantization_level: "Q4_0".to_string(),
        },
    };

    Json(OllamaTagsResponse {
        models: vec![model],
    })
}

/// List running models (Ollama ps endpoint).
#[utoipa::path(
    get,
    path = "/api/ps",
    tag = "Ollama Emulation",
    responses(
        (status = 200, description = "List of running models", body = OllamaPsResponse)
    )
)]
pub async fn ollama_ps() -> Json<OllamaPsResponse> {
    let model = OllamaRunningModel {
        name: model_name(),
        model: model_name(),
        size: OLLAMA_MODEL_SIZE,
        digest: OLLAMA_MODEL_DIGEST.to_string(),
        details: OllamaModelDetails {
            parent_model: String::new(),
            format: "gguf".to_string(),
            family: "llama".to_string(),
            families: vec!["llama".to_string()],
            parameter_size: "7B".to_string(),
            quantization_level: "Q4_0".to_string(),
        },
        expires_at: "2050-12-31T23:59:59Z".to_string(),
        size_vram: OLLAMA_MODEL_SIZE,
    };

    Json(OllamaPsResponse {
        models: vec![model],
    })
}

/// Handle generate completion requests.
///
/// This endpoint provides basic LLM generation without RAG context.
/// For RAG-enhanced responses, use the `/api/chat` endpoint.
#[utoipa::path(
    post,
    path = "/api/generate",
    tag = "Ollama Emulation",
    request_body = OllamaGenerateRequest,
    responses(
        (status = 200, description = "Generated response", body = OllamaGenerateResponse)
    )
)]
pub async fn ollama_generate(
    State(state): State<AppState>,
    Json(request): Json<OllamaGenerateRequest>,
) -> ApiResult<Response> {
    let start_time = Instant::now();
    let prompt_tokens = estimate_tokens(&request.prompt);

    // Parse query mode from prompt
    let (cleaned_query, mode, context_only) = OllamaSearchMode::from_query(&request.prompt);

    if request.stream {
        // Streaming response
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<String, std::io::Error>>(32);

        let engine = state.query_engine.clone();
        let model = model_name();

        tokio::spawn(async move {
            let start = Instant::now();

            // Execute query based on mode
            let response_result = if mode == OllamaSearchMode::Bypass {
                // For bypass mode, we'd need direct LLM access
                // For now, fall back to hybrid query
                let engine_request =
                    EngineQueryRequest::new(&cleaned_query).with_mode(QueryMode::Hybrid);
                engine.query(engine_request).await
            } else if let Some(query_mode) = mode.to_query_mode() {
                let mut engine_request =
                    EngineQueryRequest::new(&cleaned_query).with_mode(query_mode);
                if context_only {
                    engine_request = engine_request.context_only();
                }
                engine.query(engine_request).await
            } else {
                // Fallback to hybrid
                let engine_request =
                    EngineQueryRequest::new(&cleaned_query).with_mode(QueryMode::Hybrid);
                engine.query(engine_request).await
            };

            match response_result {
                Ok(response) => {
                    // Send content chunk
                    let chunk = serde_json::json!({
                        "model": model,
                        "created_at": current_timestamp(),
                        "response": response.answer,
                        "done": false
                    });
                    let _ = tx.send(Ok(format!("{}\n", chunk))).await;

                    // Send final chunk with stats
                    let elapsed = start.elapsed().as_nanos() as u64;
                    let completion_tokens = estimate_tokens(&response.answer);
                    let final_chunk = serde_json::json!({
                        "model": model,
                        "created_at": current_timestamp(),
                        "response": "",
                        "done": true,
                        "done_reason": "stop",
                        "context": [],
                        "total_duration": elapsed,
                        "load_duration": 0,
                        "prompt_eval_count": prompt_tokens,
                        "prompt_eval_duration": elapsed / 4,
                        "eval_count": completion_tokens,
                        "eval_duration": elapsed * 3 / 4
                    });
                    let _ = tx.send(Ok(format!("{}\n", final_chunk))).await;
                }
                Err(e) => {
                    let error_chunk = serde_json::json!({
                        "model": model,
                        "created_at": current_timestamp(),
                        "response": format!("Error: {}", e),
                        "done": true,
                        "done_reason": "error"
                    });
                    let _ = tx.send(Ok(format!("{}\n", error_chunk))).await;
                }
            }
        });

        let stream = ReceiverStream::new(rx);
        let body = Body::from_stream(stream);

        Ok(Response::builder()
            .header(header::CONTENT_TYPE, "application/x-ndjson")
            .header(header::CACHE_CONTROL, "no-cache")
            .header("X-Accel-Buffering", "no")
            .body(body)
            .unwrap())
    } else {
        // Non-streaming response
        let engine_request = if let Some(query_mode) = mode.to_query_mode() {
            let mut req = EngineQueryRequest::new(&cleaned_query).with_mode(query_mode);
            if context_only {
                req = req.context_only();
            }
            req
        } else {
            EngineQueryRequest::new(&cleaned_query).with_mode(QueryMode::Hybrid)
        };

        let response = state
            .query_engine
            .query(engine_request)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        let elapsed = start_time.elapsed().as_nanos() as u64;
        let completion_tokens = estimate_tokens(&response.answer);

        Ok(Json(OllamaGenerateResponse {
            model: model_name(),
            created_at: current_timestamp(),
            response: response.answer,
            done: true,
            done_reason: Some("stop".to_string()),
            context: Some(vec![]),
            total_duration: Some(elapsed),
            load_duration: Some(0),
            prompt_eval_count: Some(prompt_tokens),
            prompt_eval_duration: Some(elapsed / 4),
            eval_count: Some(completion_tokens),
            eval_duration: Some(elapsed * 3 / 4),
        })
        .into_response())
    }
}

/// Handle chat completion requests with RAG.
///
/// This endpoint processes chat messages through the EdgeQuake RAG pipeline,
/// returning responses augmented with knowledge graph context.
///
/// ## Query Mode Prefixes
///
/// The user message can include a prefix to select the query mode:
///
/// - `/local query` - Entity-centric retrieval
/// - `/global query` - Relationship-centric retrieval
/// - `/naive query` - Chunk-only retrieval
/// - `/hybrid query` - Combined entity + chunk retrieval (default)
/// - `/mix query` - Combines local + naive
/// - `/bypass query` - Skip RAG, direct LLM query
/// - `/context query` - Return only context, no generation
#[utoipa::path(
    post,
    path = "/api/chat",
    tag = "Ollama Emulation",
    request_body = OllamaChatRequest,
    responses(
        (status = 200, description = "Chat response", body = OllamaChatResponse)
    )
)]
pub async fn ollama_chat(
    State(state): State<AppState>,
    Json(request): Json<OllamaChatRequest>,
) -> ApiResult<Response> {
    if request.messages.is_empty() {
        return Err(ApiError::BadRequest("No messages provided".to_string()));
    }

    // Get the last user message as the query
    let last_message = request
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .ok_or_else(|| ApiError::BadRequest("No user message found".to_string()))?;

    let query = &last_message.content;
    let start_time = Instant::now();
    let prompt_tokens = estimate_tokens(query);

    // Parse query mode from message
    let (cleaned_query, mode, context_only) = OllamaSearchMode::from_query(query);

    // Build conversation history (excluding the current query)
    let conversation_history: Vec<edgequake_query::ConversationMessage> = request
        .messages
        .iter()
        .take(request.messages.len().saturating_sub(1))
        .map(|m| edgequake_query::ConversationMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    if request.stream {
        // Streaming response
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<String, std::io::Error>>(32);

        let engine = state.query_engine.clone();
        let model = model_name();

        tokio::spawn(async move {
            let start = Instant::now();

            // Execute query based on mode
            let response_result = if mode == OllamaSearchMode::Bypass {
                // For bypass mode, fall back to hybrid query
                let engine_request = EngineQueryRequest::new(&cleaned_query)
                    .with_mode(QueryMode::Hybrid)
                    .with_conversation_history(conversation_history);
                engine.query(engine_request).await
            } else if let Some(query_mode) = mode.to_query_mode() {
                let mut engine_request = EngineQueryRequest::new(&cleaned_query)
                    .with_mode(query_mode)
                    .with_conversation_history(conversation_history);
                if context_only {
                    engine_request = engine_request.context_only();
                }
                engine.query(engine_request).await
            } else {
                let engine_request = EngineQueryRequest::new(&cleaned_query)
                    .with_mode(QueryMode::Hybrid)
                    .with_conversation_history(conversation_history);
                engine.query(engine_request).await
            };

            match response_result {
                Ok(response) => {
                    // Send content chunk
                    let chunk = serde_json::json!({
                        "model": model,
                        "created_at": current_timestamp(),
                        "message": {
                            "role": "assistant",
                            "content": response.answer,
                            "images": null
                        },
                        "done": false
                    });
                    let _ = tx.send(Ok(format!("{}\n", chunk))).await;

                    // Send final chunk with stats
                    let elapsed = start.elapsed().as_nanos() as u64;
                    let completion_tokens = estimate_tokens(&response.answer);
                    let final_chunk = serde_json::json!({
                        "model": model,
                        "created_at": current_timestamp(),
                        "message": {
                            "role": "assistant",
                            "content": "",
                            "images": null
                        },
                        "done": true,
                        "done_reason": "stop",
                        "total_duration": elapsed,
                        "load_duration": 0,
                        "prompt_eval_count": prompt_tokens,
                        "prompt_eval_duration": elapsed / 4,
                        "eval_count": completion_tokens,
                        "eval_duration": elapsed * 3 / 4
                    });
                    let _ = tx.send(Ok(format!("{}\n", final_chunk))).await;
                }
                Err(e) => {
                    let error_chunk = serde_json::json!({
                        "model": model,
                        "created_at": current_timestamp(),
                        "message": {
                            "role": "assistant",
                            "content": format!("Error: {}", e),
                            "images": null
                        },
                        "done": true,
                        "done_reason": "error"
                    });
                    let _ = tx.send(Ok(format!("{}\n", error_chunk))).await;
                }
            }
        });

        let stream = ReceiverStream::new(rx);
        let body = Body::from_stream(stream);

        Ok(Response::builder()
            .header(header::CONTENT_TYPE, "application/x-ndjson")
            .header(header::CACHE_CONTROL, "no-cache")
            .header("X-Accel-Buffering", "no")
            .body(body)
            .unwrap())
    } else {
        // Non-streaming response
        let engine_request = if let Some(query_mode) = mode.to_query_mode() {
            let mut req = EngineQueryRequest::new(&cleaned_query)
                .with_mode(query_mode)
                .with_conversation_history(conversation_history);
            if context_only {
                req = req.context_only();
            }
            req
        } else {
            EngineQueryRequest::new(&cleaned_query)
                .with_mode(QueryMode::Hybrid)
                .with_conversation_history(conversation_history)
        };

        let response = state
            .query_engine
            .query(engine_request)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        let elapsed = start_time.elapsed().as_nanos() as u64;
        let completion_tokens = estimate_tokens(&response.answer);

        Ok(Json(OllamaChatResponse {
            model: model_name(),
            created_at: current_timestamp(),
            message: OllamaMessage {
                role: "assistant".to_string(),
                content: response.answer,
                images: None,
            },
            done: true,
            done_reason: Some("stop".to_string()),
            total_duration: Some(elapsed),
            load_duration: Some(0),
            prompt_eval_count: Some(prompt_tokens),
            prompt_eval_duration: Some(elapsed / 4),
            eval_count: Some(completion_tokens),
            eval_duration: Some(elapsed * 3 / 4),
        })
        .into_response())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_mode_parsing() {
        // Default mode
        let (query, mode, context_only) = OllamaSearchMode::from_query("hello world");
        assert_eq!(query, "hello world");
        assert_eq!(mode, OllamaSearchMode::Hybrid);
        assert!(!context_only);

        // Local mode
        let (query, mode, context_only) = OllamaSearchMode::from_query("/local what is rust?");
        assert_eq!(query, "what is rust?");
        assert_eq!(mode, OllamaSearchMode::Local);
        assert!(!context_only);

        // Global mode
        let (query, mode, context_only) = OllamaSearchMode::from_query("/global explain AI");
        assert_eq!(query, "explain AI");
        assert_eq!(mode, OllamaSearchMode::Global);
        assert!(!context_only);

        // Context only mode
        let (query, mode, context_only) = OllamaSearchMode::from_query("/localcontext entities");
        assert_eq!(query, "entities");
        assert_eq!(mode, OllamaSearchMode::Local);
        assert!(context_only);

        // Bypass mode
        let (query, mode, context_only) = OllamaSearchMode::from_query("/bypass just chat");
        assert_eq!(query, "just chat");
        assert_eq!(mode, OllamaSearchMode::Bypass);
        assert!(!context_only);
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("hello"), 1);
        assert_eq!(estimate_tokens("hello world"), 2);
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_model_name() {
        let name = model_name();
        assert!(name.contains("edgequake"));
    }

    #[test]
    fn test_ollama_constants() {
        assert_eq!(OLLAMA_MODEL_NAME, "edgequake");
        assert_eq!(OLLAMA_MODEL_TAG, "latest");
        assert_eq!(OLLAMA_API_VERSION, "0.9.3");
    }

    #[test]
    fn test_search_mode_naive() {
        let (query, mode, _) = OllamaSearchMode::from_query("/naive simple search");
        assert_eq!(query, "simple search");
        assert_eq!(mode, OllamaSearchMode::Naive);
    }

    #[test]
    fn test_search_mode_mix() {
        let (query, mode, _) = OllamaSearchMode::from_query("/mix combined");
        assert_eq!(query, "combined");
        assert_eq!(mode, OllamaSearchMode::Mix);
    }
}
