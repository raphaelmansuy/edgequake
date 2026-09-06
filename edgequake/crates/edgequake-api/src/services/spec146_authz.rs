//! SPEC-146 authz helpers for API PEPs (DRY).

use std::collections::HashSet;
use std::sync::Arc;

use edgequake_audit::{AuditEvent, AuditEventType, AuditResult};
use edgequake_auth::Permission;
use edgequake_authz::{
    AllowSet, AllowSetProvider, AuthzContext, DenyReasonCode, PrincipalId, SharedAllowSetProvider,
};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::audit::record_audit;
use crate::state::AppState;

/// Security labels dual-written to KV + `documents` (SPEC-146 admit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityAdmissionLabels {
    pub classification: String,
    pub share_mode: String,
    pub security_status: String,
    pub export_control: bool,
    pub pii: bool,
    pub project_id: Option<String>,
}

impl Default for SecurityAdmissionLabels {
    fn default() -> Self {
        Self {
            classification: edgequake_authz::DEFAULT_CLASSIFICATION.into(),
            share_mode: edgequake_authz::DEFAULT_SHARE_MODE.into(),
            security_status: "ok".into(),
            export_control: false,
            pii: false,
            project_id: None,
        }
    }
}

impl SecurityAdmissionLabels {
    /// True when labels change allow-set membership vs default workspace-visible.
    pub fn affects_allow_set(&self) -> bool {
        self.share_mode != edgequake_authz::DEFAULT_SHARE_MODE || self.security_status != "ok"
    }

    /// Fail-closed quarantine when classified without a classification value.
    pub fn normalize(mut self) -> Self {
        self.share_mode = edgequake_authz::normalize_share_mode(&self.share_mode);
        if self.share_mode == "classified" && self.classification.trim().is_empty() {
            self.security_status = "quarantined".into();
        }
        let status = self.security_status.to_lowercase();
        self.security_status = match status.as_str() {
            "quarantined" => "quarantined".into(),
            _ => "ok".into(),
        };
        self.classification = edgequake_authz::normalize_classification(&self.classification);
        self
    }
}

/// Parse security labels from request metadata JSON and/or explicit form overrides.
pub fn parse_security_labels(
    metadata: Option<&serde_json::Value>,
    form: &SecurityFormOverrides,
) -> SecurityAdmissionLabels {
    let mut labels = SecurityAdmissionLabels::default();
    if let Some(meta) = metadata {
        if let Some(s) = meta.get("classification").and_then(|v| v.as_str()) {
            labels.classification = s.to_string();
        }
        if let Some(s) = meta.get("share_mode").and_then(|v| v.as_str()) {
            labels.share_mode = s.to_string();
        }
        if let Some(s) = meta.get("security_status").and_then(|v| v.as_str()) {
            labels.security_status = s.to_string();
        }
        if let Some(b) = meta.get("export_control").and_then(|v| v.as_bool()) {
            labels.export_control = b;
        }
        if let Some(b) = meta.get("pii").and_then(|v| v.as_bool()) {
            labels.pii = b;
        }
        if let Some(s) = meta.get("project_id").and_then(|v| v.as_str()) {
            if !s.is_empty() {
                labels.project_id = Some(s.to_string());
            }
        }
    }
    if let Some(ref s) = form.classification {
        labels.classification = s.clone();
    }
    if let Some(ref s) = form.share_mode {
        labels.share_mode = s.clone();
    }
    if let Some(ref s) = form.security_status {
        labels.security_status = s.clone();
    }
    if let Some(b) = form.export_control {
        labels.export_control = b;
    }
    if let Some(b) = form.pii {
        labels.pii = b;
    }
    if let Some(ref s) = form.project_id {
        labels.project_id = if s.is_empty() { None } else { Some(s.clone()) };
    }
    labels.normalize()
}

/// Explicit multipart form overrides for security fields (PDF / file upload).
#[derive(Debug, Clone, Default)]
pub struct SecurityFormOverrides {
    pub classification: Option<String>,
    pub share_mode: Option<String>,
    pub security_status: Option<String>,
    pub export_control: Option<bool>,
    pub pii: Option<bool>,
    pub project_id: Option<String>,
}

impl SecurityFormOverrides {
    pub fn ingest_text_field(&mut self, name: &str, text: &str) {
        let t = text.trim();
        if t.is_empty() {
            return;
        }
        match name {
            "classification" => self.classification = Some(t.to_string()),
            "share_mode" => self.share_mode = Some(t.to_string()),
            "security_status" => self.security_status = Some(t.to_string()),
            "export_control" => self.export_control = t.parse().ok(),
            "pii" => self.pii = t.parse().ok(),
            "project_id" => self.project_id = Some(t.to_string()),
            _ => {}
        }
    }
}

/// Resolve workspace UUID from tenant context.
pub fn parse_workspace_uuid(tenant_ctx: &TenantContext) -> ApiResult<Option<Uuid>> {
    match tenant_ctx.workspace_id.as_deref() {
        Some(s) => Uuid::parse_str(s)
            .map(Some)
            .map_err(|_| ApiError::BadRequest("Invalid workspace id".into())),
        None => Ok(None),
    }
}

/// Load policy_generation from allow-set provider when present (default 1).
pub async fn load_policy_generation(
    provider: Option<&SharedAllowSetProvider>,
    workspace_id: Uuid,
) -> ApiResult<u64> {
    let Some(provider) = provider else {
        return Ok(1);
    };
    AllowSetProvider::current_policy_generation(provider.as_ref(), workspace_id)
        .await
        .map_err(|e| ApiError::Internal(format!("policy_generation: {e}")))
}

/// Bump policy_generation when labels/ACL affect allow-sets.
pub async fn bump_policy_generation_if_needed(
    provider: Option<&SharedAllowSetProvider>,
    workspace_id: Uuid,
    labels: &SecurityAdmissionLabels,
) -> ApiResult<()> {
    if !labels.affects_allow_set() {
        return Ok(());
    }
    let Some(provider) = provider else {
        return Ok(());
    };
    let _ = AllowSetProvider::bump_policy_generation(provider.as_ref(), workspace_id)
        .await
        .map_err(|e| ApiError::Internal(format!("bump policy_generation: {e}")))?;
    Ok(())
}

/// Build AuthzContext when ABAC is enabled; `None` when flag off.
pub async fn stamp_authz_context(
    state: &AppState,
    tenant_ctx: &TenantContext,
    user_id: Option<&str>,
    policy_generation: u64,
) -> ApiResult<Option<AuthzContext>> {
    if !state.security.doc_abac {
        return Ok(None);
    }
    let Some(ws) = parse_workspace_uuid(tenant_ctx)? else {
        return Err(ApiError::Forbidden(Some(
            "Workspace required when document ABAC is enabled".into(),
        )));
    };
    let Some(uid) = user_id else {
        return Err(ApiError::unauthorized());
    };
    let principal = PrincipalId::from_auth_user_id(uid);
    let mut ctx = AuthzContext::new(principal, ws, policy_generation, true);
    if let Some(tid) = tenant_ctx.tenant_id.as_deref() {
        if let Ok(u) = Uuid::parse_str(tid) {
            ctx = ctx.with_tenant(u);
        }
    }
    Ok(Some(ctx))
}

/// Compute allow-set once (LAW-146-24). Empty when provider missing.
pub async fn resolve_allow_set(
    provider: Option<&SharedAllowSetProvider>,
    ctx: &AuthzContext,
) -> ApiResult<AllowSet> {
    let Some(provider) = provider else {
        return Ok(AllowSet::empty());
    };
    provider
        .documents_for(ctx)
        .await
        .map_err(|e| ApiError::Internal(format!("allow-set: {e}")))
}

/// Intersect client filter ids with allow-set; never widens.
pub fn intersect_document_filter(allow: &AllowSet, client_ids: Option<&[String]>) -> AllowSet {
    let filter_uuids: Option<Vec<Uuid>> = client_ids.map(|ids| {
        ids.iter()
            .filter_map(|s| Uuid::parse_str(s).ok())
            .collect()
    });
    allow.intersect_filter(filter_uuids.as_deref())
}

/// Resolve allow-set once, then ∩ client document_filter ids (LAW-146-24).
///
/// When ABAC off: returns `client_ids` unchanged (flag-off = no behavior change).
/// When ABAC on: always returns `Some` (empty OK; never None all-pass — R5).
pub async fn resolve_query_allowed_document_ids(
    state: &AppState,
    tenant_ctx: &TenantContext,
    user_id: Option<&str>,
    client_ids: Option<Vec<String>>,
) -> ApiResult<(Option<Vec<String>>, Option<AuthzContext>, Option<AllowSet>)> {
    if !state.security.doc_abac {
        return Ok((client_ids, None, None));
    }

    let ws = parse_workspace_uuid(tenant_ctx)?.ok_or_else(|| {
        ApiError::Forbidden(Some(
            "Workspace required when document ABAC is enabled".into(),
        ))
    })?;

    let policy_generation =
        load_policy_generation(state.allow_set_provider.as_ref(), ws).await?;
    let ctx = stamp_authz_context(state, tenant_ctx, user_id, policy_generation)
        .await?
        .ok_or_else(|| ApiError::Internal("ABAC on but authz context missing".into()))?;

    let allow = resolve_allow_set(state.allow_set_provider.as_ref(), &ctx).await?;
    let intersected = intersect_document_filter(&allow, client_ids.as_deref());
    let ids = Some(intersected.as_string_vec());
    warn_if_none_allow_when_abac(true, &ids);
    Ok((ids, Some(ctx), Some(intersected)))
}

/// Existence-hiding 404 for unauthorized document resources (LAW-146-6).
pub fn existence_hiding_not_found() -> ApiError {
    ApiError::NotFound("Document not found.".into())
}

/// Log deny to audit without leaking reason to client (LAW-146-23).
pub fn audit_deny(
    state: &AppState,
    ctx: &AuthzContext,
    action: &str,
    resource_kind: &str,
    resource_id: Option<&str>,
    code: DenyReasonCode,
) {
    let tenant = ctx
        .tenant_id
        .map(|u| u.to_string())
        .unwrap_or_else(|| "unknown".into());
    let mut event = AuditEvent::new(
        tenant,
        AuditEventType::Authorization,
        action.to_string(),
        AuditResult::Blocked,
    )
    .with_workspace(ctx.workspace_id.to_string())
    .with_user(format!("{}:{}", ctx.principal.kind_str(), ctx.principal.id_str()));
    if let Some(id) = resource_id {
        event = event.with_resource(resource_kind.to_string(), id.to_string());
    }
    event.metadata = serde_json::json!({
        "reason_code": code.as_str(),
        "resource_kind": resource_kind,
        "policy_generation": ctx.policy_generation,
    });
    record_audit(state, event);
    debug!(
        reason = code.as_str(),
        resource_kind,
        "SPEC-146 deny audited (not returned to client)"
    );
}

/// Capability deny when AuthzContext is not yet stamped (still audit when possible).
pub fn audit_capability_deny(
    state: &AppState,
    tenant_ctx: &TenantContext,
    user_id: Option<&str>,
    action: &str,
    resource_kind: &str,
) {
    let ws = tenant_ctx
        .workspace_id_uuid()
        .unwrap_or_else(uuid::Uuid::nil);
    let principal = match user_id {
        Some(uid) => PrincipalId::from_auth_user_id(uid),
        None => PrincipalId::User(uuid::Uuid::nil()),
    };
    let mut ctx = AuthzContext::new(principal, ws, 0, state.security.doc_abac);
    if let Some(tid) = tenant_ctx.tenant_id_uuid() {
        ctx = ctx.with_tenant(tid);
    }
    audit_deny(
        state,
        &ctx,
        action,
        resource_kind,
        None,
        DenyReasonCode::CapabilityDenied,
    );
}

/// Filter KV metadata entries by allow-set document ids.
pub fn filter_metadata_entries_by_allow_set(
    entries: Vec<(String, serde_json::Value)>,
    allow: &AllowSet,
) -> Vec<(String, serde_json::Value)> {
    let allowed: HashSet<String> = allow.document_ids.iter().map(|u| u.to_string()).collect();
    entries
        .into_iter()
        .filter(|(_, v)| {
            let id = v
                .get("id")
                .or_else(|| v.get("document_id"))
                .and_then(|x| x.as_str())
                .map(str::to_string);
            match id {
                Some(doc_id) => allowed.contains(&doc_id),
                None => false,
            }
        })
        .collect()
}

/// Retain only summaries whose id is in the allow-set.
pub fn filter_summaries_by_allow_set<T>(
    docs: Vec<T>,
    allow: &AllowSet,
    id_of: impl Fn(&T) -> &str,
) -> Vec<T> {
    let allowed: HashSet<String> = allow.document_ids.iter().map(|u| u.to_string()).collect();
    docs.into_iter()
        .filter(|d| allowed.contains(id_of(d)))
        .collect()
}

/// Require a Permission or return 403 (capability denial).
pub fn require_perm(role: &edgequake_auth::Role, permission: Permission) -> ApiResult<()> {
    let rbac = edgequake_auth::RbacService::new();
    rbac.require_permission(role, permission)
        .map_err(|e| ApiError::Forbidden(Some(e.to_string())))
}

/// Dual-write security fields into a metadata JSON object for KV.
pub fn merge_security_into_metadata(
    meta: &mut serde_json::Value,
    classification: &str,
    share_mode: &str,
    security_status: &str,
    export_control: bool,
    pii: bool,
    project_id: Option<&str>,
) {
    if let Some(obj) = meta.as_object_mut() {
        obj.insert("classification".into(), classification.into());
        obj.insert("share_mode".into(), share_mode.into());
        obj.insert("security_status".into(), security_status.into());
        obj.insert("export_control".into(), export_control.into());
        obj.insert("pii".into(), pii.into());
        if let Some(p) = project_id {
            obj.insert("project_id".into(), p.into());
        }
    }
}

/// Apply [`SecurityAdmissionLabels`] onto metadata JSON.
pub fn apply_security_labels_to_metadata(
    meta: &mut serde_json::Value,
    labels: &SecurityAdmissionLabels,
) {
    merge_security_into_metadata(
        meta,
        &labels.classification,
        &labels.share_mode,
        &labels.security_status,
        labels.export_control,
        labels.pii,
        labels.project_id.as_deref(),
    );
}

/// Dual-write security columns on the relational `documents` row (postgres only).
#[cfg(feature = "postgres")]
pub async fn dual_write_document_security_columns(
    pool: &sqlx::PgPool,
    document_id: Uuid,
    labels: &SecurityAdmissionLabels,
    owner_principal_kind: Option<&str>,
    owner_principal_id: Option<&str>,
) -> ApiResult<()> {
    sqlx::query(
        r#"
        UPDATE documents SET
          classification = $2,
          share_mode = $3,
          export_control = $4,
          pii = $5,
          project_id = $6,
          security_status = $7,
          owner_principal_kind = COALESCE($8, owner_principal_kind),
          owner_principal_id = COALESCE($9, owner_principal_id)
        WHERE id = $1
        "#,
    )
    .bind(document_id)
    .bind(&labels.classification)
    .bind(&labels.share_mode)
    .bind(labels.export_control)
    .bind(labels.pii)
    .bind(&labels.project_id)
    .bind(&labels.security_status)
    .bind(owner_principal_kind)
    .bind(owner_principal_id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("document security dual-write: {e}")))?;
    Ok(())
}

/// Admit-path dual-write: KV labels already merged; update SQL + bump generation.
pub async fn finish_security_admit(
    state: &AppState,
    tenant_ctx: &TenantContext,
    document_id: &str,
    labels: &SecurityAdmissionLabels,
    owner_user_id: Option<&str>,
) {
    #[cfg(feature = "postgres")]
    if let Some(ref pool) = state.pg_pool {
        if let Ok(doc_uuid) = Uuid::parse_str(document_id) {
            let owner_kind = owner_user_id.map(|_| "user");
            if let Err(e) = dual_write_document_security_columns(
                pool,
                doc_uuid,
                labels,
                owner_kind,
                owner_user_id,
            )
            .await
            {
                warn!(error = %e, document_id, "SPEC-146: security column dual-write failed");
            }
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (document_id, owner_user_id);
    }
    if let Ok(Some(ws)) = parse_workspace_uuid(tenant_ctx) {
        if let Err(e) = bump_policy_generation_if_needed(
            state.allow_set_provider.as_ref(),
            ws,
            labels,
        )
        .await
        {
            warn!(error = %e, "SPEC-146: policy_generation bump failed");
        }
    }
}

pub fn warn_if_none_allow_when_abac(abac: bool, allowed: &Option<Vec<String>>) {
    if abac && allowed.is_none() {
        warn!("SPEC-146 R5: allowed_document_ids is None while ABAC on — treating as empty");
    }
}

/// Resolve allow-set document ids when ABAC is on; `None` when ABAC is off.
pub async fn resolve_optional_allow_ids(
    state: &AppState,
    tenant_ctx: &TenantContext,
    user_id: Option<&str>,
) -> ApiResult<Option<Vec<String>>> {
    if !state.security.doc_abac {
        return Ok(None);
    }
    let (ids, _, _) =
        resolve_query_allowed_document_ids(state, tenant_ctx, user_id, None).await?;
    Ok(ids)
}

/// JWT user id, else tenant-context user id.
pub fn optional_auth_user_id(
    auth_user: Option<&edgequake_auth::extractors::AuthUser>,
    tenant_ctx: &TenantContext,
) -> Option<String> {
    auth_user
        .map(|u| u.user_id.to_string())
        .or_else(|| tenant_ctx.user_id.clone())
}

/// Filter graph nodes by document provenance ∩ allow-set (fail-closed).
pub fn filter_graph_nodes_by_allow(
    nodes: Vec<(edgequake_storage::traits::GraphNode, usize)>,
    allow_ids: Option<&[String]>,
    limit: usize,
) -> Vec<(edgequake_storage::traits::GraphNode, usize)> {
    let Some(allowed) = allow_ids else {
        return nodes.into_iter().take(limit).collect();
    };
    nodes
        .into_iter()
        .filter(|(node, _)| graph_properties_in_allow(&node.properties, Some(allowed)))
        .take(limit)
        .collect()
}

/// True when ABAC is off (`allow_ids = None`) or the node has an authorized source.
pub fn graph_properties_in_allow(
    properties: &std::collections::HashMap<String, serde_json::Value>,
    allow_ids: Option<&[String]>,
) -> bool {
    let Some(allowed) = allow_ids else {
        return true;
    };
    if allowed.is_empty() {
        return false;
    }
    let set: HashSet<&str> = allowed.iter().map(|s| s.as_str()).collect();
    let refs = edgequake_storage::traits::collect_source_references(properties);
    if refs.is_empty() {
        return false;
    }
    refs.iter().any(|r| {
        if set.contains(r.as_str()) {
            return true;
        }
        edgequake_query::helpers::extract_document_id(r)
            .map(|d| set.contains(d.as_str()))
            .unwrap_or(false)
    })
}

/// Hub description for graph HTTP — drop unauthorized `<SEP>` fragments (LAW-146-10).
pub fn sanitize_graph_description(
    properties: &std::collections::HashMap<String, serde_json::Value>,
    allow_ids: Option<&[String]>,
) -> String {
    let description = properties
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let Some(allowed) = allow_ids else {
        return description.to_string();
    };
    let refs = edgequake_storage::traits::collect_source_references(properties);
    edgequake_query::context_filter::filter_hub_description_by_allow(
        description,
        &refs,
        Some(allowed),
    )
}

/// Clone node properties with description sanitized (avoid leaking via properties bag).
pub fn sanitize_graph_properties(
    properties: &std::collections::HashMap<String, serde_json::Value>,
    allow_ids: Option<&[String]>,
) -> serde_json::Value {
    let mut map = properties.clone();
    if allow_ids.is_some() {
        let sanitized = sanitize_graph_description(properties, allow_ids);
        map.insert("description".into(), serde_json::json!(sanitized));
    }
    serde_json::to_value(&map).unwrap_or_default()
}

/// Keep edge iff provenance ∩ allow-set (same polarity as query graph_expand).
pub fn edge_in_allow(
    edge: &edgequake_storage::traits::GraphEdge,
    allow_ids: Option<&[String]>,
) -> bool {
    edgequake_query::graph_expand::edge_authorized_by_allow_set(edge, allow_ids)
}

/// True when ABAC resolved an allow-set that is empty (LAW-146-7 short-circuit).
pub fn is_empty_allow_set(allow_set: &Option<AllowSet>) -> bool {
    allow_set.as_ref().is_some_and(|a| a.is_empty())
}

/// Degree counted only on edges whose provenance ∩ allow-set (LAW-146-11).
/// When ABAC is off (`allow_ids = None`), returns the raw workspace degree.
pub async fn authorized_node_degree(
    graph: &dyn edgequake_storage::traits::GraphStorage,
    node_id: &str,
    tenant_ctx: &TenantContext,
    allow_ids: Option<&[String]>,
) -> usize {
    let Some(allowed) = allow_ids else {
        return graph.node_degree(node_id).await.unwrap_or(0);
    };
    let edges = graph
        .get_incident_edges_batch(
            &[node_id.to_string()],
            tenant_ctx.tenant_id.as_deref(),
            tenant_ctx.workspace_id.as_deref(),
        )
        .await
        .unwrap_or_default();
    edges
        .into_iter()
        .filter(|e| edge_in_allow(e, Some(allowed)))
        .count()
}

/// Batch degrees with provenance gate: unauthorized nodes are omitted
/// (existence-hiding, same polarity as get_node 404).
pub async fn authorized_degrees_batch(
    graph: &dyn edgequake_storage::traits::GraphStorage,
    node_ids: &[String],
    tenant_ctx: &TenantContext,
    allow_ids: Option<&[String]>,
) -> ApiResult<Vec<(String, usize)>> {
    let Some(allowed) = allow_ids else {
        return Ok(graph.node_degrees_batch(node_ids).await?);
    };
    if allowed.is_empty() {
        return Ok(Vec::new());
    }
    let nodes = graph.get_nodes_by_ids(node_ids).await.unwrap_or_default();
    let mut out = Vec::with_capacity(nodes.len());
    for node in nodes {
        if !graph_properties_in_allow(&node.properties, Some(allowed)) {
            continue;
        }
        let degree =
            authorized_node_degree(graph, &node.id, tenant_ctx, Some(allowed)).await;
        out.push((node.id, degree));
    }
    Ok(out)
}

/// Ingest PEP when ABAC on: require document:create (G-146-14).
pub fn require_ingest_when_abac(
    state: &AppState,
    tenant_ctx: &TenantContext,
    auth: &crate::handlers::auth::ApiOptionalAuth,
) -> ApiResult<()> {
    if !state.security.doc_abac {
        return Ok(());
    }
    let auth_ctx = auth.context().ok_or_else(|| {
        audit_capability_deny(
            state,
            tenant_ctx,
            tenant_ctx.user_id.as_deref(),
            "document.create",
            "document",
        );
        ApiError::unauthorized()
    })?;
    if require_perm(&auth_ctx.role, Permission::DocumentCreate).is_err() {
        audit_capability_deny(
            state,
            tenant_ctx,
            Some(auth_ctx.user_id.as_str()),
            "document.create",
            "document",
        );
        return Err(ApiError::Forbidden(Some(
            "You don't have permission to upload documents.".into(),
        )));
    }
    Ok(())
}

/// Helper: build SharedAllowSetProvider from optional PgPool.
#[cfg(feature = "postgres")]
pub fn build_allow_set_provider(
    pool: Option<&sqlx::PgPool>,
    doc_abac: bool,
) -> Option<SharedAllowSetProvider> {
    if !doc_abac {
        return None;
    }
    let pool = pool?;
    match edgequake_authz::PostgresAllowSetProvider::new(pool.clone()) {
        Ok(p) => Some(Arc::new(p) as SharedAllowSetProvider),
        Err(e) => {
            warn!(error = %e, "SPEC-146: failed to init AllowSetProvider");
            None
        }
    }
}

/// Without postgres feature there is no SQL allow-set provider.
#[cfg(not(feature = "postgres"))]
pub fn build_allow_set_provider(_doc_abac: bool) -> Option<SharedAllowSetProvider> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existence_hiding_is_404() {
        let err = existence_hiding_not_found();
        match err {
            ApiError::NotFound(msg) => assert!(msg.contains("not found")),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn parse_defaults_workspace_ok() {
        let labels = parse_security_labels(None, &SecurityFormOverrides::default());
        assert_eq!(labels.share_mode, "workspace");
        assert_eq!(labels.security_status, "ok");
        assert!(!labels.affects_allow_set());
    }

    #[test]
    fn classified_missing_classification_quarantines() {
        let labels = parse_security_labels(
            Some(&serde_json::json!({
                "share_mode": "classified",
                "classification": ""
            })),
            &SecurityFormOverrides::default(),
        );
        assert_eq!(labels.security_status, "quarantined");
        assert!(labels.affects_allow_set());
    }

    #[test]
    fn form_overrides_win_over_metadata() {
        let labels = parse_security_labels(
            Some(&serde_json::json!({ "share_mode": "workspace" })),
            &SecurityFormOverrides {
                share_mode: Some("owner_only".into()),
                ..Default::default()
            },
        );
        assert_eq!(labels.share_mode, "owner_only");
    }
}
