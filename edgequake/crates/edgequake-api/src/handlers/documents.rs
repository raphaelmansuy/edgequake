//! Document ingestion handlers.

use axum::{extract::State, Json};
use edgequake_storage::KVStorage;
use serde::{Deserialize, Serialize};
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
pub async fn list_documents(State(state): State<AppState>) -> ApiResult<Json<ListDocumentsResponse>> {
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
}
