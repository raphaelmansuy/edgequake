//! MCP method dispatch — tools, legacy shim, deprecated rejection.

use std::sync::Arc;

use edgequake_auth::Role;
use edgequake_llm::traits::LLMProvider;
use edgequake_observability::PropagationHeaders;
use serde_json::{json, Value};
use tracing::{debug, info_span};

use crate::error::{ApiError, ApiResult};
use crate::handlers::context_types::{
    ContentGranularity, ContextRetrievalRequest, ContextSearchRequest,
};
use crate::middleware::TenantContext;
use crate::services::query_context::{
    fetch_context_by_id, resolve_keyword_llm_override, retrieve_context, search_context,
    FetchContextOptions,
};
use crate::state::AppState;

use super::json_rpc::{GatewayError, DEPRECATED_METHODS, PROTOCOL_2025_11_25, PROTOCOL_2026_07_28};
use super::meta::{propagation_from_meta, RequestMeta};
use super::tool_validation::validate_tool_call_with_role;
use super::tools::tools_list_result;
use super::workspace_policy::enforce_workspace_claim;

pub struct DispatchContext<'a> {
    pub state: &'a AppState,
    pub tenant_ctx: &'a TenantContext,
    pub protocol_version: &'a str,
    pub meta: &'a RequestMeta,
    pub workspace_header: Option<String>,
    pub auth_role: Option<Role>,
}

/// Owned context for async tool execution (SSE worker / cancellation).
#[derive(Clone)]
pub struct DispatchTaskContext {
    pub state: AppState,
    pub tenant_ctx: TenantContext,
    pub meta: RequestMeta,
    pub workspace_header: Option<String>,
    pub auth_role: Option<Role>,
}

impl<'a> DispatchContext<'a> {
    pub fn clone_for_task(&self) -> DispatchTaskContext {
        DispatchTaskContext {
            state: self.state.clone(),
            tenant_ctx: self.tenant_ctx.clone(),
            meta: self.meta.clone(),
            workspace_header: self.workspace_header.clone(),
            auth_role: self.auth_role.clone(),
        }
    }
}

pub async fn dispatch_method(
    ctx: &DispatchContext<'_>,
    method: &str,
    params: Option<Value>,
) -> Result<Value, GatewayError> {
    if DEPRECATED_METHODS.contains(&method) {
        return Err(GatewayError::transport(
            axum::http::StatusCode::NOT_FOUND,
            -32601,
            format!("Method not found (deprecated): {method}"),
        ));
    }

    match method {
        "tools/list" => Ok(tools_list_result()),
        "tools/call" => tools_call(ctx, params).await,
        "initialize" => Ok(legacy_initialize(ctx.protocol_version)),
        "server/discover" => Ok(server_discover(ctx.protocol_version)),
        "ping" => Ok(json!({})),
        other => Err(GatewayError::transport(
            axum::http::StatusCode::OK,
            -32601,
            format!("Method not found: {other}"),
        )),
    }
}

fn legacy_initialize(protocol_version: &str) -> Value {
    json!({
        "protocolVersion": protocol_version,
        "capabilities": {
            "tools": { "listChanged": false }
        },
        "serverInfo": {
            "name": "edgequake-mcp",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

fn server_discover(protocol_version: &str) -> Value {
    json!({
        "protocolVersion": protocol_version,
        "capabilities": {
            "tools": { "listChanged": false }
        },
        "serverInfo": {
            "name": "edgequake-mcp",
            "version": env!("CARGO_PKG_VERSION")
        },
        "supportedProtocolVersions": [PROTOCOL_2026_07_28, PROTOCOL_2025_11_25]
    })
}

async fn tools_call(
    ctx: &DispatchContext<'_>,
    params: Option<Value>,
) -> Result<Value, GatewayError> {
    let params =
        params.ok_or_else(|| GatewayError::Api(ApiError::BadRequest("Missing params".into())))?;
    execute_tool_call(ctx.clone_for_task(), params).await
}

/// Validate and execute a tools/call (shared by JSON and SSE paths).
pub async fn execute_tool_call(
    ctx: DispatchTaskContext,
    params: Value,
) -> Result<Value, GatewayError> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| GatewayError::Api(ApiError::BadRequest("Missing tool name".into())))?;

    let mut arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    if let Some(ws) = &ctx.workspace_header {
        if arguments.get("workspace_id").is_none() {
            if let Some(obj) = arguments.as_object_mut() {
                obj.insert("workspace_id".to_string(), json!(ws));
            }
        }
    }

    enforce_workspace_claim(&ctx.tenant_ctx, &arguments, ctx.auth_role.clone())?;
    validate_tool_call_with_role(name, &arguments, ctx.auth_role.clone())?;

    let span = info_span!(
        "mcp.tools.call",
        mcp.tool.name = name,
        mcp.client.name = ctx.meta.client_name.as_deref().unwrap_or("unknown"),
        otel.name = "mcp_tools_call",
    );
    let _guard = span.enter();

    if let Some(tp) = &ctx.meta.traceparent {
        debug!(traceparent = %tp, "MCP trace context received");
    }

    let propagation = propagation_from_meta(&ctx.meta);
    execute_tool(&ctx.state, &ctx.tenant_ctx, name, arguments, &propagation)
        .await
        .map_err(GatewayError::Api)
}

async fn execute_tool(
    state: &AppState,
    tenant_ctx: &TenantContext,
    name: &str,
    arguments: Value,
    propagation: &PropagationHeaders,
) -> ApiResult<Value> {
    let workspace =
        crate::handlers::query::resolve_query_workspace(state, tenant_ctx.workspace_id.as_deref())
            .await?;

    let propagation = propagation.clone();
    let llm_override: Option<Arc<dyn LLMProvider>> =
        resolve_keyword_llm_override(state, workspace.as_ref(), &propagation, None, None)?;

    match name {
        "edgequake_search" => {
            let req: ContextSearchRequest = serde_json::from_value(arguments)
                .map_err(|e| ApiError::BadRequest(format!("Invalid arguments: {e}")))?;
            let resp = search_context(state, tenant_ctx, req, llm_override).await?;
            Ok(serde_json::to_value(resp).unwrap())
        }
        "edgequake_fetch" => {
            let retrieval_id = arguments
                .get("retrieval_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ApiError::BadRequest("Missing retrieval_id".into()))?;
            let granularity = arguments
                .get("content_granularity")
                .and_then(|v| v.as_str())
                .map(parse_granularity)
                .unwrap_or(ContentGranularity::Agent);
            let include_subgraph = arguments
                .get("include_subgraph")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let resp = fetch_context_by_id(
                retrieval_id,
                FetchContextOptions {
                    granularity,
                    include_subgraph,
                },
            )?;
            Ok(serde_json::to_value(resp).unwrap())
        }
        "edgequake_retrieve" => {
            let req: ContextRetrievalRequest = serde_json::from_value(arguments)
                .map_err(|e| ApiError::BadRequest(format!("Invalid arguments: {e}")))?;
            let resp = retrieve_context(state, tenant_ctx, req, llm_override).await?;
            Ok(serde_json::to_value(resp).unwrap())
        }
        other => Err(ApiError::BadRequest(format!("Unknown tool: {other}"))),
    }
}

fn parse_granularity(s: &str) -> ContentGranularity {
    match s {
        "citation" => ContentGranularity::Citation,
        "debug" => ContentGranularity::Debug,
        _ => ContentGranularity::Agent,
    }
}
