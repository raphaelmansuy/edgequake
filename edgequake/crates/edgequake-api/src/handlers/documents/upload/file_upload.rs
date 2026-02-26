//! Multipart file upload handler.

use axum::http::StatusCode;
use axum::{extract::State, Json};
use chrono::Utc;
use tracing::debug;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::ContentHasher;
use crate::state::AppState;

use crate::file_validation::validate_file;
#[allow(unused_imports)]
use crate::handlers::documents::storage_helpers::get_workspace_vector_storage_with_fallback;
use crate::handlers::documents::storage_helpers::{
    delete_document_for_reingestion, get_workspace_vector_storage_strict,
};
use crate::handlers::documents_types::*;
use axum_extra::extract::Multipart;

/// Upload a file via multipart form.
///
/// Supports text-based files: .txt, .md, .json, .csv, .html
#[utoipa::path(
    post,
    path = "/api/v1/documents/upload",
    tag = "Documents",
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

    // Validate file (size, extension, UTF-8, non-empty)
    let (_extension, text_content, mime_type) =
        validate_file(&filename, &content, state.config.max_document_size)?;

    // WHY-OODA83: Use ContentHasher service for consistent hash computation (DRY)
    let content_hash = ContentHasher::hash_bytes(&content);
    debug!(content_hash = %content_hash, "Computed content hash");

    // Extract tenant context for workspace-scoped uniqueness
    // WHY-OODA81: Uniqueness must be scoped to workspace, not global
    // Same document in different workspaces is allowed (multi-tenancy)
    let workspace_id_for_storage = tenant_ctx
        .workspace_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let tenant_id_for_storage = tenant_ctx.tenant_id.clone();

    // WHY-OODA81+83: Use ContentHasher for workspace-scoped hash key
    // FIX-4: Duplicates now trigger re-ingestion instead of rejection
    let hash_key = ContentHasher::workspace_hash_key(&workspace_id_for_storage, &content_hash);
    debug!(hash_key = %hash_key, workspace_id = %workspace_id_for_storage, "Checking for workspace-scoped duplicate hash");
    if let Some(existing_doc_id) = state.kv_storage.get_by_id(&hash_key).await? {
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
        .kv_storage
        .upsert(&[(doc_metadata_key.clone(), doc_metadata)])
        .await?;

    // Store document content
    let doc_content_key = format!("{}-content", document_id);
    let doc_content = serde_json::json!({
        "content": text_content,
    });
    state
        .kv_storage
        .upsert(&[(doc_content_key, doc_content)])
        .await?;

    // Process through pipeline
    let result = state
        .pipeline
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

    // Store chunks in KV storage
    let chunks: Vec<(String, serde_json::Value)> = result
        .chunks
        .iter()
        .map(|c| {
            (
                c.id.clone(),
                serde_json::json!({
                    "content": c.content,
                    "document_id": document_id,
                    "index": c.index,
                    "source_file": filename,
                }),
            )
        })
        .collect();

    state.kv_storage.upsert(&chunks).await?;

    // SPEC-033: Get workspace-specific vector storage for file embeddings
    // WHY-OODA223: STRICT mode - fail loudly if workspace storage unavailable
    // to prevent file embeddings from being stored in the wrong (global) table
    let workspace_vector_storage =
        get_workspace_vector_storage_strict(&state, &workspace_id_for_storage).await?;

    // Store chunk embeddings in vector storage for semantic search
    let mut chunk_embeddings_stored = 0;
    for chunk in &result.chunks {
        if let Some(embedding) = &chunk.embedding {
            let mut metadata = serde_json::json!({
                "type": "chunk",
                "document_id": document_id,
                "index": chunk.index,
                "content": chunk.content,
                "source_file": filename,
            });

            // Add tenant and workspace IDs if present
            if let Some(ref tid) = tenant_id_for_storage {
                metadata["tenant_id"] = serde_json::json!(tid);
            }
            metadata["workspace_id"] = serde_json::json!(&workspace_id_for_storage);

            match workspace_vector_storage
                .upsert(&[(chunk.id.clone(), embedding.clone(), metadata)])
                .await
            {
                Ok(_) => {
                    chunk_embeddings_stored += 1;
                    tracing::info!(chunk_id = %chunk.id, "VECTOR STORAGE: Chunk embedding stored OK");
                }
                Err(e) => {
                    tracing::error!(chunk_id = %chunk.id, error = %e, "VECTOR STORAGE: Failed to store chunk embedding");
                }
            }
        }
    }
    tracing::info!(
        chunk_embeddings_stored = chunk_embeddings_stored,
        total_chunks = result.chunks.len(),
        "VECTOR STORAGE: Chunk embedding storage complete"
    );

    // Store entities and relationships in graph storage
    tracing::info!(
        extraction_count = result.extractions.len(),
        "GRAPH STORAGE: Processing extractions"
    );
    for extraction in &result.extractions {
        tracing::info!(
            entity_count = extraction.entities.len(),
            relationship_count = extraction.relationships.len(),
            "GRAPH STORAGE: Extraction content"
        );
        for entity in &extraction.entities {
            tracing::info!(
                entity_name = %entity.name,
                entity_type = %entity.entity_type,
                source_chunk_ids = ?entity.source_chunk_ids,
                "GRAPH STORAGE: Storing entity with chunk linkage"
            );
            let mut properties = std::collections::HashMap::new();
            properties.insert(
                "entity_type".to_string(),
                serde_json::json!(entity.entity_type),
            );
            properties.insert(
                "description".to_string(),
                serde_json::json!(entity.description),
            );
            properties.insert(
                "importance".to_string(),
                serde_json::json!(entity.importance),
            );
            properties.insert(
                "source_ids".to_string(),
                serde_json::json!(vec![&document_id]),
            );
            // CRITICAL: Store source_chunk_ids for Local/Global query mode chunk retrieval
            properties.insert(
                "source_chunk_ids".to_string(),
                serde_json::json!(&entity.source_chunk_ids),
            );
            // Add tenant scoping
            if let Some(ref tid) = tenant_id_for_storage {
                properties.insert("tenant_id".to_string(), serde_json::json!(tid));
            }
            properties.insert(
                "workspace_id".to_string(),
                serde_json::json!(&workspace_id_for_storage),
            );

            match state
                .graph_storage
                .upsert_node(&entity.name, properties)
                .await
            {
                Ok(_) => {
                    tracing::info!(entity_name = %entity.name, "GRAPH STORAGE: Entity stored OK")
                }
                Err(e) => {
                    tracing::error!(entity_name = %entity.name, error = %e, "GRAPH STORAGE: Failed to store entity")
                }
            }

            // CRITICAL: Also store entity embedding in vector storage for query_local retrieval
            tracing::info!(
                entity_name = %entity.name,
                has_embedding = entity.embedding.is_some(),
                embedding_dim = entity.embedding.as_ref().map(|e| e.len()).unwrap_or(0),
                "Checking entity embedding for storage"
            );
            // SPEC-033: Use workspace-specific vector storage for entity embeddings
            if let Some(embedding) = &entity.embedding {
                let mut metadata = serde_json::json!({
                    "type": "entity",
                    "entity_name": entity.name,
                    "entity_type": entity.entity_type,
                    "description": entity.description,
                    "document_id": document_id,
                    "source_chunk_ids": entity.source_chunk_ids,
                });
                if let Some(ref tid) = tenant_id_for_storage {
                    metadata["tenant_id"] = serde_json::json!(tid);
                }
                metadata["workspace_id"] = serde_json::json!(&workspace_id_for_storage);

                // Use entity name as vector ID for dedup
                let entity_id = format!("entity:{}", entity.name);
                if let Err(e) = workspace_vector_storage
                    .upsert(&[(entity_id.clone(), embedding.clone(), metadata)])
                    .await
                {
                    tracing::error!(entity_id = %entity_id, error = %e, "VECTOR STORAGE: Failed to store entity embedding");
                } else {
                    tracing::info!(entity_id = %entity_id, "VECTOR STORAGE: Entity embedding stored OK");
                }
            }
        }

        for relationship in &extraction.relationships {
            let mut properties = std::collections::HashMap::new();
            properties.insert(
                "relation_type".to_string(),
                serde_json::json!(relationship.relation_type),
            );
            properties.insert(
                "description".to_string(),
                serde_json::json!(relationship.description),
            );
            properties.insert("weight".to_string(), serde_json::json!(relationship.weight));
            properties.insert(
                "keywords".to_string(),
                serde_json::json!(relationship.keywords),
            );
            properties.insert(
                "source_ids".to_string(),
                serde_json::json!(vec![&document_id]),
            );
            // CRITICAL: Store source_chunk_id for relationship chunk linkage
            if let Some(ref chunk_id) = relationship.source_chunk_id {
                properties.insert(
                    "source_chunk_ids".to_string(),
                    serde_json::json!(vec![chunk_id]),
                );
            }
            // Add tenant scoping
            if let Some(ref tid) = tenant_id_for_storage {
                properties.insert("tenant_id".to_string(), serde_json::json!(tid));
            }
            properties.insert(
                "workspace_id".to_string(),
                serde_json::json!(&workspace_id_for_storage),
            );

            let _ = state
                .graph_storage
                .upsert_edge(&relationship.source, &relationship.target, properties)
                .await;
        }
    }

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
        "status": "completed",
        "chunk_count": result.stats.chunk_count,
        "entity_count": result.stats.entity_count,
        "relationship_count": result.stats.relationship_count,
        "tenant_id": tenant_id_for_storage,
        "workspace_id": workspace_id_for_storage,
        "custom_metadata": metadata,
        // Lineage information
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
        .kv_storage
        .upsert(&[(doc_metadata_key, completed_metadata)])
        .await?;

    // FIX-ISSUE-81 Phase 2: Dual-write document record to PostgreSQL
    // WHY: Without this, file uploads only write to KV storage. The PostgreSQL
    // `documents` table stays incomplete, causing Dashboard KPI mismatch.
    #[cfg(feature = "postgres")]
    if let Some(ref pdf_storage) = state.pdf_storage {
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
