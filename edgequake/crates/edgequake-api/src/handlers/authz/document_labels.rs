//! PATCH document security labels — dual-write SQL+KV + bump generation.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::ApiAuthenticated;
use crate::middleware::TenantContext;
use crate::services::spec146_authz::{
    apply_security_labels_to_metadata, audit_deny, existence_hiding_not_found,
    load_policy_generation, parse_security_labels, require_perm, resolve_allow_set,
    stamp_authz_context, SecurityFormOverrides,
};
#[cfg(feature = "postgres")]
use crate::services::spec146_authz::dual_write_document_security_columns;
use crate::state::AppState;
use edgequake_auth::Permission;
use edgequake_authz::DenyReasonCode;

use super::helpers::{bump_generation, require_pool};
use super::types::{PatchDocumentSecurityLabelsRequest, PatchDocumentSecurityLabelsResponse};

#[utoipa::path(
    patch,
    path = "/api/v1/documents/{document_id}/security-labels",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(("document_id" = String, Path, description = "Document UUID")),
    request_body = PatchDocumentSecurityLabelsRequest,
    responses(
        (status = 200, description = "Labels updated", body = PatchDocumentSecurityLabelsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Missing document:set_labels"),
        (status = 404, description = "Document not found (existence-hiding)"),
    )
)]
pub async fn patch_document_security_labels(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    tenant_ctx: TenantContext,
    Path(document_id): Path<String>,
    Json(req): Json<PatchDocumentSecurityLabelsRequest>,
) -> ApiResult<Json<PatchDocumentSecurityLabelsResponse>> {
    require_perm(&auth.role, Permission::DocumentSetLabels)?;

    let doc_uuid = Uuid::parse_str(&document_id)
        .map_err(|_| ApiError::BadRequest("Invalid document_id".into()))?;

    if state.security.doc_abac {
        let ws = tenant_ctx
            .workspace_id_uuid()
            .ok_or_else(|| ApiError::BadRequest("Invalid workspace id".into()))?;
        let policy_generation =
            load_policy_generation(state.allow_set_provider.as_ref(), ws).await?;
        let ctx = stamp_authz_context(
            &state,
            &tenant_ctx,
            Some(auth.user_id.as_str()),
            policy_generation,
        )
        .await?
        .ok_or_else(|| ApiError::Internal("ABAC on but authz context missing".into()))?;
        let allow = resolve_allow_set(state.allow_set_provider.as_ref(), &ctx).await?;
        if !allow.contains(&doc_uuid) {
            audit_deny(
                &state,
                &ctx,
                "document.set_labels",
                "document",
                Some(&document_id),
                DenyReasonCode::NotInAllowSet,
            );
            return Err(existence_hiding_not_found());
        }
    }

    let form = SecurityFormOverrides {
        classification: req.classification.clone(),
        share_mode: req.share_mode.clone(),
        security_status: req.security_status.clone(),
        export_control: req.export_control,
        pii: req.pii,
        project_id: req.project_id.clone(),
    };
    let labels = parse_security_labels(None, &form);

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        dual_write_document_security_columns(pool, doc_uuid, &labels, None, None).await?;

        if let Some(ids) = req.acl_principal_ids.as_ref() {
            if labels.share_mode == "acl" {
                for pid in ids {
                    if pid.trim().is_empty() {
                        continue;
                    }
                    let _ = sqlx::query(
                        r#"
                        INSERT INTO document_acl
                          (document_id, principal_kind, principal_id, permission,
                           granted_by_kind, granted_by_id, created_at)
                        VALUES ($1, 'user', $2, 'document:read', 'user', $3, NOW())
                        ON CONFLICT (document_id, principal_kind, principal_id, permission)
                          DO NOTHING
                        "#,
                    )
                    .bind(doc_uuid)
                    .bind(pid.trim())
                    .bind(&auth.user_id)
                    .execute(pool)
                    .await;
                }
            }
        }
    }

    let key = crate::services::document_metadata_scan::metadata_key_for_document(&document_id);
    if let Ok(Some(mut meta)) = state.storage.kv_storage.get_by_id(&key).await {
        apply_security_labels_to_metadata(&mut meta, &labels);
        let _ = state.storage.kv_storage.upsert(&[(key, meta)]).await;
    }

    let policy_generation = if let Ok(Some(ws)) = crate::services::spec146_authz::parse_workspace_uuid(&tenant_ctx)
    {
        bump_generation(&state, ws).await.unwrap_or(1)
    } else {
        1
    };

    Ok(Json(PatchDocumentSecurityLabelsResponse {
        classification: labels.classification,
        share_mode: labels.share_mode,
        security_status: labels.security_status,
        export_control: labels.export_control,
        pii: labels.pii,
        project_id: labels.project_id,
        policy_generation,
    }))
}
