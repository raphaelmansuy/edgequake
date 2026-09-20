use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
#[cfg(feature = "postgres")]
use crate::services::task_cancel::apply_cancel_pdf_pipeline_tasks;
#[cfg(feature = "postgres")]
use crate::services::{sync_doc_cancelled_by_document_id, sync_doc_cancelled_for_task};
use crate::state::AppState;
#[cfg(feature = "postgres")]
use std::sync::Arc;

// WHY: These imports are only used inside #[cfg(feature = "postgres")] blocks.
#[cfg(feature = "postgres")]
use super::helpers::create_pdf_processing_task;
#[cfg(feature = "postgres")]
use super::types::PdfUploadOptions;
#[cfg(feature = "postgres")]
use edgequake_storage::PdfProcessingStatus;
#[cfg(feature = "postgres")]
use tracing::info;
#[cfg(feature = "postgres")]
use uuid::Uuid;

// ============================================================================
// OODA-17: Error Recovery Endpoints
// ============================================================================

/// Response for retry/cancel operations.
///
/// OODA-17: Standard response for error recovery operations.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfOperationResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// The PDF ID.
    pub pdf_id: String,
    /// Human-readable message.
    pub message: String,
    /// New task ID (for retry operations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

/// Retry a failed PDF processing task.
///
/// OODA-17: Re-enqueue a failed PDF for processing.
///
/// # Endpoint
///
/// `POST /api/v1/documents/pdf/{pdf_id}/retry`
///
/// # Behavior
///
/// 1. Validate PDF exists and belongs to workspace
/// 2. Check status is Failed (cannot retry Pending/Processing/Completed)
/// 3. Reset status to Pending
/// 4. Create new processing task
/// 5. Return new task ID
///
/// # Errors
///
/// - 404 if PDF not found
/// - 409 if PDF is not in Failed state
/// - 500 for internal errors
#[utoipa::path(
    post,
    path = "/api/v1/documents/pdf/{pdf_id}/retry",
    params(
        ("pdf_id" = String, Path, description = "PDF document ID")
    ),
    responses(
        (status = 200, description = "PDF retry initiated", body = PdfOperationResponse),
        (status = 404, description = "PDF not found"),
        (status = 409, description = "PDF not in retriable state"),
    ),
    security(("bearer_token" = []))
)]
#[allow(clippy::needless_return)]
pub async fn retry_pdf_processing(
    State(state): State<AppState>,
    tenant: TenantContext,
    Path(pdf_id): Path<String>,
) -> ApiResult<Json<PdfOperationResponse>> {
    // OODA-17: Retry requires postgres feature for PDF storage
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (&state, &tenant, &pdf_id);
        return Err(ApiError::Internal(
            "PDF storage requires postgres feature".to_string(),
        ));
    }

    #[cfg(feature = "postgres")]
    {
        let pdf_uuid = Uuid::parse_str(&pdf_id)
            .map_err(|_| ApiError::BadRequest("Invalid PDF ID format".to_string()))?;

        let workspace_id = tenant
            .workspace_id_uuid()
            .ok_or_else(|| ApiError::BadRequest("Workspace ID required".to_string()))?;

        // OODA-17: Get PDF and validate state
        let pdf_storage = state
            .storage
            .pdf_storage
            .as_ref()
            .ok_or_else(|| ApiError::Internal("PDF storage not available".to_string()))?;

        let pdf = pdf_storage
            .get_pdf(&pdf_uuid)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {}", e)))?
            .ok_or_else(|| ApiError::NotFound(format!("PDF not found: {}", pdf_id)))?;

        // Only allow retry of failed PDFs
        if pdf.processing_status != PdfProcessingStatus::Failed {
            return Err(ApiError::Conflict(format!(
                "Cannot retry PDF with status '{}'. Only 'failed' PDFs can be retried.",
                pdf.processing_status
            )));
        }

        // OODA-17: Reset status to Pending
        pdf_storage
            .update_pdf_status(&pdf_uuid, PdfProcessingStatus::Pending)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to reset PDF status: {}", e)))?;

        // Workspace config has precedence (same chain as upload).
        let workspace = state
            .workspace_service
            .get_workspace(workspace_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        let mut options = PdfUploadOptions {
            enable_vision: true,
            vision_provider: None,
            vision_model: None,
            pdf_parser_backend: None,
            ..Default::default()
        };
        if let Some(ref ws) = workspace {
            options.apply_workspace(ws);
        }

        let enqueue = create_pdf_processing_task(
            &state,
            &tenant,
            pdf_uuid,
            &options,
            workspace.as_ref(),
            super::helpers::PdfReprocessIntent::fresh(),
            pdf.page_count,
            pdf.file_size_bytes.max(0) as u64,
        )
        .await?;

        info!(
            pdf_id = %pdf_id,
            task_id = %enqueue.track_id,
            "PDF processing retry initiated"
        );

        Ok(Json(PdfOperationResponse {
            success: true,
            pdf_id,
            message: "PDF retry initiated successfully".to_string(),
            task_id: Some(enqueue.track_id),
        }))
    }
}

/// Cancel an in-progress PDF processing task.
///
/// OODA-17: Request cancellation of an active PDF processing task.
///
/// # Endpoint
///
/// `DELETE /api/v1/documents/pdf/{pdf_id}/cancel`
///
/// # Behavior
///
/// 1. Validate PDF exists and belongs to workspace
/// 2. Check status is Processing or Pending (cannot cancel terminal)
/// 3. Request cancellation via CancellationRegistry / task row
/// 4. Update PDF status to Cancelled (SPEC-057 — not Failed)
///
/// # Errors
///
/// - 404 if PDF not found
/// - 409 if PDF is not in cancellable state
/// - 500 for internal errors
#[utoipa::path(
    delete,
    path = "/api/v1/documents/pdf/{pdf_id}/cancel",
    params(
        ("pdf_id" = String, Path, description = "PDF document ID")
    ),
    responses(
        (status = 200, description = "PDF processing cancelled", body = PdfOperationResponse),
        (status = 404, description = "PDF not found"),
        (status = 409, description = "PDF not in cancellable state"),
    ),
    security(("bearer_token" = []))
)]
#[allow(clippy::needless_return)]
pub async fn cancel_pdf_processing(
    State(state): State<AppState>,
    tenant: TenantContext,
    Path(pdf_id): Path<String>,
) -> ApiResult<Json<PdfOperationResponse>> {
    // OODA-17: Cancel requires postgres feature for PDF storage
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (&state, &tenant, &pdf_id);
        return Err(ApiError::Internal(
            "PDF storage requires postgres feature".to_string(),
        ));
    }

    #[cfg(feature = "postgres")]
    {
        let pdf_uuid = Uuid::parse_str(&pdf_id)
            .map_err(|_| ApiError::BadRequest("Invalid PDF ID format".to_string()))?;

        let workspace_id = tenant
            .workspace_id_uuid()
            .ok_or_else(|| ApiError::BadRequest("Workspace ID required".to_string()))?;

        // OODA-17: Get PDF and validate state
        let pdf_storage = state
            .storage
            .pdf_storage
            .as_ref()
            .ok_or_else(|| ApiError::Internal("PDF storage not available".to_string()))?;

        let pdf = pdf_storage
            .get_pdf(&pdf_uuid)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {}", e)))?
            .ok_or_else(|| ApiError::NotFound(format!("PDF not found: {}", pdf_id)))?;

        // SPEC-057 P2: allow cancel while convert is Pending/Processing, OR after
        // convert Completed when a follow-on Insert is still in-flight.
        let active_before = state
            .tasks
            .storage
            .find_active_pdf_processing_task(pdf_uuid, workspace_id)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to find PDF task: {}", e)))?;

        let convert_in_flight = matches!(
            pdf.processing_status,
            PdfProcessingStatus::Processing | PdfProcessingStatus::Pending
        );
        let ingest_in_flight = active_before
            .as_ref()
            .is_some_and(|t| t.task_type == edgequake_tasks::TaskType::Insert)
            || (pdf.processing_status == PdfProcessingStatus::Completed && active_before.is_some());

        if !convert_in_flight && !ingest_in_flight {
            return Err(ApiError::Conflict(format!(
                "Cannot cancel PDF with status '{}'. Only pending/processing convert or \
                 in-flight ingest after convert can be cancelled.",
                pdf.processing_status
            )));
        }

        let cancel_results = apply_cancel_pdf_pipeline_tasks(
            &state.tasks.storage,
            &state.tasks.cancellation_registry,
            pdf_uuid,
            workspace_id,
        )
        .await
        .map_err(ApiError::Internal)?;

        let mut cancelled_track_id = None;
        let workspace_key = workspace_id.to_string();
        let vector =
            crate::services::get_workspace_vector_storage_for_delete(&state, &workspace_key)
                .await?;
        for applied in &cancel_results {
            if applied.cancelled {
                if cancelled_track_id.is_none() {
                    cancelled_track_id = Some(applied.track_id.clone());
                }
                if let Some(ref cancelled_task) = applied.task {
                    if let Err(e) = sync_doc_cancelled_for_task(
                        Arc::clone(&state.storage.kv_storage),
                        cancelled_task,
                        "Task cancelled by user",
                    )
                    .await
                    {
                        tracing::warn!(
                            track_id = %applied.track_id,
                            error = %e,
                            "PDF cancel: doc KV sync from task failed"
                        );
                    }
                    crate::services::retract_indexes_for_task(
                        &state.storage.graph_storage,
                        &vector,
                        cancelled_task,
                    )
                    .await;
                }
            }
        }

        // Also flip the legacy flag for any older callers that still poll it.
        state.tasks.pipeline_state.request_cancellation().await;

        // SPEC-057 P0/P2: cancel during convert → PDF Cancelled; after convert
        // Completed, leave PDF Completed (convert artifact survives ingest cancel).
        if convert_in_flight {
            pdf_storage
                .update_pdf_status(&pdf_uuid, crate::services::pdf_status_for_cancel())
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to update PDF status: {}", e)))?;
        }

        // Sync doc KV from PDF.document_id when task payload had no link.
        if let Some(document_uuid) = pdf.document_id {
            let doc_id = document_uuid.to_string();
            if let Err(e) = sync_doc_cancelled_by_document_id(
                Arc::clone(&state.storage.kv_storage),
                &doc_id,
                "Task cancelled by user",
            )
            .await
            {
                tracing::warn!(
                    pdf_id = %pdf_id,
                    document_id = %document_uuid,
                    error = %e,
                    "PDF cancel: doc KV sync by pdf.document_id failed"
                );
            }
            crate::services::retract_indexes_for_document_id(
                &state.storage.graph_storage,
                &vector,
                &doc_id,
            )
            .await;
        }

        info!(
            pdf_id = %pdf_id,
            track_id = ?cancelled_track_id,
            cancelled_tasks = cancel_results.len(),
            "PDF pipeline cancellation requested (Convert+Insert chain)"
        );

        Ok(Json(PdfOperationResponse {
            success: true,
            pdf_id,
            message: "PDF processing cancellation requested".to_string(),
            task_id: cancelled_track_id,
        }))
    }
}
