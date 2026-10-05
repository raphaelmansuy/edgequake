//! Ollama-compatible chat completion handler (`/api/chat`).
//!
//! Supports conversation history, query-mode prefixes, and both
//! streaming (newline-delimited JSON) and non-streaming responses.

use axum::{
    body::Body,
    extract::State,
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use std::time::Instant;
use tokio_stream::wrappers::ReceiverStream;

use crate::error::{ApiError, ApiResult};
use crate::handlers::ollama_types::{
    OllamaChatRequest, OllamaChatResponse, OllamaMessage, OllamaSearchMode,
};
use crate::state::AppState;

use super::helpers::{
    current_timestamp, estimate_tokens, model_name, response_text, scoped_request,
};

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
    tenant_ctx: crate::middleware::TenantContext,
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

    let engine_request = scoped_request(
        &cleaned_query,
        mode,
        context_only,
        request.system.as_deref(),
        &tenant_ctx,
        conversation_history,
    );

    if request.stream {
        // Streaming response
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<String, std::io::Error>>(32);

        let engine = state.query.engine_impl.clone();
        let model = model_name();
        tokio::spawn(async move {
            let start = Instant::now();
            let response_result = engine.query(engine_request).await;

            match response_result {
                Ok(response) => {
                    let answer = response_text(response, context_only);
                    // Send content chunk
                    let chunk = serde_json::json!({
                        "model": model,
                        "created_at": current_timestamp(),
                        "message": {
                            "role": "assistant",
                            "content": answer,
                            "images": null
                        },
                        "done": false
                    });
                    let _ = tx.send(Ok(format!("{}\n", chunk))).await;

                    // Send final chunk with stats
                    let elapsed = start.elapsed().as_nanos() as u64;
                    let completion_tokens = estimate_tokens(&answer);
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
        let response = state
            .query
            .engine_impl
            .query(engine_request)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        let answer = response_text(response, context_only);
        let elapsed = start_time.elapsed().as_nanos() as u64;
        let completion_tokens = estimate_tokens(&answer);

        Ok(Json(OllamaChatResponse {
            model: model_name(),
            created_at: current_timestamp(),
            message: OllamaMessage {
                role: "assistant".to_string(),
                content: answer,
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
