//! Text-based document upload handler.

use axum::http::StatusCode;
use axum::{extract::State, Json};
use chrono::Utc;
use tracing::debug;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::ContentHasher;
use crate::state::AppState;

use crate::handlers::documents::storage_helpers::{
    resolve_workspace_duplicate_for_reingestion, DuplicateReingestAction,
};
#[allow(unused_imports)]
use crate::handlers::documents::storage_helpers::get_workspace_vector_storage_with_fallback;
use crate::handlers::documents_types::*;

/// Upload a document for processing.
///
/// # Implements
///
/// - **UC0001**: Upload Document
/// - **FEAT0001**: Document Ingestion Pipeline
/// - **FEAT0002**: Entity Extraction
/// - **FEAT0003**: Relationship Discovery
///
/// # Enforces
///
/// - **BR0001**: Content uniqueness (SHA-256 hash computed)
/// - **BR0201**: Tenant isolation (scoped to workspace)
/// - **BR0302**: Document size limits enforced
///
/// # Request Flow
///
/// ```text
/// POST /api/v1/documents
///        ↓
///   Validate content size
///        ↓
///   Compute SHA-256 hash
///        ↓
///   Store metadata + content
///        ↓
///   async_processing?
///     ├─ true: Create task → Return task_id
///     └─ false: Process inline → Return entities
/// ```
#[utoipa::path(
    post,
    path = "/api/v1/documents",
    tag = "Documents",
    params(
        ("X-Tenant-ID" = Option<String>, Header, description = "Tenant UUID for multi-tenant isolation"),
        ("X-Workspace-ID" = Option<String>, Header, description = "Workspace UUID — scopes uploaded documents"),
    ),
    request_body = UploadDocumentRequest,
    responses(
        (status = 201, description = "Document uploaded successfully", body = UploadDocumentResponse),
        (status = 400, description = "Invalid request"),
        (status = 413, description = "Document too large")
    )
)]
pub async fn upload_document(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<UploadDocumentRequest>,
) -> ApiResult<(StatusCode, Json<UploadDocumentResponse>)> {
    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Uploading document with tenant context"
    );

    // Validate document content
    crate::validation::validate_content(&request.content, state.config.max_document_size)?;

    // Generate or use provided track_id
    let track_id = request.track_id.unwrap_or_else(|| {
        format!(
            "upload_{}_{}",
            Utc::now().format("%Y%m%d_%H%M%S"),
            &Uuid::new_v4().to_string()[..8]
        )
    });

    // WHY-OODA83: Use ContentHasher service for consistent hash computation (DRY)
    let content_hash = ContentHasher::hash_str(&request.content);

    // Extract tenant context for storage (needed for hash_key)
    let workspace_id_for_storage = tenant_ctx.workspace_id_or_default();
    let tenant_id_for_storage = tenant_ctx.tenant_id.clone();

    // WHY-OODA81+84: Workspace-scoped duplicate detection
    // FIX-4: Duplicates now trigger re-ingestion instead of rejection
    // Same content in same workspace = re-ingest (delete old data, process new)
    // Same content in different workspace = allowed (multi-tenancy)
    let hash_key = ContentHasher::workspace_hash_key(&workspace_id_for_storage, &content_hash);
    debug!(hash_key = %hash_key, workspace_id = %workspace_id_for_storage, "Checking for workspace-scoped duplicate hash");
    match resolve_workspace_duplicate_for_reingestion(&state, &hash_key, &workspace_id_for_storage)
        .await?
    {
        DuplicateReingestAction::NoDuplicate => {}
        DuplicateReingestAction::ClearedForReingestion { old_document_id } => {
            tracing::info!(
                old_doc_id = %old_document_id,
                workspace_id = %workspace_id_for_storage,
                "Duplicate document found - old data deleted, proceeding with re-ingestion"
            );
        }
        DuplicateReingestAction::StillProcessing {
            existing_document_id,
        } => {
            tracing::warn!(
                old_doc_id = %existing_document_id,
                "Duplicate document is still being processed - cannot re-ingest"
            );
            return Ok((
                StatusCode::OK,
                Json(UploadDocumentResponse {
                    document_id: existing_document_id.clone(),
                    status: "duplicate_processing".to_string(),
                    task_id: None,
                    track_id: track_id.clone(),
                    duplicate_of: Some(existing_document_id),
                    chunk_count: None,
                    entity_count: None,
                    relationship_count: None,
                    cost: None,
                }),
            ));
        }
    }

    // Generate document ID
    let document_id = Uuid::new_v4().to_string();

    // Store hash mapping for deduplication (workspace-scoped)
    // WHY-OODA81+84: Must store before creating document to prevent race conditions
    state
        .storage
        .kv_storage
        .upsert(&[(hash_key.clone(), serde_json::json!(document_id))])
        .await?;
    debug!(hash_key = %hash_key, document_id = %document_id, "Stored workspace-scoped hash mapping");

    // Generate content summary
    let content_summary = crate::validation::generate_content_summary(&request.content);
    let content_length = request.content.len();

    // Store document metadata (including title, content_summary, content_length, track_id, tenant context)
    let doc_metadata_key = format!("{}-metadata", document_id);
    let initial_status = if request.async_processing {
        "pending"
    } else {
        "processing"
    };

    // OODA-04: Include file_size_bytes, sha256_checksum, document_type for unified lineage
    // WHY: Every document—markdown or PDF—must carry the same lineage fields so
    // API consumers get consistent metadata regardless of source type.
    let doc_metadata = serde_json::json!({
        "id": document_id,
        "title": request.title,
        "content_summary": content_summary,
        "content_length": content_length,
        "content_hash": content_hash,
        "file_size_bytes": content_length,
        "sha256_checksum": content_hash,
        "document_type": "markdown",
        "track_id": track_id,
        "created_at": Utc::now().to_rfc3339(),
        "status": initial_status,
        "tenant_id": tenant_id_for_storage,
        "workspace_id": workspace_id_for_storage,
        // SPEC-002: Unified Ingestion Pipeline fields
        "source_type": "markdown",
        "current_stage": "uploading",
        "stage_progress": 0.0,
        "stage_message": "Document received, starting processing",
    });
    state
        .storage
        .kv_storage
        .upsert(&[(doc_metadata_key.clone(), doc_metadata)])
        .await?;

    // Store the document content for processing
    let doc_content_key = format!("{}-content", document_id);
    let doc_content = serde_json::json!({
        "content": request.content,
    });
    state
        .storage
        .kv_storage
        .upsert(&[(doc_content_key, doc_content)])
        .await?;

    // P-G2b (RC-7): force async upload. The synchronous inline-persistence
    // branch (~490 lines that duplicated the processor's chunk/vector/graph
    // writes with N+1 loops and no saga compensation) is removed. Every upload
    // now enqueues a background task and returns 202 ACCEPTED + task_id. This
    // collapses three ingestion persistence paths to two (processor + merger)
    // and is the prerequisite for P-G2's single IngestionPersister. The
    // `async_processing` request field is accepted but ignored (deprecated).
    if !request.async_processing {
        tracing::info!(
            document_id = %document_id,
            "Sync upload requested but async is now mandatory; enqueuing background task (P-G2b)"
        );
    }

    {
        // Create task for background processing
        use edgequake_tasks::{Task, TaskType, TextInsertData};

        // Use tenant context for workspace_id, fallback to "default"
        let workspace_id = tenant_ctx.workspace_id_or_default();
        let tenant_id = tenant_ctx.tenant_id_or_default();

        let task_data = TextInsertData {
            text: request.content.clone(),
            file_source: request.title.clone().unwrap_or_else(|| document_id.clone()),
            workspace_id: workspace_id.clone(),
            metadata: Some(serde_json::json!({
                "document_id": document_id,
                "title": request.title,
                "tenant_id": tenant_id,
                "workspace_id": workspace_id,
            })),
        };

        let task = Task::new(
            uuid::Uuid::parse_str(&tenant_id)
                .map_err(|_| ApiError::ValidationError("Invalid tenant ID".to_string()))?,
            uuid::Uuid::parse_str(&workspace_id)
                .map_err(|_| ApiError::ValidationError("Invalid workspace ID".to_string()))?,
            TaskType::Insert,
            serde_json::to_value(task_data).unwrap(),
        );
        let task_id = task.track_id.clone();

        state.enqueue_task(task).await?;

        Ok((
            StatusCode::ACCEPTED,
            Json(UploadDocumentResponse {
                document_id,
                status: "pending".to_string(),
                task_id: Some(task_id),
                track_id,
                duplicate_of: None,
                chunk_count: None,
                entity_count: None,
                relationship_count: None,
                cost: None, // Cost will be calculated when processing completes
            }),
        ))
    }
}
