//! List all documents handler.

use axum::{
    extract::{Query, State},
    Json,
};
use tracing::{debug, warn};

use crate::error::ApiResult;
use crate::middleware::TenantContext;
use crate::state::AppState;

use crate::handlers::documents_types::*;

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
    Query(params): Query<ListDocumentsRequest>,
) -> ApiResult<Json<ListDocumentsResponse>> {
    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Listing documents with tenant context"
    );

    // SECURITY: Enforce strict tenant context requirement - NO EXCEPTIONS
    // This matches the strict filtering in entities.rs and relationships.rs (commit d11edba8)
    if tenant_ctx.tenant_id.is_none() || tenant_ctx.workspace_id.is_none() {
        warn!(
            tenant_id = ?tenant_ctx.tenant_id,
            workspace_id = ?tenant_ctx.workspace_id,
            "Tenant context missing - returning empty document list for security"
        );
        return Ok(Json(ListDocumentsResponse {
            documents: vec![],
            total: 0,
            page: 1,
            page_size: 100,
            total_pages: 0,
            has_more: false,
            status_counts: StatusCounts {
                pending: 0,
                processing: 0,
                completed: 0,
                partial_failure: 0,
                failed: 0,
                cancelled: 0,
            },
        }));
    }

    // SPEC-011: SQL LIKE filters avoid O(N) full key scan + in-memory filter.
    let metadata_keys = state.kv_storage.keys_like("%-metadata").await?;
    let chunk_keys = state.kv_storage.keys_like("%-chunk-%").await?;
    debug!(
        metadata_key_count = metadata_keys.len(),
        chunk_key_count = chunk_keys.len(),
        "KV keys loaded via LIKE filters"
    );

    // Group by document and collect metadata keys
    let mut doc_chunks: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for key in &chunk_keys {
        // Skip injection entries — they are managed by the /knowledge route (SPEC-0002)
        if key.starts_with("injection::") {
            continue;
        }
        if let Some(doc_id) = key.split("-chunk-").next() {
            // Filter out non-document keys (like -metadata, -content suffixes)
            if !doc_id.ends_with("-metadata") && !doc_id.ends_with("-content") {
                *doc_chunks.entry(doc_id.to_string()).or_default() += 1;
            }
        }
    }

    for key in &metadata_keys {
        if key.starts_with("injection::") {
            continue;
        }
        debug!(metadata_key = %key, "Found metadata key");
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
        // SPEC-002: Unified Ingestion Pipeline fields
        source_type: Option<String>,
        current_stage: Option<String>,
        stage_progress: Option<f32>,
        stage_message: Option<String>,
        pdf_id: Option<String>,
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

                // SPEC-002: Get source_type
                meta.source_type = obj
                    .get("source_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // SPEC-002: Get current_stage
                meta.current_stage = obj
                    .get("current_stage")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // SPEC-002: Get stage_progress
                meta.stage_progress = obj
                    .get("stage_progress")
                    .and_then(|v| v.as_f64())
                    .map(|n| n as f32);

                // SPEC-002: Get stage_message
                meta.stage_message = obj
                    .get("stage_message")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // SPEC-002: Get pdf_id (linked PDF document for viewing)
                meta.pdf_id = obj
                    .get("pdf_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                doc_metadata.insert(id.to_string(), meta);
            }
        }
    }

    // Filter documents by tenant context
    let filter_workspace_id = tenant_ctx.workspace_id.clone();
    let filter_tenant_id = tenant_ctx.tenant_id.clone();

    // SECURITY: STRICT tenant filtering - both tenant_id AND workspace_id must match
    // This matches the strict filtering in entities.rs and relationships.rs (commit d11edba8)
    let matches_tenant_context = |meta: &DocMetadata| -> bool {
        // Both must match exactly (None is already handled by early return above)
        meta.workspace_id.as_ref() == filter_workspace_id.as_ref()
            && meta.tenant_id.as_ref() == filter_tenant_id.as_ref()
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
                // SPEC-002: Unified Ingestion Pipeline fields
                source_type: meta.source_type,
                current_stage: meta.current_stage,
                stage_progress: meta.stage_progress,
                stage_message: meta.stage_message,
                pdf_id: meta.pdf_id,
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
            // SPEC-002: Unified Ingestion Pipeline fields
            source_type: meta.source_type,
            current_stage: meta.current_stage,
            stage_progress: meta.stage_progress,
            stage_message: meta.stage_message,
            pdf_id: meta.pdf_id,
        });
    }

    // Sort by created_at descending (newest first)
    documents.sort_by(|a, b| {
        b.created_at
            .as_deref()
            .unwrap_or("")
            .cmp(a.created_at.as_deref().unwrap_or(""))
    });

    // SPEC-005: Apply optional date range and title pattern filters
    if params.date_from.is_some() || params.date_to.is_some() || params.document_pattern.is_some() {
        let patterns: Vec<String> = params
            .document_pattern
            .as_ref()
            .map(|p| {
                p.split(',')
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        documents.retain(|doc| {
            // Date range filter (ISO 8601 string comparison)
            if let Some(ref date_from) = params.date_from {
                match doc.created_at.as_deref() {
                    Some(ca) if ca >= date_from.as_str() => {}
                    _ => return false,
                }
            }
            if let Some(ref date_to) = params.date_to {
                match doc.created_at.as_deref() {
                    Some(ca) if ca <= date_to.as_str() => {}
                    _ => return false,
                }
            }
            // Title pattern filter (case-insensitive, comma-separated OR)
            if !patterns.is_empty() {
                let title = doc.title.as_deref().unwrap_or("").to_lowercase();
                if !patterns.iter().any(|p| title.contains(p.as_str())) {
                    return false;
                }
            }
            true
        });

        debug!(
            filtered_count = documents.len(),
            "Applied SPEC-005 document listing filters"
        );
    }

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
        // FIX-5: Track partial_failure status
        partial_failure: documents
            .iter()
            .filter(|d| d.status.as_deref() == Some("partial_failure"))
            .count(),
        failed: documents
            .iter()
            .filter(|d| d.status.as_deref() == Some("failed"))
            .count(),
        cancelled: documents
            .iter()
            .filter(|d| d.status.as_deref() == Some("cancelled"))
            .count(),
    };

    let total = documents.len();
    let page_size = 20usize;
    let total_pages = (total + page_size - 1) / page_size.max(1);
    let page = 1usize;
    let has_more = page < total_pages;

    Ok(Json(ListDocumentsResponse {
        total,
        documents,
        page,
        page_size,
        total_pages,
        has_more,
        status_counts,
    }))
}
