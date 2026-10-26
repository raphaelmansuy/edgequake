//! MCP method dispatch — tools, legacy shim, deprecated rejection (SPEC-152).

use std::sync::Arc;

use edgequake_auth::Role;
use edgequake_llm::traits::LLMProvider;
use edgequake_observability::PropagationHeaders;
use serde_json::{json, Value};
use tracing::{debug, info_span};

use crate::error::{ApiError, ApiResult};
use crate::handlers::context_types::ContentGranularity;
use crate::mcp::project::assets::eq_asset_get;
use crate::mcp::project::catalog::{
    eq_document_delete, eq_document_get, eq_document_list, eq_workspace_delete, eq_workspace_list,
    eq_workspace_stats,
};
use crate::mcp::project::download::eq_document_download;
use crate::mcp::project::errors::{eq_error, ErrorCode};
use crate::mcp::project::fetch::eq_fetch;
use crate::mcp::project::graph::{eq_entity_get, eq_entity_search, eq_neighborhood};
use crate::mcp::project::graph_image::eq_graph_image;
use crate::mcp::project::ingest::{eq_ingest, eq_task_get};
use crate::mcp::project::profile::{instructions_for_profile, mcp_profile};
use crate::mcp::project::search::eq_search;
use crate::mcp::project::summary::{call_tool_error_structured, call_tool_result};
use crate::mcp::project::upload_session::{
    eq_upload_abort, eq_upload_begin, eq_upload_commit, eq_upload_write,
};
use crate::middleware::TenantContext;
use crate::oauth::types::McpAuthScopes;
use crate::services::query_context::resolve_query_llm_override;
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
    pub auth_scopes: Option<McpAuthScopes>,
}

/// Owned context for async tool execution (SSE worker / cancellation).
#[derive(Clone)]
pub struct DispatchTaskContext {
    pub state: AppState,
    pub tenant_ctx: TenantContext,
    pub meta: RequestMeta,
    pub workspace_header: Option<String>,
    pub auth_role: Option<Role>,
    pub auth_scopes: Option<McpAuthScopes>,
}

impl<'a> DispatchContext<'a> {
    pub fn clone_for_task(&self) -> DispatchTaskContext {
        DispatchTaskContext {
            state: self.state.clone(),
            tenant_ctx: self.tenant_ctx.clone(),
            meta: self.meta.clone(),
            workspace_header: self.workspace_header.clone(),
            auth_role: self.auth_role.clone(),
            auth_scopes: self.auth_scopes.clone(),
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
        "resources/list" => Ok(crate::mcp::gateway::resources::resources_list(
            ctx.tenant_ctx,
        )),
        "resources/read" => crate::mcp::gateway::resources::resources_read(ctx, params).await,
        "ping" => Ok(json!({})),
        other => Err(GatewayError::transport(
            axum::http::StatusCode::OK,
            -32601,
            format!("Method not found: {other}"),
        )),
    }
}

fn legacy_initialize(protocol_version: &str) -> Value {
    let profile = mcp_profile();
    json!({
        "protocolVersion": protocol_version,
        "capabilities": {
            "tools": { "listChanged": false },
            "resources": { "listChanged": false }
        },
        "serverInfo": {
            "name": "edgequake-mcp",
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": instructions_for_profile(profile)
    })
}

fn server_discover(protocol_version: &str) -> Value {
    let profile = mcp_profile();
    json!({
        "protocolVersion": protocol_version,
        "capabilities": {
            "tools": { "listChanged": false },
            "resources": { "listChanged": false }
        },
        "serverInfo": {
            "name": "edgequake-mcp",
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": instructions_for_profile(profile),
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

    // Aliases enforce budget=standard
    if matches!(
        name,
        "edgequake_search" | "edgequake_fetch" | "edgequake_retrieve"
    ) {
        if let Some(obj) = arguments.as_object_mut() {
            obj.insert("budget".into(), json!("standard"));
            if name != "edgequake_search" {
                obj.entry("include_subgraph".to_string())
                    .or_insert(json!(false));
            }
            if name == "edgequake_fetch" {
                obj.entry("view".to_string()).or_insert(json!("toc"));
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
    // Tool tasks may be spawned by the transport. Task locals are not
    // inherited by tokio::spawn, so each execution owns a fresh binding.
    let session = tool_clearance_session(&ctx.state, &ctx.tenant_ctx)
        .await
        .map_err(GatewayError::Api)?;
    let operation = execute_tool(
        &ctx.state,
        &ctx.tenant_ctx,
        name,
        arguments,
        &propagation,
        ctx.auth_role.as_ref(),
    );
    let result = if let Some(session) = session.clone() {
        edgequake_storage::clearance_session::scope(session, operation).await
    } else {
        operation.await
    };
    if let Some(before) = session.filter(|session| session.enforced) {
        if let Some(after) = tool_clearance_session(&ctx.state, &ctx.tenant_ctx)
            .await
            .map_err(GatewayError::Api)?
        {
            if before.policy_epoch != after.policy_epoch || before.fingerprint != after.fingerprint
            {
                // A label write intentionally changes epoch. Only successful
                // mutation acknowledgements may cross that transition.
                if !matches!(
                    name,
                    "eq_ingest"
                        | "eq_upload_begin"
                        | "eq_upload_write"
                        | "eq_upload_commit"
                        | "eq_upload_abort"
                        | "eq_document_delete"
                        | "eq_workspace_delete"
                ) {
                    return Ok(call_tool_error_structured(eq_error(
                        ErrorCode::Forbidden,
                        "Access changed; retry under the current rules",
                        None,
                    )));
                }
            }
        }
    }
    match result {
        Ok(structured) => {
            if structured.get("ok").and_then(|v| v.as_bool()) == Some(false) {
                Ok(call_tool_error_structured(structured))
            } else {
                Ok(call_tool_result(structured))
            }
        }
        Err(e) => Ok(call_tool_error_structured(eq_error(
            ErrorCode::NotFound,
            e.to_string(),
            None,
        ))),
    }
}

async fn tool_clearance_session(
    state: &AppState,
    tenant: &TenantContext,
) -> ApiResult<Option<edgequake_storage::clearance_session::ClearanceSession>> {
    #[cfg(feature = "postgres")]
    if let (Some(pool), Some(workspace)) = (state.optional_pg_pool(), tenant.workspace_id_uuid()) {
        let user = tenant
            .user_id
            .as_deref()
            .and_then(|id| uuid::Uuid::parse_str(id).ok());
        return crate::services::clearance::load_scope(pool, workspace, user)
            .await
            .map(|(_, session)| Some(session));
    }
    let _ = (state, tenant);
    Ok(None)
}

async fn execute_tool(
    state: &AppState,
    tenant_ctx: &TenantContext,
    name: &str,
    arguments: Value,
    propagation: &PropagationHeaders,
    auth_role: Option<&Role>,
) -> ApiResult<Value> {
    let workspace =
        crate::handlers::query::resolve_query_workspace(state, tenant_ctx.workspace_id.as_deref())
            .await?;

    let propagation = propagation.clone();
    // Catalog, upload and lifecycle tools do not consume a query model. Keep
    // model eligibility authoritative for the search tools that actually use it.
    let llm_override: Option<Arc<dyn LLMProvider>> = if matches!(
        name,
        "eq_search" | "eq_retrieve" | "edgequake_search" | "edgequake_retrieve"
    ) {
        resolve_query_llm_override(state, workspace.as_ref(), &propagation, None, None).await?
    } else {
        None
    };
    let ws_id = tenant_ctx.workspace_id.as_deref().unwrap_or("default");

    let canonical = match name {
        "edgequake_search" => "eq_search",
        "edgequake_fetch" => "eq_fetch",
        "edgequake_retrieve" => "eq_retrieve",
        other => other,
    };

    match canonical {
        "eq_document_list" => eq_document_list(state, tenant_ctx, &arguments).await,
        "eq_document_get" => eq_document_get(state, tenant_ctx, &arguments).await,
        "eq_workspace_list" => eq_workspace_list(state, tenant_ctx, &arguments, auth_role).await,
        "eq_workspace_stats" => eq_workspace_stats(state, tenant_ctx, &arguments).await,
        "eq_search" => eq_search(state, tenant_ctx, &arguments, llm_override).await,
        "eq_fetch" => eq_fetch(&arguments, ws_id).await,
        "eq_retrieve" => eq_retrieve(state, tenant_ctx, &arguments, llm_override).await,
        "eq_entity_search" => eq_entity_search(state, tenant_ctx, &arguments).await,
        "eq_entity_get" => eq_entity_get(state, tenant_ctx, &arguments).await,
        "eq_neighborhood" => eq_neighborhood(state, tenant_ctx, &arguments).await,
        "eq_ingest" => eq_ingest(state, tenant_ctx, &arguments).await,
        "eq_task_get" => eq_task_get(state, tenant_ctx, &arguments).await,
        "eq_document_delete" => eq_document_delete(state, tenant_ctx, &arguments).await,
        "eq_workspace_delete" => eq_workspace_delete(state, tenant_ctx, &arguments).await,
        "eq_document_download" => eq_document_download(state, tenant_ctx, &arguments).await,
        "eq_asset_get" => eq_asset_get(state, tenant_ctx, &arguments).await,
        "eq_graph_image" => eq_graph_image(state, tenant_ctx, &arguments).await,
        "eq_upload_begin" => eq_upload_begin(state, tenant_ctx, &arguments).await,
        "eq_upload_write" => eq_upload_write(state, tenant_ctx, &arguments).await,
        "eq_upload_commit" => eq_upload_commit(state, tenant_ctx, &arguments).await,
        "eq_upload_abort" => eq_upload_abort(state, tenant_ctx, &arguments).await,
        other => Err(ApiError::BadRequest(format!("Unknown tool: {other}"))),
    }
}

async fn eq_retrieve(
    state: &AppState,
    tenant_ctx: &TenantContext,
    arguments: &Value,
    llm_override: Option<Arc<dyn LLMProvider>>,
) -> ApiResult<Value> {
    let search = eq_search(state, tenant_ctx, arguments, llm_override).await?;
    if search.get("ok").and_then(|v| v.as_bool()) == Some(false) {
        return Ok(search);
    }
    let rid = search
        .get("retrieval_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let budget = arguments
        .get("budget")
        .cloned()
        .unwrap_or(json!("standard"));
    let fetch_args = json!({
        "retrieval_id": rid,
        "view": "chunks",
        "budget": budget,
        "include_subgraph": false,
    });
    let ws_id = tenant_ctx.workspace_id.as_deref().unwrap_or("default");
    let mut fetch = eq_fetch(&fetch_args, ws_id).await?;
    // Merge hits from search into retrieve envelope
    if let Some(hits) = search.get("hits").cloned() {
        if let Some(obj) = fetch.as_object_mut() {
            obj.insert("hits".into(), hits);
            obj.insert("view".into(), json!("retrieve"));
            if let Some(m) = search.get("mode_used") {
                obj.insert("mode_used".into(), m.clone());
            }
            if let Some(m) = search.get("mode_reason") {
                obj.insert("mode_reason".into(), m.clone());
            }
            if let Some(m) = search.get("cross_document") {
                obj.insert("cross_document".into(), m.clone());
            }
            if let Some(m) = search.get("score_type") {
                obj.insert("score_type".into(), m.clone());
            }
            if let Some(m) = search.get("expires_at") {
                obj.insert("expires_at".into(), m.clone());
            }
        }
    }
    Ok(fetch)
}

#[allow(dead_code)]
fn parse_granularity(s: &str) -> ContentGranularity {
    match s {
        "citation" => ContentGranularity::Citation,
        "debug" => ContentGranularity::Debug,
        _ => ContentGranularity::Agent,
    }
}
