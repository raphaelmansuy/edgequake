//! Document ingestion handlers.

use axum::{extract::State, Json};
use axum_extra::extract::Multipart;
use chrono::Utc;
use edgequake_storage::KVStorage;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Document upload request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UploadDocumentRequest {
    /// Document content.
    pub content: String,

    /// Optional document title.
    #[serde(default)]
    pub title: Option<String>,

    /// Optional document metadata.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,

    /// Whether to process asynchronously (default: false for backwards compatibility)
    #[serde(default)]
    pub async_processing: bool,

    /// Optional track ID for batch grouping. If not provided, one will be generated.
    #[serde(default)]
    pub track_id: Option<String>,
}

/// Document upload response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UploadDocumentResponse {
    /// Generated document ID.
    pub document_id: String,

    /// Processing status.
    pub status: String,

    /// Task track ID (only set when async_processing is true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,

    /// Track ID for batch grouping.
    pub track_id: String,

    /// ID of existing document if this is a duplicate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<String>,

    /// Number of chunks created (only set for sync processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_count: Option<usize>,

    /// Number of entities extracted (only set for sync processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_count: Option<usize>,

    /// Number of relationships extracted (only set for sync processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship_count: Option<usize>,
}

/// Upload a document for processing.
#[utoipa::path(
    post,
    path = "/api/v1/documents",
    tag = "Documents",
    request_body = UploadDocumentRequest,
    responses(
        (status = 201, description = "Document uploaded successfully", body = UploadDocumentResponse),
        (status = 400, description = "Invalid request"),
        (status = 413, description = "Document too large")
    )
)]
pub async fn upload_document(
    State(state): State<AppState>,
    Json(request): Json<UploadDocumentRequest>,
) -> ApiResult<Json<UploadDocumentResponse>> {
    // Validate document size
    if request.content.len() > state.config.max_document_size {
        return Err(ApiError::BadRequest(format!(
            "Document exceeds maximum size of {} bytes",
            state.config.max_document_size
        )));
    }

    if request.content.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Document content cannot be empty".to_string(),
        ));
    }

    // Generate or use provided track_id
    let track_id = request.track_id.unwrap_or_else(|| {
        format!(
            "upload_{}_{}",
            Utc::now().format("%Y%m%d_%H%M%S"),
            &Uuid::new_v4().to_string()[..8]
        )
    });

    // Compute content hash for duplicate detection
    let mut hasher = Sha256::new();
    hasher.update(request.content.as_bytes());
    let content_hash = format!("{:x}", hasher.finalize());

    // Check for duplicate content (optional - search existing documents)
    // For now, we'll store the hash and the frontend can check duplicates if needed

    // Generate document ID
    let document_id = Uuid::new_v4().to_string();

    // Generate content summary (first 200 chars)
    let content_summary = if request.content.len() > 200 {
        format!("{}...", &request.content.chars().take(200).collect::<String>())
    } else {
        request.content.clone()
    };
    let content_length = request.content.len();

    // Store document metadata (including title, content_summary, content_length, track_id)
    let doc_metadata_key = format!("{}-metadata", document_id);
    let initial_status = if request.async_processing {
        "pending"
    } else {
        "processing"
    };
    let doc_metadata = serde_json::json!({
        "id": document_id,
        "title": request.title,
        "content_summary": content_summary,
        "content_length": content_length,
        "content_hash": content_hash,
        "track_id": track_id,
        "created_at": Utc::now().to_rfc3339(),
        "status": initial_status,
    });
    state
        .kv_storage
        .upsert(&[(doc_metadata_key.clone(), doc_metadata)])
        .await?;

    // Store the document content for processing
    let doc_content_key = format!("{}-content", document_id);
    let doc_content = serde_json::json!({
        "content": request.content,
    });
    state
        .kv_storage
        .upsert(&[(doc_content_key, doc_content)])
        .await?;

    // Handle async vs sync processing
    if request.async_processing {
        // Create task for background processing
        use edgequake_tasks::{Task, TaskType, TextInsertData};

        let task_data = TextInsertData {
            text: request.content.clone(),
            file_source: request.title.clone().unwrap_or_else(|| document_id.clone()),
            workspace_id: "default".to_string(),
            metadata: Some(serde_json::json!({
                "document_id": document_id,
                "title": request.title,
            })),
        };

        let task = Task::new(TaskType::Insert, serde_json::to_value(task_data).unwrap());
        let task_id = task.track_id.clone();

        // Store task
        state
            .task_storage
            .create_task(&task)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to create task: {}", e)))?;

        // Queue task for processing
        state
            .task_queue
            .send(task)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to queue task: {}", e)))?;

        Ok(Json(UploadDocumentResponse {
            document_id,
            status: "pending".to_string(),
            task_id: Some(task_id),
            track_id,
            duplicate_of: None,
            chunk_count: None,
            entity_count: None,
            relationship_count: None,
        }))
    } else {
        // Synchronous processing (original behavior)
        let result = state
            .pipeline
            .process(&document_id, &request.content)
            .await?;

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
                    }),
                )
            })
            .collect();

        state.kv_storage.upsert(&chunks).await?;

        // Store entities and relationships in graph storage
        for extraction in &result.extractions {
            for entity in &extraction.entities {
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

                state
                    .graph_storage
                    .upsert_node(&entity.name, properties)
                    .await?;
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

                state
                    .graph_storage
                    .upsert_edge(&relationship.source, &relationship.target, properties)
                    .await?;
            }
        }

        // Update document status to completed (preserve content_summary, content_length, track_id)
        let doc_metadata = serde_json::json!({
            "id": document_id,
            "title": request.title,
            "content_summary": content_summary,
            "content_length": content_length,
            "content_hash": content_hash,
            "track_id": track_id,
            "created_at": Utc::now().to_rfc3339(),
            "status": "completed",
            "chunk_count": result.stats.chunk_count,
            "entity_count": result.stats.entity_count,
            "relationship_count": result.stats.relationship_count,
        });
        state
            .kv_storage
            .upsert(&[(doc_metadata_key, doc_metadata)])
            .await?;

        Ok(Json(UploadDocumentResponse {
            document_id,
            status: "processed".to_string(),
            task_id: None,
            track_id,
            duplicate_of: None,
            chunk_count: Some(result.stats.chunk_count),
            entity_count: Some(result.stats.entity_count),
            relationship_count: Some(result.stats.relationship_count),
        }))
    }
}

/// List documents request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListDocumentsRequest {
    /// Page number.
    #[serde(default = "default_page")]
    pub page: usize,

    /// Page size.
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    20
}

/// Status counts for document filtering.
#[derive(Debug, Clone, Serialize, Default, ToSchema)]
pub struct StatusCounts {
    /// Number of pending documents.
    pub pending: usize,
    /// Number of processing documents.
    pub processing: usize,
    /// Number of completed documents.
    pub completed: usize,
    /// Number of failed documents.
    pub failed: usize,
}

/// List documents response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListDocumentsResponse {
    /// List of documents.
    pub documents: Vec<DocumentSummary>,

    /// Total document count.
    pub total: usize,

    /// Current page.
    pub page: usize,

    /// Page size.
    pub page_size: usize,

    /// Status counts for all documents (not just current page).
    pub status_counts: StatusCounts,
}

/// Document summary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentSummary {
    /// Document ID.
    pub id: String,

    /// Document title.
    pub title: Option<String>,

    /// Original file name (used for display if title is not set).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,

    /// First 200 characters of document content (preview).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_summary: Option<String>,

    /// Total length of document content in characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_length: Option<usize>,

    /// Number of chunks.
    pub chunk_count: usize,

    /// Number of entities extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_count: Option<usize>,

    /// Document processing status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Error message if processing failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    /// Track ID for batch grouping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<String>,

    /// Creation timestamp (ISO 8601 format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Last update timestamp (ISO 8601 format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// List all documents.
#[utoipa::path(
    get,
    path = "/api/v1/documents",
    tag = "Documents",
    responses(
        (status = 200, description = "Documents retrieved", body = ListDocumentsResponse)
    )
)]
pub async fn list_documents(
    State(state): State<AppState>,
) -> ApiResult<Json<ListDocumentsResponse>> {
    let keys = state.kv_storage.keys().await?;

    // Group by document and collect metadata keys
    let mut doc_chunks: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut metadata_keys: Vec<String> = Vec::new();

    for key in &keys {
        if key.ends_with("-metadata") {
            metadata_keys.push(key.clone());
        } else if let Some(doc_id) = key.split("-chunk-").next() {
            *doc_chunks.entry(doc_id.to_string()).or_default() += 1;
        }
    }

    // Fetch all metadata and store complete document info
    let metadata_values = state.kv_storage.get_by_ids(&metadata_keys).await?;
    
    // Store complete document metadata, keyed by document ID
    #[derive(Default)]
    struct DocMetadata {
        title: Option<String>,
        file_name: Option<String>,
        content_summary: Option<String>,
        content_length: Option<usize>,
        status: Option<String>,
        error_message: Option<String>,
        track_id: Option<String>,
        created_at: Option<String>,
        updated_at: Option<String>,
        entity_count: Option<usize>,
    }
    
    let mut doc_metadata: std::collections::HashMap<String, DocMetadata> =
        std::collections::HashMap::new();

    for value in metadata_values {
        if let Some(obj) = value.as_object() {
            if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                let mut meta = DocMetadata::default();
                
                // Get title from metadata
                meta.title = obj
                    .get("title")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                // Use title as file_name fallback if it looks like a filename
                if let Some(ref title) = meta.title {
                    if title.contains('.') {
                        meta.file_name = Some(title.clone());
                    }
                }
                
                // Get content_summary
                meta.content_summary = obj
                    .get("content_summary")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                // Get content_length
                meta.content_length = obj
                    .get("content_length")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);
                
                // Get status
                meta.status = obj
                    .get("status")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                // Get error_message
                meta.error_message = obj
                    .get("error_message")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                // Get track_id
                meta.track_id = obj
                    .get("track_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                // Get created_at
                meta.created_at = obj
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                // Get updated_at
                meta.updated_at = obj
                    .get("updated_at")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                // Get entity_count
                meta.entity_count = obj
                    .get("entity_count")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);
                
                doc_metadata.insert(id.to_string(), meta);
            }
        }
    }

    let documents: Vec<DocumentSummary> = doc_chunks
        .into_iter()
        .map(|(id, chunk_count)| {
            let meta = doc_metadata.remove(&id).unwrap_or_default();
            DocumentSummary {
                id,
                title: meta.title,
                file_name: meta.file_name,
                content_summary: meta.content_summary,
                content_length: meta.content_length,
                chunk_count,
                entity_count: meta.entity_count,
                status: meta.status,
                error_message: meta.error_message,
                track_id: meta.track_id,
                created_at: meta.created_at,
                updated_at: meta.updated_at,
            }
        })
        .collect();

    // Calculate status counts for all documents
    let status_counts = StatusCounts {
        pending: documents.iter().filter(|d| d.status.as_deref() == Some("pending")).count(),
        processing: documents.iter().filter(|d| d.status.as_deref() == Some("processing")).count(),
        completed: documents.iter().filter(|d| {
            d.status.is_none() || d.status.as_deref() == Some("completed") || d.status.as_deref() == Some("indexed")
        }).count(),
        failed: documents.iter().filter(|d| d.status.as_deref() == Some("failed")).count(),
    };

    Ok(Json(ListDocumentsResponse {
        total: documents.len(),
        documents,
        page: 1,
        page_size: 20,
        status_counts,
    }))
}

/// Get document by ID request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GetDocumentRequest {
    /// Document ID.
    pub document_id: String,
}

/// Document details response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentDetailResponse {
    /// Document ID.
    pub id: String,

    /// Document title.
    pub title: Option<String>,

    /// Number of chunks.
    pub chunk_count: usize,

    /// Document status.
    pub status: String,

    /// Metadata.
    pub metadata: Option<serde_json::Value>,
}

/// Get a document by ID.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}",
    tag = "Documents",
    params(
        ("document_id" = String, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document found", body = DocumentDetailResponse),
        (status = 404, description = "Document not found")
    )
)]
pub async fn get_document(
    State(state): State<AppState>,
    axum::extract::Path(document_id): axum::extract::Path<String>,
) -> ApiResult<Json<DocumentDetailResponse>> {
    let keys = state.kv_storage.keys().await?;

    // Find chunks belonging to this document
    let chunk_count = keys
        .iter()
        .filter(|k| k.starts_with(&format!("{}-chunk-", document_id)))
        .count();

    if chunk_count == 0 {
        return Err(ApiError::NotFound(format!(
            "Document {} not found",
            document_id
        )));
    }

    Ok(Json(DocumentDetailResponse {
        id: document_id,
        title: None,
        chunk_count,
        status: "processed".to_string(),
        metadata: None,
    }))
}

/// Document deletion response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DeleteDocumentResponse {
    /// Document ID.
    pub document_id: String,

    /// Whether the document was deleted.
    pub deleted: bool,

    /// Number of chunks deleted.
    pub chunks_deleted: usize,

    /// Number of entities affected.
    pub entities_affected: usize,

    /// Number of relationships affected.
    pub relationships_affected: usize,
}

/// Delete a document by ID.
#[utoipa::path(
    delete,
    path = "/api/v1/documents/{document_id}",
    tag = "Documents",
    params(
        ("document_id" = String, Path, description = "Document ID to delete")
    ),
    responses(
        (status = 200, description = "Document deleted", body = DeleteDocumentResponse),
        (status = 404, description = "Document not found")
    )
)]
pub async fn delete_document(
    State(state): State<AppState>,
    axum::extract::Path(document_id): axum::extract::Path<String>,
) -> ApiResult<Json<DeleteDocumentResponse>> {
    let keys = state.kv_storage.keys().await?;

    // Find chunks belonging to this document
    let chunk_ids: Vec<String> = keys
        .iter()
        .filter(|k| k.starts_with(&format!("{}-chunk-", document_id)))
        .cloned()
        .collect();

    if chunk_ids.is_empty() {
        return Err(ApiError::NotFound(format!(
            "Document {} not found",
            document_id
        )));
    }

    let chunks_deleted = chunk_ids.len();

    // Delete chunks from KV storage
    state.kv_storage.delete(&chunk_ids).await?;

    // TODO: Implement cascade deletion for entities and relationships
    // This would require updating the graph storage to remove orphaned entities

    Ok(Json(DeleteDocumentResponse {
        document_id,
        deleted: true,
        chunks_deleted,
        entities_affected: 0,
        relationships_affected: 0,
    }))
}

// ============================================================================
// File Upload (Multipart)
// ============================================================================

/// File upload response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FileUploadResponse {
    /// Generated document ID.
    pub document_id: String,

    /// Original filename.
    pub filename: String,

    /// File size in bytes.
    pub size: usize,

    /// Content hash (SHA-256).
    pub content_hash: String,

    /// Processing status.
    pub status: String,

    /// Number of chunks created.
    pub chunk_count: usize,

    /// Number of entities extracted.
    pub entity_count: usize,

    /// Number of relationships extracted.
    pub relationship_count: usize,

    /// Whether this was a duplicate (already processed).
    pub is_duplicate: bool,
}

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
    mut multipart: Multipart,
) -> ApiResult<Json<FileUploadResponse>> {
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

    // Validate file size
    if content.len() > state.config.max_document_size {
        return Err(ApiError::BadRequest(format!(
            "File exceeds maximum size of {} bytes",
            state.config.max_document_size
        )));
    }

    // Validate file extension
    let extension = filename.rsplit('.').next().unwrap_or("").to_lowercase();

    let allowed_extensions = [
        "txt", "md", "json", "csv", "html", "htm", "xml", "yaml", "yml",
    ];
    if !allowed_extensions.contains(&extension.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Unsupported file type: .{}. Allowed types: {:?}",
            extension, allowed_extensions
        )));
    }

    // Convert to UTF-8 string
    let text_content = String::from_utf8(content.clone())
        .map_err(|e| ApiError::BadRequest(format!("File is not valid UTF-8: {}", e)))?;

    if text_content.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "File content cannot be empty".to_string(),
        ));
    }

    // Calculate content hash for deduplication
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let content_hash = hex::encode(hasher.finalize());

    // Check for duplicate
    let hash_key = format!("doc:hash:{}", content_hash);
    if let Some(existing_doc_id) = state.kv_storage.get_by_id(&hash_key).await? {
        if let Some(doc_id_str) = existing_doc_id.as_str() {
            return Ok(Json(FileUploadResponse {
                document_id: doc_id_str.to_string(),
                filename,
                size: content.len(),
                content_hash,
                status: "duplicate".to_string(),
                chunk_count: 0,
                entity_count: 0,
                relationship_count: 0,
                is_duplicate: true,
            }));
        }
    }

    // Generate document ID
    let document_id = Uuid::new_v4().to_string();

    // Store hash mapping for deduplication
    state
        .kv_storage
        .upsert(&[(hash_key, serde_json::json!(document_id))])
        .await?;

    // Store file metadata
    let file_meta = serde_json::json!({
        "filename": filename,
        "size": content.len(),
        "content_hash": content_hash,
        "extension": extension,
        "metadata": metadata,
        "uploaded_at": chrono::Utc::now().to_rfc3339(),
    });
    state
        .kv_storage
        .upsert(&[(format!("doc:meta:{}", document_id), file_meta)])
        .await?;

    // Process through pipeline
    let result = state.pipeline.process(&document_id, &text_content).await?;

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

    Ok(Json(FileUploadResponse {
        document_id,
        filename,
        size: content.len(),
        content_hash,
        status: "processed".to_string(),
        chunk_count: result.stats.chunk_count,
        entity_count: result.stats.entity_count,
        relationship_count: result.stats.relationship_count,
        is_duplicate: false,
    }))
}

/// Batch file upload response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchUploadResponse {
    /// Total files received.
    pub total_files: usize,

    /// Successfully processed files.
    pub processed: usize,

    /// Duplicate files (skipped).
    pub duplicates: usize,

    /// Failed files.
    pub failed: usize,

    /// Results for each file.
    pub results: Vec<BatchFileResult>,
}

/// Result for a single file in batch upload.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchFileResult {
    /// Original filename.
    pub filename: String,

    /// Document ID if successful.
    pub document_id: Option<String>,

    /// Status: processed, duplicate, or failed.
    pub status: String,

    /// Error message if failed.
    pub error: Option<String>,
}

/// Upload multiple files via multipart form.
#[utoipa::path(
    post,
    path = "/api/v1/documents/upload/batch",
    tag = "Documents",
    request_body(content_type = "multipart/form-data", description = "Files to upload"),
    responses(
        (status = 201, description = "Batch upload completed", body = BatchUploadResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn upload_files_batch(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> ApiResult<Json<BatchUploadResponse>> {
    let mut results = Vec::new();
    let mut processed = 0usize;
    let mut duplicates = 0usize;
    let mut failed = 0usize;

    // Collect all files first
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read multipart field: {}", e)))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "files" || field_name == "file" {
            let filename = field
                .file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("file_{}.txt", files.len()));

            let content = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?
                .to_vec();

            files.push((filename, content));
        }
    }

    // Process each file
    for (filename, content) in files {
        let result = process_single_file(&state, &filename, &content).await;

        match result {
            Ok((doc_id, is_duplicate)) => {
                if is_duplicate {
                    duplicates += 1;
                    results.push(BatchFileResult {
                        filename,
                        document_id: Some(doc_id),
                        status: "duplicate".to_string(),
                        error: None,
                    });
                } else {
                    processed += 1;
                    results.push(BatchFileResult {
                        filename,
                        document_id: Some(doc_id),
                        status: "processed".to_string(),
                        error: None,
                    });
                }
            }
            Err(e) => {
                failed += 1;
                results.push(BatchFileResult {
                    filename,
                    document_id: None,
                    status: "failed".to_string(),
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(Json(BatchUploadResponse {
        total_files: results.len(),
        processed,
        duplicates,
        failed,
        results,
    }))
}

/// Process a single file and return (document_id, is_duplicate).
async fn process_single_file(
    state: &AppState,
    filename: &str,
    content: &[u8],
) -> Result<(String, bool), ApiError> {
    // Validate file size
    if content.len() > state.config.max_document_size {
        return Err(ApiError::BadRequest(format!(
            "File {} exceeds maximum size",
            filename
        )));
    }

    // Validate extension
    let extension = filename.rsplit('.').next().unwrap_or("").to_lowercase();

    let allowed_extensions = [
        "txt", "md", "json", "csv", "html", "htm", "xml", "yaml", "yml",
    ];
    if !allowed_extensions.contains(&extension.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Unsupported file type: .{}",
            extension
        )));
    }

    // Convert to UTF-8
    let text_content = String::from_utf8(content.to_vec())
        .map_err(|_| ApiError::BadRequest(format!("File {} is not valid UTF-8", filename)))?;

    if text_content.trim().is_empty() {
        return Err(ApiError::ValidationError(format!(
            "File {} is empty",
            filename
        )));
    }

    // Calculate hash
    let mut hasher = Sha256::new();
    hasher.update(content);
    let content_hash = hex::encode(hasher.finalize());

    // Check for duplicate
    let hash_key = format!("doc:hash:{}", content_hash);
    if let Some(existing) = state.kv_storage.get_by_id(&hash_key).await? {
        if let Some(doc_id) = existing.as_str() {
            return Ok((doc_id.to_string(), true));
        }
    }

    // Generate document ID
    let document_id = Uuid::new_v4().to_string();

    // Store hash mapping
    state
        .kv_storage
        .upsert(&[(hash_key, serde_json::json!(document_id))])
        .await?;

    // Process through pipeline
    let result = state.pipeline.process(&document_id, &text_content).await?;

    // Store chunks
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

    Ok((document_id, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_request_validation() {
        let request = UploadDocumentRequest {
            content: "Test content".to_string(),
            title: Some("Test".to_string()),
            metadata: None,
            async_processing: false,
            track_id: None,
        };

        assert!(!request.content.is_empty());
    }

    #[test]
    fn test_upload_request_serialization() {
        let json = r#"{
            "content": "Hello world",
            "title": "Test Doc",
            "metadata": {"key": "value"}
        }"#;

        let request: UploadDocumentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "Hello world");
        assert_eq!(request.title, Some("Test Doc".to_string()));
        assert!(request.metadata.is_some());
    }

    #[test]
    fn test_upload_request_minimal() {
        let json = r#"{"content": "Just content"}"#;

        let request: UploadDocumentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "Just content");
        assert!(request.title.is_none());
        assert!(request.metadata.is_none());
    }

    #[test]
    fn test_upload_response_serialization() {
        let response = UploadDocumentResponse {
            document_id: "doc-123".to_string(),
            status: "processed".to_string(),
            task_id: None,
            track_id: "upload_20240101_abc12345".to_string(),
            duplicate_of: None,
            chunk_count: Some(5),
            entity_count: Some(3),
            relationship_count: Some(2),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-123"));
        assert!(json.contains("processed"));
    }

    #[test]
    fn test_list_documents_request_defaults() {
        let json = r#"{}"#;
        let request: ListDocumentsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.page, 1);
        assert_eq!(request.page_size, 20);
    }

    #[test]
    fn test_list_documents_request_custom() {
        let json = r#"{"page": 3, "page_size": 50}"#;
        let request: ListDocumentsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.page, 3);
        assert_eq!(request.page_size, 50);
    }

    #[test]
    fn test_document_summary_serialization() {
        let summary = DocumentSummary {
            id: "doc-456".to_string(),
            title: Some("My Document".to_string()),
            file_name: None,
            content_summary: Some("This is the first 200 chars of the document...".to_string()),
            content_length: Some(5000),
            chunk_count: 10,
            entity_count: None,
            status: Some("completed".to_string()),
            error_message: None,
            track_id: Some("upload_20240101_abc12345".to_string()),
            created_at: None,
            updated_at: None,
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("doc-456"));
        assert!(json.contains("My Document"));
    }

    #[test]
    fn test_list_documents_response_serialization() {
        let response = ListDocumentsResponse {
            documents: vec![DocumentSummary {
                id: "doc-1".to_string(),
                title: None,
                file_name: None,
                content_summary: None,
                content_length: None,
                chunk_count: 5,
                entity_count: None,
                status: Some("completed".to_string()),
                error_message: None,
                track_id: None,
                created_at: None,
                updated_at: None,
            }],
            total: 1,
            page: 1,
            page_size: 20,
            status_counts: StatusCounts {
                pending: 0,
                processing: 0,
                completed: 1,
                failed: 0,
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-1"));
        assert!(json.contains("\"total\":1"));
    }

    #[test]
    fn test_document_detail_response_serialization() {
        let response = DocumentDetailResponse {
            id: "doc-789".to_string(),
            title: Some("Test".to_string()),
            chunk_count: 5,
            status: "processed".to_string(),
            metadata: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-789"));
        assert!(json.contains("processed"));
    }

    #[test]
    fn test_delete_document_response_serialization() {
        let response = DeleteDocumentResponse {
            document_id: "doc-to-delete".to_string(),
            deleted: true,
            chunks_deleted: 7,
            entities_affected: 2,
            relationships_affected: 1,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-to-delete"));
        assert!(json.contains("\"deleted\":true"));
        assert!(json.contains("\"chunks_deleted\":7"));
    }

    #[test]
    fn test_default_page() {
        assert_eq!(default_page(), 1);
    }

    #[test]
    fn test_default_page_size() {
        assert_eq!(default_page_size(), 20);
    }
}
