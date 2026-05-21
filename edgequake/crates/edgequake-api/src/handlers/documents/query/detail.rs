//! Get single document detail handler.

use axum::{extract::State, Json};
use tracing::debug;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::state::AppState;

use crate::handlers::documents_types::*;

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

    // SPEC-011: prefix scan — no full keys() table scan
    let chunk_prefix = format!("{}-chunk-", document_id);
    let chunk_keys = state.kv_storage.keys_with_prefix(&chunk_prefix).await?;
    let chunk_count = chunk_keys.len();
    debug!(chunk_count = chunk_count, "Document chunk keys loaded");

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

    // SPEC-040: Async fallback PDF vision model lookup for backward compatibility.
    // WHY: Documents processed before pdf_vision_model was written to KV metadata JSON
    // don't have that field. We query the pdf_documents table as fallback using the
    // pdf_id that IS stored in all document metadata records.
    let (
        fallback_pdf_vision_model,
        fallback_pdf_extraction_method,
        fallback_pdf_extraction_warning,
    ): (Option<String>, Option<String>, Option<String>) = {
        let needs_fallback = meta_obj
            .and_then(|obj| obj.get("pdf_vision_model"))
            .is_none();
        let pdf_uuid_opt = if needs_fallback {
            meta_obj
                .and_then(|obj| obj.get("pdf_id"))
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
        } else {
            None
        };
        if let Some(pdf_uuid) = pdf_uuid_opt {
            #[cfg(feature = "postgres")]
            {
                if let Some(ref pool) = state.pg_pool {
                    match sqlx::query_as::<
                        _,
                        (
                            Option<String>,
                            Option<String>,
                            Option<serde_json::Value>,
                            String,
                        ),
                    >(
                        "SELECT vision_model, extraction_method, extraction_errors, processing_status FROM pdf_documents WHERE pdf_id = $1",
                    )
                    .bind(pdf_uuid)
                    .fetch_optional(pool)
                    .await
                    {
                        Ok(Some((
                            vision_model,
                            extraction_method,
                            extraction_errors,
                            processing_status,
                        ))) => (
                            vision_model.clone(),
                            extraction_method.or_else(|| {
                                if processing_status == "completed" && vision_model.is_some() {
                                    Some("vision".to_string())
                                } else {
                                    None
                                }
                            }),
                            extraction_errors
                                .and_then(|value| value.get("low_content_warning").cloned())
                                .and_then(|value| value.get("message").cloned())
                                .and_then(|value| value.as_str().map(str::to_string)),
                        ),
                        _ => (None, None, None),
                    }
                } else {
                    (None, None, None)
                }
            }
            #[cfg(not(feature = "postgres"))]
            {
                let _ = pdf_uuid;
                (None, None, None)
            }
        } else {
            (None, None, None)
        }
    };

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
        pdf_id,
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

            // SPEC-040: PDF extraction lineage fields
            // WHY: vision_model and extraction_method are stored in metadata JSON by the PDF
            // processor so the document detail view can show what model was used for extraction.
            // For documents processed before this field was added, fall back to the values
            // looked up from the pdf_documents table (fallback_pdf_vision_model).
            let pdf_vision_model = obj
                .get("pdf_vision_model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| fallback_pdf_vision_model.clone());
            let pdf_extraction_method = obj
                .get("pdf_extraction_method")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| fallback_pdf_extraction_method.clone());
            let pdf_extraction_warning = obj
                .get("pdf_extraction_warning")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| fallback_pdf_extraction_warning.clone());

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
                || pdf_extraction_method.is_some()
                || pdf_extraction_warning.is_some()
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
                    pdf_vision_model,
                    pdf_extraction_method,
                    pdf_extraction_warning,
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
            // OODA-50: Extract pdf_id from metadata for PDF viewer
            obj.get("pdf_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
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
            None,                    // pdf_id
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
        // OODA-50: Use pdf_id from metadata for PDF viewer
        pdf_id,
    }))
}
