use super::super::*;
use super::types::{TextInsertExtracted, TextInsertPersisted};
use tokio_util::sync::CancellationToken;

impl DocumentTaskProcessor {
    pub(super) async fn text_insert_persist(
        &self,
        task: &mut Task,
        extracted: TextInsertExtracted,
        cancel_token: CancellationToken,
    ) -> TaskResult<TextInsertPersisted> {
        let document_id = extracted.prepared.document_id.clone();
        let text_content = extracted.prepared.text_content.clone();
        let data = extracted.prepared.data.clone();
        let provider_lineage = extracted.prepared.provider_lineage.clone();
        let result = extracted.result.clone();
        let prepared = extracted.prepared;
        let is_pdf_source = prepared.is_pdf_source;
        let track_id = prepared.track_id.clone();

        // OODA-17: Update PDF phase progress - chunking complete, start extraction
        if is_pdf_source {
            self.pipeline_state
                .complete_pdf_phase(&track_id, PipelinePhase::Chunking)
                .await;
            // Extraction phase: estimate entity count from chunk count
            let estimated_entities = result.chunks.len() * 3; // ~3 entities per chunk heuristic
            self.pipeline_state
                .start_pdf_phase(&track_id, PipelinePhase::Extraction, estimated_entities)
                .await;
        }

        // Extract tenant_id and workspace_id from metadata for scoping
        let tenant_id = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("tenant_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let workspace_id_meta = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("workspace_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| data.workspace_id.clone());

        // Get workspace-specific vector storage using the registry
        // WHY: Different workspaces may have different embedding dimensions
        // WHY-OODA223: STRICT mode - fail loudly if workspace storage unavailable
        // to prevent embeddings from being stored in the wrong (global) table
        let workspace_vector_storage = self
            .get_workspace_vector_storage_strict(&workspace_id_meta)
            .await
            .map_err(|e| {
                let error_msg = format!(
                    "CRITICAL: Cannot obtain workspace vector storage for '{}': {}. \
                         Document ingestion aborted to prevent data isolation violation.",
                    workspace_id_meta, e
                );
                edgequake_observability::ErrorEvent::log_domain_error(
                    "task_processor",
                    "workspace_vector_storage",
                    &error_msg,
                    json!({
                        "document_id": document_id,
                        "workspace_id": workspace_id_meta,
                    }),
                );
                edgequake_tasks::TaskError::Process(error_msg)
            })?;

        // OODA-02: Update status to "embedding" - generating vector embeddings
        // WHY: Shows user that extraction is complete, now vectorizing
        self.update_document_status(&document_id, "embedding", None)
            .await?;

        // OODA-17: Update PDF phase progress - extraction complete, start embedding
        if is_pdf_source {
            self.pipeline_state
                .complete_pdf_phase(&track_id, PipelinePhase::Extraction)
                .await;
            // Embedding phase: total = chunks to embed
            self.pipeline_state
                .start_pdf_phase(&track_id, PipelinePhase::Embedding, result.chunks.len())
                .await;
        }

        // Update task progress - extraction (chunk vectors deferred to P-G2 persist below)
        task.update_progress("extraction".to_string(), 4, 60);

        // ── CANCELLATION GATE: before graph storage (heavy DB writes) ──
        self.check_cancelled(&cancel_token, "pre-graph-storage", &document_id)
            .await?;

        self.pipeline_state
            .info(format!("Extracting entities from {}...", document_id))
            .await;

        info!(
            "Storing entities with tenant_id={:?}, workspace_id={:?}",
            tenant_id, workspace_id_meta
        );

        // OODA-02: Update status to "indexing" - storing in graph and vector databases
        // WHY: Final stage before completion, indicates DB writes in progress
        self.update_document_status(&document_id, "indexing", None)
            .await?;

        // OODA-17: Update PDF phase progress - embedding complete, start graph storage
        if is_pdf_source {
            self.pipeline_state
                .complete_pdf_phase(&track_id, PipelinePhase::Embedding)
                .await;
            // GraphStorage phase: estimate operations = entities + relationships
            let total_entities: usize = result.extractions.iter().map(|e| e.entities.len()).sum();
            let total_rels: usize = result
                .extractions
                .iter()
                .map(|e| e.relationships.len())
                .sum();
            self.pipeline_state
                .start_pdf_phase(
                    &track_id,
                    PipelinePhase::GraphStorage,
                    total_entities + total_rels,
                )
                .await;
        }

        // SPEC-021 P-G2: single persist path — KV chunks + chunk vectors + KnowledgeGraphMerger
        let mut storage_errors: Vec<String> = Vec::new();

        // Merger LLM summarization must use the same extraction provider as the pipeline
        // (SPEC-032). Using the global default (often Ollama) caused 1000+ WARN spam when
        // workspace extraction runs on Mistral/OpenAI while Ollama is unreachable.
        let persist_llm = match crate::safety_limits::create_safe_llm_provider(
            &provider_lineage.extraction_provider,
            &provider_lineage.extraction_model,
        ) {
            Ok(provider) => provider,
            Err(e) => {
                warn!(
                    document_id = %document_id,
                    extraction_provider = %provider_lineage.extraction_provider,
                    extraction_model = %provider_lineage.extraction_model,
                    error = %e,
                    "Workspace extraction LLM unavailable for merge summarizer — falling back to processor default"
                );
                self.llm_provider.clone()
            }
        };

        // SPEC-032 W-04: Build merge progress callback that broadcasts
        // GraphStorageProgress events via PipelineState → WebSocket.
        // Only wired when is_pdf_source (has a track_id for progress tracking).
        let merge_progress_cb: Option<edgequake_pipeline::MergeProgressCallback> = if is_pdf_source
        {
            let pipeline_state = self.pipeline_state.clone();
            let track_id_cb = track_id.clone();
            let doc_id_cb = document_id.clone();
            Some(Box::new(move |p: edgequake_pipeline::MergeProgress| {
                let pipeline_state = pipeline_state.clone();
                let track_id = track_id_cb.clone();
                let doc_id = doc_id_cb.clone();
                // Fire-and-forget: spawn async task to broadcast without blocking merge loop
                tokio::spawn(async move {
                    pipeline_state
                        .broadcast_graph_storage_progress(
                            &track_id,
                            &doc_id,
                            p.phase.label_id(),
                            p.phase.label(),
                            p.entities_processed as u32,
                            p.entities_total as u32,
                            p.entities_created as u32,
                            p.entities_updated as u32,
                            p.relationships_processed as u32,
                            p.relationships_total as u32,
                            p.relationships_created as u32,
                            p.relationships_updated as u32,
                        )
                        .await;
                });
            }))
        } else {
            None
        };

        let chunk_embeddings_stored = match crate::services::persist_with_providers_and_progress(
            persist_llm,
            self.query_cache_invalidator
                .as_ref()
                .map(|e| e.as_ref() as &dyn edgequake_query::QueryResultCacheInvalidator),
            self.graph_storage.clone(),
            workspace_vector_storage.clone(),
            self.kv_storage.clone(),
            self.relational_sink.clone(),
            // SPEC-032 W-08: lineage sink — resolved from shared AppState pg_pool
            self.resolve_lineage_sink().await,
            crate::services::PersistIngestionParams::for_document(
                &document_id,
                tenant_id.clone(),
                workspace_id_meta.clone(),
                &result,
                ChunkVectorBuildOptions::STANDARD,
                Some(&data.file_source),
            ),
            merge_progress_cb,
        )
        .await
        {
            Ok(out) => {
                info!(
                    document_id = %document_id,
                    chunk_vectors = out.chunk_vector_ids.len(),
                    entities = out.merge_stats.entities_created + out.merge_stats.entities_updated,
                    relationships = out.merge_stats.relationships_created
                        + out.merge_stats.relationships_updated,
                    "P-G2 persist completed"
                );
                out.chunk_vector_ids.len()
            }
            Err(e) => {
                let err_msg = format!("Knowledge graph persist failed: {}", e);
                error!(document_id = %document_id, "{}", err_msg);
                storage_errors.push(err_msg);
                0
            }
        };
        info!(
            "Stored {} chunk embeddings in vector storage for document {}",
            chunk_embeddings_stored, document_id
        );

        // Update task progress - indexing complete
        task.update_progress("indexing".to_string(), 4, 100);

        // SPEC-032/OODA-198: Augment stats with provider lineage before storing
        let mut stats_with_lineage = result.stats.clone();
        stats_with_lineage.llm_provider = Some(provider_lineage.extraction_provider.clone());
        stats_with_lineage.llm_model = Some(provider_lineage.extraction_model.clone());
        stats_with_lineage.embedding_provider = Some(provider_lineage.embedding_provider.clone());
        stats_with_lineage.embedding_model = Some(provider_lineage.embedding_model.clone());
        stats_with_lineage.embedding_dimensions = Some(provider_lineage.embedding_dimension);

        // FIX-1: Validate processing results before marking completed
        // WHY: Prevent silent failures where status="completed" but entity_count=0
        // CRITICAL: This detects documents that went through pipeline but extracted nothing
        //
        // FIX-2: Also check storage_errors to catch graph/vector storage failures.
        // WHY: Previously, upsert_nodes_batch / upsert_edges_batch / entity embedding
        // failures were warn-and-continue, so document would show "completed"
        // but entities/relationships were actually missing from storage.
        let has_storage_errors = !storage_errors.is_empty();

        let final_status = if result.stats.failed_chunks == result.stats.chunk_count
            && result.stats.chunk_count > 0
        {
            // ALL chunks failed extraction - complete failure
            error!(
                document_id = %document_id,
                chunk_count = result.stats.chunk_count,
                "CRITICAL: ALL {} chunks failed entity extraction - marking as failed",
                result.stats.chunk_count
            );
            "failed"
        } else if result.stats.chunk_count == 0 {
            // No chunks created at all - chunking failed
            error!(
                document_id = %document_id,
                content_length = text_content.len(),
                "CRITICAL: Document chunking produced 0 chunks - marking as failed"
            );
            "failed"
        } else if result.stats.entity_count == 0 && result.stats.chunk_count > 0 {
            // Pipeline created chunks but extracted 0 entities - likely LLM failure
            warn!(
                document_id = %document_id,
                chunk_count = result.stats.chunk_count,
                failed_chunks = result.stats.failed_chunks,
                "ANOMALY: Document processed but extracted 0 entities from {} chunks - marking as partial_failure",
                result.stats.chunk_count
            );
            "partial_failure"
        } else if has_storage_errors {
            // Persist path failed after compensation — hard failure, not partial success.
            let combined = storage_errors.join("; ");
            error!(
                document_id = %document_id,
                storage_error_count = storage_errors.len(),
                "Knowledge graph persist failed — marking as failed: {}",
                combined
            );
            stats_with_lineage.error_details = Some(combined);
            "failed"
        } else {
            "completed"
        };

        Ok(TextInsertPersisted {
            prepared,
            result,
            workspace_id_meta,
            storage_errors,
            stats_with_lineage,
            final_status: final_status.to_string(),
        })
    }
}
