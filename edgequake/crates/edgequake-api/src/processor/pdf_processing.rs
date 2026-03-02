use super::*;
use tokio_util::sync::CancellationToken;

impl DocumentTaskProcessor {
    /// Process PDF processing task (SPEC-007).
    ///
    /// This method handles the complete PDF processing pipeline:
    /// 1. Load PDF from storage using pdf_id
    /// 2. Extract content (text mode only for now, vision TODO)
    /// 3. Convert to markdown
    /// 4. Create document and trigger standard ingestion
    /// 5. Update PDF status with results
    ///
    /// @implements SPEC-007: PDF Upload Support with Vision LLM Integration
    /// @implements FEAT0704: PDF processing worker
    /// @implements UC0704: System processes PDF in background
    /// @enforces BR0704: PDF processed async with retry logic
    #[cfg(feature = "postgres")]
    pub(super) async fn process_pdf_processing(
        &self,
        task: &mut Task,
        data: edgequake_tasks::PdfProcessingData,
        cancel_token: CancellationToken,
    ) -> TaskResult<serde_json::Value> {
        use edgequake_storage::{
            ExtractionMethod, PdfProcessingStatus, UpdatePdfProcessingRequest,
        };

        info!(
            pdf_id = %data.pdf_id,
            workspace_id = %data.workspace_id,
            enable_vision = data.enable_vision,
            "Starting PDF processing task"
        );

        // 1. Get PDF storage
        let pdf_storage = self.pdf_storage.as_ref().ok_or_else(|| {
            edgequake_tasks::TaskError::UnsupportedOperation(
                "PDF storage not available (postgres feature enabled but storage not initialized)"
                    .to_string(),
            )
        })?;

        // 2. Load PDF from storage
        let pdf = pdf_storage.get_pdf(&data.pdf_id).await.map_err(|e| {
            edgequake_tasks::TaskError::Storage(format!(
                "Failed to load PDF {}: {}",
                data.pdf_id, e
            ))
        })?;

        // Handle case where PDF not found
        let pdf = pdf.ok_or_else(|| {
            edgequake_tasks::TaskError::NotFound(format!("PDF not found: {}", data.pdf_id))
        })?;

        info!(
            pdf_id = %data.pdf_id,
            filename = %pdf.filename,
            size = pdf.file_size_bytes,
            pages = ?pdf.page_count,
            "Loaded PDF from storage"
        );

        // 3. Update status to processing
        pdf_storage
            .update_pdf_status(&data.pdf_id, PdfProcessingStatus::Processing)
            .await
            .map_err(|e| edgequake_tasks::TaskError::Storage(e.to_string()))?;

        // == Progress: loading complete, preparing for conversion ==
        task.update_progress("pdf_loading".to_string(), 1, 5);

        // 3.1 Create document metadata early with "converting" stage
        // WHY: Users need to see the document appear in the UI immediately with visual feedback
        // showing that PDF → Markdown conversion is happening.
        // OODA-ITERATION-03: Include track_id for cancel button support
        // WHY: Frontend cancel button requires doc.track_id to call POST /tasks/{track_id}/cancel
        // FIX-REBUILD: When rebuilding/reprocessing, reuse the existing document ID
        // to avoid creating orphaned duplicates. Without this, the old document still
        // references the same pdf_id whose markdown_content gets overwritten, causing
        // it to display wrong/hallucinated content from the new extraction.
        let is_reprocess = data.existing_document_id.is_some();
        let early_doc_id = data
            .existing_document_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // FIX-DUPLICATE-BUG: Persist the generated document ID back into task_data
        // so that worker retries reuse the same document ID instead of creating
        // a new UUID on each attempt. Without this, a single PDF upload that fails
        // and gets retried by the worker pool creates duplicate documents with
        // different IDs, each stuck in "processing" state.
        if !is_reprocess {
            if let Ok(mut task_data_map) = serde_json::from_value::<
                serde_json::Map<String, serde_json::Value>,
            >(task.task_data.clone())
            {
                task_data_map.insert(
                    "existing_document_id".to_string(),
                    serde_json::json!(early_doc_id.clone()),
                );
                task.task_data = serde_json::Value::Object(task_data_map);
            }
        }
        let metadata_key = format!("{}-metadata", early_doc_id);
        // OODA-04: Include file_size_bytes and sha256_checksum in early metadata
        // WHY: Enables complete lineage from the moment the document appears in UI.
        // Without these, users see metadata gaps until processing completes.
        let metadata_json = json!({
            "id": early_doc_id,
            "title": pdf.filename.clone(),
            "file_name": pdf.filename.clone(),
            "source_type": "pdf",
            "document_type": "pdf",
            "status": "processing",
            "current_stage": "converting",
            "stage_message": match pdf.page_count {
                Some(n) if n > 0 => format!("Converting PDF to Markdown (0/{} pages)", n),
                _ => "Converting PDF to Markdown (detecting pages...)".to_string(),
            },
            "stage_progress": 0.0,
            "pdf_id": data.pdf_id.to_string(),
            "file_size_bytes": pdf.file_size_bytes,
            "sha256_checksum": pdf.sha256_checksum,
            "page_count": pdf.page_count,
            "tenant_id": data.tenant_id.to_string(),
            "workspace_id": data.workspace_id.to_string(),
            "track_id": task.track_id.clone(),
            "created_at": chrono::Utc::now().to_rfc3339(),
            "updated_at": chrono::Utc::now().to_rfc3339(),
        });

        self.kv_storage
            .upsert(&[(metadata_key.clone(), metadata_json.clone())])
            .await
            .map_err(|e| edgequake_tasks::TaskError::Storage(e.to_string()))?;

        // FIX-REBUILD: When reprocessing, clean up old content and chunk KV entries
        // WHY: Old chunks with stale content must be removed before the pipeline
        // creates new ones, otherwise the document ends up with a mix of old and new chunks.
        if is_reprocess {
            info!(
                document_id = %early_doc_id,
                pdf_id = %data.pdf_id,
                "Reprocessing: cleaning up old content and chunks before re-extraction"
            );
            // Remove old content entry
            let content_key = format!("{}-content", early_doc_id);
            let _ = self.kv_storage.delete(&[content_key]).await;

            // Remove old chunk entries
            let all_keys = self.kv_storage.keys().await.unwrap_or_default();
            let chunk_prefix = format!("{}-chunk-", early_doc_id);
            let chunk_keys: Vec<String> = all_keys
                .into_iter()
                .filter(|k| k.starts_with(&chunk_prefix))
                .collect();
            if !chunk_keys.is_empty() {
                info!(
                    document_id = %early_doc_id,
                    chunk_count = chunk_keys.len(),
                    "Removing old chunk entries"
                );
                let _ = self.kv_storage.delete(&chunk_keys).await;
            }
        }

        info!(
            document_id = %early_doc_id,
            pdf_id = %data.pdf_id,
            is_reprocess = is_reprocess,
            "{}document metadata with 'converting' stage",
            if is_reprocess { "Updated existing " } else { "Created early " }
        );

        // OODA-09: Create progress callback for real-time page-by-page feedback
        // WHY: Users need to see extraction progress like "Extracting page 5/10..."
        // OODA-10: Also attach progress broadcaster if available for WebSocket delivery
        // OODA-16: Add filename for progress display
        let mut callback = PipelineProgressCallback::new(
            self.pipeline_state.clone(),
            data.pdf_id.to_string(),
            task.track_id.clone(),
        )
        .with_filename(pdf.filename.clone())
        .with_document_metadata(early_doc_id.clone(), Arc::clone(&self.kv_storage));

        if let Some(ref broadcaster) = self.progress_broadcaster {
            callback = callback.with_broadcaster(broadcaster.clone());
        }
        let progress_callback: Arc<dyn edgequake_pdf2md::ConversionProgressCallback> =
            Arc::new(callback);

        // 4. Extract content (vision or text mode)
        // == Progress: starting conversion (this can take 5-10+ minutes) ==
        task.update_progress("pdf_converting".to_string(), 2, 10);

        // ── CANCELLATION GATE: before vision extraction (most expensive PDF stage) ──
        self.check_cancelled(&cancel_token, "pre-vision-extraction", &early_doc_id)
            .await?;

        // SPEC-007: Vision → edgequake-pdf2md v0.7.0 (bundled pdfium, multi-provider,
        //           lazy streaming pipeline, progress callbacks, page-level checkpointing).
        //
        // WHY spawn_blocking + Handle::block_on:
        // process_page(... prior_page: Option<&str> ...) holds &str across
        // .await points inside the process_concurrent state machine, preventing the future
        // from being Send in async_trait contexts.
        // Handle::block_on requires no Send bound on the future, bypassing this.
        let (markdown, extraction_method, used_vision_model) = if data.enable_vision {
            #[cfg(feature = "vision")]
            {
                use edgequake_pdf2md::{convert_from_bytes, ConversionConfig};

                let model = data
                    .vision_model
                    .clone()
                    .unwrap_or_else(|| "gpt-4.1-nano".to_string());
                let pdf_bytes = pdf.pdf_data.clone();

                // WHY: Vision extraction uses a provider selected per-workspace
                // (e.g. OpenAI gpt-4o-mini), which may differ from the system
                // entity-extraction LLM (e.g. Ollama). Cloning self.llm_provider
                // would silently send vision requests to the wrong provider and
                // produce hallucinated content. We create a dedicated provider
                // using data.vision_provider so the correct API key and endpoint
                // are used (SPEC-040 fix).
                let provider = {
                    use crate::safety_limits::create_safe_llm_provider;
                    create_safe_llm_provider(&data.vision_provider, &model).map_err(|e| {
                        edgequake_tasks::TaskError::Processing(format!(
                            "Failed to create vision provider '{}': {e}",
                            data.vision_provider
                        ))
                    })?
                };
                let model_owned = model.clone();

                // LARGE-DOC: Adaptive concurrency based on page count.
                // WHY: For documents with 1000+ pages, we need to limit concurrency
                // to avoid overwhelming the LLM provider and running out of memory.
                // Small docs (< 50 pages): default 10 concurrent requests
                // Medium docs (50-200 pages): 8 concurrent requests
                // Large docs (200-500 pages): 5 concurrent requests
                // Very large docs (500+ pages): 3 concurrent requests
                let page_count = pdf.page_count.unwrap_or(0) as usize;
                let concurrency = std::env::var("EDGEQUAKE_PDF_CONCURRENCY")
                    .ok()
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(match page_count {
                        0..=49 => 10,
                        50..=199 => 8,
                        200..=499 => 5,
                        _ => 3,
                    });

                // LARGE-DOC: Adaptive DPI based on page count.
                // WHY: For very large documents, lower DPI reduces memory usage
                // per page image while keeping acceptable quality.
                // Small docs: 150 DPI (default quality)
                // Large docs (500+ pages): 120 DPI (saves ~36% memory per image)
                // Very large docs (1000+ pages): 100 DPI (saves ~55% memory per image)
                let dpi = std::env::var("EDGEQUAKE_PDF_DPI")
                    .ok()
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(match page_count {
                        0..=499 => 150,
                        500..=999 => 120,
                        _ => 100,
                    });

                info!(
                    pdf_id = %data.pdf_id,
                    vision_provider = %data.vision_provider,
                    vision_model = %model,
                    page_count = page_count,
                    concurrency = concurrency,
                    dpi = dpi,
                    "Starting vision extraction via edgequake-pdf2md v0.6.1 (progress callback connected, adaptive concurrency)"
                );

                // WHY Handle::current before spawn_blocking: must capture the runtime
                // handle on the async thread before entering the blocking thread.
                let handle = tokio::runtime::Handle::current();

                // FIX-TIMEOUT: Adaptive timeout based on page count.
                // WHY: A fixed 10-minute timeout is insufficient for 1000+ page documents.
                // Scale timeout linearly: base 60s + 5s per page, minimum 600s.
                // A 1000-page doc gets ~5060 seconds (~84 minutes).
                let base_timeout_secs: u64 = std::env::var("EDGEQUAKE_VISION_TIMEOUT_SECS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                let vision_timeout_secs = if base_timeout_secs > 0 {
                    base_timeout_secs // Explicit override
                } else {
                    let adaptive = 60 + (page_count as u64 * 5);
                    adaptive.max(600) // Minimum 10 minutes
                };
                let vision_timeout = std::time::Duration::from_secs(vision_timeout_secs);

                info!(
                    pdf_id = %data.pdf_id,
                    timeout_secs = vision_timeout_secs,
                    "Vision extraction timeout set (adaptive for {} pages)",
                    page_count
                );

                let spawn_result = tokio::time::timeout(
                    vision_timeout,
                    tokio::task::spawn_blocking(move || {
                        // CHECKPOINT: Create FileCheckpointStore for resumable PDF→MD conversion.
                        // WHY: If the server crashes mid-conversion of a 1000-page PDF,
                        // already-converted pages are saved to disk and skipped on retry,
                        // saving hours of LLM calls and API costs.
                        // The checkpoint ID is a SHA-256 of (PDF content prefix + settings),
                        // so the same PDF with the same settings always resumes correctly.
                        let checkpoint_dir = std::env::var("EDGEQUAKE_CHECKPOINT_DIR")
                            .unwrap_or_else(|_| {
                                let mut dir = std::env::temp_dir();
                                dir.push("edgequake-checkpoints");
                                dir.to_string_lossy().to_string()
                            });
                        let checkpoint_store: Option<
                            std::sync::Arc<dyn edgequake_pdf2md::CheckpointStore>,
                        > = {
                            let store = edgequake_pdf2md::FileCheckpointStore::new(&checkpoint_dir);
                            tracing::info!(
                                checkpoint_dir = %checkpoint_dir,
                                "PDF checkpoint store initialized for resumable conversion"
                            );
                            Some(std::sync::Arc::new(store))
                        };

                        // CHECKPOINT: Force fresh conversion on rebuild/reprocess.
                        // WHY: When a user explicitly triggers rebuild, they want fresh
                        // extraction with potentially different LLM settings. Reusing
                        // old checkpoints would silently serve stale content.
                        let force_no_resume = is_reprocess;

                        let mut builder = ConversionConfig::builder()
                            .provider(provider)
                            .model(model_owned)
                            .concurrency(concurrency)
                            .dpi(dpi)
                            .progress_callback(progress_callback);

                        if let Some(store) = checkpoint_store {
                            builder = builder.checkpoint_store(store);
                        }
                        if force_no_resume {
                            builder = builder.no_resume(true);
                        }

                        let config = builder.build().map_err(|e| format!("Vision config: {e}"))?;
                        // Handle::block_on has no Send bound on the future
                        handle
                            .block_on(convert_from_bytes(&pdf_bytes, &config))
                            .map_err(|e| format!("Vision extraction: {e}"))
                    }),
                )
                .await;

                let output = match spawn_result {
                    Ok(join_result) => join_result
                        .map_err(|e| {
                            edgequake_tasks::TaskError::Processing(format!("Spawn error: {e}"))
                        })?
                        .map_err(edgequake_tasks::TaskError::Processing)?,
                    Err(_elapsed) => {
                        error!(
                            pdf_id = %data.pdf_id,
                            timeout_secs = vision_timeout.as_secs(),
                            "Vision extraction timed out - LLM provider may be unresponsive"
                        );
                        // Update document status to failed with clear timeout message
                        let _ = self
                            .update_document_status(
                                &early_doc_id,
                                "failed",
                                Some(&format!(
                                    "Vision extraction timed out after {}s. Check that the LLM provider ({}) is reachable.",
                                    vision_timeout.as_secs(),
                                    data.vision_provider
                                )),
                            )
                            .await;
                        return Err(edgequake_tasks::TaskError::Timeout(format!(
                            "Vision extraction timed out after {}s for PDF {}. Provider '{}' may be unresponsive.",
                            vision_timeout.as_secs(),
                            data.pdf_id,
                            data.vision_provider
                        )));
                    }
                };

                info!(
                    pdf_id = %data.pdf_id,
                    pages = output.stats.total_pages,
                    processed = output.stats.processed_pages,
                    markdown_len = output.markdown.len(),
                    "Vision extraction completed"
                );
                (output.markdown, ExtractionMethod::Vision, Some(model))
            }
            #[cfg(not(feature = "vision"))]
            {
                return Err(edgequake_tasks::TaskError::UnsupportedOperation(
                    "Vision extraction requires the 'vision' feature flag".to_string(),
                ));
            }
        } else {
            // Text-only extraction removed: edgequake-pdf crate moved to legacy/ (SPEC-040).
            // All callers set enable_vision=true; this branch is unreachable in practice.
            return Err(edgequake_tasks::TaskError::UnsupportedOperation(
                "Text-only PDF extraction is no longer supported. Use vision mode (enable_vision=true)."
                    .to_string(),
            ));
        };

        info!(
            pdf_id = %data.pdf_id,
            markdown_len = markdown.len(),
            extraction_method = ?extraction_method,
            "Extracted markdown from PDF"
        );

        // == Progress: conversion done, storing markdown ==
        task.update_progress("storing_markdown".to_string(), 3, 45);

        // 5. Store markdown in pdf_documents with extraction method
        let update_req = UpdatePdfProcessingRequest {
            pdf_id: data.pdf_id,
            processing_status: PdfProcessingStatus::Completed,
            markdown_content: Some(markdown.clone()),
            extraction_method: Some(extraction_method),
            extraction_errors: None,
            document_id: None, // Will be set after document creation
            vision_model: used_vision_model.clone(),
        };

        pdf_storage
            .update_pdf_processing(update_req.clone())
            .await
            .map_err(|e| edgequake_tasks::TaskError::Storage(e.to_string()))?;

        // 6. Create document via standard pipeline
        // == Progress: markdown stored, starting entity extraction + indexing ==
        task.update_progress("entity_extraction".to_string(), 4, 50);

        // ── CANCELLATION GATE: before handing off to text_insert pipeline ──
        self.check_cancelled(&cancel_token, "pre-text-insert", &early_doc_id)
            .await?;

        // SPEC-002: Include source_type: "pdf" for unified pipeline tracking
        // OODA-05: Include tenant_id/workspace_id for multi-tenant document visibility
        // Pass the early_doc_id so we reuse the same document that's already showing in UI
        // OODA-04: Include sha256_checksum for end-to-end lineage traceability
        // WHY: Downstream ensure_document_source_type needs checksum for integrity verification
        let text_data = edgequake_tasks::TextInsertData {
            text: markdown.clone(),
            file_source: pdf.filename.clone(),
            workspace_id: data.workspace_id.to_string(),
            metadata: Some(json!({
                "document_id": early_doc_id.clone(),  // Reuse early document ID
                "source": "pdf_upload",
                "source_type": "pdf",
                "document_type": "pdf",
                "pdf_id": data.pdf_id.to_string(),
                "filename": pdf.filename,
                "page_count": pdf.page_count,
                "file_size_bytes": pdf.file_size_bytes,
                "sha256_checksum": pdf.sha256_checksum,
                "tenant_id": data.tenant_id.to_string(),
                "workspace_id": data.workspace_id.to_string(),
                // SPEC-040: Store PDF extraction lineage for document detail view
                // WHY: The lineage builder in documents.rs reads from this metadata JSON.
                // vision_model and extraction_method are stored in pdf_documents table but
                // not in the KV document metadata, making them invisible in the lineage view.
                "pdf_vision_model": used_vision_model,
                "pdf_extraction_method": extraction_method.as_str(),
            })),
        };

        let result = self
            .process_text_insert(task, text_data, cancel_token)
            .await?;

        // == Progress: extraction complete, linking PDF ==
        task.update_progress("linking".to_string(), 5, 95);

        // 7. Link PDF to created document (use early_doc_id)
        if let Ok(document_uuid) = uuid::Uuid::parse_str(&early_doc_id) {
            // FIX-ISSUE-74: Ensure a row in the `documents` relational table exists
            // BEFORE setting pdf_documents.document_id (which has a FK constraint).
            // WHY: Without this, the UPDATE violates the foreign key constraint
            // "pdf_documents_document_id_fkey" because no matching documents(id) row exists.
            let workspace_uuid = data.workspace_id;
            let tenant_uuid = Some(data.tenant_id);
            // WHY: Truncate content to 64KB for the relational record to avoid bloat.
            // Full content lives in KV storage. Use floor_char_boundary to avoid
            // splitting a multi-byte UTF-8 codepoint, which would panic.
            let truncate_at = if markdown.len() > 65_536 {
                // Find the largest char boundary <= 65_536
                markdown
                    .char_indices()
                    .map(|(i, _)| i)
                    .take_while(|&i| i <= 65_536)
                    .last()
                    .unwrap_or(0)
            } else {
                markdown.len()
            };
            if let Err(e) = pdf_storage
                .ensure_document_record(
                    &document_uuid,
                    &workspace_uuid,
                    tenant_uuid.as_ref(),
                    &pdf.filename,
                    &markdown[..truncate_at],
                    // WHY: The relational `documents` table has a CHECK constraint
                    // that only allows 'pending', 'processing', 'indexed', 'failed'.
                    // KV storage uses 'completed' but the relational table uses 'indexed'.
                    "indexed",
                )
                .await
            {
                error!(
                    "Failed to ensure document record: {} - continuing anyway",
                    e
                );
            }

            if let Err(e) = pdf_storage
                .link_pdf_to_document(&data.pdf_id, &document_uuid)
                .await
            {
                error!("Failed to link PDF to document: {} - continuing anyway", e);
                // Non-fatal - PDF still processed successfully
            }
        }

        // 8. Status already set to Completed in step 5 via update_pdf_processing
        info!(
            pdf_id = %data.pdf_id,
            "PDF processing completed successfully"
        );

        // OODA-16: Clean up progress tracking (fire-and-forget)
        // WHY: Free memory for completed uploads. GET endpoint will return 404.
        let state = self.pipeline_state.clone();
        let track_id = task.track_id.clone();
        tokio::spawn(async move {
            state.remove_pdf_progress(&track_id).await;
        });

        Ok(result)
    }

    #[cfg(not(feature = "postgres"))]
    pub(super) async fn process_pdf_processing(
        &self,
        _task: &mut Task,
        data: edgequake_tasks::PdfProcessingData,
        _cancel_token: CancellationToken,
    ) -> TaskResult<serde_json::Value> {
        warn!(
            pdf_id = %data.pdf_id,
            "PDF processing not available (postgres feature disabled)"
        );
        Err(edgequake_tasks::TaskError::UnsupportedOperation(
            "PDF processing requires postgres feature".to_string(),
        ))
    }
}
