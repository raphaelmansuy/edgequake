//! List all documents handler.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use tracing::debug;

use crate::error::ApiResult;
use crate::middleware::TenantContext;
use crate::read_path::{
    run_with_read_path_guard, should_skip_entity_reconcile, ReadPathDbPermit,
    MAX_LIST_METADATA_ENTRIES,
};
use crate::services::document_metadata_scan::canonical_document_id;
use crate::services::list_pagination::paginate_vec;
use crate::services::tenant_guard::{
    empty_documents_list, has_full_tenant_context, warn_missing_tenant_context,
};
use crate::state::{PostgresRuntime, StorageRuntime, TaskRuntime};
use edgequake_core::ResourceBudgetConfig;

use crate::handlers::documents_types::*;

/// List all documents.
#[utoipa::path(
    get,
    path = "/api/v1/documents",
    tag = "Documents",
    params(
        ("page" = Option<usize>, Query, description = "Page number (default 1)"),
        ("page_size" = Option<usize>, Query, description = "Page size (default 20, max 100)"),
        ("date_from" = Option<String>, Query, description = "Inclusive start date (ISO 8601)"),
        ("date_to" = Option<String>, Query, description = "Inclusive end date (ISO 8601)"),
        ("document_pattern" = Option<String>, Query, description = "Case-insensitive title substring (comma = OR)"),
        ("status" = Option<String>, Query, description = "Filter by status before pagination (e.g. failed, completed). status_counts remain global (SPEC-084 / GH-319)"),
    ),
    responses(
        (status = 200, description = "Documents retrieved", body = ListDocumentsResponse),
        (status = 503, description = "Read path busy under ingest load")
    )
)]
#[allow(clippy::field_reassign_with_default)]
pub async fn list_documents(
    State(storage): State<StorageRuntime>,
    State(_pg_runtime): State<PostgresRuntime>,
    State(budget): State<ResourceBudgetConfig>,
    State(tasks): State<TaskRuntime>,
    State(read_path_db): State<Arc<ReadPathDbPermit>>,
    tenant_ctx: TenantContext,
    Query(params): Query<ListDocumentsRequest>,
) -> ApiResult<Json<ListDocumentsResponse>> {
    run_with_read_path_guard(&read_path_db, || {
        list_documents_inner(storage, _pg_runtime, budget, tasks, tenant_ctx, params)
    })
    .await
}

#[allow(clippy::field_reassign_with_default)]
async fn list_documents_inner(
    storage: StorageRuntime,
    _pg_runtime: PostgresRuntime,
    budget: ResourceBudgetConfig,
    tasks: TaskRuntime,
    tenant_ctx: TenantContext,
    params: ListDocumentsRequest,
) -> ApiResult<Json<ListDocumentsResponse>> {
    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Listing documents with tenant context"
    );

    // SECURITY: Enforce strict tenant context requirement - NO EXCEPTIONS
    // This matches the strict filtering in entities.rs and relationships.rs (commit d11edba8)
    if !has_full_tenant_context(&tenant_ctx) {
        warn_missing_tenant_context(&tenant_ctx, "list_documents");
        return Ok(Json(empty_documents_list()));
    }

    // SPEC-027: scoped metadata scan SSOT — cap keys *before* value fetch so
    // large workspaces never pay unbounded get_by_ids under ingest load.
    let scoped =
        crate::services::document_metadata_scan::load_scoped_document_metadata_entries_limited(
            storage.kv_storage.as_ref(),
            &tenant_ctx,
            MAX_LIST_METADATA_ENTRIES,
        )
        .await?;
    let metadata_entries = scoped.entries;
    let truncated = scoped.truncated;
    if truncated {
        tracing::warn!(
            entry_count_cap = MAX_LIST_METADATA_ENTRIES,
            "Document metadata key scan truncated before value fetch (interactive list)"
        );
    }
    debug!(
        metadata_entries_count = metadata_entries.len(),
        truncated, "Scoped metadata entries retrieved"
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
        warning_message: Option<String>,
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
        chunk_count: Option<usize>,
    }

    impl DocMetadata {
        fn normalized_notices(&self) -> (Option<String>, Option<String>) {
            crate::document_metadata::normalize_notice_fields(
                self.status.as_deref(),
                self.error_message.clone(),
                self.warning_message.clone(),
            )
        }
    }

    let mut doc_metadata: std::collections::HashMap<String, DocMetadata> =
        std::collections::HashMap::new();

    for (metadata_key, value) in metadata_entries {
        // WHY: avoid DEBUG-dumping full metadata JSON on every list row — with
        // large workspaces this floods logs and adds measurable latency.
        tracing::trace!(metadata_key = %metadata_key, "Processing metadata entry");
        if let Some(obj) = value.as_object() {
            let id = canonical_document_id(&metadata_key, &value);
            tracing::trace!(
                doc_id = %id,
                title = ?obj.get("title"),
                "Extracted canonical ID and title"
            );

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

            // Get error_message / warning_message (normalized at response build time)
            meta.error_message = obj
                .get("error_message")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            meta.warning_message = obj
                .get("warning_message")
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

            meta.chunk_count = obj
                .get("chunk_count")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);

            if let Some(existing) = doc_metadata.get(&id) {
                if crate::document_metadata::should_prefer_incoming_document_metadata(
                    existing.updated_at.as_deref(),
                    existing.status.as_deref(),
                    existing.current_stage.as_deref(),
                    meta.updated_at.as_deref(),
                    meta.status.as_deref(),
                    meta.current_stage.as_deref(),
                ) {
                    doc_metadata.insert(id, meta);
                }
            } else {
                doc_metadata.insert(id, meta);
            }
        }
    }

    // Build document list from scoped metadata (chunk_count from metadata — SPEC-027 IMP-019).
    let mut documents: Vec<DocumentSummary> = doc_metadata
        .into_iter()
        .map(|(id, meta)| {
            let (error_message, warning_message) = meta.normalized_notices();
            DocumentSummary {
                id,
                title: meta.title,
                file_name: meta.file_name,
                content_summary: meta.content_summary,
                content_length: meta.content_length,
                chunk_count: meta.chunk_count.unwrap_or(0),
                entity_count: meta.entity_count,
                status: meta.status,
                error_message,
                warning_message,
                track_id: meta.track_id,
                created_at: meta.created_at,
                updated_at: meta.updated_at,
                cost_usd: meta.cost_usd,
                input_tokens: meta.input_tokens,
                output_tokens: meta.output_tokens,
                total_tokens: meta.total_tokens,
                llm_model: meta.llm_model,
                embedding_model: meta.embedding_model,
                source_type: meta.source_type,
                current_stage: meta.current_stage,
                stage_progress: meta.stage_progress,
                stage_message: meta.stage_message,
                pdf_id: meta.pdf_id,
                display_status: None,
                ui_phase: None,
            }
        })
        .collect();

    // Sort by created_at descending (newest first)
    documents.sort_by(|a, b| {
        b.created_at
            .as_deref()
            .unwrap_or("")
            .cmp(a.created_at.as_deref().unwrap_or(""))
    });

    // SPEC-021 P5-01: Merge relational documents missing from KV metadata.
    // WHY: the relational `documents` table is the durable source of truth for
    // uploads; KV `*-metadata` drifts (legacy workspaces, missing writes). When
    // KV returns nothing for a workspace, this merge is the ONLY source of the
    // list. A silently-swallowed error here produces the "0 documents" UI state
    // while the graph (populated from a separate write path) shows entities —
    // so we log at ERROR and track a warning string to surface, not just warn.
    #[cfg(feature = "postgres")]
    if _pg_runtime.pool.is_some() {
        match crate::document_read_model::list_relational_document_summaries(
            _pg_runtime.pool.as_ref(),
            &tenant_ctx,
        )
        .await
        {
            Ok(relational) if !relational.is_empty() => {
                documents =
                    crate::document_read_model::merge_document_summaries(documents, relational);
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!(
                    error = %e,
                    tenant = ?tenant_ctx.tenant_id,
                    workspace = ?tenant_ctx.workspace_id,
                    "Relational document backfill failed — list may show 0 docs erroneously"
                );
            }
        }
    }

    if should_skip_entity_reconcile(&tasks.storage).await {
        // Serve KV/relational counts under queue/storage pressure — never hang on AGE.
    } else {
        crate::document_read_model::reconcile_entity_counts_with_graph(&storage, &mut documents)
            .await;
    }

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

    // Calculate status counts for all documents (after date/pattern, before status filter).
    // SPEC-084 / GH-319 LAW-10: counts stay global; list items honor optional status.
    let status_counts = StatusCounts {
        pending: documents
            .iter()
            .filter(|d| {
                matches!(d.status.as_deref(), Some("pending" | "queued"))
                    || d.current_stage.as_deref() == Some("queued")
            })
            .count(),
        processing: documents
            .iter()
            .filter(|d| {
                matches!(d.status.as_deref(), Some("processing"))
                    || matches!(
                        d.current_stage.as_deref(),
                        Some(
                            "converting"
                                | "preprocessing"
                                | "chunking"
                                | "extracting"
                                | "gleaning"
                                | "merging"
                                | "summarizing"
                                | "embedding"
                                | "storing"
                                | "indexing"
                        )
                    )
            })
            .count(),
        // SPEC-021 P-B2: only count explicit completed/indexed status, NOT NULL.
        completed: documents
            .iter()
            .filter(|d| {
                d.status.as_deref() == Some("completed") || d.status.as_deref() == Some("indexed")
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
        // SPEC-021 P-B2: NULL/unknown status is its own bucket, not completed.
        unknown: documents
            .iter()
            .filter(|d| {
                d.status.is_none()
                    || !matches!(
                        d.status.as_deref(),
                        Some(
                            "pending"
                                | "queued"
                                | "processing"
                                | "completed"
                                | "indexed"
                                | "partial_failure"
                                | "failed"
                                | "cancelled"
                        )
                    )
            })
            .count(),
    };

    // SPEC-084 / GH-319: filter by status before pagination so Failed chip rows match counts.
    if let Some(ref status_raw) = params.status {
        let status_filter = status_raw.trim().to_lowercase();
        if !status_filter.is_empty() && status_filter != "all" {
            documents.retain(|doc| {
                let doc_status = doc.status.as_deref().unwrap_or("").to_lowercase();
                match status_filter.as_str() {
                    "pending" => {
                        matches!(doc_status.as_str(), "pending" | "queued")
                            || doc.current_stage.as_deref() == Some("queued")
                    }
                    "processing" => {
                        doc_status == "processing"
                            || matches!(
                                doc.current_stage.as_deref(),
                                Some(
                                    "converting"
                                        | "preprocessing"
                                        | "chunking"
                                        | "extracting"
                                        | "gleaning"
                                        | "merging"
                                        | "summarizing"
                                        | "embedding"
                                        | "storing"
                                        | "indexing"
                                )
                            )
                    }
                    "completed" => matches!(doc_status.as_str(), "completed" | "indexed"),
                    "unknown" => {
                        doc.status.is_none()
                            || !matches!(
                                doc_status.as_str(),
                                "pending"
                                    | "queued"
                                    | "processing"
                                    | "completed"
                                    | "indexed"
                                    | "partial_failure"
                                    | "failed"
                                    | "cancelled"
                            )
                    }
                    other => doc_status == other,
                }
            });
            debug!(
                status = %status_filter,
                filtered_count = documents.len(),
                "Applied SPEC-084 document status filter before pagination"
            );
        }
    }

    // SPEC-057 P4: project display_status / ui_phase SSOT before pagination.
    crate::services::ingestion_status_mapper::enrich_document_summaries_with_cancel(
        &mut documents,
        &tasks.cancellation_registry,
    )
    .await;

    // SPEC-027 IMP-020: honor query pagination (status_counts remain over full pre-status set).
    let page_size = budget.clamp_page_size(params.page_size.min(u32::MAX as usize) as u32) as usize;
    let page = params.page.max(1);
    let (documents, pagination) = paginate_vec(documents, page, page_size);

    Ok(Json(ListDocumentsResponse {
        total: pagination.total,
        documents,
        page: pagination.page,
        page_size: pagination.page_size,
        total_pages: pagination.total_pages,
        has_more: pagination.has_more,
        status_counts,
        truncated: truncated.then_some(true),
    }))
}
