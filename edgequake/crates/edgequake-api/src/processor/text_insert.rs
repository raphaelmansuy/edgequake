use super::*;
use tokio_util::sync::CancellationToken;

impl DocumentTaskProcessor {
    /// Check if the task has been cancelled and return early if so.
    ///
    /// WHY: This is called at every major stage boundary so that a cancel
    /// request interrupts processing within seconds rather than minutes.
    pub(crate) async fn check_cancelled(
        &self,
        cancel_token: &CancellationToken,
        stage: &str,
        document_id: &str,
    ) -> TaskResult<()> {
        if cancel_token.is_cancelled() {
            let msg = format!(
                "Task cancelled during '{}' stage for document {}",
                stage, document_id
            );
            tracing::info!(
                error.source = "task_processor",
                error.action = "cancelled",
                document_id = %document_id,
                stage = %stage,
                error.message = %msg,
                "Task cancelled"
            );
            self.update_document_status(document_id, "cancelled", Some(&msg))
                .await
                .ok(); // best-effort status update
            return Err(TaskError::Cancelled(msg));
        }
        Ok(())
    }

    /// Process a text insert task.
    pub(super) async fn process_text_insert(
        &self,
        task: &mut Task,
        data: TextInsertData,
        cancel_token: CancellationToken,
    ) -> TaskResult<serde_json::Value> {
        let processing_start = std::time::Instant::now();
        let document_id = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("document_id"))
            .and_then(|v| v.as_str())
            .unwrap_or(&data.file_source)
            .to_string();

        // SPEC-002: Extract source_type from task metadata for unified pipeline tracking
        let source_type = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("source_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("markdown") // Default to markdown for text uploads
            .to_string();

        // OODA-05: Extract tenant_id from metadata for multi-tenant visibility
        let tenant_id = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("tenant_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // OODA-49: Extract pdf_id from metadata for PDF document viewing
        // WHY: PDF documents need pdf_id stored in metadata for the frontend to build download URLs
        let pdf_id = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("pdf_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // SPEC-002: Ensure document metadata includes source_type
        // This is needed for PDFs that bypass the upload handler
        // OODA-05: Pass tenant_id/workspace_id for multi-tenant context
        // OODA-49: Pass pdf_id for PDF document viewing
        // OODA-ITERATION-03: Pass track_id for cancel button support
        self.ensure_document_source_type(
            &document_id,
            &source_type,
            tenant_id.as_deref(),
            Some(&data.workspace_id),
            pdf_id.as_deref(),
            Some(&task.track_id),
        )
        .await?;

        // OODA-04: Enrich document metadata with lineage fields from task metadata
        // WHY: file_size_bytes, sha256_checksum, document_type must be stored early
        // so lineage queries always return complete data regardless of processing stage.
        {
            let file_size_bytes = data
                .metadata
                .as_ref()
                .and_then(|m| m.get("file_size_bytes"))
                .cloned();
            let sha256_checksum = data
                .metadata
                .as_ref()
                .and_then(|m| m.get("sha256_checksum"))
                .cloned();
            let document_type = data
                .metadata
                .as_ref()
                .and_then(|m| m.get("document_type"))
                .cloned()
                .or_else(|| Some(json!(source_type)));

            let metadata_key = edgequake_storage::kv_keys::doc_metadata(&document_id);
            if let Ok(Some(existing)) = self.kv_storage.get_by_id(&metadata_key).await {
                if let Some(obj) = existing.as_object() {
                    let mut updated = obj.clone();
                    let mut changed = false;
                    if obj.get("file_size_bytes").is_none() {
                        if let Some(v) = file_size_bytes {
                            updated.insert("file_size_bytes".to_string(), v);
                            changed = true;
                        }
                    }
                    if obj.get("sha256_checksum").is_none() {
                        if let Some(v) = sha256_checksum {
                            updated.insert("sha256_checksum".to_string(), v);
                            changed = true;
                        }
                    }
                    if obj.get("document_type").is_none() {
                        if let Some(v) = document_type {
                            updated.insert("document_type".to_string(), v);
                            changed = true;
                        }
                    }
                    if changed {
                        updated.insert(
                            "updated_at".to_string(),
                            json!(chrono::Utc::now().to_rfc3339()),
                        );
                        let _ = self
                            .kv_storage
                            .upsert(&[(metadata_key, json!(updated))])
                            .await;
                    }
                }
            }
        }

        // SPEC-032: Extract workspace_id to use workspace-specific pipeline
        // Prefer the direct field (data.workspace_id), fallback to metadata if needed
        let workspace_id = if !data.workspace_id.is_empty() && data.workspace_id != "default" {
            Some(data.workspace_id.as_str())
        } else {
            data.metadata
                .as_ref()
                .and_then(|m| m.get("workspace_id"))
                .and_then(|v| v.as_str())
        };

        // OODA-16: Get workspace-specific pipeline with strict mode support
        // WHY: In strict mode, fail the task if workspace providers can't be created
        // instead of silently falling back to default (wrong dimensions, wrong provider)
        let pipeline = if self.strict_workspace_mode {
            match self.get_workspace_pipeline_strict(workspace_id).await {
                Ok(p) => p,
                Err(e) => {
                    error!(
                        document_id = %document_id,
                        workspace_id = ?workspace_id,
                        error = %e,
                        "OODA-16: Failed to create workspace pipeline in strict mode"
                    );
                    // Update document status to Failed with clear error message
                    let _ = self
                        .update_document_status(
                            &document_id,
                            "failed",
                            Some(&format!("Workspace provider error: {}", e)),
                        )
                        .await;
                    return Err(TaskError::Process(format!(
                        "Workspace pipeline error: {}",
                        e
                    )));
                }
            }
        } else {
            // Non-strict mode: fallback to default pipeline (legacy behavior)
            self.get_workspace_pipeline(workspace_id).await
        };

        // SPEC-032/OODA-198: Capture provider lineage for tracking
        let provider_lineage = self.get_workspace_provider_lineage(workspace_id).await;

        info!(
            document_id = %document_id,
            workspace_id = ?workspace_id,
            file_source = %data.file_source,
            extraction_provider = %provider_lineage.extraction_provider,
            extraction_model = %provider_lineage.extraction_model,
            embedding_provider = %provider_lineage.embedding_provider,
            "[PIPELINE] Processing document with workspace-specific pipeline"
        );

        // Update task progress - chunking
        task.update_progress("chunking".to_string(), 4, 10);

        // Log to pipeline state
        self.pipeline_state
            .info(format!("Chunking document {}...", document_id))
            .await;

        // OODA-02: Update document status to "chunking" for frontend visibility
        // WHY: Users need to see exactly which processing stage their document is in
        self.update_document_status(&document_id, "chunking", None)
            .await?;

        // OODA-17: Update PDF phase progress for PDF uploads
        // WHY: PDFs need all 6 phases tracked (Upload, PdfConversion, Chunking, Embedding, Extraction, GraphStorage)
        // The PdfConversion phase is tracked by PipelineProgressCallback, but remaining phases need explicit tracking
        let is_pdf_source = source_type == "pdf";
        let track_id = task.track_id.clone();
        if is_pdf_source {
            // Estimate: text length / 2000 chars per chunk (rough heuristic)
            let estimated_chunks = std::cmp::max(1, data.text.len() / 2000);
            self.pipeline_state
                .start_pdf_phase(&track_id, PipelinePhase::Chunking, estimated_chunks)
                .await;
        }

        // SPEC-001/Objective-A: Create chunk progress callback for real-time updates
        // WHY: Users need to see granular progress like "Chunk 12/35 (34%) - ETA: 53s"
        // OODA-PERF-01: Enhanced to update document metadata for UI polling fallback
        // WHY: If WebSocket fails, users still see extraction progress via metadata polling
        let task_id = task.track_id.clone();
        let doc_id_for_callback = document_id.clone();
        let doc_id_for_metadata = document_id.clone();
        let pipeline_state_for_callback = self.pipeline_state.clone();
        let kv_storage_for_callback = Arc::clone(&self.kv_storage);
        let chunk_progress_callback: ChunkProgressCallback =
            Arc::new(move |update: ChunkProgressUpdate| {
                // Emit real-time WebSocket event for chunk progress
                pipeline_state_for_callback.emit_chunk_progress(
                    doc_id_for_callback.clone(),
                    task_id.clone(),
                    update.chunk_index as u32,
                    update.total_chunks as u32,
                    update.chunk_preview.clone(),
                    update.processing_time_ms,
                    update.eta_seconds,
                    update.cumulative_input_tokens,
                    update.cumulative_output_tokens,
                    update.cumulative_cost_usd,
                );

                // OODA-PERF-01: Update document metadata every 3 chunks for UI polling
                // WHY: Reduce KV writes while maintaining visibility (update ~every 3-5 seconds)
                let should_update_metadata = update.chunk_index.is_multiple_of(3)
                    || update.chunk_index == update.total_chunks - 1;
                if should_update_metadata {
                    let doc_id_clone = doc_id_for_metadata.clone();
                    let kv_clone = Arc::clone(&kv_storage_for_callback);
                    let chunk_idx = update.chunk_index;
                    let total = update.total_chunks;

                    // Fire-and-forget metadata update to avoid blocking extraction
                    tokio::spawn(async move {
                        let metadata_key = edgequake_storage::kv_keys::doc_metadata(&doc_id_clone);
                        if let Ok(Some(existing)) = kv_clone.get_by_id(&metadata_key).await {
                            if let Some(obj) = existing.as_object() {
                                let mut updated = obj.clone();
                                let progress_pct =
                                    ((chunk_idx as f64 / total as f64) * 100.0).round() as u32;
                                updated.insert("current_stage".to_string(), json!("extracting"));
                                updated.insert(
                                    "stage_message".to_string(),
                                    json!(format!(
                                        "Extracting entities: chunk {}/{} ({}%)",
                                        chunk_idx + 1,
                                        total,
                                        progress_pct
                                    )),
                                );
                                updated.insert(
                                    "stage_progress".to_string(),
                                    json!(progress_pct as f64 / 100.0),
                                );
                                updated.insert(
                                    "updated_at".to_string(),
                                    json!(chrono::Utc::now().to_rfc3339()),
                                );

                                let _ = kv_clone.upsert(&[(metadata_key, json!(updated))]).await;
                            }
                        }
                    });
                }
            });

        // SPEC-003: Process through pipeline with RESILIENT chunk-level extraction
        // WHY: Uses map-reduce pattern to continue processing even if some chunks fail
        // This enables partial results instead of complete document failure
        // @implements FEAT0022: Chunk-level resilience and error isolation (processor)
        // @implements UC2305: System continues processing when individual chunks fail

        // FIX-EXCEL-CHUNKING: Preprocess tabular content before pipeline processing
        // WHY: Large markdown tables (e.g. Excel exports) create 100+ chunks that split
        // mid-row without headers, leading to poor entity extraction and high LLM costs.
        // The preprocessor groups rows by category and adds headers per section for better chunking.
        let processed_text = {
            let preprocess_result = edgequake_pipeline::preprocess_tabular_content(
                &data.text,
                &edgequake_pipeline::TablePreprocessorConfig::default(),
            );
            if preprocess_result.was_restructured {
                info!(
                    document_id = %document_id,
                    table_rows = preprocess_result.table_rows,
                    groups = preprocess_result.groups,
                    duplicates_removed = preprocess_result.duplicates_removed,
                    "[TABLE-PREPROCESS] Restructured tabular content into {} groups ({} dupes removed)",
                    preprocess_result.groups,
                    preprocess_result.duplicates_removed,
                );
            }
            preprocess_result.content
        };

        // CHECKPOINT: Try to load a saved pipeline checkpoint before running
        // expensive LLM extraction. This saves minutes of processing when
        // a server crashed after extraction but before storage completed.

        // ── CANCELLATION GATE: before LLM extraction (most expensive stage) ──
        self.check_cancelled(&cancel_token, "pre-extraction", &document_id)
            .await?;

        // PDF re-conversion (Full mode): clear any saved KG pipeline checkpoint
        // so entity extraction re-runs against the freshly converted markdown.
        // WHY: Even though a re-converted PDF usually produces a different
        // content hash (so the checkpoint would not match anyway), clearing it
        // explicitly guarantees no stale extraction results are reused when the
        // user explicitly asked for a full re-conversion.
        let force_fresh_extraction = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("force_fresh_extraction"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if force_fresh_extraction {
            info!(
                document_id = %document_id,
                "Fresh extraction requested — clearing KG pipeline checkpoint"
            );
            super::pipeline_checkpoint::clear_pipeline_checkpoint(&self.kv_storage, &document_id)
                .await;
        }

        let checkpoint_result = super::pipeline_checkpoint::load_pipeline_checkpoint(
            &self.kv_storage,
            &document_id,
            &data.workspace_id,
            &provider_lineage.extraction_provider,
            &provider_lineage.embedding_provider,
            &processed_text,
        )
        .await;

        let (result, resumed_from_checkpoint) = if let Some(checkpointed) = checkpoint_result {
            info!(
                document_id = %document_id,
                chunks = checkpointed.chunks.len(),
                entities = checkpointed.stats.entity_count,
                "CHECKPOINT-RESUME: Skipping LLM extraction — loaded from checkpoint"
            );
            (checkpointed, true)
        } else {
            // OODA-PERF-02: Build embed progress callback for KV metadata updates.
            // WHY: generate_all_embeddings runs AFTER chunk 141/141 extraction completes
            // (status = "99%"). For a large document it embeds thousands of entity /
            // relationship texts via the API — with zero UI feedback. This fire-and-forget
            // callback updates document metadata so the UI shows progress instead of
            // freezing at "99%" for minutes.
            let doc_id_for_embed = document_id.clone();
            let kv_for_embed = Arc::clone(&self.kv_storage);
            let embed_progress_callback: EmbedProgressCallback =
                Arc::new(move |update: EmbedProgressUpdate| {
                    let doc_id_clone = doc_id_for_embed.clone();
                    let kv_clone = Arc::clone(&kv_for_embed);
                    let stage = update.stage;
                    let current = update.current;
                    let total = update.total;

                    // Fire-and-forget metadata update — same pattern as chunk callback
                    tokio::spawn(async move {
                        let metadata_key = edgequake_storage::kv_keys::doc_metadata(&doc_id_clone);
                        if let Ok(Some(existing)) = kv_clone.get_by_id(&metadata_key).await {
                            if let Some(obj) = existing.as_object() {
                                let mut updated = obj.clone();
                                let pct = if total == 0 {
                                    100u32
                                } else {
                                    ((current as f64 / total as f64) * 100.0).round() as u32
                                };
                                let label = match stage {
                                    "chunks" => "chunk",
                                    "entities" => "entit",
                                    "relationships" => "relationship",
                                    other => other,
                                };
                                let msg = if current == 0 {
                                    format!("Embedding {}ies: starting ({} total)", label, total)
                                } else {
                                    format!(
                                        "Embedding {}ies: {}/{} ({}%)",
                                        label, current, total, pct
                                    )
                                };
                                updated.insert("current_stage".to_string(), json!("embedding"));
                                updated.insert("stage_message".to_string(), json!(msg));
                                updated.insert(
                                    "stage_progress".to_string(),
                                    json!(0.99 + (0.01 * pct as f64 / 100.0)),
                                );
                                updated.insert(
                                    "updated_at".to_string(),
                                    json!(chrono::Utc::now().to_rfc3339()),
                                );
                                let _ = kv_clone.upsert(&[(metadata_key, json!(updated))]).await;
                            }
                        }
                    });
                });

            // No valid checkpoint — run the full pipeline
            let fresh_result = match pipeline
                .process_with_resilience_cancellable(
                    &document_id,
                    &processed_text,
                    Some(chunk_progress_callback),
                    Some(cancel_token.clone()),
                    Some(embed_progress_callback),
                )
                .await
            {
                Ok(result) => {
                    // SPEC-003: Log partial success if some chunks failed
                    if result.stats.failed_chunks > 0 {
                        warn!(
                            document_id = %document_id,
                            successful_chunks = result.stats.successful_chunks,
                            failed_chunks = result.stats.failed_chunks,
                            total_chunks = result.stats.chunk_count,
                            error.code = "EXTRACTION_PARTIAL_FAILURE",
                            error.source = "pipeline",
                            "Document processed with partial success - some chunks failed extraction"
                        );
                        edgequake_observability::ErrorEvent::log_domain_warn(
                            "pipeline",
                            "partial_extraction",
                            "Some chunks failed extraction",
                            serde_json::json!({
                                "document_id": document_id,
                                "successful_chunks": result.stats.successful_chunks,
                                "failed_chunks": result.stats.failed_chunks,
                                "total_chunks": result.stats.chunk_count,
                            }),
                        );

                        // Emit WebSocket events for failed chunks
                        if let Some(ref chunk_errors) = result.stats.chunk_errors {
                            for error_info in chunk_errors {
                                self.pipeline_state.emit_chunk_failure(
                                    document_id.clone(),
                                    task.track_id.clone(),
                                    error_info.chunk_index as u32,
                                    result.stats.chunk_count as u32,
                                    error_info.error_message.clone(),
                                    error_info.was_timeout,
                                    error_info.retry_attempts,
                                );
                            }
                        }
                    }
                    result
                }
                Err(e) => {
                    // FIX-3: Comprehensive error logging with context
                    let error_msg = format!("Pipeline processing failed: {}", e);
                    error!(
                        document_id = %document_id,
                        workspace_id = ?workspace_id,
                        tenant_id = ?tenant_id,
                        content_length = data.text.len(),
                        error = %e,
                        error.source = "pipeline",
                        error.code = "PIPELINE_PROCESSING_FAILED",
                        "CRITICAL: Pipeline processing failed - document marked as failed"
                    );
                    edgequake_observability::record_document_processing(
                        "text_insert",
                        "pipeline",
                        "failure",
                        0.0,
                    );

                    // Update document status to failed with detailed error
                    self.update_document_status(&document_id, "failed", Some(&error_msg))
                        .await?;

                    self.pipeline_state
                        .document_failed(&document_id, &error_msg)
                        .await;

                    return Err(edgequake_tasks::TaskError::Process(error_msg));
                }
            };

            // CHECKPOINT-SAVE: Persist pipeline results so a crash during
            // storage won't force re-running the expensive LLM extraction.
            if let Err(e) = super::pipeline_checkpoint::save_pipeline_checkpoint(
                &self.kv_storage,
                &document_id,
                &fresh_result,
                &data.workspace_id,
                &provider_lineage.extraction_provider,
                &provider_lineage.embedding_provider,
                &processed_text,
            )
            .await
            {
                warn!(
                    document_id = %document_id,
                    error = %e,
                    "Failed to save pipeline checkpoint — processing continues without checkpoint"
                );
            }

            (fresh_result, false)
        };

        // Log checkpoint usage metrics
        if resumed_from_checkpoint {
            info!(
                document_id = %document_id,
                "CHECKPOINT-STATS: Resumed from checkpoint — saved LLM extraction time"
            );
        }

        // Update task progress - embedding
        task.update_progress("embedding".to_string(), 4, 30);

        // ── CANCELLATION GATE: after extraction, before embedding storage ──
        self.check_cancelled(&cancel_token, "post-extraction", &document_id)
            .await?;

        self.pipeline_state
            .info(format!(
                "Generated {} chunks for {}",
                result.chunks.len(),
                document_id
            ))
            .await;

        // OODA-02: Update status to "extracting" - LLM entity extraction in progress
        // WHY: This is often the longest stage, users need visibility
        self.update_document_status(&document_id, "extracting", None)
            .await?;

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

        // Store chunks in KV storage
        // OODA-05: Include position metadata and token count for lineage traceability
        // WHY: Each chunk must carry its exact position in the source document so that
        // lineage queries can map entity → chunk → source location without extra lookups.
        let chunks: Vec<(String, serde_json::Value)> = result
            .chunks
            .iter()
            .map(|c| {
                (
                    c.id.clone(),
                    json!({
                        "content": c.content,
                        "document_id": document_id,
                        "index": c.index,
                        "start_line": c.start_line,
                        "end_line": c.end_line,
                        "start_offset": c.start_offset,
                        "end_offset": c.end_offset,
                        "token_count": c.token_count,
                    }),
                )
            })
            .collect();

        if let Err(e) = self.kv_storage.upsert(&chunks).await {
            let error_msg = format!("Failed to store chunks: {}", e);
            edgequake_observability::ErrorEvent::log_domain_error(
                "task_processor",
                "store_chunks",
                &error_msg,
                json!({ "document_id": document_id, "chunk_count": chunks.len() }),
            );

            self.update_document_status(&document_id, "failed", Some(&error_msg))
                .await?;

            return Err(edgequake_tasks::TaskError::Storage(error_msg));
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

        // SPEC-021 P-G2: single persist path — chunk vectors + KnowledgeGraphMerger
        // (replaces manual upsert_nodes_batch / entity-vector / upsert_edges_batch).
        let mut storage_errors: Vec<String> = Vec::new();
        let persist_ctx = IngestionPersistContext::new(
            document_id.clone(),
            tenant_id.clone(),
            Some(workspace_id_meta.clone()),
        );

        let persister = DefaultIngestionPersister::from_settings(
            self.graph_storage.clone(),
            workspace_vector_storage.clone(),
            IngestionPersistSettings::default(),
            self.relational_sink.clone(),
            Some(self.llm_provider.clone()),
        );

        let chunk_embeddings_stored = match persister
            .persist(&persist_ctx, &result, ChunkVectorBuildOptions::STANDARD)
            .await
        {
            Ok(out) => {
                if let Some(invalidator) = &self.query_cache_invalidator {
                    invalidator.invalidate_query_result_cache();
                }
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
                content_length = data.text.len(),
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

        // Update document status with validation
        self.update_document_status_with_stats(&document_id, final_status, &stats_with_lineage)
            .await?;

        // FIX-ISSUE-81 Phase 2: Dual-write document record to PostgreSQL (async path)
        // WHY: Without this, async text/markdown uploads only write to KV storage.
        // The PostgreSQL `documents` table stays incomplete, causing Dashboard KPI mismatch.
        #[cfg(feature = "postgres")]
        if let Some(ref pdf_storage) = self.pdf_storage {
            if let Ok(doc_uuid) = uuid::Uuid::parse_str(&document_id) {
                if let Ok(workspace_uuid) = uuid::Uuid::parse_str(&workspace_id_meta) {
                    let tenant_uuid = tenant_id
                        .as_ref()
                        .and_then(|t| uuid::Uuid::parse_str(t).ok());
                    let pg_status = if final_status == "completed" {
                        "indexed"
                    } else {
                        final_status
                    };
                    let title = data
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("title"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(&data.file_source);
                    // Truncate content for summary field (first 500 chars)
                    let content_summary: String = data.text.chars().take(500).collect();
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

        // OODA-06: Persist DocumentLineage to KV storage for lineage API queries

        // ── CANCELLATION GATE: before lineage persistence ──
        self.check_cancelled(&cancel_token, "pre-lineage", &document_id)
            .await?;

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
        if let Some(workspace_id_str) = workspace_id {
            if let Ok(workspace_uuid) = uuid::Uuid::parse_str(workspace_id_str) {
                crate::handlers::workspaces::invalidate_workspace_stats_cache(workspace_uuid).await;
            }
        }

        // CHECKPOINT-CLEAR: All storage stages completed successfully.
        // Remove the checkpoint so it won't be reloaded on next run.
        // WHY: If we reach here, every piece of data is safely persisted.
        // Keeping the checkpoint would waste storage and risk stale reloads.
        super::pipeline_checkpoint::clear_pipeline_checkpoint(&self.kv_storage, &document_id).await;

        // Log success
        self.pipeline_state
            .document_processed(&document_id, result.stats.entity_count)
            .await;

        info!(
            document_id = %document_id,
            chunk_count = result.stats.chunk_count,
            entity_count = result.stats.entity_count,
            relationship_count = result.stats.relationship_count,
            failed_chunks = result.stats.failed_chunks,
            "Document processed successfully"
        );

        let outcome = if result.stats.failed_chunks > 0 {
            "partial"
        } else {
            "success"
        };
        edgequake_observability::record_document_processing(
            "text_insert",
            "pipeline",
            outcome,
            processing_start.elapsed().as_secs_f64(),
        );

        Ok(json!({
            "document_id": document_id,
            "chunk_count": result.stats.chunk_count,
            "entity_count": result.stats.entity_count,
            "relationship_count": result.stats.relationship_count,
        }))
    }
}
