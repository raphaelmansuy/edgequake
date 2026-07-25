use super::*;
use crate::document_metadata::{
    is_informational_notice, is_terminal_document_status, is_terminal_failure_status,
    is_terminal_success_status,
};

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
            let workspace = metadata
                .get("workspace_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            edgequake_observability::record_ingestion_failure(failure.as_str(), workspace);
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

pub(crate) fn graph_merge_progress_message(
    sub_phase_label: &str,
    entities_processed: usize,
    entities_total: usize,
    relationships_processed: usize,
    relationships_total: usize,
) -> String {
    if entities_total > 0 || relationships_total > 0 {
        format!(
            "Storing in knowledge graph — {} ({}, {})",
            sub_phase_label,
            format_merge_counter("entities", entities_processed, entities_total),
            format_merge_counter(
                "relationships",
                relationships_processed,
                relationships_total
            ),
        )
    } else {
        format!("Storing in knowledge graph — {}...", sub_phase_label)
    }
}

/// Fraction of graph-merge work complete (entities + relationships weighted equally).
pub(crate) fn graph_merge_progress_fraction(
    entities_processed: usize,
    entities_total: usize,
    relationships_processed: usize,
    relationships_total: usize,
) -> f32 {
    let total = entities_total.saturating_add(relationships_total);
    if total == 0 {
        return 0.0;
    }
    let done = entities_processed.saturating_add(relationships_processed);
    (done as f32 / total as f32).clamp(0.0, 1.0)
}

fn format_merge_counter(label: &str, processed: usize, total: usize) -> String {
    if total == 0 {
        return format!("{label}: —");
    }
    let pct = processed.saturating_mul(100) / total.max(1);
    format!("{processed}/{total} {label} ({pct}%)")
}

/// Patch document KV with merge progress message + fractional stage_progress.
pub(crate) async fn patch_document_graph_merge_progress(
    kv: std::sync::Arc<dyn edgequake_storage::traits::KVStorage>,
    document_id: &str,
    progress: &edgequake_pipeline::MergeProgress,
) {
    let msg = graph_merge_progress_message(
        progress.phase.label(),
        progress.entities_processed,
        progress.entities_total,
        progress.relationships_processed,
        progress.relationships_total,
    );
    let fraction = graph_merge_progress_fraction(
        progress.entities_processed,
        progress.entities_total,
        progress.relationships_processed,
        progress.relationships_total,
    );
    patch_document_indexing_progress_with_fraction(kv, document_id, &msg, Some(fraction)).await;
}

async fn patch_document_indexing_progress_with_fraction(
    kv: std::sync::Arc<dyn edgequake_storage::traits::KVStorage>,
    document_id: &str,
    stage_message: &str,
    stage_progress: Option<f32>,
) {
    // Prefer the key that already exists; never recreate staging after promote.
    // A late spawn that resolved staging before promote can otherwise resurrect
    // `staging:…-metadata` and leave final metadata stuck at `indexing`.
    //
    // IMP-075-04/10: SSOT dual-key batch via load_staging_and_final_metadata.
    let Ok(pair) = crate::services::load_staging_and_final_metadata(kv.as_ref(), document_id).await
    else {
        return;
    };

    let (metadata_key, existing) = if pair.staging.is_some() {
        // If final is already terminal, ignore staging (stale race) and skip.
        if let Some(final_meta) = pair.final_meta.as_ref() {
            if final_meta
                .get("status")
                .and_then(|v| v.as_str())
                .is_some_and(is_terminal_document_status)
            {
                tracing::debug!(
                    document_id = %document_id,
                    "Skipping merge-progress patch — final metadata already terminal"
                );
                return;
            }
        }
        match pair.staging {
            Some(m) => (pair.staging_key, m),
            None => return,
        }
    } else {
        match pair.final_meta {
            Some(m) => (pair.final_key, m),
            None => return,
        }
    };
    let Some(obj) = existing.as_object() else {
        return;
    };
    // Guard: never clobber a terminal status with a stale fire-and-forget merge patch.
    // Evidence: invoice 019f475d… finished (task=indexed, SQL=indexed) while KV stayed
    // `indexing`/`storing` at 100% merge because a late progress spawn rewrote status.
    if obj
        .get("status")
        .and_then(|v| v.as_str())
        .is_some_and(is_terminal_document_status)
    {
        tracing::debug!(
            document_id = %document_id,
            status = obj.get("status").and_then(|v| v.as_str()).unwrap_or(""),
            "Skipping merge-progress KV patch — document already terminal"
        );
        return;
    }
    let mut updated = obj.clone();
    updated.insert("status".to_string(), json!("indexing"));
    updated.insert("current_stage".to_string(), json!("storing"));
    updated.insert("stage_message".to_string(), json!(stage_message));
    if let Some(progress) = stage_progress {
        updated.insert("stage_progress".to_string(), json!(progress));
    }
    updated.insert(
        "updated_at".to_string(),
        json!(chrono::Utc::now().to_rfc3339()),
    );
    apply_status_notice_fields(&mut updated, "indexing", Some(stage_message));
    let _ = upsert_metadata_with_wsdoc_index(&kv, &metadata_key, json!(updated)).await;
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
        // IMP-075-10: one RT for staging+final (not resolve key then re-get).
        let pair =
            crate::services::load_staging_and_final_metadata(self.kv_storage.as_ref(), document_id)
                .await
                .map_err(edgequake_tasks::TaskError::StorageError)?;
        let metadata_key = pair.preferred_key_or_final();
        let existing = pair.preferred().map(|(_, v)| v);

        // SPEC-057 P4: SSOT for legacy→unified stage + default messages.
        let unified_stage =
            crate::services::ingestion_status_mapper::legacy_status_to_unified_stage(status);
        let stage_message =
            crate::services::ingestion_status_mapper::default_stage_message_for_status(status);

        let updated_json = if let Some(existing_val) = existing {
            if let Some(obj) = existing_val.as_object() {
                // Reliability: do not let a late in-flight stage overwrite a
                // terminal status (same class of bug as merge-progress race).
                // Terminal→terminal (e.g. completed→failed/cancelled) is allowed.
                let existing_status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
                if is_terminal_document_status(existing_status)
                    && !is_terminal_document_status(status)
                {
                    tracing::debug!(
                        document_id = %document_id,
                        existing_status,
                        requested_status = status,
                        "Skipping status update — document already terminal"
                    );
                    return Ok(());
                }
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

        upsert_metadata_with_wsdoc_index(&self.kv_storage, &metadata_key, updated_json.clone())
            .await?;

        // SPEC-086: stage-transition WS for all Insert tracks (not only every-3rd chunk).
        let task_id = updated_json
            .get("track_id")
            .or_else(|| updated_json.get("task_id"))
            .and_then(|v| v.as_str())
            .unwrap_or(document_id)
            .to_string();
        let stage_progress = updated_json
            .get("stage_progress")
            .and_then(|v| v.as_f64())
            .map(|p| p as f32);
        self.pipeline_state.emit_stage_transition(
            document_id.to_string(),
            task_id,
            unified_stage.to_string(),
            stage_message.to_string(),
            stage_progress,
        );

        // SPEC-047 P1: mirror non-terminal status into relational documents so
        // soft-resume / indexing is visible (do not wait for finalize stats).
        self.touch_relational_document_status(document_id, status)
            .await;

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
        // IMP-075-10: one RT for staging+final (not resolve key then re-get).
        let pair =
            crate::services::load_staging_and_final_metadata(self.kv_storage.as_ref(), document_id)
                .await
                .map_err(edgequake_tasks::TaskError::StorageError)?;
        let metadata_key = pair.preferred_key_or_final();
        let existing = pair.preferred().map(|(_, v)| v);

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
        // IMP-075-10: one RT for staging+final (not resolve key then re-get).
        let Ok(pair) =
            crate::services::load_staging_and_final_metadata(self.kv_storage.as_ref(), document_id)
                .await
        else {
            return Ok(());
        };
        let Some((metadata_key, existing)) = pair.preferred() else {
            return Ok(());
        };

        if let Some(obj) = existing.as_object() {
            // Never downgrade terminal-success with a non-terminal stats write
            // (edge case #6 — late finalize race / double-call).
            let existing_status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
            if is_terminal_success_status(existing_status) && !is_terminal_document_status(status) {
                tracing::debug!(
                    document_id = %document_id,
                    existing_status,
                    requested_status = status,
                    "Skipping stats status update — document already terminal-success"
                );
                return Ok(());
            }
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
                } else if status == "partial_failure" || !updated.contains_key("error_message") {
                    updated.insert("error_message".to_string(), json!(stage_message));
                }
                updated.remove("warning_message");
            } else if status == "cancelled" {
                updated.remove("warning_message");
            }

            let updated_json = json!(updated);
            upsert_metadata_with_wsdoc_index(&self.kv_storage, &metadata_key, updated_json.clone())
                .await?;

            // SPEC-086: terminal / stats stage transition for Insert tracks.
            let task_id = updated_json
                .get("track_id")
                .or_else(|| updated_json.get("task_id"))
                .and_then(|v| v.as_str())
                .unwrap_or(document_id)
                .to_string();
            self.pipeline_state.emit_stage_transition(
                document_id.to_string(),
                task_id,
                unified_stage.to_string(),
                stage_message.clone(),
                Some(1.0),
            );

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

        // WHY: KV uses `completed`; relational CHECK + UI prefer `indexed`.
        let pg_status = if status == "completed" {
            "indexed"
        } else {
            status
        };

        let update = edgequake_storage::DocumentStatsUpdate {
            document_id: doc_uuid,
            status: pg_status,
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

    /// Best-effort status-only sync to relational `documents` (SPEC-047 P1).
    #[cfg(feature = "postgres")]
    async fn touch_relational_document_status(&self, document_id: &str, status: &str) {
        let Some(ref pdf_storage) = self.pdf_storage else {
            return;
        };
        let Ok(doc_uuid) = uuid::Uuid::parse_str(document_id) else {
            return;
        };
        if let Err(e) = pdf_storage.touch_document_status(&doc_uuid, status).await {
            tracing::warn!(
                document_id = document_id,
                error = %e,
                "SPEC-047 P1: touch_document_status failed (non-fatal)"
            );
        }
    }

    #[cfg(not(feature = "postgres"))]
    async fn touch_relational_document_status(&self, _document_id: &str, _status: &str) {}
}

#[cfg(test)]
mod merge_progress_patch_tests {
    use std::sync::Arc;

    use edgequake_pipeline::{MergePhase, MergeProgress};
    use edgequake_storage::adapters::memory::MemoryKVStorage;
    use edgequake_storage::traits::KVStorage;
    use serde_json::json;

    use super::patch_document_graph_merge_progress;

    #[tokio::test]
    async fn late_merge_progress_does_not_clobber_completed_status() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("merge-progress-guard"));
        let doc_id = "doc-terminal";
        let key = format!("{doc_id}-metadata");
        kv.upsert(&[(
            key.clone(),
            json!({
                "id": doc_id,
                "status": "completed",
                "current_stage": "completed",
                "stage_message": "Processed 1 chunks, extracted 12 entities and 9 relationships",
                "stage_progress": 1.0,
                "entity_count": 12,
                "relationship_count": 9,
            }),
        )])
        .await
        .unwrap();

        let progress = MergeProgress {
            phase: MergePhase::RelationshipGraph,
            entities_processed: 12,
            entities_total: 12,
            relationships_processed: 9,
            relationships_total: 9,
            entities_created: 12,
            entities_updated: 0,
            relationships_created: 9,
            relationships_updated: 0,
        };
        patch_document_graph_merge_progress(kv.clone(), doc_id, &progress).await;

        let meta = kv.get_by_id(&key).await.unwrap().unwrap();
        assert_eq!(meta["status"], "completed");
        assert_eq!(meta["current_stage"], "completed");
        assert!(
            meta["stage_message"]
                .as_str()
                .unwrap()
                .starts_with("Processed"),
            "stage_message must stay terminal, got {:?}",
            meta["stage_message"]
        );
    }

    #[tokio::test]
    async fn merge_progress_still_updates_while_indexing() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("merge-progress-active"));
        let doc_id = "doc-active";
        let key = format!("{doc_id}-metadata");
        kv.upsert(&[(
            key.clone(),
            json!({
                "id": doc_id,
                "status": "indexing",
                "current_stage": "storing",
                "stage_message": "Storing...",
                "stage_progress": 0.1,
            }),
        )])
        .await
        .unwrap();

        let progress = MergeProgress {
            phase: MergePhase::RelationshipGraph,
            entities_processed: 12,
            entities_total: 12,
            relationships_processed: 9,
            relationships_total: 9,
            entities_created: 12,
            entities_updated: 0,
            relationships_created: 9,
            relationships_updated: 0,
        };
        patch_document_graph_merge_progress(kv.clone(), doc_id, &progress).await;

        let meta = kv.get_by_id(&key).await.unwrap().unwrap();
        assert_eq!(meta["status"], "indexing");
        assert_eq!(meta["current_stage"], "storing");
        assert!(
            meta["stage_message"]
                .as_str()
                .unwrap()
                .contains("Merging relationships"),
            "expected merge progress message, got {:?}",
            meta["stage_message"]
        );
        assert_eq!(meta["stage_progress"].as_f64().unwrap(), 1.0);
    }
}
