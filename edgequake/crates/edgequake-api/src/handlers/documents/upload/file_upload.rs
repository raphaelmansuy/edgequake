//! Multipart file upload handler.

use axum::http::StatusCode;
use axum::{extract::State, Json};
use chrono::Utc;
use tracing::debug;
use uuid::Uuid;

use edgequake_audit::{AuditEventType, AuditResult};

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::{record_compliance_event, ContentHasher};
use crate::state::AppState;

use crate::file_validation::{image_mime_type, is_image_extension, validate_file};
#[allow(unused_imports)]
use crate::handlers::documents::storage_helpers::get_workspace_vector_storage_with_fallback;
use crate::handlers::documents::storage_helpers::{
    delete_document_for_reingestion, get_workspace_vector_storage_strict,
};
use crate::handlers::documents::upload::image_extract::extract_text_from_image;
use crate::handlers::documents_types::*;
use axum_extra::extract::Multipart;

/// Upload a file via multipart form.
///
/// Supports text-based files: .txt, .md, .json, .csv, .html
#[utoipa::path(
    post,
    path = "/api/v1/documents/upload",
    tag = "Documents",
    params(
        ("X-Tenant-ID" = Option<String>, Header, description = "Tenant UUID for multi-tenant isolation"),
        ("X-Workspace-ID" = Option<String>, Header, description = "Workspace UUID — scopes uploaded documents"),
    ),
    request_body(content_type = "multipart/form-data", description = "File to upload"),
    responses(
        (status = 201, description = "File uploaded successfully", body = FileUploadResponse),
        (status = 400, description = "Invalid file or request"),
        (status = 409, description = "Duplicate file (already processed)"),
        (status = 413, description = "File too large")
    )
)]
pub async fn upload_file(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<FileUploadResponse>)> {
    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Uploading file with tenant context"
    );

    let mut filename = String::new();
    let mut content = Vec::new();
    let mut metadata: Option<serde_json::Value> = None;

    // Process multipart fields
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read multipart field: {}", e)))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "file" => {
                // Get filename
                filename = field
                    .file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unnamed.txt".to_string());

                // Read file content
                content = field
                    .bytes()
                    .await
                    .map_err(|e| {
                        ApiError::BadRequest(format!("Failed to read file content: {}", e))
                    })?
                    .to_vec();
            }
            "metadata" => {
                // Optional metadata field
                let text = field
                    .text()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read metadata: {}", e)))?;

                if !text.is_empty() {
                    metadata = serde_json::from_str(&text).ok();
                }
            }
            _ => {
                // Ignore unknown fields
            }
        }
    }

    // Validate we got a file
    if content.is_empty() {
        return Err(ApiError::BadRequest("No file provided".to_string()));
    }

    // Determine if this is an image or a text-based document.
    let raw_ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    let (text_content, mime_type) = if is_image_extension(&raw_ext) {
        // ── Image path: extract text via the workspace vision LLM ────────────
        // WHY: Images are binary; the standard UTF-8 validation path would
        // reject them.  Instead we call the vision LLM once to extract all
        // readable text/structure, then treat the result as the document body.
        let mime = image_mime_type(&raw_ext).unwrap_or("image/png");
        // WHY: If the configured LLM doesn't support vision (e.g. Mistral text-only),
        // we still ingest the image as a document with a descriptive placeholder rather
        // than returning a hard error to the user.
        let extracted = match extract_text_from_image(
            &content,
            mime,
            &filename,
            state.query.llm_provider.as_ref(),
        )
        .await
        {
            Ok(text) => text,
            Err(e) => {
                tracing::warn!(
                    filename = %filename,
                    error = %e,
                    "Vision extraction failed; storing image with placeholder text"
                );
                format!(
                    "# Image Document: {filename}\n\n\
                     *Automatic text extraction failed: {e}*\n\n\
                     Configure a vision-capable LLM (e.g., gpt-4o, gemma3:12b, llava) \
                     to enable OCR/text extraction from image uploads."
                )
            }
        };
        (extracted, mime)
    } else {
        // ── Text path: validate size, extension, and UTF-8 ───────────────────
        let (_, text, mt) = validate_file(&filename, &content, state.config.max_document_size)?;
        (text, mt)
    };

    // WHY-OODA83: Use ContentHasher service for consistent hash computation (DRY)
    let content_hash = ContentHasher::hash_bytes(&content);
    debug!(content_hash = %content_hash, "Computed content hash");

    // Extract tenant context for workspace-scoped uniqueness
    // WHY-OODA81: Uniqueness must be scoped to workspace, not global
    // Same document in different workspaces is allowed (multi-tenancy)
    let workspace_id_for_storage = tenant_ctx.workspace_id_or_default();
    let tenant_id_for_storage = tenant_ctx.tenant_id.clone();

    // WHY-OODA81+83: Use ContentHasher for workspace-scoped hash key
    // FIX-4: Duplicates now trigger re-ingestion instead of rejection
    let hash_key = ContentHasher::workspace_hash_key(&workspace_id_for_storage, &content_hash);
    debug!(hash_key = %hash_key, workspace_id = %workspace_id_for_storage, "Checking for workspace-scoped duplicate hash");
    if let Some(existing_doc_id) = state.storage.kv_storage.get_by_id(&hash_key).await? {
        debug!(existing_doc_id = ?existing_doc_id, "Found existing document for hash in workspace");
        if let Some(doc_id_str) = existing_doc_id.as_str() {
            // FIX-4: Try to delete old document data for re-ingestion
            match delete_document_for_reingestion(doc_id_str, &state, &workspace_id_for_storage)
                .await
            {
                Ok(true) => {
                    // Successfully deleted - proceed with new upload
                    tracing::info!(
                        old_doc_id = %doc_id_str,
                        workspace_id = %workspace_id_for_storage,
                        filename = %filename,
                        "Duplicate file found - old data deleted, proceeding with re-ingestion"
                    );
                    // Hash key will be updated below with new document_id
                }
                Ok(false) => {
                    // Document still processing - return duplicate response
                    tracing::warn!(
                        old_doc_id = %doc_id_str,
                        filename = %filename,
                        "Duplicate file is still being processed - cannot re-ingest"
                    );
                    return Ok((
                        StatusCode::OK,
                        Json(FileUploadResponse {
                            document_id: doc_id_str.to_string(),
                            filename,
                            size: content.len(),
                            content_hash,
                            status: "duplicate_processing".to_string(),
                            chunk_count: 0,
                            entity_count: 0,
                            relationship_count: 0,
                            is_duplicate: true,
                        }),
                    ));
                }
                Err(e) => {
                    // Failed to delete - log error and proceed with re-ingestion anyway
                    tracing::warn!(
                        old_doc_id = %doc_id_str,
                        filename = %filename,
                        error = %e,
                        "Failed to delete old file data - proceeding with re-ingestion"
                    );
                }
            }
        }
    }

    // Generate document ID
    let document_id = Uuid::new_v4().to_string();

    // Store hash mapping for deduplication (workspace-scoped)
    state
        .storage
        .kv_storage
        .upsert(&[(hash_key, serde_json::json!(document_id))])
        .await?;

    // Generate content summary
    let content_summary = crate::validation::generate_content_summary(&text_content);

    // Generate track ID
    let track_id = format!(
        "upload_{}_{}",
        Utc::now().format("%Y%m%d%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );

    // Store comprehensive document metadata
    let doc_metadata_key = format!("{}-metadata", document_id);
    let doc_metadata = serde_json::json!({
        "id": document_id,
        "title": filename,
        "file_name": filename,
        "file_size": content.len(),
        "mime_type": mime_type,
        "source_type": "file",
        "content_summary": content_summary,
        "content_length": text_content.len(),
        "content_hash": content_hash,
        "track_id": track_id,
        "created_at": Utc::now().to_rfc3339(),
        "status": "processing",
        "tenant_id": tenant_id_for_storage,
        "workspace_id": workspace_id_for_storage,
        "custom_metadata": metadata,
    });
    state
        .storage
        .kv_storage
        .upsert(&[(doc_metadata_key.clone(), doc_metadata)])
        .await?;

    // Store document content
    let doc_content_key = format!("{}-content", document_id);
    let doc_content = serde_json::json!({
        "content": text_content,
    });
    state
        .storage
        .kv_storage
        .upsert(&[(doc_content_key, doc_content)])
        .await?;

    // Process through the workspace-aware pipeline so ingestion uses the same
    // provider configuration as later queries and vector storage.
    let workspace_pipeline = state
        .create_workspace_pipeline(&workspace_id_for_storage)
        .await;
    let result = workspace_pipeline
        .process_with_resilience(&document_id, &text_content, None)
        .await?;

    // Log partial failures but continue (resilient processing)
    if result.stats.failed_chunks > 0 {
        tracing::warn!(
            document_id = %document_id,
            failed_chunks = result.stats.failed_chunks,
            chunk_count = result.stats.chunk_count,
            "File upload pipeline completed with partial failures"
        );
    }

    // Store chunks in KV storage (outside persister scope — same as text_insert)
    let chunks = crate::services::build_chunk_kv_records(&document_id, &filename, &result);
    state.storage.kv_storage.upsert(&chunks).await?;

    let workspace_vector_storage =
        get_workspace_vector_storage_strict(&state, &workspace_id_for_storage).await?;

    let relational_sink = crate::services::resolve_relational_sink(&state).await;
    let persist_result = crate::services::persist_ingestion_result(
        &state,
        state.storage.graph_storage.clone(),
        workspace_vector_storage,
        relational_sink,
        crate::services::PersistIngestionParams::for_document(
            &document_id,
            tenant_id_for_storage.clone(),
            workspace_id_for_storage.clone(),
            &result,
            edgequake_pipeline::ChunkVectorBuildOptions::STANDARD,
        ),
    )
    .await;

    let persist_failed = persist_result.is_err();
    if let Err(ref e) = persist_result {
        tracing::error!(
            document_id = %document_id,
            error = %e,
            "File upload persist failed (P-H1 IngestionPersister)"
        );
    } else if let Ok(ref out) = persist_result {
        tracing::info!(
            document_id = %document_id,
            chunk_vectors = out.chunk_vector_ids.len(),
            entities = out.merge_stats.entities_created + out.merge_stats.entities_updated,
            relationships = out.merge_stats.relationships_created
                + out.merge_stats.relationships_updated,
            "File upload persist completed via IngestionPersister"
        );
    }

    let final_status = if persist_failed {
        "failed"
    } else if result.stats.failed_chunks > 0
        || (result.stats.entity_count == 0 && result.stats.chunk_count > 0)
    {
        "partial_failure"
    } else {
        "completed"
    };

    // Update document metadata with completion stats and lineage
    let completed_metadata = serde_json::json!({
        "id": document_id,
        "title": filename,
        "file_name": filename,
        "file_size": content.len(),
        "mime_type": mime_type,
        "source_type": "file",
        "content_summary": content_summary,
        "content_length": text_content.len(),
        "content_hash": content_hash,
        "track_id": track_id,
        "created_at": Utc::now().to_rfc3339(),
        "processed_at": Utc::now().to_rfc3339(),
        "status": final_status,
        "chunk_count": result.stats.chunk_count,
        "entity_count": result.stats.entity_count,
        "relationship_count": result.stats.relationship_count,
        "tenant_id": tenant_id_for_storage,
        "workspace_id": workspace_id_for_storage,
        "custom_metadata": metadata,
        "llm_model": result.stats.llm_model,
        "embedding_model": result.stats.embedding_model,
        "embedding_dimensions": result.stats.embedding_dimensions,
        "entity_types": result.stats.entity_types,
        "relationship_types": result.stats.relationship_types,
        "keywords": result.stats.keywords,
        "chunking_strategy": result.stats.chunking_strategy,
        "avg_chunk_size": result.stats.avg_chunk_size,
        "processing_duration_ms": result.stats.processing_time_ms,
    });
    state
        .storage
        .kv_storage
        .upsert(&[(doc_metadata_key, completed_metadata)])
        .await?;

    if persist_failed {
        return Err(ApiError::Internal(format!(
            "Knowledge graph persist failed: {}",
            persist_result.unwrap_err()
        )));
    }

    // FIX-ISSUE-81 Phase 2: Dual-write document record to PostgreSQL
    // WHY: Without this, file uploads only write to KV storage. The PostgreSQL
    // `documents` table stays incomplete, causing Dashboard KPI mismatch.
    #[cfg(feature = "postgres")]
    if let Some(ref pdf_storage) = state.storage.pdf_storage {
        if let Ok(doc_uuid) = Uuid::parse_str(&document_id) {
            if let Ok(workspace_uuid) = Uuid::parse_str(&workspace_id_for_storage) {
                let tenant_uuid = tenant_id_for_storage
                    .as_ref()
                    .and_then(|t| Uuid::parse_str(t).ok());
                if let Err(e) = pdf_storage
                    .ensure_document_record(
                        &doc_uuid,
                        &workspace_uuid,
                        tenant_uuid.as_ref(),
                        &filename,
                        &content_summary,
                        "indexed",
                    )
                    .await
                {
                    tracing::warn!(
                        document_id = %document_id,
                        error = %e,
                        "FIX-ISSUE-81: Failed to dual-write file document record to PostgreSQL (non-fatal)"
                    );
                } else {
                    tracing::debug!(
                        document_id = %document_id,
                        "FIX-ISSUE-81: File document record dual-written to PostgreSQL"
                    );
                }
            }
        }
    }

    let tenant_for_audit = tenant_ctx
        .tenant_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    record_compliance_event(
        &state,
        tenant_for_audit,
        AuditEventType::DocumentUpload,
        "upload_file",
        AuditResult::Success,
        tenant_ctx.workspace_id.clone(),
        tenant_ctx.user_id.clone(),
        Some(("document".to_string(), document_id.clone())),
    );

    Ok((
        StatusCode::CREATED,
        Json(FileUploadResponse {
            document_id,
            filename,
            size: content.len(),
            content_hash,
            status: "processed".to_string(),
            chunk_count: result.stats.chunk_count,
            entity_count: result.stats.entity_count,
            relationship_count: result.stats.relationship_count,
            is_duplicate: false,
        }),
    ))
}
