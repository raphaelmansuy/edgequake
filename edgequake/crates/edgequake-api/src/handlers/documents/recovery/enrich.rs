//! Trigger metadata enrichment for a single document.
//!
//! Allows users to enrich documents that were uploaded before the enrichment
//! pipeline existed, or that previously failed/skipped enrichment.

use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::state::AppState;
use edgequake_tasks::{MetadataEnrichData, Task, TaskStatus, TaskType};

#[derive(Debug, Serialize, ToSchema)]
pub struct EnrichDocumentResponse {
    pub document_id: String,
    pub track_id: String,
    pub message: String,
}

/// Trigger metadata enrichment for a document.
///
/// Enqueues a MetadataEnrich task for any document that has a linked PDF,
/// regardless of its current enrichment_status. Useful for documents uploaded
/// before the enrichment pipeline was added.
#[utoipa::path(
    post,
    path = "/api/v1/documents/{document_id}/enrich",
    tag = "Documents",
    params(("document_id" = String, Path, description = "Document ID")),
    responses(
        (status = 200, description = "Enrichment task queued", body = EnrichDocumentResponse),
        (status = 404, description = "Document not found"),
        (status = 400, description = "Document has no linked PDF")
    )
)]
pub async fn enrich_document(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(document_id): Path<String>,
) -> ApiResult<Json<EnrichDocumentResponse>> {
    let metadata_key = format!("{}-metadata", document_id);

    let metadata = state
        .kv_storage
        .get_by_id(&metadata_key)
        .await
        .map_err(|e| ApiError::Internal(format!("KV lookup failed: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("Document not found: {}", document_id)))?;

    let obj = metadata
        .as_object()
        .ok_or_else(|| ApiError::Internal("Invalid metadata format".to_string()))?;

    let pdf_id_str = obj
        .get("pdf_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ApiError::BadRequest("Document has no linked PDF — only PDF documents can be enriched".to_string())
        })?;

    let pdf_id = Uuid::parse_str(pdf_id_str)
        .map_err(|_| ApiError::Internal("Invalid pdf_id in metadata".to_string()))?;

    let tenant_id = obj
        .get("tenant_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .or_else(|| tenant_ctx.tenant_id_uuid())
        .ok_or_else(|| ApiError::BadRequest("Cannot determine tenant_id".to_string()))?;

    let workspace_id = obj
        .get("workspace_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .or_else(|| tenant_ctx.workspace_id_uuid())
        .ok_or_else(|| ApiError::BadRequest("Cannot determine workspace_id".to_string()))?;

    // Reset enrichment_status so the processor writes fresh results
    let mut updated = obj.clone();
    updated.insert(
        "enrichment_status".to_string(),
        serde_json::json!("pending"),
    );
    updated.remove("enrichment_topic");
    updated.remove("enrichment_summary");
    updated.remove("enrichment_language");
    updated.remove("enrichment_keywords");
    updated.remove("enrichment_completed_at");
    updated.remove("enrichment_error");

    state
        .kv_storage
        .upsert(&[(
            metadata_key,
            serde_json::Value::Object(updated),
        )])
        .await
        .map_err(|e| ApiError::Internal(format!("KV update failed: {}", e)))?;

    let enrich_data = MetadataEnrichData {
        document_id: document_id.clone(),
        pdf_id,
        tenant_id,
        workspace_id,
        max_pages: 5,
    };

    let track_id = format!("metadata_enrich-{}", Uuid::new_v4());

    let task = Task {
        track_id: track_id.clone(),
        tenant_id,
        workspace_id,
        task_type: TaskType::MetadataEnrich,
        status: TaskStatus::Pending,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        started_at: None,
        completed_at: None,
        error_message: None,
        error: None,
        retry_count: 0,
        max_retries: 3,
        consecutive_timeout_failures: 0,
        circuit_breaker_tripped: false,
        task_data: serde_json::to_value(&enrich_data)
            .map_err(|e| ApiError::Internal(format!("Serialization failed: {}", e)))?,
        metadata: None,
        progress: None,
        result: None,
    };

    state
        .task_storage
        .create_task(&task)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create task: {}", e)))?;

    state
        .enrich_queue
        .send(task)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to queue task: {}", e)))?;

    tracing::info!(
        document_id = %document_id,
        pdf_id = %pdf_id,
        track_id = %track_id,
        "Enrichment task queued manually"
    );

    Ok(Json(EnrichDocumentResponse {
        document_id,
        track_id,
        message: "Enrichment task queued successfully".to_string(),
    }))
}
