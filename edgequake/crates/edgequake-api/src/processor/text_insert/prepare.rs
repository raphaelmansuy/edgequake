use super::super::*;
use super::types::TextInsertPrepared;

impl DocumentTaskProcessor {
    pub(super) async fn text_insert_prepare(
        &self,
        task: &mut Task,
        data: TextInsertData,
    ) -> TaskResult<TextInsertPrepared> {
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

        let text_content = crate::services::resolve_text_insert_content(
            &self.kv_storage,
            &document_id,
            &data.text,
        )
        .await
        .map_err(|e| TaskError::Process(format!("Text content resolution failed: {e}")))?;

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

            let metadata_key = crate::services::text_insert_content::resolve_document_metadata_key(
                &document_id,
                &self.kv_storage,
            )
            .await;
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
                        let payload = json!(updated);
                        let _ = crate::services::upsert_metadata_kv_with_index(
                            self.kv_storage.as_ref(),
                            &metadata_key,
                            payload,
                        )
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

        // OODA-16: Get workspace-specific pipeline with adaptive chunk + gleaning (SPEC-025)
        let enable_gleaning = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("enable_gleaning"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let max_gleaning = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("max_gleaning"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1);

        let page_count_for_profile = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("page_count"))
            .and_then(|v| v.as_i64())
            .map(|n| n.max(0) as usize)
            .unwrap_or(0);
        let large_profile = crate::services::LargeDocumentProfile::new(
            page_count_for_profile.max(1),
            text_content.len() as u64,
        );
        let (enable_gleaning, max_gleaning) = if large_profile.should_disable_gleaning() {
            (false, 0)
        } else {
            (enable_gleaning, max_gleaning)
        };

        let chunk_strategy = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("chunk_strategy"))
            .and_then(|v| v.as_str())
            .and_then(edgequake_pipeline::ChunkStrategy::parse)
            .unwrap_or_default();

        let chunk_options = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("chunk_options"))
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let ingestion_options =
            edgequake_pipeline::IngestionPipelineOptions::from_document_size(text_content.len())
                .with_gleaning(enable_gleaning, max_gleaning)
                .with_chunk_strategy(chunk_strategy);
        // SPEC-032 W-09: auto-select Pdf chunking strategy for PDF sources so
        // chunks never cross page boundaries. Only applies when no explicit
        // strategy override was provided by the caller.
        let source_is_pdf = source_type.eq_ignore_ascii_case("pdf")
            || source_type.eq_ignore_ascii_case("pdf_upload")
            || text_content.contains(edgequake_pipeline::PAGE_MARKER_PREFIX);
        let ingestion_options =
            if source_is_pdf && chunk_strategy == edgequake_pipeline::ChunkStrategy::default() {
                ingestion_options.for_pdf()
            } else {
                ingestion_options
            };
        let ingestion_options = if let Some(opts) = chunk_options {
            ingestion_options.with_chunk_options(opts)
        } else {
            ingestion_options
        };

        let pipeline = if self.strict_workspace_mode {
            match self
                .get_workspace_pipeline_for_ingestion(
                    workspace_id,
                    ingestion_options,
                    crate::workspace_pipeline_factory::PipelineFallbackPolicy::Strict,
                )
                .await
            {
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
            match self
                .get_workspace_pipeline_for_ingestion(
                    workspace_id,
                    ingestion_options,
                    crate::workspace_pipeline_factory::PipelineFallbackPolicy::LenientGlobal,
                )
                .await
            {
                Ok(p) => p,
                Err(_) => self.get_workspace_pipeline(workspace_id).await,
            }
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
            let estimated_chunks = std::cmp::max(1, text_content.len() / 2000);
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
                        let progress_pct =
                            ((chunk_idx as f64 / total as f64) * 100.0).round() as u32;
                        let _ = crate::services::patch_document_metadata(
                            &kv_clone,
                            &doc_id_clone,
                            |updated| {
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
                            },
                        )
                        .await;
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
                &text_content,
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
            let metadata_key = crate::services::text_insert_content::resolve_document_metadata_key(
                &document_id,
                &self.kv_storage,
            )
            .await;
            let doc_metadata = self
                .kv_storage
                .get_by_id(&metadata_key)
                .await
                .ok()
                .flatten();
            crate::services::enrich_processed_text_with_mm_chunks(
                self.kv_storage.as_ref(),
                &document_id,
                doc_metadata.as_ref(),
                preprocess_result.content,
            )
            .await
        };

        Ok(TextInsertPrepared {
            document_id,
            text_content,
            source_type,
            tenant_id,
            workspace_id: workspace_id.map(str::to_string),
            is_pdf_source,
            track_id,
            data,
            pipeline,
            provider_lineage,
            processed_text,
            chunk_progress_callback,
        })
    }
}
