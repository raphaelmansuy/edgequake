//! Shared tenant/workspace isolation utilities used across all API handlers.
//!
//! # Why this module exists (SRP + DRY)
//!
//! Before this module, the same filtering logic was duplicated verbatim in:
//! - `entities.rs` → `filter_nodes_by_tenant_context`
//! - `relationships.rs` → `filter_nodes_by_tenant_context` (marked `dead_code`) + `filter_edges_by_tenant_context`
//! - `graph.rs` → `matches_tenant_context` closure (inline)
//! - `documents.rs` → `matches_tenant_context` closure (inline)
//!
//! Centralising here eliminates all duplication and makes security decisions
//! easy to audit in a single known location.
//!
//! # Enforces
//!
//! - **BR0201**: Tenant isolation (strict mode — both `tenant_id` AND `workspace_id`
//!   must match; any node/edge/document missing either field is **excluded**)
//! - **BR0541**: Lineage queries must respect workspace isolation

use std::collections::HashMap;

use edgequake_storage::traits::GraphStorageReadOps;
use edgequake_storage::{GraphEdge, GraphNode};
use tracing::warn;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;

// ============================================================================
// Low-level property matching
// ============================================================================

/// Return `true` when a property map matches the current tenant context.
///
/// # Security contract
///
/// - If `ctx.tenant_id` **or** `ctx.workspace_id` is `None` → returns `false`.
/// - If the property map lacks `"tenant_id"` or `"workspace_id"` → returns `false`.
/// - Both values must match exactly; no wildcards / admin override.
///
/// This single predicate is the canonical implementation used by all filter
/// helpers in this module.  Changing the semantics here changes them everywhere.
#[inline]
pub fn properties_match_tenant_context(
    properties: &HashMap<String, serde_json::Value>,
    ctx: &TenantContext,
) -> bool {
    let (Some(ctx_tid), Some(ctx_wid)) = (ctx.tenant_id.as_deref(), ctx.workspace_id.as_deref())
    else {
        return false; // SECURITY: context incomplete → deny
    };

    let prop_tid = properties.get("tenant_id").and_then(|v| v.as_str());
    let prop_wid = properties.get("workspace_id").and_then(|v| v.as_str());

    matches!((prop_tid, prop_wid), (Some(t), Some(w)) if t == ctx_tid && w == ctx_wid)
}

/// Stamp `tenant_id` and `workspace_id` onto a property map for graph writes.
///
/// Returns `BadRequest` when the request context is incomplete — graph mutations
/// must never persist nodes/edges without tenant scope.
pub fn stamp_tenant_context_properties(
    properties: &mut HashMap<String, serde_json::Value>,
    ctx: &TenantContext,
) -> ApiResult<()> {
    let (Some(tid), Some(wid)) = (ctx.tenant_id.as_deref(), ctx.workspace_id.as_deref()) else {
        return Err(ApiError::BadRequest(
            "Tenant and workspace context are required for graph mutations".to_string(),
        ));
    };
    properties.insert("tenant_id".to_string(), tid.into());
    properties.insert("workspace_id".to_string(), wid.into());
    Ok(())
}

/// Load a graph node and verify it belongs to the current tenant context.
///
/// Cross-tenant IDOR attempts receive `NotFound` (not `Forbidden`) to avoid
/// leaking entity existence across tenants.
pub async fn load_node_for_tenant_context(
    graph: &dyn GraphStorageReadOps,
    node_id: &str,
    ctx: &TenantContext,
) -> ApiResult<GraphNode> {
    let node = graph
        .get_node(node_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Entity '{}' not found", node_id)))?;

    if !properties_match_tenant_context(&node.properties, ctx) {
        return Err(ApiError::NotFound(format!(
            "Entity '{}' not found",
            node_id
        )));
    }

    Ok(node)
}

// ============================================================================
// Graph node / edge filtering
// ============================================================================

/// Filter graph nodes to those matching the tenant context.
///
/// Nodes that are missing `tenant_id` or `workspace_id` properties are
/// **excluded** (strict mode; see `properties_match_tenant_context`).
///
/// # Implements
///
/// - **BR0201**: Tenant isolation
pub fn filter_nodes_by_tenant_context(
    nodes: Vec<GraphNode>,
    ctx: &TenantContext,
) -> Vec<GraphNode> {
    if ctx.tenant_id.is_none() || ctx.workspace_id.is_none() {
        warn!(
            tenant_id = ?ctx.tenant_id,
            workspace_id = ?ctx.workspace_id,
            "Tenant context missing — returning empty node list for security"
        );
        return Vec::new();
    }

    nodes
        .into_iter()
        .filter(|n| properties_match_tenant_context(&n.properties, ctx))
        .collect()
}

/// Filter graph edges to those matching the tenant context.
///
/// Edges that are missing `tenant_id` or `workspace_id` properties are
/// **excluded** (strict mode).
///
/// # Implements
///
/// - **BR0201**: Tenant isolation
pub fn filter_edges_by_tenant_context(
    edges: Vec<GraphEdge>,
    ctx: &TenantContext,
) -> Vec<GraphEdge> {
    if ctx.tenant_id.is_none() || ctx.workspace_id.is_none() {
        warn!(
            tenant_id = ?ctx.tenant_id,
            workspace_id = ?ctx.workspace_id,
            "Tenant context missing — returning empty edge list for security"
        );
        return Vec::new();
    }

    edges
        .into_iter()
        .filter(|e| properties_match_tenant_context(&e.properties, ctx))
        .collect()
}

// ============================================================================
// Document (KV metadata) matching
// ============================================================================

/// Return `true` when a document metadata JSON object belongs to the tenant context.
///
/// Checks `metadata["workspace_id"]` and `metadata["tenant_id"]` using the same
/// strict rules as `properties_match_tenant_context`.
pub fn doc_matches_tenant_context(metadata: &serde_json::Value, ctx: &TenantContext) -> bool {
    let (Some(ctx_tid), Some(ctx_wid)) = (ctx.tenant_id.as_deref(), ctx.workspace_id.as_deref())
    else {
        return false;
    };

    let doc_tid = metadata.get("tenant_id").and_then(|v| v.as_str());
    let doc_wid = metadata.get("workspace_id").and_then(|v| v.as_str());

    matches!((doc_tid, doc_wid), (Some(t), Some(w)) if t == ctx_tid && w == ctx_wid)
}

// ============================================================================
// Document access verification (KV-backed)
// ============================================================================

/// Verify that `document_id` belongs to the current tenant/workspace.
///
/// Fetches `"{document_id}-metadata"` from KV storage, checks isolation, and
/// returns the metadata on success.  Returns `ApiError::NotFound` on any failure
/// (not-found or wrong workspace) to avoid leaking cross-tenant document IDs.
///
/// Used by lineage handlers that receive a raw `document_id` path parameter.
pub async fn verify_document_access(
    kv_storage: &dyn edgequake_storage::traits::KVStorage,
    document_id: &str,
    ctx: &TenantContext,
) -> ApiResult<serde_json::Value> {
    let metadata_key =
        crate::services::document_metadata_scan::metadata_key_for_document(document_id);
    let metadata = kv_storage
        .get_by_id(&metadata_key)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Document '{}' not found", document_id)))?;

    if !doc_matches_tenant_context(&metadata, ctx) {
        // Return 404 (not NotAuthorized) to avoid leaking the fact that the
        // document exists in a different workspace.
        return Err(ApiError::NotFound(format!(
            "Document '{}' not found",
            document_id
        )));
    }

    Ok(metadata)
}

// ============================================================================
// Workspace rebuild scoping (for workspaces.rs rebuild handlers)
// ============================================================================

/// Return `true` when a document (identified by its stored `workspace_id`) belongs
/// to a workspace being rebuilt.
///
/// ## Rules
///
/// 1. **Exact match**: the document's `workspace_id` equals the target `workspace_id`.
/// 2. **Legacy default**: documents ingested before multi-tenancy used the literal
///    string `"default"` as their workspace identifier.  These belong to the workspace
///    whose `slug` is also `"default"`.
///
/// Any other combination → `false` (document is excluded from the rebuild).
///
/// Having a single function here means the three rebuild handlers in `workspaces.rs`
/// share exactly the same semantics (DRY) and the security logic is easy to audit.
#[inline]
pub fn doc_belongs_to_workspace(
    doc_workspace: &str,
    target_workspace_id: &str,
    target_workspace_slug: &str,
) -> bool {
    // Direct match
    if doc_workspace == target_workspace_id {
        return true;
    }
    // Legacy "default" docs belong only to the default workspace
    doc_workspace == "default" && target_workspace_slug == "default"
}
