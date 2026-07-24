//! Get single document detail handler.

use std::sync::Arc;

use axum::{extract::State, Json};
use serde_json::Value;
use tracing::debug;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::read_path::{run_with_read_path_guard, ReadPathDbPermit};
use crate::services::document_body_loader::load_document_body;
use crate::services::tenant_isolation::PgIsolationScope;
use crate::state::{ApiSecurityConfig, PostgresRuntime, StorageRuntime, TaskRuntime};

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
        (status = 403, description = "Access denied - document belongs to different tenant"),
        (status = 503, description = "Read path busy under ingest load")
    )
)]
pub async fn get_document(
    State(storage): State<StorageRuntime>,
    State(pg_runtime): State<PostgresRuntime>,
    State(security): State<ApiSecurityConfig>,
    State(tasks): State<TaskRuntime>,
    State(read_path_db): State<Arc<ReadPathDbPermit>>,
    tenant_ctx: TenantContext,
    axum::extract::Path(document_id): axum::extract::Path<String>,
) -> ApiResult<Json<DocumentDetailResponse>> {
    run_with_read_path_guard(&read_path_db, || {
        get_document_inner(
            storage,
            pg_runtime,
            security,
            tasks,
            tenant_ctx,
            document_id,
        )
    })
    .await
}

async fn get_document_inner(
    storage: StorageRuntime,
    pg_runtime: PostgresRuntime,
    security: ApiSecurityConfig,
    tasks: TaskRuntime,
    tenant_ctx: TenantContext,
    document_id: String,
) -> ApiResult<Json<DocumentDetailResponse>> {
    debug!(
        document_id = %document_id,
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Getting document by ID with tenant context"
    );

    // Fetch document metadata (final, then staging — SPEC-086 list-visible shells).
    let metadata_key =
        crate::services::document_metadata_scan::metadata_key_for_document(&document_id);
    let staging_key = edgequake_storage::kv_keys::staging_doc_metadata(&document_id);
    debug!(metadata_key = %metadata_key, "Looking up metadata key");
    let mut metadata = storage.kv_storage.get_by_id(&metadata_key).await?;
    if metadata.is_none() {
        metadata = storage.kv_storage.get_by_id(&staging_key).await?;
    }
    debug!(has_metadata = metadata.is_some(), "Metadata value present");

    // SPEC-011: prefix scan — no full keys() table scan
    let chunk_prefix = format!("{}-chunk-", document_id);
    let chunk_keys = storage.kv_storage.keys_with_prefix(&chunk_prefix).await?;
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
                    return Err(ApiError::forbidden());
                }
            }
        }

        // Verify workspace access
        if let Some(ref filter_ws) = tenant_ctx.workspace_id {
            if let Some(doc_ws) = doc_workspace_id {
                if doc_ws != filter_ws {
                    return Err(ApiError::forbidden());
                }
            }
        }
    }

    // Fetch document content via SSOT loader (KV first, then PDF pipeline markdown).
    let metadata_value = metadata
        .clone()
        .unwrap_or(Value::Object(Default::default()));
    let content = load_document_body(&storage, &document_id, &metadata_value)
        .await
        .map(|b| b.markdown);

    // SPEC-040: Async fallback PDF vision model lookup for backward compatibility.
    // WHY: Documents processed before pdf_vision_model was written to KV metadata JSON
    // don't have that field. We query the pdf_documents table as fallback using the
    // pdf_id that IS stored in all document metadata records.
    // SPEC-027 phase 42: pdf_documents query runs under RLS via with_optional_pg_rls.
    let pg_isolation_scope = PgIsolationScope::from_tenant_context(&tenant_ctx, None);
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
                if let Some(ref pool) = pg_runtime.pool {
                    match crate::services::pdf_lineage::fetch_pdf_extraction_metadata(
                        pool,
                        &security,
                        pg_isolation_scope,
                        pdf_uuid,
                    )
                    .await
                    {
                        Ok(Some(meta)) => (
                            meta.vision_model,
                            meta.extraction_method,
                            meta.extraction_warning,
                        ),
                        Ok(None) | Err(_) => (None, None, None),
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

    let raw_warning = meta_obj.and_then(|obj| {
        obj.get("warning_message")
            .and_then(|v| v.as_str())
            .map(str::to_string)
    });
    let (error_message, warning_message) = crate::document_metadata::normalize_notice_fields(
        Some(status.as_str()),
        error_message,
        raw_warning,
    );

    let multimodal_summary = metadata
        .as_ref()
        .and_then(crate::services::summary_from_metadata);
    let multimodal_items =
        crate::services::load_manifest(storage.kv_storage.as_ref(), &document_id)
            .await
            .map(|manifest| crate::services::manifest_item_status_views(&manifest));

    let current_stage = meta_obj.and_then(|obj| {
        obj.get("current_stage")
            .and_then(|v| v.as_str())
            .map(str::to_string)
    });
    let failure_class = meta_obj.and_then(|obj| {
        obj.get("failure_class")
            .and_then(|v| v.as_str())
            .map(str::to_string)
    });
    let stage_message_kv = meta_obj.and_then(|obj| {
        obj.get("stage_message")
            .and_then(|v| v.as_str())
            .map(str::to_string)
    });
    let cancel_intent = match track_id.as_deref() {
        Some(tid) => tasks.cancellation_registry.has_cancel_intent(tid).await,
        None => false,
    };
    let status_view =
        crate::services::map_ingestion_status(crate::services::IngestionStatusInputs {
            task_status: None,
            doc_status: Some(status.as_str()),
            current_stage: current_stage.as_deref(),
            failure_class: failure_class.as_deref(),
            pdf_status: None,
            error_message: error_message.as_deref(),
            stage_message: stage_message_kv.as_deref(),
            cancel_intent,
        });

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
        display_status: Some(status_view.display_status),
        ui_phase: Some(status_view.ui_phase),
        error_message,
        warning_message,
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
        multimodal_summary,
        multimodal_items,
    }))
}
