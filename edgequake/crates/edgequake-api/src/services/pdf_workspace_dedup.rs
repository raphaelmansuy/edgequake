//! Workspace-scoped PDF duplicate resolution.
//!
//! WHY: `pdf_documents` enforces checksum uniqueness per workspace, but the
//! documents list is built from KV metadata. Orphan PDF rows (metadata deleted
//! or scope-mismatched) caused false duplicate dialogs while the UI showed 0 docs.

use std::sync::Arc;
use tracing::warn;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::document_metadata_scan::load_scoped_document_metadata;
use crate::state::AppState;
use crate::workspace_scope::metadata_matches_tenant_context;
use edgequake_storage::traits::KVStorage;
use edgequake_storage::{PdfDocument, PdfDocumentStorage};

/// Find a workspace-visible KV document id linked to `pdf_id`, if any.
pub async fn find_kv_document_id_for_pdf(
    kv_storage: &dyn KVStorage,
    pdf_id: &str,
    tenant_ctx: &TenantContext,
) -> Option<String> {
    let scoped = load_scoped_document_metadata(kv_storage, tenant_ctx)
        .await
        .ok()?;

    for meta in scoped {
        let linked_pdf = meta.get("pdf_id").and_then(|v| v.as_str());
        if linked_pdf == Some(pdf_id) {
            return meta.get("id").and_then(|v| v.as_str()).map(str::to_string);
        }
    }

    None
}

/// Returns true when a workspace-visible KV document still backs this PDF row.
pub async fn workspace_has_visible_document_for_pdf(
    state: &AppState,
    tenant_ctx: &TenantContext,
    pdf: &PdfDocument,
) -> ApiResult<bool> {
    if let Some(document_id) = pdf.document_id {
        let doc_id_str = document_id.to_string();
        let metadata_key = edgequake_storage::kv_keys::doc_metadata(&doc_id_str);
        if let Ok(Some(meta)) = state.storage.kv_storage.get_by_id(&metadata_key).await {
            if metadata_matches_tenant_context(&meta, tenant_ctx) {
                return Ok(true);
            }
        }
    }

    let pdf_id_str = pdf.pdf_id.to_string();
    Ok(
        find_kv_document_id_for_pdf(state.storage.kv_storage.as_ref(), &pdf_id_str, tenant_ctx)
            .await
            .is_some(),
    )
}

/// Remove a PDF row that no longer has a visible workspace document.
pub async fn recycle_orphan_workspace_pdf(
    pdf_storage: Arc<dyn PdfDocumentStorage>,
    pdf: &PdfDocument,
) -> ApiResult<()> {
    warn!(
        pdf_id = %pdf.pdf_id,
        workspace_id = %pdf.workspace_id,
        "Recycling orphan PDF row (no visible workspace document)"
    );
    pdf_storage
        .delete_pdf(&pdf.pdf_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to recycle orphan PDF: {}", e)))?;
    Ok(())
}

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use super::*;
    use crate::middleware::{default_workspace_uuid, TenantContext};
    use edgequake_storage::CreatePdfRequest;

    fn test_tenant_ctx() -> TenantContext {
        TenantContext {
            tenant_id: Some("default".to_string()),
            workspace_id: Some("default".to_string()),
            user_id: None,
        }
    }

    #[tokio::test]
    async fn orphan_pdf_without_kv_metadata_is_not_visible() {
        let state = AppState::test_state();
        let pdf_storage = state
            .storage
            .pdf_storage
            .as_ref()
            .expect("memory pdf storage");

        let workspace_id = default_workspace_uuid();
        let pdf_data = b"%PDF-1.4 orphan-test".to_vec();
        let checksum = edgequake_storage::calculate_pdf_checksum(&pdf_data);

        let pdf_id = pdf_storage
            .create_pdf(CreatePdfRequest {
                workspace_id,
                filename: "orphan.pdf".to_string(),
                content_type: "application/pdf".to_string(),
                file_size_bytes: pdf_data.len() as i64,
                sha256_checksum: checksum,
                page_count: Some(1),
                pdf_data,
                vision_model: None,
            })
            .await
            .expect("create pdf");

        let pdf = pdf_storage
            .get_pdf(&pdf_id)
            .await
            .expect("get pdf")
            .expect("pdf row");

        let visible = workspace_has_visible_document_for_pdf(&state, &test_tenant_ctx(), &pdf)
            .await
            .expect("visibility check");
        assert!(!visible, "orphan pdf must not block workspace upload");
    }
}
