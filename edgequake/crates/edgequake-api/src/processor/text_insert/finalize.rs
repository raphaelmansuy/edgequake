use super::super::*;
use super::types::TextInsertPersisted;
use tokio_util::sync::CancellationToken;

impl DocumentTaskProcessor {
    pub(super) async fn text_insert_finalize(
        &self,
        persisted: TextInsertPersisted,
        cancel_token: CancellationToken,
        processing_start: std::time::Instant,
    ) -> TaskResult<serde_json::Value> {
        let document_id = persisted.prepared.document_id.clone();
        let workspace_id_meta = persisted.workspace_id_meta.clone();
        let is_pdf_source = persisted.prepared.is_pdf_source;
        let track_id = persisted.prepared.track_id.clone();
        let result = persisted.result.clone();
        let final_status = persisted.final_status.clone();
        let stats_with_lineage = persisted.stats_with_lineage.clone();

        if !persisted.storage_errors.is_empty() {
            warn!(
                document_id = %document_id,
                storage_error_count = persisted.storage_errors.len(),
                errors = ?persisted.storage_errors,
                "Document indexed with non-fatal storage errors"
            );
        }

        // ── CANCELLATION GATE: before promote + terminal stats ──
        // WHY: Sync staging dismiss cancels the track then wipes staging. If we
        // promote first, dismiss can return deleted:true while a ghost final
        // row remains (SPEC-086 delete/promote race).
        // Also: must run before update_document_status_with_stats so a reprocess
        // race does not write "completed" stats then overwrite with "cancelled".
        self.check_cancelled(&cancel_token, "pre-promote", &document_id)
            .await?;

        // SPEC-026 P-11: promote staging KV → final keys on successful processing
        if let (Some(hash), Some(ws)) = (
            persisted
                .prepared
                .data
                .metadata
                .as_ref()
                .and_then(|m| m.get("content_hash"))
                .and_then(|v| v.as_str()),
            persisted.prepared.data.metadata.as_ref().and_then(|m| {
                m.get("workspace_id")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            }),
        ) {
            if let Err(e) =
                crate::services::promote_staging_to_final(&self.kv_storage, &document_id, &ws, hash)
                    .await
            {
                warn!(
                    document_id = %document_id,
                    error = %e,
                    "Failed to promote staging KV (non-fatal if legacy path)"
                );
            }
        }

        // FIX-ISSUE-81 / reliability: ensure the relational row exists BEFORE
        // update_document_status_with_stats → refresh_relational_document_stats,
        // otherwise the stats UPDATE affects 0 rows and relationship_count stays 0.
        #[cfg(feature = "postgres")]
        if let Some(ref pdf_storage) = self.pdf_storage {
            let text_content = persisted.prepared.text_content.clone();
            let tenant_id = persisted.prepared.tenant_id.clone();
            let data = persisted.prepared.data.clone();
            if let Ok(doc_uuid) = uuid::Uuid::parse_str(&document_id) {
                if let Ok(workspace_uuid) = uuid::Uuid::parse_str(&workspace_id_meta) {
                    let tenant_uuid = tenant_id
                        .as_ref()
                        .and_then(|t| uuid::Uuid::parse_str(t).ok());
                    let pg_status = if final_status == "completed" {
                        "indexed"
                    } else {
                        final_status.as_str()
                    };
                    let title = data
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("title"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(&data.file_source);
                    let content_summary: String = text_content.chars().take(500).collect();
                    if let Err(e) = pdf_storage
                        .ensure_document_record(
                            &doc_uuid,
                            &workspace_uuid,
                            tenant_uuid.as_ref(),
                            title,
                            &content_summary,
                            pg_status,
                        )
                        .await
                    {
                        warn!(
                            document_id = %document_id,
                            error = %e,
                            "FIX-ISSUE-81: Failed to dual-write document record to PostgreSQL (non-fatal)"
                        );
                    } else {
                        info!(
                            document_id = %document_id,
                            "FIX-ISSUE-81: Document record dual-written to PostgreSQL (async path)"
                        );
                    }
                }
            }
        }

        self.update_document_status_with_stats(&document_id, &final_status, &stats_with_lineage)
            .await?;

        // OODA-06: Persist DocumentLineage to KV storage for lineage API queries
        // WHY: Without persistence, lineage data only exists in memory during processing
        // and is lost. Lineage endpoints need to read it back from storage.
        if let Some(ref lineage) = result.lineage {
            let lineage_key = format!("{}-lineage", document_id);
            match serde_json::to_value(lineage) {
                Ok(lineage_json) => {
                    if let Err(e) = self
                        .kv_storage
                        .upsert(&[(lineage_key.clone(), lineage_json)])
                        .await
                    {
                        warn!(
                            document_id = %document_id,
                            error = %e,
                            "Failed to persist document lineage to KV storage"
                        );
                    } else {
                        info!(
                            document_id = %document_id,
                            chunks = lineage.total_chunks,
                            entities = lineage.entities.len(),
                            relationships = lineage.relationships.len(),
                            "Persisted document lineage to KV storage"
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        document_id = %document_id,
                        error = %e,
                        "Failed to serialize document lineage"
                    );
                }
            }
        }

        // OODA-17: Update PDF phase progress - graph storage complete, all phases done
        if is_pdf_source {
            self.pipeline_state
                .complete_pdf_phase(&track_id, PipelinePhase::GraphStorage)
                .await;
            info!(
                track_id = %track_id,
                document_id = %document_id,
                "PDF pipeline phases complete: all 6 phases finished"
            );
        }

        // OODA-ITERATION-03-FIX: Invalidate workspace stats cache after async document processing
        // WHY: The cache contains stale entity/relationship counts. Without this, Dashboard
        // shows 0 entities while Workspace page shows correct counts because both pages use
        // the same cached stats, but cache was populated before the document was processed.
        // This ensures the next stats request fetches fresh data.
        if let Ok(workspace_uuid) = uuid::Uuid::parse_str(&workspace_id_meta) {
            crate::handlers::workspaces::invalidate_workspace_stats_cache(workspace_uuid).await;
        }

        // CHECKPOINT-CLEAR + P7e snapshot: keep durable extractions for soft-reprocess.
        // WHY: Clearing crash checkpoint frees mid-flight KV; snapshot enables merge-only
        // without re-paying LLM extract (LightRAG-style reuse of stored tuples).
        if final_status == "completed" || final_status == "partial_failure" {
            let prepared = &persisted.prepared;
            if let Err(e) = super::pipeline_checkpoint::save_extraction_snapshot(
                &self.kv_storage,
                &document_id,
                &result,
                &prepared.data.workspace_id,
                &prepared.provider_lineage.extraction_provider,
                &prepared.provider_lineage.embedding_provider,
                &prepared.processed_text,
            )
            .await
            {
                warn!(
                    document_id = %document_id,
                    error = %e,
                    "P7e: failed to save extraction snapshot (non-fatal)"
                );
            }
            super::pipeline_checkpoint::clear_pipeline_checkpoint(&self.kv_storage, &document_id)
                .await;
        }

        // Log success
        self.pipeline_state
            .document_processed(&document_id, result.stats.entity_count)
            .await;

        if final_status == "failed" {
            warn!(
                document_id = %document_id,
                storage_error_count = persisted.storage_errors.len(),
                "Document finalize completed with failed status"
            );
        } else {
            info!(
                document_id = %document_id,
                chunk_count = result.stats.chunk_count,
                entity_count = result.stats.entity_count,
                relationship_count = result.stats.relationship_count,
                failed_chunks = result.stats.failed_chunks,
                "Document processed successfully"
            );
        }

        let outcome = if result.stats.failed_chunks > 0 {
            "partial"
        } else {
            "success"
        };
        let chunk_strategy = persisted
            .prepared
            .data
            .metadata
            .as_ref()
            .and_then(|m| m.get("chunk_strategy"))
            .and_then(|v| v.as_str())
            .or(result.stats.chunking_strategy.as_deref());
        let section_context_used = result.chunks.iter().any(|c| {
            c.section
                .as_ref()
                .is_some_and(|s| !s.heading_path.is_empty())
        });
        edgequake_observability::record_document_processing_with_labels(
            "text_insert",
            "pipeline",
            outcome,
            processing_start.elapsed().as_secs_f64(),
            chunk_strategy,
            section_context_used,
        );

        Ok(json!({
            "document_id": document_id,
            "chunk_count": result.stats.chunk_count,
            "entity_count": result.stats.entity_count,
            "relationship_count": result.stats.relationship_count,
        }))
    }
}
