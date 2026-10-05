//! Ollama-compatible text generation handler (`/api/generate`).
//!
//! Supports both streaming (newline-delimited JSON) and non-streaming modes.

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
    OllamaGenerateRequest, OllamaGenerateResponse, OllamaSearchMode,
};
use crate::state::AppState;

use super::helpers::{
    current_timestamp, estimate_tokens, model_name, response_text, scoped_request,
};

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
    tenant_ctx: crate::middleware::TenantContext,
    Json(request): Json<OllamaGenerateRequest>,
) -> ApiResult<Response> {
    let start_time = Instant::now();
    let prompt_tokens = estimate_tokens(&request.prompt);

    // Parse query mode from prompt
    let (cleaned_query, mode, context_only) = OllamaSearchMode::from_query(&request.prompt);

    let engine_request = scoped_request(
        &cleaned_query,
        mode,
        context_only,
        request.system.as_deref(),
        &tenant_ctx,
        Vec::new(),
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
                        "response": answer,
                        "done": false
                    });
                    let _ = tx.send(Ok(format!("{}\n", chunk))).await;

                    // Send final chunk with stats
                    let elapsed = start.elapsed().as_nanos() as u64;
                    let completion_tokens = estimate_tokens(&answer);
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
        let response = state
            .query
            .engine_impl
            .query(engine_request)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        let answer = response_text(response, context_only);
        let elapsed = start_time.elapsed().as_nanos() as u64;
        let completion_tokens = estimate_tokens(&answer);

        Ok(Json(OllamaGenerateResponse {
            model: model_name(),
            created_at: current_timestamp(),
            response: answer,
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
