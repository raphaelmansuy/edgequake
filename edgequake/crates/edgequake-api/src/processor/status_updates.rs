use super::*;
use crate::document_metadata::{is_informational_notice, is_terminal_failure_status};

fn apply_status_notice_fields(
    metadata: &mut serde_json::Map<String, serde_json::Value>,
    status: &str,
    message: Option<&str>,
) {
    if let Some(msg) = message {
        if is_terminal_failure_status(status) {
            metadata.insert("error_message".to_string(), json!(msg));
            let failure = crate::services::classify_ingestion_failure(msg);
            metadata.insert("failure_class".to_string(), json!(failure.as_str()));
            metadata.insert(
                "recommended_action".to_string(),
                json!(failure.recommended_action()),
            );
        } else {
            metadata.insert("warning_message".to_string(), json!(msg));
            // Scrub legacy informational errors that would render as Failed in the WebUI.
            if let Some(existing) = metadata
                .get("error_message")
                .and_then(|v| v.as_str())
                .filter(|s| is_informational_notice(s))
            {
                if metadata.get("warning_message").is_none() {
                    metadata.insert("warning_message".to_string(), json!(existing));
                }
                metadata.remove("error_message");
            }
        }
        metadata.insert("stage_message".to_string(), json!(msg));
    } else if !is_terminal_failure_status(status) && status != "cancelled" {
        // Forward progress: drop stale terminal errors from prior attempts / legacy writes.
        if let Some(existing) = metadata.get("error_message").and_then(|v| v.as_str()) {
            if is_informational_notice(existing) && metadata.get("warning_message").is_none() {
                metadata.insert("warning_message".to_string(), json!(existing));
            }
            metadata.remove("error_message");
        }
    }
}

async fn upsert_metadata_with_wsdoc_index(
    kv: &std::sync::Arc<dyn edgequake_storage::traits::KVStorage>,
    metadata_key: &str,
    value: serde_json::Value,
) -> Result<(), edgequake_tasks::TaskError> {
    crate::services::upsert_metadata_kv_with_index(kv.as_ref(), metadata_key, value)
        .await
        .map_err(|e| edgequake_tasks::TaskError::Storage(e.to_string()))
}

impl DocumentTaskProcessor {
    /// Update document metadata status.
    ///
    /// @implements SPEC-002: Unified Ingestion Pipeline
    /// Updates both legacy `status` field and new `current_stage` field for backward compatibility.
    /// Creates metadata if it doesn't exist (for PDF documents that bypass upload handler).
    pub(super) async fn update_document_status(
        &self,
        document_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> TaskResult<()> {
        let metadata_key =
            crate::services::resolve_document_metadata_key(document_id, &self.kv_storage).await;

        // SPEC-002: Map legacy status names to unified stage names
        let unified_stage = match status {
            "pending" => "uploading",
            "processing" => "preprocessing",
            "chunking" => "chunking",
            "extracting" => "extracting",
            "embedding" => "embedding",
            "indexing" => "storing",
            "completed" | "indexed" => "completed",
            "failed" => "failed",
            "partial_failure" => "partial_failure",
            "cancelled" => "cancelled",
            other => other, // Pass through unknown statuses
        };

        // SPEC-002: Build stage message based on status
        let stage_message = match status {
            "pending" => "Document queued for processing",
            "processing" | "preprocessing" => "Preprocessing document...",
            "chunking" => "Splitting document into chunks...",
            "extracting" => "Extracting entities and relationships...",
            "embedding" => "Generating vector embeddings...",
            "indexing" | "storing" => "Storing in knowledge graph...",
            "completed" | "indexed" => "Processing complete",
            "failed" => "Processing failed",
            "partial_failure" => "Processing completed with issues",
            "cancelled" => "Processing cancelled",
            _ => "Processing...",
        };

        // Get existing metadata or create new
        let existing = self
            .kv_storage
            .get_by_id(&metadata_key)
            .await
            .ok()
            .flatten();

        let updated_json = if let Some(existing_val) = existing {
            if let Some(obj) = existing_val.as_object() {
                let mut updated = obj.clone();
                updated.insert("status".to_string(), json!(status));
                updated.insert(
                    "updated_at".to_string(),
                    json!(chrono::Utc::now().to_rfc3339()),
                );
                updated.insert("current_stage".to_string(), json!(unified_stage));
                updated.insert("stage_message".to_string(), json!(stage_message));

                apply_status_notice_fields(&mut updated, status, error_message);

                json!(updated)
            } else {
                return Ok(()); // Malformed metadata, skip update
            }
        } else {
            // SPEC-002: Create new metadata for documents that don't have it
            // This happens for PDFs that bypass the upload handler
            let mut new_metadata = serde_json::Map::new();
            new_metadata.insert("id".to_string(), json!(document_id));
            new_metadata.insert("status".to_string(), json!(status));
            new_metadata.insert("current_stage".to_string(), json!(unified_stage));
            new_metadata.insert("stage_message".to_string(), json!(stage_message));
            new_metadata.insert(
                "created_at".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
            new_metadata.insert(
                "updated_at".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
            // Note: source_type will be set later if available from task metadata

            apply_status_notice_fields(&mut new_metadata, status, error_message);

            json!(new_metadata)
        };

        upsert_metadata_with_wsdoc_index(&self.kv_storage, &metadata_key, updated_json).await?;

        Ok(())
    }

    /// Ensure document metadata includes source_type.
    ///
    /// @implements SPEC-002: Unified Ingestion Pipeline
    /// Sets source_type (pdf, markdown, text) for unified pipeline display.
    /// Creates metadata if it doesn't exist (for PDFs that bypass upload handler).
    ///
    /// OODA-05: Added tenant_id/workspace_id parameters to ensure multi-tenant context
    /// is propagated when creating new document metadata. Without these fields,
    /// documents become invisible in workspace-filtered queries.
    ///
    /// OODA-49: Added pdf_id parameter for PDF documents to enable frontend PDF viewing.
    /// The pdf_id is a UUID that references the PDF binary stored in pdf_storage.
    ///
    /// OODA-ITERATION-03: Added track_id parameter for cancel button support.
    /// WHY: Frontend cancel button requires doc.track_id to call POST /tasks/{track_id}/cancel
    pub(super) async fn ensure_document_source_type(
        &self,
        document_id: &str,
        source_type: &str,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
        pdf_id: Option<&str>,
        track_id: Option<&str>,
    ) -> TaskResult<()> {
        let metadata_key =
            crate::services::resolve_document_metadata_key(document_id, &self.kv_storage).await;

        // Get existing metadata or create new
        let existing = self
            .kv_storage
            .get_by_id(&metadata_key)
            .await
            .ok()
            .flatten();

        let updated_json = if let Some(existing_val) = existing {
            if let Some(obj) = existing_val.as_object() {
                // Only update if source_type is not already set
                if obj.get("source_type").is_none() {
                    let mut updated = obj.clone();
                    updated.insert("source_type".to_string(), json!(source_type));
                    updated.insert(
                        "updated_at".to_string(),
                        json!(chrono::Utc::now().to_rfc3339()),
                    );
                    // OODA-05: Also update tenant/workspace if missing
                    if obj.get("tenant_id").is_none() {
                        if let Some(tid) = tenant_id {
                            updated.insert("tenant_id".to_string(), json!(tid));
                        }
                    }
                    if obj.get("workspace_id").is_none() {
                        if let Some(wid) = workspace_id {
                            updated.insert("workspace_id".to_string(), json!(wid));
                        }
                    }
                    // OODA-49: Also update pdf_id if missing
                    // WHY: PDF documents need pdf_id for frontend to build download URLs
                    if obj.get("pdf_id").is_none() {
                        if let Some(pid) = pdf_id {
                            updated.insert("pdf_id".to_string(), json!(pid));
                        }
                    }
                    // OODA-ITERATION-03: Also update track_id if missing
                    // WHY: Cancel button requires track_id to call cancel API
                    if obj.get("track_id").is_none() {
                        if let Some(tid) = track_id {
                            updated.insert("track_id".to_string(), json!(tid));
                        }
                    }
                    Some(json!(updated))
                } else {
                    // OODA-49: Even if source_type is set, check if pdf_id needs to be added
                    // WHY: Fix existing documents that have source_type but missing pdf_id
                    let needs_pdf_id = pdf_id.is_some() && obj.get("pdf_id").is_none();
                    // OODA-ITERATION-03: Also check if track_id needs to be added
                    let needs_track_id = track_id.is_some() && obj.get("track_id").is_none();

                    if needs_pdf_id || needs_track_id {
                        let mut updated = obj.clone();
                        if let Some(pid) = pdf_id {
                            if obj.get("pdf_id").is_none() {
                                updated.insert("pdf_id".to_string(), json!(pid));
                            }
                        }
                        if let Some(tid) = track_id {
                            if obj.get("track_id").is_none() {
                                updated.insert("track_id".to_string(), json!(tid));
                            }
                        }
                        updated.insert(
                            "updated_at".to_string(),
                            json!(chrono::Utc::now().to_rfc3339()),
                        );
                        Some(json!(updated))
                    } else {
                        None // Already has source_type, pdf_id, and track_id (or not needed), skip update
                    }
                }
            } else {
                None // Malformed metadata, skip update
            }
        } else {
            // Create new metadata for documents that don't have it (e.g., PDFs)
            // OODA-05: Include tenant_id/workspace_id for multi-tenant visibility
            let mut new_metadata = serde_json::Map::new();
            new_metadata.insert("id".to_string(), json!(document_id));
            new_metadata.insert("source_type".to_string(), json!(source_type));
            // OODA-04: Store document_type = source_type for lineage consistency
            // WHY: Lineage queries expect document_type to distinguish pdf vs markdown
            new_metadata.insert("document_type".to_string(), json!(source_type));
            new_metadata.insert("current_stage".to_string(), json!("preprocessing"));
            new_metadata.insert("stage_message".to_string(), json!("Processing document..."));
            new_metadata.insert("status".to_string(), json!("processing"));
            new_metadata.insert(
                "created_at".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
            new_metadata.insert(
                "updated_at".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
            // OODA-05: Critical - include tenant/workspace context
            if let Some(tid) = tenant_id {
                new_metadata.insert("tenant_id".to_string(), json!(tid));
            }
            if let Some(wid) = workspace_id {
                new_metadata.insert("workspace_id".to_string(), json!(wid));
            }
            // OODA-49: Include pdf_id for PDF documents
            // WHY: Frontend needs pdf_id to build download URLs for PDF viewing
            if let Some(pid) = pdf_id {
                new_metadata.insert("pdf_id".to_string(), json!(pid));
            }
            // OODA-ITERATION-03: Include track_id for cancel button support
            // WHY: Frontend cancel button requires doc.track_id to call POST /tasks/{track_id}/cancel
            if let Some(tid) = track_id {
                new_metadata.insert("track_id".to_string(), json!(tid));
            }
            Some(json!(new_metadata))
        };

        if let Some(json) = updated_json {
            upsert_metadata_with_wsdoc_index(&self.kv_storage, &metadata_key, json).await?;
        }

        Ok(())
    }

    /// Update document metadata with processing stats and lineage information.
    ///
    /// @implements SPEC-002: Unified Ingestion Pipeline
    /// Sets both legacy `status` and new `current_stage` fields.
    pub(super) async fn update_document_status_with_stats(
        &self,
        document_id: &str,
        status: &str,
        stats: &edgequake_pipeline::pipeline::ProcessingStats,
    ) -> TaskResult<()> {
        let metadata_key =
            crate::services::resolve_document_metadata_key(document_id, &self.kv_storage).await;

        // Get existing metadata
        if let Ok(Some(existing)) = self.kv_storage.get_by_id(&metadata_key).await {
            if let Some(obj) = existing.as_object() {
                let mut updated = obj.clone();
                updated.insert("status".to_string(), json!(status));
                updated.insert(
                    "updated_at".to_string(),
                    json!(chrono::Utc::now().to_rfc3339()),
                );
                updated.insert(
                    "processed_at".to_string(),
                    json!(chrono::Utc::now().to_rfc3339()),
                );

                // SPEC-002: Set unified current_stage and stage_message for completion
                let unified_stage = if status == "completed" || status == "indexed" {
                    "completed"
                } else {
                    status
                };
                updated.insert("current_stage".to_string(), json!(unified_stage));
                updated.insert("stage_progress".to_string(), json!(1.0)); // 100% complete

                // SPEC-002: Informative completion message with stats
                let stage_message = format!(
                    "Processed {} chunks, extracted {} entities and {} relationships",
                    stats.chunk_count, stats.entity_count, stats.relationship_count
                );
                updated.insert("stage_message".to_string(), json!(stage_message));

                // Basic stats
                updated.insert("chunk_count".to_string(), json!(stats.chunk_count));
                updated.insert("entity_count".to_string(), json!(stats.entity_count));
                updated.insert(
                    "relationship_count".to_string(),
                    json!(stats.relationship_count),
                );
                updated.insert(
                    "processing_duration_ms".to_string(),
                    json!(stats.processing_time_ms),
                );

                // Cost tracking fields
                updated.insert("cost_usd".to_string(), json!(stats.cost_usd));
                updated.insert("input_tokens".to_string(), json!(stats.input_tokens));
                updated.insert("output_tokens".to_string(), json!(stats.output_tokens));
                updated.insert("total_tokens".to_string(), json!(stats.total_tokens));

                // Lineage information
                if let Some(ref llm_model) = stats.llm_model {
                    updated.insert("llm_model".to_string(), json!(llm_model));
                }
                // SPEC-032/OODA-198: Store LLM provider for lineage tracking
                if let Some(ref llm_provider) = stats.llm_provider {
                    updated.insert("llm_provider".to_string(), json!(llm_provider));
                }
                if let Some(ref embedding_model) = stats.embedding_model {
                    updated.insert("embedding_model".to_string(), json!(embedding_model));
                }
                // SPEC-032/OODA-198: Store embedding provider for lineage tracking
                if let Some(ref embedding_provider) = stats.embedding_provider {
                    updated.insert("embedding_provider".to_string(), json!(embedding_provider));
                }
                if let Some(ref embedding_dimensions) = stats.embedding_dimensions {
                    updated.insert(
                        "embedding_dimensions".to_string(),
                        json!(embedding_dimensions),
                    );
                }
                if let Some(ref entity_types) = stats.entity_types {
                    updated.insert("entity_types".to_string(), json!(entity_types));
                }
                if let Some(ref relationship_types) = stats.relationship_types {
                    updated.insert("relationship_types".to_string(), json!(relationship_types));
                }
                if let Some(ref keywords) = stats.keywords {
                    updated.insert("keywords".to_string(), json!(keywords));
                }
                if let Some(ref chunking_strategy) = stats.chunking_strategy {
                    updated.insert("chunking_strategy".to_string(), json!(chunking_strategy));
                }
                if let Some(ref avg_chunk_size) = stats.avg_chunk_size {
                    updated.insert("avg_chunk_size".to_string(), json!(avg_chunk_size));
                }

                if status == "completed" || status == "indexed" {
                    updated.remove("error_message");
                    updated.remove("warning_message");
                } else if status == "failed" || status == "partial_failure" {
                    if let Some(ref details) = stats.error_details {
                        updated.insert("error_message".to_string(), json!(details));
                    } else if status == "partial_failure" || !updated.contains_key("error_message")
                    {
                        updated.insert("error_message".to_string(), json!(stage_message));
                    }
                    updated.remove("warning_message");
                } else if status == "cancelled" {
                    updated.remove("warning_message");
                }

                upsert_metadata_with_wsdoc_index(&self.kv_storage, &metadata_key, json!(updated))
                    .await?;

                // SPEC-021 P-A1: mirror the stats into the relational
                // `documents` table so the P5-01 relational read-model
                // fallback returns accurate per-doc counts (fixes the
                // "Completed / 0 entities" screenshot, file 16 §3).
                //
                // Best-effort: a failure here MUST NOT fail ingestion — the
                // KV metadata write above is the authoritative stats carrier.
                // The relational write only needs to converge eventually.
                self.refresh_relational_document_stats(document_id, status, stats)
                    .await;
            }
        }

        Ok(())
    }

    /// Best-effort relational stats refresh (SPEC-021 P-A1).
    ///
    /// WHY: `ensure_document_record` only writes `title/content/status`; the
    /// `chunk_count`/`entity_count`/cost columns of `documents` were never
    /// refreshed, so the P5-01 relational read-model fallback returned stale
    /// `0` values. This mirrors the KV stats write into the relational table
    /// so the per-doc read model converges with the KV truth.
    ///
    /// Non-fatal: logs a warn on error and returns. The KV metadata write is
    /// the authoritative stats carrier; the relational write only needs to
    /// converge eventually. No-op when `pdf_storage` is absent (memory mode or
    /// when the postgres feature is disabled).
    #[cfg(feature = "postgres")]
    async fn refresh_relational_document_stats(
        &self,
        document_id: &str,
        status: &str,
        stats: &edgequake_pipeline::pipeline::ProcessingStats,
    ) {
        let Some(ref pdf_storage) = self.pdf_storage else {
            return;
        };
        let Ok(doc_uuid) = uuid::Uuid::parse_str(document_id) else {
            tracing::warn!(
                document_id = document_id,
                "refresh_relational_document_stats: not a UUID — skipping relational stats write"
            );
            return;
        };

        let update = edgequake_storage::DocumentStatsUpdate {
            document_id: doc_uuid,
            status,
            chunk_count: stats.chunk_count as i32,
            entity_count: stats.entity_count as i32,
            relationship_count: stats.relationship_count as i32,
            // cost_usd is f64 (0.0 when the mock provider is used); only
            // surface a non-zero value to the relational column to avoid
            // overwriting a prior real cost with a mock 0.0 on re-runs.
            cost_usd: (stats.cost_usd > 0.0).then_some(stats.cost_usd),
            input_tokens: (stats.input_tokens > 0).then_some(stats.input_tokens as i64),
            output_tokens: (stats.output_tokens > 0).then_some(stats.output_tokens as i64),
            total_tokens: (stats.total_tokens > 0).then_some(stats.total_tokens as i64),
            error_message: stats.error_details.as_deref(),
        };

        if let Err(e) = pdf_storage.update_document_stats(&update).await {
            tracing::warn!(
                document_id = document_id,
                error = %e,
                "SPEC-021 P-A1: relational document stats refresh failed (non-fatal)"
            );
        }
    }

    #[cfg(not(feature = "postgres"))]
    async fn refresh_relational_document_stats(
        &self,
        _document_id: &str,
        _status: &str,
        _stats: &edgequake_pipeline::pipeline::ProcessingStats,
    ) {
        // No-op: postgres feature disabled → no relational `documents` table.
    }
}
