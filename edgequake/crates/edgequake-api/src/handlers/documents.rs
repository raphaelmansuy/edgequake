//! Document ingestion handlers.

use axum::{extract::State, Json};
use axum_extra::extract::Multipart;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::debug;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
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

    /// Enable gleaning (multiple extraction passes) for higher quality entity extraction.
    #[serde(default = "default_enable_gleaning")]
    pub enable_gleaning: bool,

    /// Maximum number of gleaning passes (1-3 recommended).
    #[serde(default = "default_max_gleaning")]
    pub max_gleaning: usize,

    /// Enable LLM-powered description summarization during merge.
    #[serde(default = "default_use_llm_summarization")]
    pub use_llm_summarization: bool,
}

fn default_enable_gleaning() -> bool {
    true
}

fn default_max_gleaning() -> usize {
    1
}

fn default_use_llm_summarization() -> bool {
    true
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

    /// Cost information (only set for sync processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<DocumentCostInfo>,
}

/// Cost information for a processed document.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentCostInfo {
    /// Total cost in USD.
    pub total_cost_usd: f64,

    /// Formatted cost string (e.g., "$0.0045").
    pub formatted_cost: String,

    /// Total input tokens used.
    pub input_tokens: usize,

    /// Total output tokens used.
    pub output_tokens: usize,

    /// Total tokens (input + output).
    pub total_tokens: usize,

    /// LLM model used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,

    /// Embedding model used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
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
    tenant_ctx: TenantContext,
    Json(request): Json<UploadDocumentRequest>,
) -> ApiResult<Json<UploadDocumentResponse>> {
    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Uploading document with tenant context"
    );

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
        format!(
            "{}...",
            &request.content.chars().take(200).collect::<String>()
        )
    } else {
        request.content.clone()
    };
    let content_length = request.content.len();

    // Store document metadata (including title, content_summary, content_length, track_id, tenant context)
    let doc_metadata_key = format!("{}-metadata", document_id);
    let initial_status = if request.async_processing {
        "pending"
    } else {
        "processing"
    };

    // Extract tenant context for storage
    let workspace_id_for_storage = tenant_ctx
        .workspace_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let tenant_id_for_storage = tenant_ctx.tenant_id.clone();

    let doc_metadata = serde_json::json!({
        "id": document_id,
        "title": request.title,
        "content_summary": content_summary,
        "content_length": content_length,
        "content_hash": content_hash,
        "track_id": track_id,
        "created_at": Utc::now().to_rfc3339(),
        "status": initial_status,
        "tenant_id": tenant_id_for_storage,
        "workspace_id": workspace_id_for_storage,
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

        // Use tenant context for workspace_id, fallback to "default"
        let workspace_id = tenant_ctx
            .workspace_id
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let tenant_id = tenant_ctx.tenant_id.clone();

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
            cost: None, // Cost will be calculated when processing completes
        }))
    } else {
        // Synchronous processing (original behavior)
        // Broadcast job started
        let start_time = std::time::Instant::now();
        state.progress_broadcaster.job_started(&document_id, 1, 1);

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

        // Store chunk embeddings in vector storage for semantic search
        let mut chunk_embeddings_stored = 0;
        for chunk in &result.chunks {
            if let Some(embedding) = &chunk.embedding {
                let mut metadata = serde_json::json!({
                    "type": "chunk",
                    "document_id": document_id,
                    "index": chunk.index,
                    "content": chunk.content,
                    "start_line": chunk.start_line,
                    "end_line": chunk.end_line,
                    "chunk_index": chunk.index,
                });

                // Add tenant and workspace IDs if present
                if let Some(ref tid) = tenant_id_for_storage {
                    metadata["tenant_id"] = serde_json::json!(tid);
                }
                metadata["workspace_id"] = serde_json::json!(&workspace_id_for_storage);

                match state
                    .vector_storage
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

        // Broadcast document progress (chunking complete)
        state
            .progress_broadcaster
            .document_progress(&document_id, 0, 1, 3);

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
                // CRITICAL: Store source_chunk_ids for Local/Global query mode chunk retrieval
                properties.insert(
                    "source_chunk_ids".to_string(),
                    serde_json::json!(&entity.source_chunk_ids),
                );

                state
                    .graph_storage
                    .upsert_node(&entity.name, properties)
                    .await?;

                // CRITICAL: Also store entity embedding in vector storage for query_local retrieval
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

                    let entity_id = format!("entity:{}", entity.name);
                    if let Err(e) = state
                        .vector_storage
                        .upsert(&[(entity_id.clone(), embedding.clone(), metadata)])
                        .await
                    {
                        tracing::error!(entity_id = %entity_id, error = %e, "Failed to store entity embedding");
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

                state
                    .graph_storage
                    .upsert_edge(&relationship.source, &relationship.target, properties)
                    .await?;
            }
        }

        // Broadcast document progress (extraction complete)
        state
            .progress_broadcaster
            .document_progress(&document_id, result.stats.entity_count, 2, 3);

        // Update document status to completed (preserve content_summary, content_length, track_id, tenant context)
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
            "tenant_id": tenant_id_for_storage,
            "workspace_id": workspace_id_for_storage,
            "cost_usd": result.stats.cost_usd,
            "input_tokens": result.stats.input_tokens,
            "output_tokens": result.stats.output_tokens,
            "total_tokens": result.stats.total_tokens,
            "llm_model": result.stats.llm_model,
            "embedding_model": result.stats.embedding_model,
        });
        state
            .kv_storage
            .upsert(&[(doc_metadata_key, doc_metadata)])
            .await?;

        // Broadcast job finished
        let duration = start_time.elapsed();
        state
            .progress_broadcaster
            .document_progress(&document_id, result.stats.entity_count, 3, 3);
        state
            .progress_broadcaster
            .job_finished(1, duration.as_millis() as u64);

        // Build cost info from stats
        let cost = Some(DocumentCostInfo {
            total_cost_usd: result.stats.cost_usd,
            formatted_cost: format!("${:.6}", result.stats.cost_usd),
            input_tokens: result.stats.input_tokens,
            output_tokens: result.stats.output_tokens,
            total_tokens: result.stats.total_tokens,
            llm_model: result.stats.llm_model.clone(),
            embedding_model: result.stats.embedding_model.clone(),
        });

        Ok(Json(UploadDocumentResponse {
            document_id,
            status: "processed".to_string(),
            task_id: None,
            track_id,
            duplicate_of: None,
            chunk_count: Some(result.stats.chunk_count),
            entity_count: Some(result.stats.entity_count),
            relationship_count: Some(result.stats.relationship_count),
            cost,
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

    /// Total cost in USD for processing this document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,

    /// Input tokens used for processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<usize>,

    /// Output tokens used for processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<usize>,

    /// Total tokens (input + output).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<usize>,

    /// LLM model used for processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,

    /// Embedding model used for processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
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
#[allow(clippy::field_reassign_with_default)]
pub async fn list_documents(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
) -> ApiResult<Json<ListDocumentsResponse>> {
    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Listing documents with tenant context"
    );

    let keys = state.kv_storage.keys().await?;
    debug!(key_count = keys.len(), "Total keys in KV storage");
    debug!(keys = ?keys, "All keys in KV storage");

    // Group by document and collect metadata keys
    let mut doc_chunks: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut metadata_keys: Vec<String> = Vec::new();

    for key in &keys {
        if key.ends_with("-metadata") {
            debug!(metadata_key = %key, "Found metadata key");
            metadata_keys.push(key.clone());
        } else if key.contains("-chunk-") {
            // Only count actual chunk keys (e.g., "doc-id-chunk-0")
            if let Some(doc_id) = key.split("-chunk-").next() {
                // Filter out non-document keys (like -metadata, -content suffixes)
                if !doc_id.ends_with("-metadata") && !doc_id.ends_with("-content") {
                    *doc_chunks.entry(doc_id.to_string()).or_default() += 1;
                }
            }
        }
    }

    // Fetch all metadata and store complete document info
    debug!(
        metadata_keys_count = metadata_keys.len(),
        "Fetching metadata for keys"
    );
    let metadata_values = state.kv_storage.get_by_ids(&metadata_keys).await?;
    debug!(
        metadata_values_count = metadata_values.len(),
        "Metadata values retrieved"
    );

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
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        cost_usd: Option<f64>,
        input_tokens: Option<usize>,
        output_tokens: Option<usize>,
        total_tokens: Option<usize>,
        llm_model: Option<String>,
        embedding_model: Option<String>,
    }

    let mut doc_metadata: std::collections::HashMap<String, DocMetadata> =
        std::collections::HashMap::new();

    for value in metadata_values {
        debug!(value = ?value, "Processing metadata value");
        if let Some(obj) = value.as_object() {
            if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                let title_val = obj.get("title");
                debug!(doc_id = %id, title = ?title_val, "Extracted ID and title from metadata");

                // WHY: We build DocMetadata incrementally because fields are extracted
                // conditionally from JSON, and some fields depend on others (e.g., file_name
                // is derived from title). Struct initializer syntax doesn't work well here.
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

                // Get tenant_id
                meta.tenant_id = obj
                    .get("tenant_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Get workspace_id
                meta.workspace_id = obj
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Get cost_usd
                meta.cost_usd = obj.get("cost_usd").and_then(|v| v.as_f64());

                // Get input_tokens
                meta.input_tokens = obj
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);

                // Get output_tokens
                meta.output_tokens = obj
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);

                // Get total_tokens
                meta.total_tokens = obj
                    .get("total_tokens")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);

                // Get llm_model
                meta.llm_model = obj
                    .get("llm_model")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Get embedding_model
                meta.embedding_model = obj
                    .get("embedding_model")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                doc_metadata.insert(id.to_string(), meta);
            }
        }
    }

    // Filter documents by tenant context
    let filter_workspace_id = tenant_ctx.workspace_id.clone();
    let filter_tenant_id = tenant_ctx.tenant_id.clone();

    // Helper function to check if document matches tenant context
    let matches_tenant_context = |meta: &DocMetadata| -> bool {
        // If filter_workspace_id is set, document must match
        if let Some(ref filter_ws) = filter_workspace_id {
            if meta.workspace_id.as_ref() != Some(filter_ws) {
                return false;
            }
        }
        // If filter_tenant_id is set, document must match
        if let Some(ref filter_tid) = filter_tenant_id {
            if meta.tenant_id.as_ref() != Some(filter_tid) {
                return false;
            }
        }
        true
    };

    // Build document list from BOTH:
    // 1. Documents with chunks (processed)
    // 2. Documents with metadata but no chunks yet (pending/processing)
    let mut documents: Vec<DocumentSummary> = doc_chunks
        .into_iter()
        .filter_map(|(id, chunk_count)| {
            let meta = doc_metadata.remove(&id).unwrap_or_default();
            // Filter by tenant context
            if !matches_tenant_context(&meta) {
                return None;
            }
            Some(DocumentSummary {
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
                cost_usd: meta.cost_usd,
                input_tokens: meta.input_tokens,
                output_tokens: meta.output_tokens,
                total_tokens: meta.total_tokens,
                llm_model: meta.llm_model,
                embedding_model: meta.embedding_model,
            })
        })
        .collect();

    // Add documents that have metadata but no chunks yet (pending/processing)
    for (id, meta) in doc_metadata {
        // Filter by tenant context
        if !matches_tenant_context(&meta) {
            continue;
        }
        documents.push(DocumentSummary {
            id,
            title: meta.title,
            file_name: meta.file_name,
            content_summary: meta.content_summary,
            content_length: meta.content_length,
            chunk_count: 0,
            entity_count: meta.entity_count,
            status: meta.status,
            error_message: meta.error_message,
            track_id: meta.track_id,
            created_at: meta.created_at,
            updated_at: meta.updated_at,
            cost_usd: meta.cost_usd,
            input_tokens: meta.input_tokens,
            output_tokens: meta.output_tokens,
            total_tokens: meta.total_tokens,
            llm_model: meta.llm_model,
            embedding_model: meta.embedding_model,
        });
    }

    // Sort by created_at descending (newest first)
    documents.sort_by(|a, b| {
        b.created_at
            .as_deref()
            .unwrap_or("")
            .cmp(a.created_at.as_deref().unwrap_or(""))
    });

    // Calculate status counts for all documents
    let status_counts = StatusCounts {
        pending: documents
            .iter()
            .filter(|d| d.status.as_deref() == Some("pending"))
            .count(),
        processing: documents
            .iter()
            .filter(|d| d.status.as_deref() == Some("processing"))
            .count(),
        completed: documents
            .iter()
            .filter(|d| {
                d.status.is_none()
                    || d.status.as_deref() == Some("completed")
                    || d.status.as_deref() == Some("indexed")
            })
            .count(),
        failed: documents
            .iter()
            .filter(|d| d.status.as_deref() == Some("failed"))
            .count(),
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

/// Document details response with full content.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentDetailResponse {
    /// Document ID.
    pub id: String,

    /// Document title.
    pub title: Option<String>,

    /// Original file name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,

    /// Full document content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Content summary (first 200 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_summary: Option<String>,

    /// Total content length in characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_length: Option<usize>,

    /// Content hash (SHA-256) for deduplication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,

    /// Number of chunks.
    pub chunk_count: usize,

    /// Number of entities extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_count: Option<usize>,

    /// Number of relationships extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship_count: Option<usize>,

    /// Document processing status.
    pub status: String,

    /// Error message if processing failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    /// Source type (file, text, url).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,

    /// MIME type of the document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// File size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<usize>,

    /// Track ID for batch grouping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<String>,

    /// Tenant ID for multi-tenancy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,

    /// Workspace ID for multi-tenancy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,

    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Last update timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,

    /// Processing completed timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processed_at: Option<String>,

    /// Extraction lineage information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lineage: Option<DocumentLineage>,

    /// Additional custom metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Extraction lineage information for a document.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentLineage {
    /// LLM model used for entity extraction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,

    /// Embedding model used for vector embeddings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,

    /// Embedding dimensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_dimensions: Option<usize>,

    /// List of keywords extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    /// Entity types extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_types: Option<Vec<String>>,

    /// Relationship types extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship_types: Option<Vec<String>>,

    /// Chunking strategy used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_strategy: Option<String>,

    /// Average chunk size in characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_chunk_size: Option<usize>,

    /// Processing duration in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_duration_ms: Option<u64>,

    /// Input tokens consumed during LLM processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<usize>,

    /// Output tokens generated during LLM processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<usize>,

    /// Total tokens (input + output).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<usize>,

    /// Estimated cost in USD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
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
        (status = 404, description = "Document not found"),
        (status = 403, description = "Access denied - document belongs to different tenant")
    )
)]
pub async fn get_document(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    axum::extract::Path(document_id): axum::extract::Path<String>,
) -> ApiResult<Json<DocumentDetailResponse>> {
    debug!(
        document_id = %document_id,
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Getting document by ID with tenant context"
    );

    // Fetch document metadata
    let metadata_key = format!("{}-metadata", document_id);
    debug!(metadata_key = %metadata_key, "Looking up metadata key");
    let metadata_values = state
        .kv_storage
        .get_by_ids(std::slice::from_ref(&metadata_key))
        .await?;
    debug!(
        metadata_count = metadata_values.len(),
        "Metadata values retrieved"
    );

    let metadata = metadata_values.into_iter().next();
    debug!(has_metadata = metadata.is_some(), "Metadata value present");

    // Check if document exists by metadata or chunks
    let keys = state.kv_storage.keys().await?;
    debug!(total_keys = keys.len(), "Total keys in storage");
    let matching_keys: Vec<_> = keys
        .iter()
        .filter(|k| k.contains(&document_id))
        .cloned()
        .collect();
    debug!(matching_keys = ?matching_keys, "Keys matching document ID");
    let chunk_count = keys
        .iter()
        .filter(|k| k.starts_with(&format!("{}-chunk-", document_id)))
        .count();

    // Document must have either metadata or chunks
    if metadata.is_none() && chunk_count == 0 {
        return Err(ApiError::NotFound(format!(
            "Document {} not found",
            document_id
        )));
    }

    // Parse metadata if available
    let meta_obj = metadata.as_ref().and_then(|v| v.as_object());

    // Check tenant context (multi-tenancy)
    if let Some(obj) = meta_obj {
        let doc_tenant_id = obj.get("tenant_id").and_then(|v| v.as_str());
        let doc_workspace_id = obj.get("workspace_id").and_then(|v| v.as_str());

        // Verify tenant access
        if let Some(ref filter_tid) = tenant_ctx.tenant_id {
            if let Some(doc_tid) = doc_tenant_id {
                if doc_tid != filter_tid {
                    return Err(ApiError::Forbidden);
                }
            }
        }

        // Verify workspace access
        if let Some(ref filter_ws) = tenant_ctx.workspace_id {
            if let Some(doc_ws) = doc_workspace_id {
                if doc_ws != filter_ws {
                    return Err(ApiError::Forbidden);
                }
            }
        }
    }

    // Fetch document content
    let content_key = format!("{}-content", document_id);
    let content_values = state.kv_storage.get_by_ids(&[content_key]).await?;
    let content = content_values.into_iter().next().and_then(|v| {
        v.get("content")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string())
    });

    // Build response from metadata
    let (
        title,
        file_name,
        content_summary,
        content_length,
        content_hash,
        entity_count,
        relationship_count,
        status,
        error_message,
        source_type,
        mime_type,
        file_size,
        track_id,
        tenant_id,
        workspace_id,
        created_at,
        updated_at,
        processed_at,
        lineage,
        custom_metadata,
    ) = if let Some(obj) = meta_obj {
        // Build lineage information from stored metadata
        let lineage = {
            let llm_model = obj
                .get("llm_model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let embedding_model = obj
                .get("embedding_model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let embedding_dimensions = obj
                .get("embedding_dimensions")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            let keywords = obj.get("keywords").and_then(|v| v.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            });
            let entity_types = obj
                .get("entity_types")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                });
            let relationship_types = obj
                .get("relationship_types")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                });
            let chunking_strategy = obj
                .get("chunking_strategy")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let avg_chunk_size = obj
                .get("avg_chunk_size")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            let processing_duration_ms = obj.get("processing_duration_ms").and_then(|v| v.as_u64());

            // Token usage and cost fields
            let input_tokens = obj
                .get("input_tokens")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            let output_tokens = obj
                .get("output_tokens")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            let total_tokens = obj
                .get("total_tokens")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            let cost_usd = obj.get("cost_usd").and_then(|v| v.as_f64());

            // Only include lineage if we have at least one field
            if llm_model.is_some()
                || embedding_model.is_some()
                || keywords.is_some()
                || entity_types.is_some()
                || relationship_types.is_some()
                || chunking_strategy.is_some()
                || processing_duration_ms.is_some()
                || input_tokens.is_some()
                || cost_usd.is_some()
            {
                Some(DocumentLineage {
                    llm_model,
                    embedding_model,
                    embedding_dimensions,
                    keywords,
                    entity_types,
                    relationship_types,
                    chunking_strategy,
                    avg_chunk_size,
                    processing_duration_ms,
                    input_tokens,
                    output_tokens,
                    total_tokens,
                    cost_usd,
                })
            } else {
                None
            }
        };

        (
            obj.get("title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("file_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    obj.get("title")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                }),
            obj.get("content_summary")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("content_length")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize),
            obj.get("content_hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("entity_count")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize),
            obj.get("relationship_count")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize),
            obj.get("status")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "completed".to_string()),
            obj.get("error_message")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("source_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("mime_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("file_size")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize),
            obj.get("track_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("tenant_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("workspace_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("created_at")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("updated_at")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("processed_at")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            lineage,
            obj.get("custom_metadata").cloned(),
        )
    } else {
        // Fallback for documents without metadata (legacy)
        (
            None,                    // title
            None,                    // file_name
            None,                    // content_summary
            None,                    // content_length
            None,                    // content_hash
            None,                    // entity_count
            None,                    // relationship_count
            "completed".to_string(), // status
            None,                    // error_message
            None,                    // source_type
            None,                    // mime_type
            None,                    // file_size
            None,                    // track_id
            None,                    // tenant_id
            None,                    // workspace_id
            None,                    // created_at
            None,                    // updated_at
            None,                    // processed_at
            None,                    // lineage
            None,                    // custom_metadata
        )
    };

    Ok(Json(DocumentDetailResponse {
        id: document_id,
        title,
        file_name,
        content,
        content_summary,
        content_length,
        content_hash,
        chunk_count,
        entity_count,
        relationship_count,
        status,
        error_message,
        source_type,
        mime_type,
        file_size,
        track_id,
        tenant_id,
        workspace_id,
        created_at,
        updated_at,
        processed_at,
        lineage,
        metadata: custom_metadata,
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
    let chunk_prefix = format!("{}-chunk-", document_id);
    let chunk_ids: Vec<String> = keys
        .iter()
        .filter(|k| k.starts_with(&chunk_prefix))
        .cloned()
        .collect();

    // Also check for metadata and content keys
    let metadata_key = format!("{}-metadata", document_id);
    let content_key = format!("{}-content", document_id);
    let has_metadata = keys.contains(&metadata_key);
    let has_content = keys.contains(&content_key);

    // Document must have either chunks, metadata, or content
    if chunk_ids.is_empty() && !has_metadata && !has_content {
        return Err(ApiError::NotFound(format!(
            "Document {} not found",
            document_id
        )));
    }

    let chunks_deleted = chunk_ids.len();
    let mut entities_removed = 0usize;
    let mut entities_updated = 0usize;
    let mut relationships_removed = 0usize;
    let mut relationships_updated = 0usize;

    // Cascade delete: Process graph entities - remove document sources
    let all_nodes = state.graph_storage.get_all_nodes().await?;
    for node in all_nodes {
        if let Some(source_id) = node.properties.get("source_id").and_then(|v| v.as_str()) {
            let sources: Vec<&str> = source_id.split('|').collect();
            let remaining_sources: Vec<&str> = sources
                .into_iter()
                .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(&document_id))
                .collect();

            if remaining_sources.is_empty() {
                // No sources left - delete the entity entirely
                // First delete all connected edges
                let edges = state.graph_storage.get_node_edges(&node.id).await?;
                for edge in edges {
                    state
                        .graph_storage
                        .delete_edge(&edge.source, &edge.target)
                        .await?;
                    relationships_removed += 1;
                }
                // Then delete the node
                state.graph_storage.delete_node(&node.id).await?;
                // Also delete from vector storage
                let _ = state.vector_storage.delete_entity(&node.id).await;
                entities_removed += 1;
            } else if remaining_sources.len() < source_id.split('|').count() {
                // Some sources were removed - update the entity
                let mut updated_props = node.properties.clone();
                updated_props.insert(
                    "source_id".to_string(),
                    serde_json::json!(remaining_sources.join("|")),
                );
                state
                    .graph_storage
                    .upsert_node(&node.id, updated_props)
                    .await?;
                entities_updated += 1;
            }
        }
    }

    // Process graph edges - remove document sources
    let all_edges = state.graph_storage.get_all_edges().await?;
    for edge in all_edges {
        if let Some(source_id) = edge.properties.get("source_id").and_then(|v| v.as_str()) {
            let sources: Vec<&str> = source_id.split('|').collect();
            let remaining_sources: Vec<&str> = sources
                .into_iter()
                .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(&document_id))
                .collect();

            if remaining_sources.is_empty() {
                // No sources left - delete the relationship
                state
                    .graph_storage
                    .delete_edge(&edge.source, &edge.target)
                    .await?;
                relationships_removed += 1;
            } else if remaining_sources.len() < source_id.split('|').count() {
                // Some sources were removed - update the relationship
                let mut updated_props = edge.properties.clone();
                updated_props.insert(
                    "source_id".to_string(),
                    serde_json::json!(remaining_sources.join("|")),
                );
                state
                    .graph_storage
                    .upsert_edge(&edge.source, &edge.target, updated_props)
                    .await?;
                relationships_updated += 1;
            }
        }
    }

    // Collect all keys to delete
    let mut keys_to_delete = chunk_ids;
    if has_metadata {
        keys_to_delete.push(metadata_key);
    }
    if has_content {
        keys_to_delete.push(content_key);
    }

    // Delete all document data from KV storage
    state.kv_storage.delete(&keys_to_delete).await?;

    tracing::info!(
        document_id = %document_id,
        chunks = chunks_deleted,
        entities_removed = entities_removed,
        entities_updated = entities_updated,
        relationships_removed = relationships_removed,
        relationships_updated = relationships_updated,
        "Document suppression complete"
    );

    Ok(Json(DeleteDocumentResponse {
        document_id,
        deleted: true,
        chunks_deleted,
        entities_affected: entities_removed + entities_updated,
        relationships_affected: relationships_removed + relationships_updated,
    }))
}

/// Document deletion impact analysis response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DeletionImpactResponse {
    /// Document ID.
    pub document_id: String,

    /// Number of chunks that would be deleted.
    pub chunks_to_delete: usize,

    /// Number of entities that would be completely removed (no other sources).
    pub entities_to_remove: usize,

    /// Number of entities that would be updated (some sources remaining).
    pub entities_to_update: usize,

    /// Number of relationships that would be completely removed.
    pub relationships_to_remove: usize,

    /// Number of relationships that would be updated.
    pub relationships_to_update: usize,

    /// Preview is read-only; document NOT deleted.
    pub preview_only: bool,
}

/// Analyze the impact of deleting a document before actually deleting it.
///
/// This endpoint allows users to preview what would be affected by a document deletion
/// without actually performing the deletion. This is useful for understanding the
/// cascade effects before committing to a destructive operation.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/deletion-impact",
    tag = "Documents",
    params(
        ("document_id" = String, Path, description = "Document ID to analyze")
    ),
    responses(
        (status = 200, description = "Deletion impact analysis", body = DeletionImpactResponse),
        (status = 404, description = "Document not found")
    )
)]
pub async fn analyze_deletion_impact(
    State(state): State<AppState>,
    axum::extract::Path(document_id): axum::extract::Path<String>,
) -> ApiResult<Json<DeletionImpactResponse>> {
    let keys = state.kv_storage.keys().await?;

    // Find chunks belonging to this document
    let chunk_prefix = format!("{}-chunk-", document_id);
    let chunk_ids: Vec<String> = keys
        .iter()
        .filter(|k| k.starts_with(&chunk_prefix))
        .cloned()
        .collect();

    // Also check for metadata and content keys
    let metadata_key = format!("{}-metadata", document_id);
    let content_key = format!("{}-content", document_id);
    let has_metadata = keys.contains(&metadata_key);
    let has_content = keys.contains(&content_key);

    // Document must have either chunks, metadata, or content
    if chunk_ids.is_empty() && !has_metadata && !has_content {
        return Err(ApiError::NotFound(format!(
            "Document {} not found",
            document_id
        )));
    }

    let chunks_to_delete = chunk_ids.len();
    let mut entities_to_remove = 0usize;
    let mut entities_to_update = 0usize;
    let mut relationships_to_remove = 0usize;
    let mut relationships_to_update = 0usize;

    // Analyze entities (read-only)
    let all_nodes = state.graph_storage.get_all_nodes().await?;
    for node in all_nodes {
        if let Some(source_id) = node.properties.get("source_id").and_then(|v| v.as_str()) {
            let sources: Vec<&str> = source_id.split('|').collect();
            let remaining = sources
                .iter()
                .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(&document_id))
                .count();

            if remaining == 0 {
                entities_to_remove += 1;
            } else if remaining < sources.len() {
                entities_to_update += 1;
            }
        }
    }

    // Analyze edges (read-only)
    let all_edges = state.graph_storage.get_all_edges().await?;
    for edge in all_edges {
        if let Some(source_id) = edge.properties.get("source_id").and_then(|v| v.as_str()) {
            let sources: Vec<&str> = source_id.split('|').collect();
            let remaining = sources
                .iter()
                .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(&document_id))
                .count();

            if remaining == 0 {
                relationships_to_remove += 1;
            } else if remaining < sources.len() {
                relationships_to_update += 1;
            }
        }
    }

    Ok(Json(DeletionImpactResponse {
        document_id,
        chunks_to_delete,
        entities_to_remove,
        entities_to_update,
        relationships_to_remove,
        relationships_to_update,
        preview_only: true,
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
    tenant_ctx: TenantContext,
    mut multipart: Multipart,
) -> ApiResult<Json<FileUploadResponse>> {
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
    debug!(content_hash = %content_hash, "Computed content hash");

    // Check for duplicate
    let hash_key = format!("doc:hash:{}", content_hash);
    debug!(hash_key = %hash_key, "Checking for duplicate hash");
    if let Some(existing_doc_id) = state.kv_storage.get_by_id(&hash_key).await? {
        debug!(existing_doc_id = ?existing_doc_id, "Found existing document for hash");
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

    // Generate content summary (first 200 chars)
    let content_summary = if text_content.len() > 200 {
        format!("{}...", &text_content.chars().take(200).collect::<String>())
    } else {
        text_content.clone()
    };

    // Determine MIME type from extension
    let mime_type = match extension.as_str() {
        "txt" => "text/plain",
        "md" => "text/markdown",
        "json" => "application/json",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "xml" => "application/xml",
        "yaml" | "yml" => "application/x-yaml",
        _ => "application/octet-stream",
    };

    // Generate track ID
    let track_id = format!(
        "upload_{}_{}",
        Utc::now().format("%Y%m%d%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );

    // Extract tenant context for storage
    let workspace_id_for_storage = tenant_ctx
        .workspace_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let tenant_id_for_storage = tenant_ctx.tenant_id.clone();

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

            match state
                .vector_storage
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
                if let Err(e) = state
                    .vector_storage
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

    // Store chunk embeddings in vector storage for semantic search
    for chunk in &result.chunks {
        if let Some(embedding) = &chunk.embedding {
            let metadata = serde_json::json!({
                "type": "chunk",
                "document_id": document_id,
                "index": chunk.index,
                "content": chunk.content,
                "source_file": filename,
            });

            let _ = state
                .vector_storage
                .upsert(&[(chunk.id.clone(), embedding.clone(), metadata)])
                .await;
        }
    }

    Ok((document_id, false))
}

// ============================================================================
// Track Status (Phase 2)
// ============================================================================

/// Track status response for batch grouping.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TrackStatusResponse {
    /// Track ID for this batch.
    pub track_id: String,

    /// When the first document was uploaded.
    pub created_at: Option<String>,

    /// Documents in this batch.
    pub documents: Vec<DocumentSummary>,

    /// Total number of documents.
    pub total_count: usize,

    /// Status summary for the batch.
    pub status_summary: StatusCounts,

    /// Whether processing is complete (all docs completed or failed).
    pub is_complete: bool,

    /// Latest processing message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_message: Option<String>,
}

/// Get track status by track ID.
///
/// Returns all documents uploaded with a specific track_id, along with status summary.
#[utoipa::path(
    get,
    path = "/api/v1/documents/track/{track_id}",
    tag = "Documents",
    params(
        ("track_id" = String, Path, description = "Track ID for the batch")
    ),
    responses(
        (status = 200, description = "Track status retrieved", body = TrackStatusResponse),
        (status = 404, description = "Track not found")
    )
)]
pub async fn get_track_status(
    State(state): State<AppState>,
    axum::extract::Path(track_id): axum::extract::Path<String>,
) -> ApiResult<Json<TrackStatusResponse>> {
    let keys = state.kv_storage.keys().await?;

    // Find all metadata keys
    let metadata_keys: Vec<String> = keys
        .iter()
        .filter(|k| k.ends_with("-metadata"))
        .cloned()
        .collect();

    // Fetch all metadata
    let metadata_values = state.kv_storage.get_by_ids(&metadata_keys).await?;

    // Group chunks by document
    let mut doc_chunks: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for key in &keys {
        if let Some(doc_id) = key.split("-chunk-").next() {
            if !doc_id.ends_with("-metadata") && !doc_id.ends_with("-content") {
                *doc_chunks.entry(doc_id.to_string()).or_default() += 1;
            }
        }
    }

    // Filter documents by track_id
    let mut track_docs: Vec<DocumentSummary> = Vec::new();
    let mut created_times: Vec<String> = Vec::new();

    for value in metadata_values {
        if let Some(obj) = value.as_object() {
            let doc_track_id = obj.get("track_id").and_then(|v| v.as_str()).unwrap_or("");

            if doc_track_id == track_id {
                let id = obj
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let chunk_count = doc_chunks.get(&id).copied().unwrap_or(0);

                if let Some(created_at) = obj.get("created_at").and_then(|v| v.as_str()) {
                    created_times.push(created_at.to_string());
                }

                track_docs.push(DocumentSummary {
                    id,
                    title: obj.get("title").and_then(|v| v.as_str()).map(String::from),
                    file_name: obj
                        .get("file_name")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    content_summary: obj
                        .get("content_summary")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    content_length: obj
                        .get("content_length")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                    chunk_count,
                    entity_count: obj
                        .get("entity_count")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                    status: obj.get("status").and_then(|v| v.as_str()).map(String::from),
                    error_message: obj
                        .get("error_message")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    track_id: Some(track_id.clone()),
                    created_at: obj
                        .get("created_at")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    updated_at: obj
                        .get("updated_at")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    cost_usd: obj.get("cost_usd").and_then(|v| v.as_f64()),
                    input_tokens: obj
                        .get("input_tokens")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                    output_tokens: obj
                        .get("output_tokens")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                    total_tokens: obj
                        .get("total_tokens")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                    llm_model: obj
                        .get("llm_model")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    embedding_model: obj
                        .get("embedding_model")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                });
            }
        }
    }

    if track_docs.is_empty() {
        return Err(ApiError::NotFound(format!("Track not found: {}", track_id)));
    }

    // Calculate status summary
    let status_summary = StatusCounts {
        pending: track_docs
            .iter()
            .filter(|d| d.status.as_deref() == Some("pending"))
            .count(),
        processing: track_docs
            .iter()
            .filter(|d| d.status.as_deref() == Some("processing"))
            .count(),
        completed: track_docs
            .iter()
            .filter(|d| {
                d.status.is_none()
                    || d.status.as_deref() == Some("completed")
                    || d.status.as_deref() == Some("indexed")
            })
            .count(),
        failed: track_docs
            .iter()
            .filter(|d| d.status.as_deref() == Some("failed"))
            .count(),
    };

    // Find earliest created_at
    created_times.sort();
    let created_at = created_times.first().cloned();

    // Check if complete (no pending or processing)
    let is_complete = status_summary.pending == 0 && status_summary.processing == 0;

    // Build latest message
    let latest_message = if !is_complete {
        Some(format!(
            "Processing {}/{} documents...",
            status_summary.completed + status_summary.failed,
            track_docs.len()
        ))
    } else if status_summary.failed > 0 {
        Some(format!("Completed with {} errors", status_summary.failed))
    } else {
        Some("All documents processed successfully".to_string())
    };

    Ok(Json(TrackStatusResponse {
        track_id,
        created_at,
        documents: track_docs.clone(),
        total_count: track_docs.len(),
        status_summary,
        is_complete,
        latest_message,
    }))
}

// ============================================
// GAP-014: Document Scan API
// ============================================

/// Request to scan a directory for documents.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ScanDirectoryRequest {
    /// Path to the directory to scan.
    pub path: String,

    /// File extensions to include (e.g., ["txt", "md", "pdf"]).
    /// If empty, all files are included.
    #[serde(default)]
    pub extensions: Vec<String>,

    /// Whether to scan subdirectories recursively.
    #[serde(default = "default_recursive")]
    pub recursive: bool,

    /// Maximum number of files to scan.
    #[serde(default = "default_max_files")]
    pub max_files: usize,

    /// Whether to process documents asynchronously.
    #[serde(default = "default_true")]
    pub async_processing: bool,

    /// Optional track ID for batch grouping.
    #[serde(default)]
    pub track_id: Option<String>,
}

fn default_recursive() -> bool {
    true
}

fn default_max_files() -> usize {
    1000
}

fn default_true() -> bool {
    true
}

/// Response from directory scan.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScanDirectoryResponse {
    /// Track ID for the scan batch.
    pub track_id: String,

    /// Number of files found.
    pub files_found: usize,

    /// Number of files queued for processing.
    pub files_queued: usize,

    /// Number of files skipped (already processed or filtered).
    pub files_skipped: usize,

    /// List of queued file paths.
    pub queued_files: Vec<String>,

    /// List of skipped file paths with reasons.
    pub skipped_files: Vec<SkippedFile>,
}

/// Information about a skipped file.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SkippedFile {
    /// Path to the file.
    pub path: String,

    /// Reason for skipping.
    pub reason: String,
}

/// Scan a directory and queue documents for processing.
#[utoipa::path(
    post,
    path = "/api/v1/documents/scan",
    tag = "Documents",
    request_body = ScanDirectoryRequest,
    responses(
        (status = 200, description = "Directory scanned successfully", body = ScanDirectoryResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Directory not found")
    )
)]
pub async fn scan_directory(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<ScanDirectoryRequest>,
) -> ApiResult<Json<ScanDirectoryResponse>> {
    debug!(
        "scan_directory called with tenant context: tenant_id={:?}, workspace_id={:?}",
        tenant_ctx.tenant_id, tenant_ctx.workspace_id
    );

    use std::path::Path;

    let base_path = Path::new(&request.path);

    // Validate path exists and is a directory
    if !base_path.exists() {
        return Err(ApiError::NotFound(format!(
            "Directory not found: {}",
            request.path
        )));
    }

    if !base_path.is_dir() {
        return Err(ApiError::BadRequest(format!(
            "Path is not a directory: {}",
            request.path
        )));
    }

    // Generate track ID
    let track_id = request.track_id.unwrap_or_else(|| {
        format!(
            "scan_{}_{}",
            Utc::now().format("%Y%m%d_%H%M%S"),
            &Uuid::new_v4().to_string()[..8]
        )
    });

    let mut queued_files = Vec::new();
    let mut skipped_files = Vec::new();
    let mut files_found = 0;

    // Collect files to process
    let entries = collect_files(base_path, request.recursive, request.max_files)?;

    for entry in entries {
        files_found += 1;

        let file_path = entry.path();
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Check extension filter
        if !request.extensions.is_empty() {
            if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                if !request
                    .extensions
                    .iter()
                    .any(|e| e.eq_ignore_ascii_case(ext))
                {
                    skipped_files.push(SkippedFile {
                        path: file_path.display().to_string(),
                        reason: format!("Extension .{} not in filter list", ext),
                    });
                    continue;
                }
            } else {
                skipped_files.push(SkippedFile {
                    path: file_path.display().to_string(),
                    reason: "No extension".to_string(),
                });
                continue;
            }
        }

        // Try to read file content
        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                skipped_files.push(SkippedFile {
                    path: file_path.display().to_string(),
                    reason: format!("Failed to read: {}", e),
                });
                continue;
            }
        };

        if content.trim().is_empty() {
            skipped_files.push(SkippedFile {
                path: file_path.display().to_string(),
                reason: "Empty file".to_string(),
            });
            continue;
        }

        // Check size limit
        if content.len() > state.config.max_document_size {
            skipped_files.push(SkippedFile {
                path: file_path.display().to_string(),
                reason: format!(
                    "Exceeds max size ({} > {})",
                    content.len(),
                    state.config.max_document_size
                ),
            });
            continue;
        }

        // Generate document ID
        let document_id = Uuid::new_v4().to_string();

        // Generate content summary
        let content_summary = if content.len() > 200 {
            format!("{}...", &content.chars().take(200).collect::<String>())
        } else {
            content.clone()
        };

        // Store document metadata
        let doc_metadata_key = format!("{}-metadata", document_id);
        let doc_metadata = serde_json::json!({
            "id": document_id,
            "title": file_name,
            "file_path": file_path.display().to_string(),
            "content_summary": content_summary,
            "content_length": content.len(),
            "track_id": track_id,
            "created_at": Utc::now().to_rfc3339(),
            "status": "pending",
        });
        state
            .kv_storage
            .upsert(&[(doc_metadata_key, doc_metadata)])
            .await?;

        // Store document content
        let doc_content_key = format!("{}-content", document_id);
        let doc_content = serde_json::json!({
            "content": content,
        });
        state
            .kv_storage
            .upsert(&[(doc_content_key, doc_content)])
            .await?;

        if request.async_processing {
            // Create task for background processing
            use edgequake_tasks::{Task, TaskType, TextInsertData};

            // Use tenant context for workspace_id, fallback to "default"
            let workspace_id = tenant_ctx
                .workspace_id
                .clone()
                .unwrap_or_else(|| "default".to_string());
            let tenant_id = tenant_ctx.tenant_id.clone();

            let task_data = TextInsertData {
                text: content,
                file_source: file_path.display().to_string(),
                workspace_id: workspace_id.clone(),
                metadata: Some(serde_json::json!({
                    "document_id": document_id,
                    "title": file_name,
                    "track_id": track_id,
                    "tenant_id": tenant_id,
                    "workspace_id": workspace_id,
                })),
            };

            let task = Task::new(TaskType::Insert, serde_json::to_value(task_data).unwrap());

            state
                .task_storage
                .create_task(&task)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to create task: {}", e)))?;

            state
                .task_queue
                .send(task)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to queue task: {}", e)))?;
        }

        queued_files.push(file_path.display().to_string());
    }

    Ok(Json(ScanDirectoryResponse {
        track_id,
        files_found,
        files_queued: queued_files.len(),
        files_skipped: skipped_files.len(),
        queued_files,
        skipped_files,
    }))
}

/// Collect files from a directory.
fn collect_files(
    path: &std::path::Path,
    recursive: bool,
    max_files: usize,
) -> Result<Vec<std::fs::DirEntry>, ApiError> {
    let mut files = Vec::new();

    fn visit_dir(
        dir: &std::path::Path,
        recursive: bool,
        max_files: usize,
        files: &mut Vec<std::fs::DirEntry>,
    ) -> Result<(), ApiError> {
        if files.len() >= max_files {
            return Ok(());
        }

        let entries = std::fs::read_dir(dir).map_err(|e| {
            ApiError::Internal(format!("Failed to read directory {}: {}", dir.display(), e))
        })?;

        for entry in entries {
            if files.len() >= max_files {
                break;
            }

            let entry = entry.map_err(|e| {
                ApiError::Internal(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();

            if path.is_file() {
                files.push(entry);
            } else if path.is_dir() && recursive {
                visit_dir(&path, recursive, max_files, files)?;
            }
        }

        Ok(())
    }

    visit_dir(path, recursive, max_files, &mut files)?;
    Ok(files)
}

// ============================================
// GAP-039: Reprocess Failed Documents
// ============================================

/// Request to reprocess failed documents.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReprocessFailedRequest {
    /// Optional track ID to reprocess. If not provided, all failed documents are reprocessed.
    #[serde(default)]
    pub track_id: Option<String>,

    /// Maximum number of documents to reprocess.
    #[serde(default = "default_max_reprocess")]
    pub max_documents: usize,
}

fn default_max_reprocess() -> usize {
    100
}

/// Response from reprocess operation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReprocessFailedResponse {
    /// Track ID for the reprocess batch.
    pub track_id: String,

    /// Number of failed documents found.
    pub failed_found: usize,

    /// Number of documents queued for reprocessing.
    pub requeued: usize,

    /// List of document IDs being reprocessed.
    pub document_ids: Vec<String>,
}

/// Reprocess failed documents.
#[utoipa::path(
    post,
    path = "/api/v1/documents/reprocess",
    tag = "Documents",
    request_body = ReprocessFailedRequest,
    responses(
        (status = 200, description = "Failed documents requeued", body = ReprocessFailedResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn reprocess_failed(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<ReprocessFailedRequest>,
) -> ApiResult<Json<ReprocessFailedResponse>> {
    debug!(
        "reprocess_failed called with tenant context: tenant_id={:?}, workspace_id={:?}",
        tenant_ctx.tenant_id, tenant_ctx.workspace_id
    );

    // Generate new track ID for reprocess batch
    let new_track_id = format!(
        "reprocess_{}_{}",
        Utc::now().format("%Y%m%d_%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );

    // Get all metadata keys
    let all_keys: Vec<String> = state.kv_storage.keys().await?;

    let mut failed_docs = Vec::new();
    let mut requeued_ids = Vec::new();

    // Find failed documents
    for key in all_keys.iter().filter(|k| k.ends_with("-metadata")) {
        if failed_docs.len() >= request.max_documents {
            break;
        }

        if let Some(value) = state.kv_storage.get_by_id(key).await? {
            if let Some(obj) = value.as_object() {
                let status = obj.get("status").and_then(|v| v.as_str());
                let doc_track_id = obj.get("track_id").and_then(|v| v.as_str());

                // Check if document is failed
                if status == Some("failed") {
                    // If track_id filter specified, check it
                    if let Some(ref filter_track) = request.track_id {
                        if doc_track_id != Some(filter_track.as_str()) {
                            continue;
                        }
                    }

                    if let Some(doc_id) = obj.get("id").and_then(|v| v.as_str()) {
                        failed_docs.push((doc_id.to_string(), key.replace("-metadata", "")));
                    }
                }
            }
        }
    }

    // Requeue failed documents
    for (doc_id, _doc_key) in &failed_docs {
        // Get document content
        let content_key = format!("{}-content", doc_id);
        if let Some(content_value) = state.kv_storage.get_by_id(&content_key).await? {
            if let Some(content) = content_value.get("content").and_then(|v| v.as_str()) {
                // Update status to pending
                let metadata_key = format!("{}-metadata", doc_id);
                if let Some(mut metadata) = state.kv_storage.get_by_id(&metadata_key).await? {
                    if let Some(obj) = metadata.as_object_mut() {
                        obj.insert("status".to_string(), serde_json::json!("pending"));
                        obj.insert("track_id".to_string(), serde_json::json!(new_track_id));
                        obj.insert(
                            "retry_at".to_string(),
                            serde_json::json!(Utc::now().to_rfc3339()),
                        );

                        state.kv_storage.upsert(&[(metadata_key, metadata)]).await?;
                    }
                }

                // Create new task
                use edgequake_tasks::{Task, TaskType, TextInsertData};

                // Use tenant context for workspace_id, fallback to "default"
                let workspace_id = tenant_ctx
                    .workspace_id
                    .clone()
                    .unwrap_or_else(|| "default".to_string());
                let tenant_id = tenant_ctx.tenant_id.clone();

                let title = doc_id.clone();
                let task_data = TextInsertData {
                    text: content.to_string(),
                    file_source: title.clone(),
                    workspace_id: workspace_id.clone(),
                    metadata: Some(serde_json::json!({
                        "document_id": doc_id,
                        "title": title,
                        "track_id": new_track_id,
                        "is_retry": true,
                        "tenant_id": tenant_id,
                        "workspace_id": workspace_id,
                    })),
                };

                let task = Task::new(TaskType::Insert, serde_json::to_value(task_data).unwrap());

                state
                    .task_storage
                    .create_task(&task)
                    .await
                    .map_err(|e| ApiError::Internal(format!("Failed to create task: {}", e)))?;

                state
                    .task_queue
                    .send(task)
                    .await
                    .map_err(|e| ApiError::Internal(format!("Failed to queue task: {}", e)))?;

                requeued_ids.push(doc_id.clone());
            }
        }
    }

    Ok(Json(ReprocessFailedResponse {
        track_id: new_track_id,
        failed_found: failed_docs.len(),
        requeued: requeued_ids.len(),
        document_ids: requeued_ids,
    }))
}

// ============================================
// Recovery for Stuck Processing Documents
// ============================================

/// Request to recover stuck processing documents.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RecoverStuckRequest {
    /// Minimum age in minutes for a document to be considered "stuck".
    /// Default: 10 minutes.
    #[serde(default = "default_stuck_threshold_minutes")]
    pub stuck_threshold_minutes: u64,

    /// Maximum number of documents to recover.
    #[serde(default = "default_max_reprocess")]
    pub max_documents: usize,

    /// Optional list of specific document IDs to recover.
    #[serde(default)]
    pub document_ids: Option<Vec<String>>,
}

fn default_stuck_threshold_minutes() -> u64 {
    10
}

/// Response from recover stuck operation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RecoverStuckResponse {
    /// Track ID for the recovery batch.
    pub track_id: String,

    /// Number of stuck documents found.
    pub stuck_found: usize,

    /// Number of documents queued for reprocessing.
    pub requeued: usize,

    /// List of document IDs being recovered.
    pub document_ids: Vec<String>,

    /// List of document titles for reference.
    pub document_titles: Vec<String>,
}

/// Recover documents stuck in "processing" status.
///
/// This endpoint finds documents that have been in "processing" status for longer
/// than the specified threshold and requeues them for processing. This is useful
/// for recovering from server restarts or crashes that left tasks in an incomplete state.
#[utoipa::path(
    post,
    path = "/api/v1/documents/recover-stuck",
    tag = "Documents",
    request_body = RecoverStuckRequest,
    responses(
        (status = 200, description = "Stuck documents recovered", body = RecoverStuckResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn recover_stuck(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<RecoverStuckRequest>,
) -> ApiResult<Json<RecoverStuckResponse>> {
    use chrono::Duration;

    debug!(
        "recover_stuck called with tenant context: tenant_id={:?}, workspace_id={:?}, threshold={}min",
        tenant_ctx.tenant_id, tenant_ctx.workspace_id, request.stuck_threshold_minutes
    );

    // Generate new track ID for recovery batch
    let new_track_id = format!(
        "recover_{}_{}",
        Utc::now().format("%Y%m%d_%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );

    let threshold = Duration::minutes(request.stuck_threshold_minutes as i64);
    let cutoff_time = Utc::now() - threshold;

    // Get all metadata keys
    let all_keys: Vec<String> = state.kv_storage.keys().await?;

    let mut stuck_docs = Vec::new();
    let mut requeued_ids = Vec::new();
    let mut requeued_titles = Vec::new();

    // Find stuck processing documents
    for key in all_keys.iter().filter(|k| k.ends_with("-metadata")) {
        if stuck_docs.len() >= request.max_documents {
            break;
        }

        if let Some(value) = state.kv_storage.get_by_id(key).await? {
            if let Some(obj) = value.as_object() {
                let status = obj.get("status").and_then(|v| v.as_str());
                let doc_id = obj.get("id").and_then(|v| v.as_str());
                let title = obj.get("title").and_then(|v| v.as_str());
                let updated_at = obj.get("updated_at").and_then(|v| v.as_str());

                // Check if document is stuck in processing
                if status == Some("processing") {
                    // If specific document IDs provided, check if this one is in the list
                    if let Some(ref filter_ids) = request.document_ids {
                        if let Some(id) = doc_id {
                            if !filter_ids.contains(&id.to_string()) {
                                continue;
                            }
                        }
                    }

                    // Check if document is older than threshold
                    let is_stuck = if let Some(updated) = updated_at {
                        if let Ok(updated_time) = chrono::DateTime::parse_from_rfc3339(updated) {
                            updated_time.with_timezone(&chrono::Utc) < cutoff_time
                        } else {
                            // If we can't parse the time, assume it's stuck
                            true
                        }
                    } else {
                        // No updated_at, assume it's stuck
                        true
                    };

                    if is_stuck {
                        if let Some(id) = doc_id {
                            stuck_docs.push((id.to_string(), title.unwrap_or(id).to_string()));
                        }
                    }
                }
            }
        }
    }

    // Requeue stuck documents
    for (doc_id, doc_title) in &stuck_docs {
        // Get document content
        let content_key = format!("{}-content", doc_id);
        if let Some(content_value) = state.kv_storage.get_by_id(&content_key).await? {
            if let Some(content) = content_value.get("content").and_then(|v| v.as_str()) {
                // Update status back to pending
                let metadata_key = format!("{}-metadata", doc_id);
                if let Some(mut metadata) = state.kv_storage.get_by_id(&metadata_key).await? {
                    if let Some(obj) = metadata.as_object_mut() {
                        obj.insert("status".to_string(), serde_json::json!("pending"));
                        obj.insert("track_id".to_string(), serde_json::json!(new_track_id));
                        obj.insert(
                            "recovered_at".to_string(),
                            serde_json::json!(Utc::now().to_rfc3339()),
                        );
                        obj.insert(
                            "recovery_reason".to_string(),
                            serde_json::json!("stuck_in_processing"),
                        );

                        state.kv_storage.upsert(&[(metadata_key, metadata)]).await?;
                    }
                }

                // Create new task
                use edgequake_tasks::{Task, TaskType, TextInsertData};

                // Use tenant context for workspace_id, fallback to "default"
                let workspace_id = tenant_ctx
                    .workspace_id
                    .clone()
                    .unwrap_or_else(|| "default".to_string());
                let tenant_id = tenant_ctx.tenant_id.clone();

                let task_data = TextInsertData {
                    text: content.to_string(),
                    file_source: doc_title.clone(),
                    workspace_id: workspace_id.clone(),
                    metadata: Some(serde_json::json!({
                        "document_id": doc_id,
                        "title": doc_title,
                        "track_id": new_track_id,
                        "is_recovery": true,
                        "tenant_id": tenant_id,
                        "workspace_id": workspace_id,
                    })),
                };

                let task = Task::new(TaskType::Insert, serde_json::to_value(task_data).unwrap());

                state
                    .task_storage
                    .create_task(&task)
                    .await
                    .map_err(|e| ApiError::Internal(format!("Failed to create task: {}", e)))?;

                state
                    .task_queue
                    .send(task)
                    .await
                    .map_err(|e| ApiError::Internal(format!("Failed to queue task: {}", e)))?;

                requeued_ids.push(doc_id.clone());
                requeued_titles.push(doc_title.clone());

                tracing::info!("Recovered stuck document: {} ({})", doc_id, doc_title);
            }
        }
    }

    Ok(Json(RecoverStuckResponse {
        track_id: new_track_id,
        stuck_found: stuck_docs.len(),
        requeued: requeued_ids.len(),
        document_ids: requeued_ids,
        document_titles: requeued_titles,
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
            async_processing: false,
            track_id: None,
            enable_gleaning: true,
            max_gleaning: 1,
            use_llm_summarization: true,
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
            cost: Some(DocumentCostInfo {
                total_cost_usd: 0.0045,
                formatted_cost: "$0.004500".to_string(),
                input_tokens: 1000,
                output_tokens: 500,
                total_tokens: 1500,
                llm_model: Some("gpt-4o-mini".to_string()),
                embedding_model: Some("text-embedding-3-small".to_string()),
            }),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-123"));
        assert!(json.contains("processed"));
        assert!(json.contains("cost"));
        assert!(json.contains("0.0045"));
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
            cost_usd: None,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            llm_model: None,
            embedding_model: None,
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
                cost_usd: None,
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                llm_model: None,
                embedding_model: None,
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
            file_name: None,
            content: None,
            content_summary: None,
            content_length: None,
            content_hash: None,
            chunk_count: 5,
            entity_count: None,
            relationship_count: None,
            status: "processed".to_string(),
            error_message: None,
            source_type: None,
            mime_type: None,
            file_size: None,
            track_id: None,
            tenant_id: None,
            workspace_id: None,
            created_at: None,
            updated_at: None,
            processed_at: None,
            lineage: None,
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

    #[test]
    fn test_track_status_response_serialization() {
        let response = TrackStatusResponse {
            track_id: "upload_20240101_abc12345".to_string(),
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
            documents: vec![DocumentSummary {
                id: "doc-1".to_string(),
                title: Some("Test Doc".to_string()),
                file_name: None,
                content_summary: None,
                content_length: None,
                chunk_count: 5,
                entity_count: Some(3),
                status: Some("completed".to_string()),
                error_message: None,
                track_id: Some("upload_20240101_abc12345".to_string()),
                created_at: None,
                updated_at: None,
                cost_usd: None,
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                llm_model: None,
                embedding_model: None,
            }],
            total_count: 1,
            status_summary: StatusCounts {
                pending: 0,
                processing: 0,
                completed: 1,
                failed: 0,
            },
            is_complete: true,
            latest_message: Some("All documents processed successfully".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("upload_20240101_abc12345"));
        assert!(json.contains("\"is_complete\":true"));
        assert!(json.contains("\"total_count\":1"));
    }
}
