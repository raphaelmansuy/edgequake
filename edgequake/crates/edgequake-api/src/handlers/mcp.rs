//! MCP Streamable HTTP transport handler.
//!
//! Exposes EdgeQuake as an MCP server at `POST /mcp`.
//! Implements JSON-RPC 2.0 as required by the MCP Streamable HTTP spec.
//! Tool parity with the stdio npm package (@romysaputrasihanandaa/edgequake-mcp).

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use edgequake_core::{CreateWorkspaceRequest, WorkspaceService};
use edgequake_query::{QueryMode, QueryRequest as EngineQueryRequest};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::warn;
use uuid::Uuid;

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

// ── Allowed topics ────────────────────────────────────────────────────────────

const ALLOWED_TOPICS: &[&str] = &[
    "Politics", "Indonesia", "Psychology", "Business", "Communication",
    "Technology", "Science", "SocialMedia", "Religion", "Media",
    "AI", "Culinary", "Finance", "Literature", "Law",
    "Sports", "Education", "History", "Uncategorized",
];

// ── Tool schema ───────────────────────────────────────────────────────────────

fn tools_list() -> Value {
    json!({
        "tools": [
            // ── Query ──────────────────────────────────────────────────────
            {
                "name": "query",
                "description": "Execute a RAG query against the EdgeQuake knowledge graph. Returns an AI-generated answer with source references. Use 'hybrid' mode (default) for best results combining local entity graph traversal with global semantic search.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Natural language question" },
                        "mode": {
                            "type": "string",
                            "enum": ["naive","local","global","hybrid","mix"],
                            "description": "Query mode: naive (vector-only), local (entity graph), global (community search), hybrid (local+global, default), mix (weighted blend)"
                        },
                        "max_results": { "type": "integer", "description": "Maximum number of source references to return" },
                        "include_references": { "type": "boolean", "description": "Include source snippets in response (default: true)" },
                        "topic": {
                            "type": "string",
                            "enum": ["Politics","Indonesia","Psychology","Business","Communication","Technology","Science","SocialMedia","Religion","Media","AI","Culinary","Finance","Literature","Law","Sports","Education","History","Uncategorized"],
                            "description": "Filter query to documents with this enrichment topic"
                        }
                    },
                    "required": ["query"]
                }
            },
            // ── Documents ─────────────────────────────────────────────────
            {
                "name": "document_upload",
                "description": "Upload a text document to EdgeQuake for knowledge graph extraction. The document will be chunked, entities extracted, and relationships mapped.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "content": { "type": "string", "description": "Document text content" },
                        "title": { "type": "string", "description": "Document title" }
                    },
                    "required": ["content"]
                }
            },
            {
                "name": "document_list",
                "description": "List documents with pagination and optional filtering",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "page": { "type": "integer", "description": "Page number (default: 1)" },
                        "page_size": { "type": "integer", "description": "Items per page (default: 20)" },
                        "status": {
                            "type": "string",
                            "enum": ["pending","processing","completed","failed"],
                            "description": "Filter by processing status"
                        },
                        "search": { "type": "string", "description": "Search in title" }
                    }
                }
            },
            {
                "name": "document_get",
                "description": "Get document details including metadata and enrichment data",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "document_id": { "type": "string", "description": "Document UUID" }
                    },
                    "required": ["document_id"]
                }
            },
            {
                "name": "document_delete",
                "description": "Delete a document and its extracted knowledge (entities, relationships, chunks)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "document_id": { "type": "string", "description": "Document UUID" }
                    },
                    "required": ["document_id"]
                }
            },
            {
                "name": "document_status",
                "description": "Check the processing status of a document",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "document_id": { "type": "string", "description": "Document UUID" }
                    },
                    "required": ["document_id"]
                }
            },
            // ── Graph ─────────────────────────────────────────────────────
            {
                "name": "graph_search_entities",
                "description": "Search for entities in the knowledge graph. Entities are people, organizations, technologies, concepts, etc. extracted from documents.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "search": { "type": "string", "description": "Search term to filter entities by name or description" },
                        "label": { "type": "string", "description": "Filter by entity type: PERSON, ORGANIZATION, TECHNOLOGY, CONCEPT, EVENT, LOCATION, PRODUCT" },
                        "limit": { "type": "integer", "description": "Max results (default: 20)" }
                    }
                }
            },
            {
                "name": "graph_get_entity",
                "description": "Get detailed information about a specific entity including its properties",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity_name": { "type": "string", "description": "Entity name (e.g. RUST, OPENAI, MACHINE_LEARNING)" }
                    },
                    "required": ["entity_name"]
                }
            },
            {
                "name": "graph_entity_neighborhood",
                "description": "Get an entity's neighborhood — all directly connected entities and their relationships.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity_name": { "type": "string", "description": "Entity name" }
                    },
                    "required": ["entity_name"]
                }
            },
            {
                "name": "graph_search_relationships",
                "description": "Search relationships between entities in the knowledge graph",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source": { "type": "string", "description": "Source entity name (partial match)" },
                        "target": { "type": "string", "description": "Target entity name (partial match)" },
                        "label": { "type": "string", "description": "Relationship type/label (partial match)" },
                        "limit": { "type": "integer", "description": "Max results (default: 20)" }
                    }
                }
            },
            // ── Health ────────────────────────────────────────────────────
            {
                "name": "health",
                "description": "Check EdgeQuake server health and component status",
                "inputSchema": { "type": "object", "properties": {} }
            },
            // ── Workspaces ────────────────────────────────────────────────
            {
                "name": "workspace_list",
                "description": "List all workspaces across all tenants",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "workspace_create",
                "description": "Create a new workspace for document ingestion and knowledge graph",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Workspace name" },
                        "description": { "type": "string", "description": "Workspace description" },
                        "tenant_id": { "type": "string", "description": "Tenant UUID (uses first available tenant if omitted)" },
                        "llm_model": { "type": "string", "description": "LLM model for extraction" },
                        "llm_provider": { "type": "string", "description": "LLM provider (ollama, openai, lmstudio)" }
                    },
                    "required": ["name"]
                }
            },
            {
                "name": "workspace_get",
                "description": "Get workspace details",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "workspace_id": { "type": "string", "description": "Workspace UUID" }
                    },
                    "required": ["workspace_id"]
                }
            },
            {
                "name": "workspace_delete",
                "description": "Delete a workspace and all its data (documents, entities, relationships)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "workspace_id": { "type": "string", "description": "Workspace UUID" }
                    },
                    "required": ["workspace_id"]
                }
            },
            {
                "name": "workspace_stats",
                "description": "Get statistics for a workspace (document, entity, relationship, chunk counts)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "workspace_id": { "type": "string", "description": "Workspace UUID" }
                    },
                    "required": ["workspace_id"]
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

    if req.method.starts_with("notifications/") {
        return (StatusCode::OK, Json(json!(null))).into_response();
    }

    match req.method.as_str() {
        "initialize" => Json(JsonRpcResponse::ok(id, json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "EdgeQuake MCP",
                "version": env!("CARGO_PKG_VERSION")
            }
        }))).into_response(),

        "ping" => Json(JsonRpcResponse::ok(id, json!({}))).into_response(),

        "tools/list" => Json(JsonRpcResponse::ok(id, tools_list())).into_response(),

        "tools/call" => {
            let resp = dispatch_tool(&state, &req.params).await;
            Json(match resp {
                Ok(r) => JsonRpcResponse::ok(id, r),
                Err((code, msg)) => JsonRpcResponse::err(id, code, msg),
            }).into_response()
        }

        other => Json(JsonRpcResponse::err(id, -32601, format!("Method not found: {other}"))).into_response(),
    }
}

// ── Tool dispatch ─────────────────────────────────────────────────────────────

async fn dispatch_tool(state: &AppState, params: &Value) -> Result<Value, (i32, String)> {
    let name = params.get("name").and_then(|v| v.as_str())
        .ok_or((-32602, "Missing tool name".to_string()))?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    match name {
        "query"                      => tool_query(state, &args).await,
        "document_upload"            => tool_document_upload(state, &args).await,
        "document_list"              => tool_document_list(state, &args).await,
        "document_get"               => tool_document_get(state, &args).await,
        "document_delete"            => tool_document_delete(state, &args).await,
        "document_status"            => tool_document_status(state, &args).await,
        "graph_search_entities"      => tool_graph_search_entities(state, &args).await,
        "graph_get_entity"           => tool_graph_get_entity(state, &args).await,
        "graph_entity_neighborhood"  => tool_graph_neighborhood(state, &args).await,
        "graph_search_relationships" => tool_graph_search_relationships(state, &args).await,
        "health"                     => tool_health(state).await,
        "workspace_list"             => tool_workspace_list(state).await,
        "workspace_create"           => tool_workspace_create(state, &args).await,
        "workspace_get"              => tool_workspace_get(state, &args).await,
        "workspace_delete"           => tool_workspace_delete(state, &args).await,
        "workspace_stats"            => tool_workspace_stats(state, &args).await,
        other => Err((-32602, format!("Unknown tool: {other}"))),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn text_result(text: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": text.into() }] })
}

fn json_result(v: &Value) -> Value {
    text_result(serde_json::to_string_pretty(v).unwrap_or_else(|_| "{}".to_string()))
}

fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, (i32, String)> {
    args.get(key).and_then(|v| v.as_str())
        .ok_or_else(|| (-32602, format!("Missing required parameter: {key}")))
}

fn node_label(node: &edgequake_storage::traits::GraphNode) -> &str {
    node.properties.get("entity_type")
        .or_else(|| node.properties.get("label"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
}

fn edge_label(edge: &edgequake_storage::traits::GraphEdge) -> &str {
    edge.properties.get("label")
        .or_else(|| edge.properties.get("relationship_type"))
        .or_else(|| edge.properties.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
}

fn edge_weight(edge: &edgequake_storage::traits::GraphEdge) -> f64 {
    edge.properties.get("weight")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0)
}

// ── query ─────────────────────────────────────────────────────────────────────

async fn tool_query(state: &AppState, args: &Value) -> Result<Value, (i32, String)> {
    let query_text = require_str(args, "query")?;

    let mode = args.get("mode").and_then(|v| v.as_str())
        .and_then(QueryMode::parse)
        .unwrap_or(QueryMode::Hybrid);

    let mut engine_req = EngineQueryRequest::new(query_text).with_mode(mode);

    if let Some(topic) = args.get("topic").and_then(|v| v.as_str()) {
        if ALLOWED_TOPICS.iter().any(|&t| t.eq_ignore_ascii_case(topic)) {
            match crate::handlers::query::topic_resolver::resolve_topic_filter(
                state.kv_storage.as_ref(), topic, None, None,
            ).await {
                Ok(Some(ids)) => { engine_req = engine_req.with_allowed_document_ids(ids); }
                Ok(None) => {}
                Err(e) => { warn!(error = %e, "MCP topic filter failed — unscoped"); }
            }
        }
    }

    match state.sota_engine.query(engine_req).await {
        Ok(result) => {
            let chunks = &result.context.chunks;
            let sources = if !chunks.is_empty() {
                let refs: Vec<String> = chunks.iter().take(5).enumerate()
                    .map(|(i, c)| format!("[{}] {}", i + 1, c.content.chars().take(200).collect::<String>()))
                    .collect();
                format!("\n\n---\n**Sources:**\n{}", refs.join("\n\n"))
            } else {
                String::new()
            };
            Ok(text_result(format!("{}{}", result.answer, sources)))
        }
        Err(e) => Ok(json!({
            "content": [{ "type": "text", "text": format!("Query failed: {e}") }],
            "isError": true
        })),
    }
}

// ── document_upload ───────────────────────────────────────────────────────────

async fn tool_document_upload(state: &AppState, args: &Value) -> Result<Value, (i32, String)> {
    let content = require_str(args, "content")?;
    let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
    let document_id = Uuid::new_v4().to_string();

    let _ = state.kv_storage.upsert(&[(
        format!("{}-metadata", document_id),
        json!({
            "document_id": document_id,
            "title": title,
            "status": "processing",
            "created_at": chrono::Utc::now().to_rfc3339(),
        }),
    )]).await;

    match state.pipeline.process_with_resilience(&document_id, content, None).await {
        Ok(result) => Ok(json_result(&json!({
            "document_id": document_id,
            "status": "completed",
            "chunk_count": result.stats.chunk_count,
            "entity_count": result.stats.entity_count,
            "relationship_count": result.stats.relationship_count,
        }))),
        Err(e) => Ok(json!({
            "content": [{ "type": "text", "text": format!("Upload failed: {e}") }],
            "isError": true
        })),
    }
}

// ── document_list ─────────────────────────────────────────────────────────────

async fn tool_document_list(state: &AppState, args: &Value) -> Result<Value, (i32, String)> {
    let page = args.get("page").and_then(|v| v.as_u64()).unwrap_or(1).max(1) as usize;
    let page_size = args.get("page_size").and_then(|v| v.as_u64()).unwrap_or(20).min(100) as usize;
    let status_filter = args.get("status").and_then(|v| v.as_str());
    let search_filter = args.get("search").and_then(|v| v.as_str()).map(|s| s.to_lowercase());

    let keys = state.kv_storage.keys_with_suffix("-metadata").await
        .map_err(|e| (-32603, format!("Storage error: {e}")))?;

    let mut docs = Vec::new();
    for key in &keys {
        if let Ok(Some(meta)) = state.kv_storage.get_by_id(key).await {
            let doc_id = key.trim_end_matches("-metadata");
            let doc_status = meta.get("enrichment_status").or_else(|| meta.get("status"))
                .and_then(|v| v.as_str()).unwrap_or("");
            let title = meta.get("title").and_then(|v| v.as_str()).unwrap_or(doc_id);

            if let Some(sf) = status_filter {
                if doc_status != sf { continue; }
            }
            if let Some(ref q) = search_filter {
                if !title.to_lowercase().contains(q.as_str()) { continue; }
            }

            docs.push(json!({
                "id": doc_id,
                "title": title,
                "status": doc_status,
                "topic": meta.get("enrichment_topic").and_then(|v| v.as_str()).unwrap_or(""),
                "language": meta.get("enrichment_language").and_then(|v| v.as_str()).unwrap_or(""),
                "summary": meta.get("enrichment_summary").and_then(|v| v.as_str()).unwrap_or(""),
                "created_at": meta.get("created_at").and_then(|v| v.as_str()).unwrap_or(""),
            }));
        }
    }

    let total = docs.len();
    let start = ((page - 1) * page_size).min(total);
    let end = (start + page_size).min(total);

    Ok(json_result(&json!({
        "documents": &docs[start..end],
        "total": total,
        "page": page,
        "page_size": page_size,
        "has_more": end < total,
    })))
}

// ── document_get ──────────────────────────────────────────────────────────────

async fn tool_document_get(state: &AppState, args: &Value) -> Result<Value, (i32, String)> {
    let doc_id = require_str(args, "document_id")?;
    let key = format!("{}-metadata", doc_id);

    match state.kv_storage.get_by_id(&key).await {
        Ok(Some(meta)) => Ok(json_result(&json!({
            "id": doc_id,
            "title": meta.get("title").and_then(|v| v.as_str()).unwrap_or(doc_id),
            "status": meta.get("enrichment_status").or_else(|| meta.get("status")).and_then(|v| v.as_str()).unwrap_or(""),
            "topic": meta.get("enrichment_topic").and_then(|v| v.as_str()).unwrap_or(""),
            "tags": meta.get("enrichment_tags").cloned().unwrap_or(json!([])),
            "language": meta.get("enrichment_language").and_then(|v| v.as_str()).unwrap_or(""),
            "summary": meta.get("enrichment_summary").and_then(|v| v.as_str()).unwrap_or(""),
            "keywords": meta.get("enrichment_keywords").cloned().unwrap_or(json!([])),
            "enrichment_completed_at": meta.get("enrichment_completed_at").and_then(|v| v.as_str()).unwrap_or(""),
            "created_at": meta.get("created_at").and_then(|v| v.as_str()).unwrap_or(""),
        }))),
        Ok(None) => Ok(json!({
            "content": [{ "type": "text", "text": format!("Document not found: {doc_id}") }],
            "isError": true
        })),
        Err(e) => Err((-32603, format!("Storage error: {e}"))),
    }
}

// ── document_delete ───────────────────────────────────────────────────────────

async fn tool_document_delete(state: &AppState, args: &Value) -> Result<Value, (i32, String)> {
    let doc_id = require_str(args, "document_id")?;

    let chunk_keys = state.kv_storage.keys_with_prefix(&format!("{}-chunk-", doc_id)).await
        .map_err(|e| (-32603, format!("Storage error: {e}")))?;

    let mut keys_to_delete: Vec<String> = chunk_keys;
    keys_to_delete.push(format!("{}-metadata", doc_id));
    keys_to_delete.push(format!("{}-summary", doc_id));

    let _ = state.kv_storage.delete(&keys_to_delete).await;
    let _ = state.vector_storage.delete(&keys_to_delete).await;

    // Remove graph nodes linked to this document (best-effort)
    let entity_key = format!("{}-entities", doc_id);
    if let Ok(Some(entity_list)) = state.kv_storage.get_by_id(&entity_key).await {
        if let Some(arr) = entity_list.as_array() {
            for v in arr {
                if let Some(entity_id) = v.as_str() {
                    let _ = state.graph_storage.delete_node(entity_id).await;
                }
            }
        }
    }

    Ok(json_result(&json!({ "success": true, "document_id": doc_id })))
}

// ── document_status ───────────────────────────────────────────────────────────

async fn tool_document_status(state: &AppState, args: &Value) -> Result<Value, (i32, String)> {
    let doc_id = require_str(args, "document_id")?;
    let key = format!("{}-metadata", doc_id);

    match state.kv_storage.get_by_id(&key).await {
        Ok(Some(meta)) => Ok(json_result(&json!({
            "id": doc_id,
            "status": meta.get("enrichment_status").or_else(|| meta.get("status")).and_then(|v| v.as_str()).unwrap_or("unknown"),
            "enrichment_completed_at": meta.get("enrichment_completed_at").and_then(|v| v.as_str()),
            "enrichment_error": meta.get("enrichment_error").and_then(|v| v.as_str()),
        }))),
        Ok(None) => Ok(json!({
            "content": [{ "type": "text", "text": format!("Document not found: {doc_id}") }],
            "isError": true
        })),
        Err(e) => Err((-32603, format!("Storage error: {e}"))),
    }
}

// ── graph_search_entities ─────────────────────────────────────────────────────

async fn tool_graph_search_entities(state: &AppState, args: &Value) -> Result<Value, (i32, String)> {
    let search = args.get("search").and_then(|v| v.as_str()).unwrap_or("");
    let label = args.get("label").and_then(|v| v.as_str());
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

    let results = state.graph_storage
        .search_nodes(search, limit, label, None, None)
        .await
        .map_err(|e| (-32603, format!("Graph error: {e}")))?;

    let entities: Vec<Value> = results.iter().map(|(n, degree)| json!({
        "name": n.id,
        "label": node_label(n),
        "description": n.properties.get("description").and_then(|v| v.as_str()).unwrap_or(""),
        "degree": degree,
    })).collect();

    Ok(json_result(&json!(entities)))
}

// ── graph_get_entity ──────────────────────────────────────────────────────────

async fn tool_graph_get_entity(state: &AppState, args: &Value) -> Result<Value, (i32, String)> {
    let name = require_str(args, "entity_name")?;

    match state.graph_storage.get_node(name).await {
        Ok(Some(node)) => Ok(json_result(&json!({
            "name": node.id,
            "label": node_label(&node),
            "description": node.properties.get("description").and_then(|v| v.as_str()).unwrap_or(""),
            "properties": node.properties,
        }))),
        Ok(None) => Ok(json!({
            "content": [{ "type": "text", "text": format!("Entity not found: {name}") }],
            "isError": true
        })),
        Err(e) => Err((-32603, format!("Graph error: {e}"))),
    }
}

// ── graph_entity_neighborhood ─────────────────────────────────────────────────

async fn tool_graph_neighborhood(state: &AppState, args: &Value) -> Result<Value, (i32, String)> {
    let name = require_str(args, "entity_name")?;

    let center = state.graph_storage.get_node(name).await
        .map_err(|e| (-32603, format!("Graph error: {e}")))?
        .ok_or_else(|| (-32602, format!("Entity not found: {name}")))?;

    let edges = state.graph_storage.get_node_edges(&center.id).await
        .map_err(|e| (-32603, format!("Graph error: {e}")))?;

    let mut neighbor_ids: Vec<String> = edges.iter().map(|e| {
        if e.source == center.id { e.target.clone() } else { e.source.clone() }
    }).collect();
    neighbor_ids.dedup();

    let mut neighbors = Vec::new();
    for neighbor_id in neighbor_ids.iter().take(20) {
        if let Ok(Some(node)) = state.graph_storage.get_node(neighbor_id).await {
            let rel = edges.iter().find(|e| &e.source == neighbor_id || &e.target == neighbor_id);
            neighbors.push(json!({
                "entity": {
                    "name": node.id,
                    "label": node_label(&node),
                    "description": node.properties.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                },
                "relationship": rel.map(|r| edge_label(r)).unwrap_or(""),
                "direction": rel.map(|r| if r.source == center.id { "outgoing" } else { "incoming" }).unwrap_or(""),
            }));
        }
    }

    Ok(json_result(&json!({
        "center": {
            "name": center.id,
            "label": node_label(&center),
            "description": center.properties.get("description").and_then(|v| v.as_str()).unwrap_or(""),
        },
        "neighbors": neighbors,
    })))
}

// ── graph_search_relationships ────────────────────────────────────────────────

async fn tool_graph_search_relationships(state: &AppState, args: &Value) -> Result<Value, (i32, String)> {
    let source_filter = args.get("source").and_then(|v| v.as_str());
    let target_filter = args.get("target").and_then(|v| v.as_str());
    let label_filter = args.get("label").and_then(|v| v.as_str()).map(|s| s.to_uppercase());
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

    let edges = if let Some(src) = source_filter {
        state.graph_storage.get_node_edges(src).await
            .map_err(|e| (-32603, format!("Graph error: {e}")))?
    } else if let Some(tgt) = target_filter {
        state.graph_storage.get_node_edges(tgt).await
            .map_err(|e| (-32603, format!("Graph error: {e}")))?
    } else {
        state.graph_storage.get_all_edges().await
            .map_err(|e| (-32603, format!("Graph error: {e}")))?
    };

    let filtered: Vec<Value> = edges.iter()
        .filter(|e| {
            source_filter.map_or(true, |s| e.source.to_lowercase().contains(&s.to_lowercase()))
                && target_filter.map_or(true, |t| e.target.to_lowercase().contains(&t.to_lowercase()))
                && label_filter.as_deref().map_or(true, |l| edge_label(e).to_uppercase().contains(l))
        })
        .take(limit)
        .map(|e| json!({
            "source": e.source,
            "target": e.target,
            "label": edge_label(e),
            "description": e.properties.get("description").and_then(|v| v.as_str()).unwrap_or(""),
            "weight": edge_weight(e),
        }))
        .collect();

    Ok(json_result(&json!(filtered)))
}

// ── health ────────────────────────────────────────────────────────────────────

async fn tool_health(state: &AppState) -> Result<Value, (i32, String)> {
    let kv_ok = state.kv_storage.ping().await.is_ok();
    let vector_ok = state.vector_storage.ping().await.is_ok();
    let graph_ok = state.graph_storage.ping().await.is_ok();

    Ok(json_result(&json!({
        "status": if kv_ok && vector_ok && graph_ok { "healthy" } else { "degraded" },
        "components": {
            "kv_storage": if kv_ok { "ok" } else { "error" },
            "vector_storage": if vector_ok { "ok" } else { "error" },
            "graph_storage": if graph_ok { "ok" } else { "error" },
            "llm_provider": "ok",
        }
    })))
}

// ── workspace_list ────────────────────────────────────────────────────────────

async fn tool_workspace_list(state: &AppState) -> Result<Value, (i32, String)> {
    let tenants = state.workspace_service.list_tenants(100, 0).await
        .map_err(|e| (-32603, format!("Workspace service error: {e}")))?;

    let mut all_workspaces = Vec::new();
    for tenant in &tenants {
        if let Ok(workspaces) = state.workspace_service.list_workspaces(tenant.tenant_id).await {
            for w in workspaces {
                all_workspaces.push(json!({
                    "id": w.workspace_id,
                    "name": w.name,
                    "slug": w.slug,
                    "description": w.description,
                    "tenant_id": tenant.tenant_id,
                    "tenant_name": tenant.name,
                    "llm_provider": w.llm_provider,
                    "llm_model": w.llm_model,
                }));
            }
        }
    }

    Ok(json_result(&json!(all_workspaces)))
}

// ── workspace_create ──────────────────────────────────────────────────────────

async fn tool_workspace_create(state: &AppState, args: &Value) -> Result<Value, (i32, String)> {
    let name = require_str(args, "name")?;

    let tenant_id = if let Some(id_str) = args.get("tenant_id").and_then(|v| v.as_str()) {
        Uuid::parse_str(id_str).map_err(|_| (-32602, "Invalid tenant_id UUID".to_string()))?
    } else {
        let tenants = state.workspace_service.list_tenants(1, 0).await
            .map_err(|e| (-32603, format!("Workspace service error: {e}")))?;
        tenants.first().map(|t| t.tenant_id)
            .ok_or((-32603, "No tenants found".to_string()))?
    };

    let create_req = CreateWorkspaceRequest {
        name: name.to_string(),
        slug: None,
        description: args.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
        max_documents: None,
        llm_model: args.get("llm_model").and_then(|v| v.as_str()).map(|s| s.to_string()),
        llm_provider: args.get("llm_provider").and_then(|v| v.as_str()).map(|s| s.to_string()),
        embedding_model: None,
        embedding_provider: None,
        embedding_dimension: None,
        vision_llm_model: None,
        vision_llm_provider: None,
        pdf_parser_backend: None,
        entity_types: None,
        entity_types_strict: None,
    };

    let workspace = state.workspace_service.create_workspace(tenant_id, create_req).await
        .map_err(|e| (-32603, format!("Create workspace failed: {e}")))?;

    Ok(json_result(&json!({
        "id": workspace.workspace_id,
        "name": workspace.name,
        "slug": workspace.slug,
        "tenant_id": tenant_id,
    })))
}

// ── workspace_get ─────────────────────────────────────────────────────────────

async fn tool_workspace_get(state: &AppState, args: &Value) -> Result<Value, (i32, String)> {
    let id_str = require_str(args, "workspace_id")?;
    let workspace_id = Uuid::parse_str(id_str).map_err(|_| (-32602, "Invalid workspace_id UUID".to_string()))?;

    match state.workspace_service.get_workspace(workspace_id).await {
        Ok(Some(w)) => Ok(json_result(&json!({
            "id": w.workspace_id,
            "name": w.name,
            "slug": w.slug,
            "description": w.description,
            "llm_provider": w.llm_provider,
            "llm_model": w.llm_model,
            "embedding_provider": w.embedding_provider,
            "embedding_model": w.embedding_model,
        }))),
        Ok(None) => Ok(json!({
            "content": [{ "type": "text", "text": format!("Workspace not found: {id_str}") }],
            "isError": true
        })),
        Err(e) => Err((-32603, format!("Workspace service error: {e}"))),
    }
}

// ── workspace_delete ──────────────────────────────────────────────────────────

async fn tool_workspace_delete(state: &AppState, args: &Value) -> Result<Value, (i32, String)> {
    let id_str = require_str(args, "workspace_id")?;
    let workspace_id = Uuid::parse_str(id_str).map_err(|_| (-32602, "Invalid workspace_id UUID".to_string()))?;

    state.workspace_service.delete_workspace(workspace_id).await
        .map_err(|e| (-32603, format!("Delete workspace failed: {e}")))?;

    Ok(json_result(&json!({ "success": true, "workspace_id": id_str })))
}

// ── workspace_stats ───────────────────────────────────────────────────────────

async fn tool_workspace_stats(state: &AppState, args: &Value) -> Result<Value, (i32, String)> {
    let id_str = require_str(args, "workspace_id")?;
    let workspace_id = Uuid::parse_str(id_str).map_err(|_| (-32602, "Invalid workspace_id UUID".to_string()))?;

    let stats = state.workspace_service.get_workspace_stats(workspace_id).await
        .map_err(|e| (-32603, format!("Workspace service error: {e}")))?;

    Ok(json_result(&json!({
        "workspace_id": id_str,
        "document_count": stats.document_count,
        "entity_count": stats.entity_count,
        "relationship_count": stats.relationship_count,
        "chunk_count": stats.chunk_count,
        "embedding_count": stats.embedding_count,
        "storage_bytes": stats.storage_bytes,
    })))
}
