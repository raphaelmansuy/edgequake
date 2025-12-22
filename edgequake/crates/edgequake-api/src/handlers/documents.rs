//! Document ingestion handlers.

use axum::{
    extract::State,
    Json,
};
use axum_extra::extract::Multipart;
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
}

/// Document upload response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UploadDocumentResponse {
    /// Generated document ID.
    pub document_id: String,

    /// Processing status.
    pub status: String,

    /// Number of chunks created.
    pub chunk_count: usize,

    /// Number of entities extracted.
    pub entity_count: usize,

    /// Number of relationships extracted.
    pub relationship_count: usize,
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

    // Generate document ID
    let document_id = Uuid::new_v4().to_string();

    // Process through pipeline
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

    Ok(Json(UploadDocumentResponse {
        document_id,
        status: "processed".to_string(),
        chunk_count: result.stats.chunk_count,
        entity_count: result.stats.entity_count,
        relationship_count: result.stats.relationship_count,
    }))
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
}

/// Document summary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentSummary {
    /// Document ID.
    pub id: String,

    /// Document title.
    pub title: Option<String>,

    /// Number of chunks.
    pub chunk_count: usize,
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

    // Group by document
    let mut doc_chunks: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for key in &keys {
        if let Some(doc_id) = key.split("-chunk-").next() {
            *doc_chunks.entry(doc_id.to_string()).or_default() += 1;
        }
    }

    let documents: Vec<DocumentSummary> = doc_chunks
        .into_iter()
        .map(|(id, chunk_count)| DocumentSummary {
            id,
            title: None,
            chunk_count,
        })
        .collect();

    Ok(Json(ListDocumentsResponse {
        total: documents.len(),
        documents,
        page: 1,
        page_size: 20,
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
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        ApiError::BadRequest(format!("Failed to read multipart field: {}", e))
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        
        match field_name.as_str() {
            "file" => {
                // Get filename
                filename = field
                    .file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unnamed.txt".to_string());
                
                // Read file content
                content = field.bytes().await.map_err(|e| {
                    ApiError::BadRequest(format!("Failed to read file content: {}", e))
                })?.to_vec();
            }
            "metadata" => {
                // Optional metadata field
                let text = field.text().await.map_err(|e| {
                    ApiError::BadRequest(format!("Failed to read metadata: {}", e))
                })?;
                
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
    let extension = filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase();
    
    let allowed_extensions = ["txt", "md", "json", "csv", "html", "htm", "xml", "yaml", "yml"];
    if !allowed_extensions.contains(&extension.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Unsupported file type: .{}. Allowed types: {:?}",
            extension, allowed_extensions
        )));
    }
    
    // Convert to UTF-8 string
    let text_content = String::from_utf8(content.clone()).map_err(|e| {
        ApiError::BadRequest(format!("File is not valid UTF-8: {}", e))
    })?;
    
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
    state.kv_storage.upsert(&[
        (hash_key, serde_json::json!(document_id)),
    ]).await?;
    
    // Store file metadata
    let file_meta = serde_json::json!({
        "filename": filename,
        "size": content.len(),
        "content_hash": content_hash,
        "extension": extension,
        "metadata": metadata,
        "uploaded_at": chrono::Utc::now().to_rfc3339(),
    });
    state.kv_storage.upsert(&[
        (format!("doc:meta:{}", document_id), file_meta),
    ]).await?;
    
    // Process through pipeline
    let result = state
        .pipeline
        .process(&document_id, &text_content)
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
    
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        ApiError::BadRequest(format!("Failed to read multipart field: {}", e))
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "files" || field_name == "file" {
            let filename = field
                .file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("file_{}.txt", files.len()));
            
            let content = field.bytes().await.map_err(|e| {
                ApiError::BadRequest(format!("Failed to read file: {}", e))
            })?.to_vec();
            
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
    let extension = filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase();
    
    let allowed_extensions = ["txt", "md", "json", "csv", "html", "htm", "xml", "yaml", "yml"];
    if !allowed_extensions.contains(&extension.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Unsupported file type: .{}",
            extension
        )));
    }
    
    // Convert to UTF-8
    let text_content = String::from_utf8(content.to_vec()).map_err(|_| {
        ApiError::BadRequest(format!("File {} is not valid UTF-8", filename))
    })?;
    
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
    state.kv_storage.upsert(&[
        (hash_key, serde_json::json!(document_id)),
    ]).await?;
    
    // Process through pipeline
    let result = state
        .pipeline
        .process(&document_id, &text_content)
        .await?;
    
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
            chunk_count: 5,
            entity_count: 3,
            relationship_count: 2,
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
            chunk_count: 10,
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
                chunk_count: 5,
            }],
            total: 1,
            page: 1,
            page_size: 20,
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
