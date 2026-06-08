//! MCP Streamable HTTP transport handler.
//!
//! Exposes EdgeQuake as an MCP server over HTTP at `POST /mcp`.
//! Implements JSON-RPC 2.0 as required by the MCP Streamable HTTP spec.
//!
//! Supported methods:
//!   - `initialize`       — MCP handshake
//!   - `ping`             — keepalive
//!   - `tools/list`       — enumerate available tools
//!   - `tools/call`       — invoke a tool
//!   - `notifications/*`  — accepted and silently ignored

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use edgequake_query::{QueryMode, QueryRequest as EngineQueryRequest};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::warn;

use crate::state::AppState;

// ── JSON-RPC 2.0 types ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

impl JsonRpcResponse {
    fn ok(id: Option<Value>, result: Value) -> Self {
        Self { jsonrpc: "2.0".into(), result: Some(result), error: None, id }
    }
    fn err(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(RpcError { code, message: message.into() }),
            id,
        }
    }
}

// ── Tool schema constants ──────────────────────────────────────────────────────

const ALLOWED_TOPICS: &[&str] = &[
    "Politics", "Indonesia", "Psychology", "Business", "Communication",
    "Technology", "Science", "SocialMedia", "Religion", "Media",
    "AI", "Culinary", "Finance", "Literature", "Law",
    "Sports", "Education", "History", "Uncategorized",
];

fn tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "query",
                "description": "Execute a RAG query against the EdgeQuake knowledge graph. Returns an AI-generated answer with source references. Use 'hybrid' mode (default) for best results.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Natural language question"
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["naive", "local", "global", "hybrid", "mix"],
                            "description": "Query mode (default: hybrid)"
                        },
                        "max_results": {
                            "type": "integer",
                            "description": "Maximum number of source references to return"
                        },
                        "topic": {
                            "type": "string",
                            "enum": [
                                "Politics","Indonesia","Psychology","Business","Communication",
                                "Technology","Science","SocialMedia","Religion","Media",
                                "AI","Culinary","Finance","Literature","Law",
                                "Sports","Education","History","Uncategorized"
                            ],
                            "description": "Filter query to documents with this topic"
                        }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "list_documents",
                "description": "List documents stored in the EdgeQuake knowledge base.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of documents to return (default: 20, max: 100)"
                        }
                    }
                }
            }
        ]
    })
}

// ── Main handler ──────────────────────────────────────────────────────────────

pub async fn mcp_handler(
    State(state): State<AppState>,
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let id = req.id.clone();

    match req.method.as_str() {
        // Notifications have no response body
        m if m.starts_with("notifications/") => {
            return (StatusCode::OK, Json(json!(null))).into_response();
        }

        "initialize" => {
            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "EdgeQuake MCP",
                    "version": env!("CARGO_PKG_VERSION")
                }
            });
            Json(JsonRpcResponse::ok(id, result)).into_response()
        }

        "ping" => Json(JsonRpcResponse::ok(id, json!({}))).into_response(),

        "tools/list" => Json(JsonRpcResponse::ok(id, tools_list())).into_response(),

        "tools/call" => {
            let resp = handle_tool_call(&state, &req.params).await;
            Json(match resp {
                Ok(r) => JsonRpcResponse::ok(id, r),
                Err((code, msg)) => JsonRpcResponse::err(id, code, msg),
            })
            .into_response()
        }

        other => Json(JsonRpcResponse::err(
            id,
            -32601,
            format!("Method not found: {other}"),
        ))
        .into_response(),
    }
}

// ── Tool dispatch ─────────────────────────────────────────────────────────────

async fn handle_tool_call(
    state: &AppState,
    params: &Value,
) -> Result<Value, (i32, String)> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or((-32602, "Missing tool name".to_string()))?;

    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    match name {
        "query" => tool_query(state, &args).await,
        "list_documents" => tool_list_documents(state, &args).await,
        other => Err((-32602, format!("Unknown tool: {other}"))),
    }
}

// ── query tool ────────────────────────────────────────────────────────────────

async fn tool_query(state: &AppState, args: &Value) -> Result<Value, (i32, String)> {
    let query_text = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or((-32602, "Missing required parameter: query".to_string()))?;

    let mode = args
        .get("mode")
        .and_then(|v| v.as_str())
        .and_then(QueryMode::parse)
        .unwrap_or(QueryMode::Hybrid);

    let mut engine_req = EngineQueryRequest::new(query_text).with_mode(mode);

    // Apply topic filter if provided and valid
    if let Some(topic) = args.get("topic").and_then(|v| v.as_str()) {
        if ALLOWED_TOPICS.iter().any(|&t| t.eq_ignore_ascii_case(topic)) {
            match crate::handlers::query::topic_resolver::resolve_topic_filter(
                state.kv_storage.as_ref(),
                topic,
                None,
                None,
            )
            .await
            {
                Ok(Some(doc_ids)) => {
                    engine_req = engine_req.with_allowed_document_ids(doc_ids);
                }
                Ok(None) => {}
                Err(e) => {
                    warn!(error = %e, "MCP topic filter failed — proceeding unscoped");
                }
            }
        }
    }

    match state.sota_engine.query(engine_req).await {
        Ok(result) => {
            let chunks = &result.context.chunks;
            let sources_text = if !chunks.is_empty() {
                let refs: Vec<String> = chunks
                    .iter()
                    .take(5)
                    .enumerate()
                    .map(|(i, c)| {
                        format!("[{}] {}", i + 1, c.content.chars().take(150).collect::<String>())
                    })
                    .collect();
                format!("\n\n---\n**Sources:**\n{}", refs.join("\n"))
            } else {
                String::new()
            };

            let text = format!("{}{}", result.answer, sources_text);
            Ok(json!({ "content": [{ "type": "text", "text": text }] }))
        }
        Err(e) => Ok(json!({
            "content": [{ "type": "text", "text": format!("Query failed: {e}") }],
            "isError": true
        })),
    }
}

// ── list_documents tool ───────────────────────────────────────────────────────

async fn tool_list_documents(state: &AppState, args: &Value) -> Result<Value, (i32, String)> {
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(20)
        .min(100) as usize;

    let keys = state
        .kv_storage
        .keys_with_suffix("-metadata")
        .await
        .map_err(|e| (-32603, format!("Storage error: {e}")))?;

    let mut docs = Vec::with_capacity(limit);
    for key in keys.iter().take(limit) {
        let doc_id = key.trim_end_matches("-metadata");
        if let Ok(Some(meta)) = state.kv_storage.get_by_id(key).await {
            docs.push(json!({
                "id": doc_id,
                "topic": meta.get("enrichment_topic").and_then(|v| v.as_str()).unwrap_or(""),
                "language": meta.get("enrichment_language").and_then(|v| v.as_str()).unwrap_or(""),
                "status": meta.get("enrichment_status").and_then(|v| v.as_str()).unwrap_or(""),
                "summary": meta.get("enrichment_summary").and_then(|v| v.as_str()).unwrap_or(""),
            }));
        }
    }

    let text = serde_json::to_string_pretty(&docs).unwrap_or_else(|_| "[]".to_string());
    Ok(json!({ "content": [{ "type": "text", "text": text }] }))
}
