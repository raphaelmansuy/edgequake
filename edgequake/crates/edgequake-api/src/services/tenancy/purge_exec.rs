//! Scope purge executor (SPEC-181 LAW-181-7).
//!
//! [`super::purge`] defines *what* a purge is and how its receipt is shaped. This module *runs*
//! the planes. Every plane is best-effort and recorded: a failure on one plane never aborts the
//! others, because a half-deleted organization is worse than a recorded leak.
//!
//! The workspace planes are the body of the pre-181 `delete_workspace` handler, moved rather
//! than rewritten, so behaviour is unchanged; the organization planes are new and are why the
//! module exists.

use edgequake_audit::{AuditEventType, AuditResult};
use uuid::Uuid;

use super::purge::{Plane, PlaneOutcome, PurgePreview, PurgeReceipt, ScopeKind};
use crate::error::ApiError;
use crate::state::AppState;

/// Delete one workspace across every plane.
pub async fn purge_workspace(
    state: &AppState,
    workspace_id: Uuid,
    requested_by: Option<Uuid>,
) -> Result<PurgeReceipt, ApiError> {
    let started = chrono::Utc::now();
    let workspace = state
        .workspace_service
        .get_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let Some(workspace) = workspace else {
        return Err(ApiError::NotFound(format!(
            "Workspace {} not found",
            workspace_id
        )));
    };
    let tenant_id = workspace.tenant_id;
    let ws = workspace_id.to_string();
    let mut outcomes: Vec<(Plane, PlaneOutcome)> = Vec::with_capacity(Plane::ALL.len());

    // 1. Cancel in-flight tasks. Finished rows stay until the FK cascade drops them.
    let tasks =
        crate::handlers::documents::storage_helpers::purge_workspace_tasks(state, workspace_id)
            .await;
    outcomes.push((Plane::Tasks, PlaneOutcome::done_count(tasks as u64)));

    // 2. Vector rows for this workspace.
    outcomes.push((
        Plane::Vectors,
        match state
            .storage
            .vector_storage
            .clear_workspace(&workspace_id)
            .await
        {
            Ok(count) => PlaneOutcome::done_count(count as u64),
            Err(e) => PlaneOutcome::failed(e.to_string()),
        },
    ));

    // 3. Knowledge graph (entities and relationships).
    outcomes.push((
        Plane::Graph,
        match state
            .storage
            .graph_storage
            .clear_workspace(&workspace_id)
            .await
        {
            Ok((nodes, edges)) => PlaneOutcome::done_count((nodes + edges) as u64),
            Err(e) => PlaneOutcome::failed(e.to_string()),
        },
    ));

    // 4. KV: document metadata, content and chunks, via the metadata SSOT.
    outcomes.push((
        Plane::Kv,
        match crate::services::document_metadata_scan::plan_workspace_document_kv_deletion(
            state.storage.kv_storage.as_ref(),
            state.optional_pg_pool(),
            &ws,
        )
        .await
        {
            Ok(plan) => {
                if plan.keys.is_empty() {
                    PlaneOutcome::skipped("no keys")
                } else {
                    match state.storage.kv_storage.delete(&plan.keys).await {
                        Ok(()) => PlaneOutcome::done_count(plan.keys.len() as u64),
                        Err(e) => PlaneOutcome::failed(e.to_string()),
                    }
                }
            }
            Err(e) => PlaneOutcome::failed(e.to_string()),
        },
    ));

    // 5. PDF rows (duplicate detection and listings must not resurface them).
    outcomes.push((Plane::PdfRows, purge_pdf_rows(state, workspace_id).await));

    // 6. Original objects. Database mode stores bytes in the row, so nothing to delete here.
    outcomes.push((
        Plane::Originals,
        PlaneOutcome::skipped("stored with the row"),
    ));

    // 7. Decision cache and review rows follow the workspace (EC-160-37).
    crate::services::decision_cleanup::purge_decision_workspace(state, &ws).await;
    outcomes.push((Plane::Decision, PlaneOutcome::done()));

    // 8. Drop the physical vector table. `evict` alone leaves an orphan `eq_*_ws_*_vectors`.
    outcomes.push((
        Plane::VectorTable,
        match state
            .storage
            .vector_registry
            .drop_workspace_table(&workspace_id)
            .await
        {
            Ok(()) => PlaneOutcome::done(),
            Err(e) => PlaneOutcome::failed(e.to_string()),
        },
    ));
    state.storage.vector_registry.evict(&workspace_id).await;

    // 9. The row itself.
    outcomes.push((
        Plane::Row,
        match state.workspace_service.delete_workspace(workspace_id).await {
            Ok(()) => PlaneOutcome::done(),
            Err(e) => PlaneOutcome::failed(e.to_string()),
        },
    ));

    let receipt = PurgeReceipt::from_outcomes(
        Uuid::new_v4(),
        ScopeKind::Workspace,
        workspace_id,
        tenant_id,
        outcomes,
        started,
    );
    persist_receipt(state, &receipt).await;
    crate::services::record_compliance_event(
        state,
        tenant_id.to_string(),
        AuditEventType::WorkspaceAccess,
        "purge_workspace",
        if receipt.status == "done" {
            AuditResult::Success
        } else {
            AuditResult::Warning
        },
        Some(workspace_id.to_string()),
        requested_by.map(|u| u.to_string()),
        Some(("purge_receipt".to_string(), receipt.receipt_id.to_string())),
    );
    Ok(receipt)
}

/// Delete one organization across every plane.
///
/// The organization is archived **first**, which disables the `organization_requires_owner`
/// trigger for this tenant; otherwise deleting memberships mid-purge would trip it.
pub async fn purge_organization(
    state: &AppState,
    tenant_id: Uuid,
    requested_by: Option<Uuid>,
) -> Result<PurgeReceipt, ApiError> {
    let started = chrono::Utc::now();
    let tenant = state
        .workspace_service
        .get_tenant(tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization {tenant_id} not found")))?;

    // 1. Archive first: the owner trigger skips archived tenants, so membership deletion is safe.
    let archived = archive_tenant(state, tenant_id, requested_by).await;
    let mut outcomes: Vec<(Plane, PlaneOutcome)> = Vec::with_capacity(Plane::ALL.len());
    outcomes.push((
        Plane::Tasks,
        match archived {
            Ok(()) => PlaneOutcome::done(),
            Err(e) => PlaneOutcome::failed(e),
        },
    ));

    // 2. Enumerate every workspace, live *and* archived, and purge each through the same code.
    let workspaces = state
        .workspace_service
        .list_workspaces(tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let mut per_ws: Vec<PurgeReceipt> = Vec::with_capacity(workspaces.len());
    for ws in &workspaces {
        match purge_workspace(state, ws.workspace_id, requested_by).await {
            Ok(r) => per_ws.push(r),
            Err(e) => {
                tracing::warn!(workspace_id = %ws.workspace_id, error = %e, "workspace purge failed");
            }
        }
    }
    let ws_total = per_ws.len() as u64;

    // Each workspace plane is reported as the aggregate of its children.
    for plane in [
        Plane::Vectors,
        Plane::Graph,
        Plane::Kv,
        Plane::PdfRows,
        Plane::Originals,
        Plane::Decision,
        Plane::VectorTable,
    ] {
        let key = plane.as_str();
        let failures = per_ws
            .iter()
            .filter(|r| r.planes.get(key).is_some_and(|o| !o.is_done()))
            .count();
        outcomes.push((
            plane,
            if failures == 0 {
                PlaneOutcome::done_count(ws_total)
            } else {
                PlaneOutcome::failed(format!("{failures} of {ws_total} workspaces"))
            },
        ));
    }

    // 3. Organization-level rows. The workspaces are gone, so this is memberships + tenant.
    let memberships = state
        .workspace_service
        .get_tenant_memberships(tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let removed = state
        .workspace_service
        .delete_tenant(tenant_id)
        .await
        .map(|()| memberships.len() as u64)
        .map_err(|e| e.to_string());
    outcomes.push((
        Plane::Row,
        match removed {
            Ok(count) => PlaneOutcome::done_count(count),
            Err(e) => PlaneOutcome::failed(e),
        },
    ));

    let receipt = PurgeReceipt::from_outcomes(
        Uuid::new_v4(),
        ScopeKind::Organization,
        tenant_id,
        tenant_id,
        outcomes,
        started,
    );
    persist_receipt(state, &receipt).await;
    crate::services::record_compliance_event(
        state,
        tenant_id.to_string(),
        AuditEventType::TenantAccess,
        "purge_organization",
        if receipt.status == "done" {
            AuditResult::Success
        } else {
            AuditResult::Warning
        },
        Some(tenant.slug.clone()),
        requested_by.map(|u| u.to_string()),
        Some((
            format!("purge_receipt:{}:{}", receipt.status, ws_total),
            receipt.receipt_id.to_string(),
        )),
    );
    Ok(receipt)
}

/// What a delete confirmation shows before anything happens.
pub async fn preview(
    state: &AppState,
    kind: ScopeKind,
    scope_id: Uuid,
) -> Result<PurgePreview, ApiError> {
    match kind {
        ScopeKind::Workspace => {
            let ws = state
                .workspace_service
                .get_workspace(scope_id)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?
                .ok_or_else(|| ApiError::NotFound(format!("Workspace {scope_id} not found")))?;
            let stats = state
                .workspace_service
                .get_workspace_stats(scope_id)
                .await
                .ok();
            Ok(PurgePreview {
                scope_kind: "workspace".into(),
                scope_id,
                name: ws.name,
                workspaces: 1,
                documents: stats.as_ref().map_or(0, |s| s.document_count),
                entities: stats.as_ref().map_or(0, |s| s.entity_count),
                relationships: stats.as_ref().map_or(0, |s| s.relationship_count),
                vector_rows: stats.as_ref().map_or(0, |s| s.embedding_count),
                original_objects: 0,
            })
        }
        ScopeKind::Organization => {
            let tenant = state
                .workspace_service
                .get_tenant(scope_id)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?
                .ok_or_else(|| ApiError::NotFound(format!("Organization {scope_id} not found")))?;
            let workspaces = state
                .workspace_service
                .list_workspaces(scope_id)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;
            let mut preview = PurgePreview {
                scope_kind: "organization".into(),
                scope_id,
                name: tenant.name,
                workspaces: workspaces.len(),
                ..Default::default()
            };
            for ws in &workspaces {
                if let Ok(stats) = state
                    .workspace_service
                    .get_workspace_stats(ws.workspace_id)
                    .await
                {
                    preview.documents += stats.document_count;
                    preview.entities += stats.entity_count;
                    preview.relationships += stats.relationship_count;
                    preview.vector_rows += stats.embedding_count;
                }
            }
            Ok(preview)
        }
    }
}

/// Mark a tenant archived. Archived tenants grant nothing and skip the owner trigger.
pub async fn archive_tenant(
    state: &AppState,
    tenant_id: Uuid,
    actor: Option<Uuid>,
) -> Result<(), String> {
    let mut tenant = state
        .workspace_service
        .get_tenant(tenant_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Organization {tenant_id} not found"))?;
    if tenant.archived_at.is_some() {
        return Ok(());
    }
    tenant.archived_at = Some(chrono::Utc::now());
    tenant.archived_by = actor;
    tenant.is_active = false;
    state
        .workspace_service
        .update_tenant(tenant)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

async fn purge_pdf_rows(state: &AppState, workspace_id: Uuid) -> PlaneOutcome {
    #[cfg(feature = "postgres")]
    {
        use edgequake_storage::ListPdfFilter;
        let mut deleted = 0u64;
        if let Some(ref pdf_storage) = state.storage.pdf_storage {
            let filter = ListPdfFilter {
                workspace_id: Some(workspace_id),
                processing_status: None,
                page: Some(1),
                page_size: Some(10_000),
                ..Default::default()
            };
            match pdf_storage.list_pdfs(filter).await {
                Ok(list) => {
                    for pdf in list.items {
                        if pdf_storage.delete_pdf(&pdf.pdf_id).await.is_ok() {
                            deleted += 1;
                        }
                    }
                    PlaneOutcome::done_count(deleted)
                }
                Err(e) => PlaneOutcome::failed(e.to_string()),
            }
        } else {
            PlaneOutcome::skipped("no pdf storage")
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, workspace_id);
        PlaneOutcome::skipped("postgres feature disabled")
    }
}

/// Store the receipt. Best-effort: a receipt write failure must not undo a successful purge, but
/// it is logged loudly because it removes the retry record.
async fn persist_receipt(state: &AppState, receipt: &PurgeReceipt) {
    #[cfg(feature = "postgres")]
    {
        let pool = state.optional_pg_pool();
        let Some(pool) = pool else {
            return;
        };
        let result = sqlx::query(
            r#"
            INSERT INTO scope_purge_receipts
                (receipt_id, scope_kind, scope_id, tenant_id, status, planes, started_at, finished_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7::timestamptz, $8::timestamptz)
            ON CONFLICT (receipt_id) DO NOTHING
            "#,
        )
        .bind(receipt.receipt_id)
        .bind(receipt.scope_kind.as_str())
        .bind(receipt.scope_id)
        .bind(receipt.tenant_id)
        .bind(&receipt.status)
        .bind(serde_json::to_value(&receipt.planes).unwrap_or_default())
        .bind(&receipt.started_at)
        .bind(receipt.finished_at.clone())
        .execute(pool)
        .await;
        if let Err(e) = result {
            tracing::warn!(
                receipt = %receipt.receipt_id,
                scope = %receipt.scope_id,
                error = %e,
                "Could not persist purge receipt"
            );
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, receipt);
    }
}
