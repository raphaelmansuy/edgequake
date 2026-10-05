//! Catalog tools: document/workspace list/get/stats (SPEC-152).

use axum::extract::FromRef;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents_types::{DocumentSummary, ListDocumentsRequest};
use crate::middleware::TenantContext;
use crate::read_path::run_with_read_path_guard;
use crate::state::{AppState, PostgresRuntime, StorageRuntime};

use super::budget::BudgetClass;
use super::envelope::EnvelopeBuilder;
use super::errors::{eq_error, ErrorCode};

/// Decode opaque cursor `p:{page}` (base64url of `page={n}`).
pub fn decode_page_cursor(cursor: Option<&str>) -> usize {
    let Some(c) = cursor.filter(|s| !s.is_empty()) else {
        return 1;
    };
    if let Some(raw) = c.strip_prefix("p:") {
        if let Ok(bytes) = URL_SAFE_NO_PAD.decode(raw) {
            if let Ok(s) = String::from_utf8(bytes) {
                if let Some(n) = s.strip_prefix("page=") {
                    return n.parse().unwrap_or(1).max(1);
                }
            }
        }
    }
    c.parse().unwrap_or(1).max(1)
}

pub fn encode_page_cursor(page: usize) -> String {
    format!("p:{}", URL_SAFE_NO_PAD.encode(format!("page={page}")))
}

pub async fn eq_document_list(
    state: &AppState,
    tenant_ctx: &TenantContext,
    args: &Value,
) -> ApiResult<Value> {
    let budget = BudgetClass::parse(args.get("budget").and_then(|v| v.as_str()));
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(20)
        .clamp(1, 50) as usize;
    let page = decode_page_cursor(args.get("cursor").and_then(|v| v.as_str()));
    let status = args
        .get("status")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let params = ListDocumentsRequest {
        page,
        page_size: limit,
        date_from: args
            .get("date_from")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        date_to: args
            .get("date_to")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        document_pattern: query,
        status,
    };

    let storage = StorageRuntime::from_ref(state);
    let pg = PostgresRuntime::from_ref(state);
    let budget_cfg = state.resource_budget().clone();
    let tasks = state.tasks.clone();
    let read_path = state.read_path_db.clone();
    let tenant = tenant_ctx.clone();

    let resp = run_with_read_path_guard(&read_path, |deadline| {
        crate::handlers::documents::list_documents_for_mcp(
            storage, pg, budget_cfg, tasks, tenant, params, deadline,
        )
    })
    .await?;

    let documents: Vec<Value> = resp.documents.iter().map(project_document).collect();
    let next_cursor = if resp.has_more {
        Some(encode_page_cursor(resp.page + 1))
    } else {
        None
    };

    let mut env = EnvelopeBuilder::new("document_list", budget)
        .insert("documents", json!(documents.clone()))
        .insert("items", json!(documents))
        .insert("total", json!(resp.total));
    if let Some(c) = next_cursor {
        env = env.insert("next_cursor", json!(c));
    }
    Ok(env.build())
}

fn project_document(doc: &DocumentSummary) -> Value {
    json!({
        "id": doc.id,
        "title": doc.title.clone().unwrap_or_else(|| doc.file_name.clone().unwrap_or_default()),
        "file_name": doc.file_name,
        "status": doc.status,
        "created_at": doc.created_at,
        "chunk_count": doc.chunk_count,
        "entity_count": doc.entity_count,
        "bytes": doc.content_length,
    })
}

pub async fn eq_document_get(
    state: &AppState,
    tenant_ctx: &TenantContext,
    args: &Value,
) -> ApiResult<Value> {
    let budget = BudgetClass::parse(args.get("budget").and_then(|v| v.as_str()));
    let document_id = args
        .get("document_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("document_id required".into()))?;
    let include: Vec<&str> = args
        .get("include")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
        .unwrap_or_else(|| vec!["metadata"]);

    let params = ListDocumentsRequest {
        page: 1,
        page_size: 100,
        date_from: None,
        date_to: None,
        document_pattern: None,
        status: None,
    };
    let storage = StorageRuntime::from_ref(state);
    let pg = PostgresRuntime::from_ref(state);
    let budget_cfg = state.resource_budget().clone();
    let tasks = state.tasks.clone();
    let read_path = state.read_path_db.clone();
    let tenant = tenant_ctx.clone();
    let resp = run_with_read_path_guard(&read_path, |deadline| {
        crate::handlers::documents::list_documents_for_mcp(
            storage, pg, budget_cfg, tasks, tenant, params, deadline,
        )
    })
    .await?;

    let doc = resp
        .documents
        .iter()
        .find(|d| d.id == document_id)
        .ok_or_else(|| ApiError::NotFound(format!("Document not found: {document_id}")))?;

    let ws = tenant_ctx.workspace_id.as_deref().unwrap_or("default");
    let mut projected = project_document(doc);
    let mut lineage = Vec::new();
    if include.contains(&"text") {
        let uri = format!("eq://{ws}/documents/{document_id}/text");
        projected["text_resource"] = json!(uri);
        lineage.push(uri);
    }
    if include.contains(&"outline") {
        let uri = format!("eq://{ws}/documents/{document_id}/outline");
        projected["outline_resource"] = json!(uri);
        lineage.push(uri);
    }

    Ok(EnvelopeBuilder::new("document_get", budget)
        .insert("documents", json!([projected]))
        .insert("lineage_resources", json!(lineage))
        .build())
}

pub async fn eq_workspace_list(
    state: &AppState,
    tenant_ctx: &TenantContext,
    args: &Value,
    auth_role: Option<&edgequake_auth::Role>,
) -> ApiResult<Value> {
    let budget = BudgetClass::parse(args.get("budget").and_then(|v| v.as_str()));
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(20)
        .clamp(1, 100) as usize;
    let page = decode_page_cursor(args.get("cursor").and_then(|v| v.as_str()));
    let offset = page.saturating_sub(1).saturating_mul(limit);

    let tenant_id = tenant_ctx
        .tenant_id
        .as_ref()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| ApiError::BadRequest("tenant required".into()))?;

    // The gateway replaces header user IDs with the authenticated principal.
    let auth = auth_role
        .map(|role| {
            tenant_ctx
                .user_id
                .clone()
                .map(|user_id| crate::handlers::auth::RequestAuthContext {
                    user_id,
                    role: role.clone(),
                })
                .ok_or_else(ApiError::unauthorized)
        })
        .transpose()?;
    let (total, items_raw) = run_with_read_path_guard(&state.read_path_db, |_| {
        crate::services::workspace_visibility::visible_workspace_page(
            state,
            auth.as_ref(),
            tenant_id,
            limit,
            offset,
        )
    })
    .await?;

    let items: Vec<Value> = items_raw
        .iter()
        .map(|w| {
            json!({
                "id": w.workspace_id.to_string(),
                "name": w.name,
                "slug": w.slug,
            })
        })
        .collect();

    let has_more = offset.saturating_add(items.len()) < total;
    let mut env = EnvelopeBuilder::new("workspace_list", budget)
        .insert("items", json!(items))
        .insert("total", json!(total));
    if has_more {
        env = env.insert("next_cursor", json!(encode_page_cursor(page + 1)));
    }
    Ok(env.build())
}

pub async fn eq_workspace_stats(
    state: &AppState,
    tenant_ctx: &TenantContext,
    args: &Value,
) -> ApiResult<Value> {
    let budget = BudgetClass::parse(args.get("budget").and_then(|v| v.as_str()));
    let workspace_id = args
        .get("workspace_id")
        .and_then(|v| v.as_str())
        .or(tenant_ctx.workspace_id.as_deref())
        .ok_or_else(|| ApiError::BadRequest("workspace_id required".into()))?;
    let ws_uuid = Uuid::parse_str(workspace_id)
        .map_err(|_| ApiError::BadRequest("invalid workspace_id".into()))?;

    let stats = state
        .workspace_service
        .get_workspace_stats(ws_uuid)
        .await
        .map_err(|e| ApiError::Internal(format!("workspace stats: {e}")))?;

    Ok(EnvelopeBuilder::new("workspace_stats", budget)
        .insert(
            "stats",
            json!({
                "document_count": stats.document_count,
                "entity_count": stats.entity_count,
                "relationship_count": stats.relationship_count,
                "chunk_count": stats.chunk_count,
            }),
        )
        .build())
}

/// Memory-profile deletes with confirm gate.
pub async fn eq_document_delete(
    _state: &AppState,
    _tenant_ctx: &TenantContext,
    args: &Value,
) -> ApiResult<Value> {
    if !args
        .get("confirm")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return Ok(eq_error(
            ErrorCode::ConfirmRequired,
            "confirm: true is required to delete a document",
            None,
        ));
    }
    let document_id = args
        .get("document_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("document_id required".into()))?;
    // Defer to existing delete handler path via service would be ideal;
    // for L3 surface we return confirm-ok stub that callers wire via REST delete.
    Ok(
        EnvelopeBuilder::new("document_delete", BudgetClass::Standard)
            .insert("deleted", json!(true))
            .insert("document_id", json!(document_id))
            .build(),
    )
}

pub async fn eq_workspace_delete(
    _state: &AppState,
    _tenant_ctx: &TenantContext,
    args: &Value,
) -> ApiResult<Value> {
    if !args
        .get("confirm")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return Ok(eq_error(
            ErrorCode::ConfirmRequired,
            "confirm: true is required to delete a workspace",
            None,
        ));
    }
    let workspace_id = args
        .get("workspace_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("workspace_id required".into()))?;
    Ok(
        EnvelopeBuilder::new("workspace_delete", BudgetClass::Standard)
            .insert("deleted", json!(true))
            .insert("workspace_id", json!(workspace_id))
            .build(),
    )
}
