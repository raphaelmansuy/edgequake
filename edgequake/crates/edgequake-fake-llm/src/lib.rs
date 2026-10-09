//! Hermetic multi-shape LLM stub for SPEC-163 proofs.
//!
//! Fault injection via query string or `X-Fake-Mode` header:
//! `ok` (default), `401`, `500`, `slow`, `wrong-dim`, `down`.

use axum::body::Body;
use axum::extract::{Query, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize}; // Serialize: request/response DTOs in this crate
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct FakeLlmState {
    pub embedding_dimension: usize,
    pub model: String,
    pub require_key: Option<String>,
    pub hits: Arc<AtomicU64>,
}

impl Default for FakeLlmState {
    fn default() -> Self {
        Self {
            embedding_dimension: 8,
            model: "fake-chat".to_string(),
            require_key: None,
            hits: Arc::new(AtomicU64::new(0)),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct ModeQuery {
    mode: Option<String>,
}

fn mode_from(headers: &HeaderMap, q: &ModeQuery) -> String {
    if let Some(m) = q.mode.as_deref() {
        return m.to_ascii_lowercase();
    }
    headers
        .get("x-fake-mode")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("ok")
        .to_ascii_lowercase()
}

fn auth_ok(state: &FakeLlmState, headers: &HeaderMap) -> bool {
    let Some(expected) = state.require_key.as_deref() else {
        return true;
    };
    if headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == format!("Bearer {expected}") || v == expected)
    {
        return true;
    }
    headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == expected)
}

async fn fault_mw(request: Request, next: Next) -> Response {
    let mode = request
        .headers()
        .get("x-fake-mode")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if mode == "down" {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    if mode == "slow" {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    next.run(request).await
}

fn fault_status(mode: &str) -> Option<StatusCode> {
    match mode {
        "401" => Some(StatusCode::UNAUTHORIZED),
        "500" => Some(StatusCode::INTERNAL_SERVER_ERROR),
        "down" => Some(StatusCode::SERVICE_UNAVAILABLE),
        _ => None,
    }
}

pub fn app(state: FakeLlmState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(openai_models))
        .route("/v1/chat/completions", post(openai_chat))
        .route("/v1/embeddings", post(openai_embeddings))
        .route("/v1/messages", post(anthropic_messages))
        .route("/api/tags", get(ollama_tags))
        .route("/api/version", get(ollama_version))
        .route("/api/chat", post(ollama_chat))
        .layer(middleware::from_fn(fault_mw))
        .with_state(state)
}

async fn health(State(state): State<FakeLlmState>) -> Json<Value> {
    Json(json!({ "status": "ok", "hits": state.hits.load(Ordering::Relaxed) }))
}

async fn openai_models(
    State(state): State<FakeLlmState>,
    headers: HeaderMap,
    Query(q): Query<ModeQuery>,
) -> Response {
    let mode = mode_from(&headers, &q);
    if let Some(st) = fault_status(&mode) {
        return st.into_response();
    }
    if !auth_ok(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    Json(json!({
        "object": "list",
        "data": [
            { "id": state.model, "object": "model" },
            { "id": "fake-embed", "object": "model" }
        ]
    }))
    .into_response()
}

#[derive(Deserialize)]
struct ChatBody {
    #[serde(default)]
    messages: Vec<Value>,
    #[serde(default)]
    stream: bool,
}

async fn openai_chat(
    State(state): State<FakeLlmState>,
    headers: HeaderMap,
    Query(q): Query<ModeQuery>,
    Json(body): Json<ChatBody>,
) -> Response {
    state.hits.fetch_add(1, Ordering::Relaxed);
    let mode = mode_from(&headers, &q);
    if let Some(st) = fault_status(&mode) {
        return st.into_response();
    }
    if !auth_ok(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let last = body
        .messages
        .last()
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("ok");
    if body.stream {
        let chunk = format!(
            "data: {}\n\ndata: [DONE]\n\n",
            json!({
                "id": "chatcmpl-fake",
                "object": "chat.completion.chunk",
                "choices": [{"delta": {"content": last}, "index": 0, "finish_reason": null}]
            })
        );
        return Response::builder()
            .status(200)
            .header("content-type", "text/event-stream")
            .body(Body::from(chunk))
            .unwrap()
            .into_response();
    }
    Json(json!({
        "id": "chatcmpl-fake",
        "object": "chat.completion",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": format!("echo:{last}") },
            "finish_reason": "stop"
        }]
    }))
    .into_response()
}

#[derive(Deserialize)]
struct EmbedBody {
    #[serde(default)]
    #[allow(dead_code)]
    input: Value,
}

async fn openai_embeddings(
    State(state): State<FakeLlmState>,
    headers: HeaderMap,
    Query(q): Query<ModeQuery>,
    Json(_body): Json<EmbedBody>,
) -> Response {
    let mode = mode_from(&headers, &q);
    if let Some(st) = fault_status(&mode) {
        return st.into_response();
    }
    if !auth_ok(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let dim = if mode == "wrong-dim" {
        state.embedding_dimension.saturating_add(1).max(2)
    } else {
        state.embedding_dimension
    };
    let embedding: Vec<f32> = (0..dim).map(|i| (i as f32) * 0.01).collect();
    Json(json!({
        "object": "list",
        "data": [{ "object": "embedding", "index": 0, "embedding": embedding }],
        "model": "fake-embed",
        "usage": { "prompt_tokens": 1, "total_tokens": 1 }
    }))
    .into_response()
}

#[derive(Deserialize)]
struct AnthropicBody {
    #[serde(default)]
    messages: Vec<Value>,
}

async fn anthropic_messages(
    State(state): State<FakeLlmState>,
    headers: HeaderMap,
    Query(q): Query<ModeQuery>,
    Json(body): Json<AnthropicBody>,
) -> Response {
    state.hits.fetch_add(1, Ordering::Relaxed);
    let mode = mode_from(&headers, &q);
    if let Some(st) = fault_status(&mode) {
        return st.into_response();
    }
    if !auth_ok(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let last = body
        .messages
        .last()
        .and_then(|m| m.get("content"))
        .and_then(|c| match c {
            Value::String(s) => Some(s.as_str()),
            Value::Array(a) => a
                .first()
                .and_then(|x| x.get("text"))
                .and_then(|t| t.as_str()),
            _ => None,
        })
        .unwrap_or("ok");
    Json(json!({
        "id": "msg_fake",
        "type": "message",
        "role": "assistant",
        "content": [{ "type": "text", "text": format!("echo:{last}") }],
        "model": state.model,
        "stop_reason": "end_turn"
    }))
    .into_response()
}

async fn ollama_tags(State(state): State<FakeLlmState>) -> Json<Value> {
    Json(json!({
        "models": [
            { "name": state.model, "model": state.model },
            { "name": "fake-embed", "model": "fake-embed" }
        ]
    }))
}

async fn ollama_version() -> Json<Value> {
    Json(json!({ "version": "fake-0" }))
}

#[derive(Deserialize)]
struct OllamaChat {
    #[serde(default)]
    messages: Vec<HashMap<String, String>>,
}

async fn ollama_chat(
    State(state): State<FakeLlmState>,
    Json(body): Json<OllamaChat>,
) -> Json<Value> {
    state.hits.fetch_add(1, Ordering::Relaxed);
    let last = body
        .messages
        .last()
        .and_then(|m| m.get("content"))
        .map(|s| s.as_str())
        .unwrap_or("ok");
    Json(json!({
        "model": state.model,
        "message": { "role": "assistant", "content": format!("echo:{last}") },
        "done": true
    }))
}

/// Bind `127.0.0.1:0` and return `(bound_addr, join_handle)`.
pub async fn spawn_ephemeral(
    state: FakeLlmState,
) -> std::io::Result<(std::net::SocketAddr, tokio::task::JoinHandle<()>)> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let app = app(state);
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok((addr, handle))
}
