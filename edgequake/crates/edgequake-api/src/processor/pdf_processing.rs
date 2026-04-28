use super::*;
use tokio_util::sync::CancellationToken;

#[cfg(feature = "postgres")]
fn strip_nul_bytes(text: String) -> String {
    if !text.contains('\0') {
        return text;
    }

    let nul_count = text.chars().filter(|&ch| ch == '\0').count();
    let sanitized = text.replace('\0', "");
    warn!(
        nul_count,
        sanitized_len = sanitized.len(),
        "Removed NUL bytes from extracted PDF markdown before persistence"
    );
    sanitized
}

#[cfg(feature = "postgres")]
fn should_fallback_to_edgeparse(
    requested_backend: edgequake_pdf::PdfParserBackend,
    error: &edgequake_tasks::TaskError,
) -> bool {
    if requested_backend != edgequake_pdf::PdfParserBackend::Vision {
        return false;
    }

    matches!(
        error,
        edgequake_tasks::TaskError::Timeout(_)
            | edgequake_tasks::TaskError::Processing(_)
            | edgequake_tasks::TaskError::UnsupportedOperation(_)
    )
}

#[cfg(feature = "postgres")]
fn build_edgeparse_fallback_message(provider: &str, error: &edgequake_tasks::TaskError) -> String {
    format!(
        "Vision extraction via {} was unavailable ({}). Falling back to EdgeParse for a more reliable text extraction.",
        provider, error
    )
}

#[cfg(feature = "postgres")]
fn merge_extraction_notice(
    extraction_errors: &mut Option<serde_json::Value>,
    key: &str,
    message: String,
) {
    let notice = json!({ "message": message });
    match extraction_errors {
        Some(serde_json::Value::Object(map)) => {
            map.insert(key.to_string(), notice);
        }
        _ => {
            *extraction_errors = Some(json!({ key: notice }));
        }
    }
}

#[cfg(feature = "postgres")]
fn should_resume_pdf_conversion(has_existing_document: bool, restart_from_scratch: bool) -> bool {
    has_existing_document && !restart_from_scratch
}

#[cfg(feature = "postgres")]
fn should_restart_pdf_conversion(has_existing_document: bool, restart_from_scratch: bool) -> bool {
    has_existing_document && restart_from_scratch
}

#[cfg(feature = "postgres")]
fn compute_safe_pdf_resource_profile(
    page_count: usize,
    file_size_bytes: i64,
    vision_provider: &str,
) -> (usize, u32) {
    use crate::safety_limits::is_local_provider;

    let is_local = is_local_provider(vision_provider);
    let large_file = file_size_bytes >= 25 * 1024 * 1024;
    let huge_file = file_size_bytes >= 50 * 1024 * 1024;

    let concurrency = if is_local {
        if huge_file || page_count >= 200 {
            1
        } else {
            2
        }
    } else if huge_file || page_count >= 1000 {
        1
    } else {
        match page_count {
            0..=49 => 8,
            50..=199 => 6,
            200..=499 => 3,
            _ => 2,
        }
    };

    let dpi = if huge_file || page_count >= 1000 {
        96
    } else if large_file || page_count >= 500 {
        110
    } else if page_count >= 200 {
        120
    } else {
        150
    };

    (concurrency.max(1), dpi)
}

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

        let filename = pdf.filename.clone();
        let file_size_bytes = pdf.file_size_bytes;
        let page_count_opt = pdf.page_count;
        let sha256_checksum = pdf.sha256_checksum.clone();
        let pdf_data = pdf.pdf_data;

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
        let has_existing_document = data.existing_document_id.is_some();
        let should_resume_from_checkpoint =
            should_resume_pdf_conversion(has_existing_document, data.restart_from_scratch);
        let should_cleanup_existing_content =
            should_restart_pdf_conversion(has_existing_document, data.restart_from_scratch);
        let early_doc_id = data
            .existing_document_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // FIX-DUPLICATE-BUG: Persist the generated document ID back into task_data
        // so that worker retries reuse the same document ID instead of creating
        // a new UUID on each attempt. Without this, a single PDF upload that fails
        // and gets retried by the worker pool creates duplicate documents with
        // different IDs, each stuck in "processing" state.
        if !has_existing_document {
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
            "title": filename.clone(),
            "file_name": filename.clone(),
            "source_type": "pdf",
            "document_type": "pdf",
            "status": "processing",
            "current_stage": "converting",
            "stage_message": if should_resume_from_checkpoint {
                match page_count_opt {
                    Some(n) if n > 0 => format!("Resuming PDF to Markdown conversion from saved progress (up to {} pages)", n),
                    _ => "Resuming PDF to Markdown conversion from saved progress...".to_string(),
                }
            } else {
                match page_count_opt {
                    Some(n) if n > 0 => format!("Converting PDF to Markdown (0/{} pages)", n),
                    _ => "Converting PDF to Markdown (detecting pages...)".to_string(),
                }
            },
            "stage_progress": 0.0,
            "pdf_id": data.pdf_id.to_string(),
            "file_size_bytes": file_size_bytes,
            "sha256_checksum": sha256_checksum,
            "page_count": page_count_opt,
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
        if should_cleanup_existing_content {
            info!(
                document_id = %early_doc_id,
                pdf_id = %data.pdf_id,
                "Fresh reprocess requested: cleaning up old content and chunks before re-extraction"
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
            has_existing_document,
            should_resume_from_checkpoint,
            retry_count = task.retry_count,
            "{}document metadata with 'converting' stage",
            if should_resume_from_checkpoint {
                "Resumed existing "
            } else if has_existing_document {
                "Updated existing "
            } else {
                "Created early "
            }
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
        .with_filename(filename.clone())
        .with_document_metadata(early_doc_id.clone(), Arc::clone(&self.kv_storage));

        if let Some(ref broadcaster) = self.progress_broadcaster {
            callback = callback.with_broadcaster(broadcaster.clone());
        }
        let progress_callback: Arc<dyn edgequake_pdf2md::ConversionProgressCallback> =
            Arc::new(callback);

        // 4. Extract content (vision or text mode)
        //
        // RESUME SHORTCUT: If this is a retry and the markdown was already stored
        // in the pdf_documents table from the previous run, skip the expensive
        // PDF→Markdown conversion entirely and jump straight to text_insert.
        //
        // WHY: A failed job (e.g., entity extraction failed at chunk 140/142)
        // should not redo the multi-minute PDF conversion. The markdown is
        // already in the DB; we only need to re-run the ingestion pipeline.
        if should_resume_from_checkpoint {
            if let Some(stored_markdown) = pdf.markdown_content.clone() {
                if !stored_markdown.is_empty() {
                    info!(
                        document_id = %early_doc_id,
                        pdf_id = %data.pdf_id,
                        markdown_len = stored_markdown.len(),
                        "RESUME: Markdown already stored — skipping PDF conversion, resuming at text_insert"
                    );

                    // Update status so the UI shows we're resuming extraction, not reconverting.
                    let _ = self
                        .update_document_status(
                            &early_doc_id,
                            "processing",
                            Some("Resuming entity extraction from previously converted markdown"),
                        )
                        .await;

                    task.update_progress("entity_extraction".to_string(), 4, 50);

                    self.check_cancelled(&cancel_token, "pre-text-insert-resume", &early_doc_id)
                        .await?;

                    let stored_extraction_method = pdf.extraction_method;
                    let stored_vision_model = pdf.vision_model.clone();
                    // Clone for linking step after process_text_insert consumes the string.
                    let stored_markdown_for_link = stored_markdown.clone();

                    let text_data = edgequake_tasks::TextInsertData {
                        text: stored_markdown,
                        file_source: filename.clone(),
                        workspace_id: data.workspace_id.to_string(),
                        metadata: Some(json!({
                            "document_id": early_doc_id.clone(),
                            "source": "pdf_upload",
                            "source_type": "pdf",
                            "document_type": "pdf",
                            "pdf_id": data.pdf_id.to_string(),
                            "filename": filename.clone(),
                            "page_count": page_count_opt,
                            "file_size_bytes": file_size_bytes,
                            "sha256_checksum": sha256_checksum.clone(),
                            "tenant_id": data.tenant_id.to_string(),
                            "workspace_id": data.workspace_id.to_string(),
                            "pdf_vision_model": stored_vision_model,
                            "pdf_extraction_method": stored_extraction_method.as_ref().map(|m| m.as_str()),
                        })),
                    };

                    let result = self
                        .process_text_insert(task, text_data, cancel_token)
                        .await?;

                    // Link PDF to document (same as the normal path below).
                    task.update_progress("linking".to_string(), 5, 95);
                    if let Ok(document_uuid) = uuid::Uuid::parse_str(&early_doc_id) {
                        let workspace_uuid = data.workspace_id;
                        let tenant_uuid = Some(data.tenant_id);
                        let truncate_at = stored_markdown_for_link.len().min(65_536);
                        let safe_truncate = stored_markdown_for_link[..truncate_at]
                            .char_indices()
                            .map(|(i, _)| i)
                            .take_while(|&i| i <= 65_536)
                            .last()
                            .unwrap_or(0);
                        let _ = pdf_storage
                            .ensure_document_record(
                                &document_uuid,
                                &workspace_uuid,
                                tenant_uuid.as_ref(),
                                &filename,
                                &stored_markdown_for_link[..safe_truncate],
                                "indexed",
                            )
                            .await;
                        let _ = pdf_storage
                            .link_pdf_to_document(&data.pdf_id, &document_uuid)
                            .await;
                    }

                    task.update_progress("complete".to_string(), 6, 100);
                    return Ok(result);
                }
            }
        }

        // == Progress: starting conversion (this can take 5-10+ minutes) ==
        task.update_progress("pdf_converting".to_string(), 2, 10);

        // ── CANCELLATION GATE: before vision extraction (most expensive PDF stage) ──
        self.check_cancelled(&cancel_token, "pre-vision-extraction", &early_doc_id)
            .await?;

        let backend = data.pdf_parser_backend;
        let page_count = page_count_opt.unwrap_or(0) as usize;
        let mut extraction_method = match backend {
            edgequake_pdf::PdfParserBackend::Vision => ExtractionMethod::Vision,
            edgequake_pdf::PdfParserBackend::EdgeParse => ExtractionMethod::EdgeParse,
        };

        let default_vision_model = || {
            use crate::handlers::pdf_upload::types::default_vision_model_for_provider;
            data.vision_model
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| default_vision_model_for_provider(&data.vision_provider))
        };

        let mut vision_model = match backend {
            edgequake_pdf::PdfParserBackend::Vision => Some(default_vision_model()),
            edgequake_pdf::PdfParserBackend::EdgeParse => None,
        };
        let mut fallback_warning: Option<String> = None;

        let converter = match backend {
            edgequake_pdf::PdfParserBackend::Vision => {
                if !data.enable_vision {
                    let error = edgequake_tasks::TaskError::UnsupportedOperation(
                        "Vision PDF extraction requires enable_vision=true.".to_string(),
                    );
                    let message = build_edgeparse_fallback_message(&data.vision_provider, &error);
                    warn!(
                        pdf_id = %data.pdf_id,
                        "Vision disabled for requested vision extraction; falling back to EdgeParse"
                    );
                    let _ = self
                        .update_document_status(&early_doc_id, "processing", Some(&message))
                        .await;
                    fallback_warning = Some(message);
                    extraction_method = ExtractionMethod::EdgeParse;
                    vision_model = None;
                    edgequake_pdf::create_pdf_converter(
                        edgequake_pdf::PdfParserBackend::EdgeParse,
                        None,
                    )
                } else {
                    #[cfg(feature = "vision")]
                    {
                        use crate::safety_limits::create_safe_vision_provider;

                        match create_safe_vision_provider(
                            &data.vision_provider,
                            vision_model.as_deref().unwrap_or_default(),
                        ) {
                            Ok(provider) => {
                                edgequake_pdf::create_pdf_converter(backend, Some(provider))
                            }
                            Err(e) => {
                                let error = edgequake_tasks::TaskError::Processing(format!(
                                    "Failed to create vision provider '{}': {e}",
                                    data.vision_provider
                                ));
                                if !should_fallback_to_edgeparse(backend, &error) {
                                    return Err(error);
                                }
                                let message =
                                    build_edgeparse_fallback_message(&data.vision_provider, &error);
                                warn!(
                                    pdf_id = %data.pdf_id,
                                    error = %error,
                                    "Vision provider setup failed; falling back to EdgeParse"
                                );
                                let _ = self
                                    .update_document_status(
                                        &early_doc_id,
                                        "processing",
                                        Some(&message),
                                    )
                                    .await;
                                fallback_warning = Some(message);
                                extraction_method = ExtractionMethod::EdgeParse;
                                vision_model = None;
                                edgequake_pdf::create_pdf_converter(
                                    edgequake_pdf::PdfParserBackend::EdgeParse,
                                    None,
                                )
                            }
                        }
                    }
                    #[cfg(not(feature = "vision"))]
                    {
                        let error = edgequake_tasks::TaskError::UnsupportedOperation(
                            "Vision extraction requires the 'vision' feature flag".to_string(),
                        );
                        let message =
                            build_edgeparse_fallback_message(&data.vision_provider, &error);
                        warn!(
                            pdf_id = %data.pdf_id,
                            "Vision feature is unavailable; falling back to EdgeParse"
                        );
                        let _ = self
                            .update_document_status(&early_doc_id, "processing", Some(&message))
                            .await;
                        fallback_warning = Some(message);
                        extraction_method = ExtractionMethod::EdgeParse;
                        vision_model = None;
                        edgequake_pdf::create_pdf_converter(
                            edgequake_pdf::PdfParserBackend::EdgeParse,
                            None,
                        )
                    }
                }
            }
            edgequake_pdf::PdfParserBackend::EdgeParse => {
                edgequake_pdf::create_pdf_converter(backend, None)
            }
        };

        // WHY: Local providers (Ollama, LM Studio) run on a single GPU that is
        // memory-bound. High concurrency causes VRAM thrashing and *increases*
        // total conversion time. Cap local concurrency at 2. Cloud providers
        // retain the original scale-with-page-count formula.
        // See ADR-04-003 in mission/04-heavy-pdf.md.
        let (safe_concurrency, safe_dpi) =
            compute_safe_pdf_resource_profile(page_count, file_size_bytes, &data.vision_provider);
        let concurrency = std::env::var("EDGEQUAKE_PDF_CONCURRENCY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(safe_concurrency)
            .max(1)
            .min(safe_concurrency);
        let dpi = std::env::var("EDGEQUAKE_PDF_DPI")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(safe_dpi)
            .clamp(96, safe_dpi.max(96));
        let checkpoint_dir = std::env::var("EDGEQUAKE_CHECKPOINT_DIR").unwrap_or_else(|_| {
            let mut dir = std::env::temp_dir();
            dir.push("edgequake-checkpoints");
            dir.to_string_lossy().to_string()
        });
        let conversion_config = edgequake_pdf::PdfConversionConfig {
            page_count_hint: page_count_opt.map(|count| count as usize),
            table_method: None,
            filename: Some(filename.clone()),
            vision: vision_model
                .clone()
                .map(|model| edgequake_pdf::VisionConversionConfig {
                    model: Some(model),
                    concurrency: Some(concurrency),
                    dpi: Some(dpi),
                    checkpoint_dir: Some(checkpoint_dir),
                    no_resume: should_cleanup_existing_content,
                    progress_callback: Some(progress_callback),
                }),
        };

        let edgeparse_config = edgequake_pdf::PdfConversionConfig {
            vision: None,
            ..conversion_config.clone()
        };

        let markdown = match extraction_method {
            ExtractionMethod::Vision => {
                // WHY: EDGEQUAKE_VISION_TIMEOUT_SECS is kept for backwards
                // compatibility. When not set, use the provider-aware formula:
                //   120 + (page_count × secs_per_page_for_provider)
                // This gives ~3 720s for 120 pages with Ollama vs the previous
                // 660s, matching the real hardware requirement.
                // See ADR-04-002 in mission/04-heavy-pdf.md.
                let base_timeout_secs: u64 = std::env::var("EDGEQUAKE_VISION_TIMEOUT_SECS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                let vision_timeout_secs = if base_timeout_secs > 0 {
                    base_timeout_secs
                } else {
                    use crate::safety_limits::vision_outer_timeout_secs;
                    vision_outer_timeout_secs(&data.vision_provider, page_count)
                };
                let vision_timeout = std::time::Duration::from_secs(vision_timeout_secs);

                info!(
                    pdf_id = %data.pdf_id,
                    vision_provider = %data.vision_provider,
                    vision_model = %vision_model.clone().unwrap_or_default(),
                    page_count = page_count,
                    concurrency = concurrency,
                    dpi = dpi,
                    timeout_secs = vision_timeout_secs,
                    "Starting Vision PDF conversion"
                );

                match tokio::time::timeout(
                    vision_timeout,
                    converter.convert(&pdf_data, &conversion_config),
                )
                .await
                {
                    Ok(Ok(markdown)) => markdown,
                    Ok(Err(e)) => {
                        let error = edgequake_tasks::TaskError::Processing(format!(
                            "PDF conversion failed: {e}"
                        ));
                        if !should_fallback_to_edgeparse(backend, &error) {
                            return Err(error);
                        }

                        let message =
                            build_edgeparse_fallback_message(&data.vision_provider, &error);
                        warn!(
                            pdf_id = %data.pdf_id,
                            error = %error,
                            "Vision conversion failed; falling back to EdgeParse"
                        );
                        let _ = self
                            .update_document_status(&early_doc_id, "processing", Some(&message))
                            .await;
                        fallback_warning = Some(message);
                        extraction_method = ExtractionMethod::EdgeParse;
                        vision_model = None;

                        edgequake_pdf::create_pdf_converter(
                            edgequake_pdf::PdfParserBackend::EdgeParse,
                            None,
                        )
                        .convert(&pdf_data, &edgeparse_config)
                        .await
                        .map_err(|e| {
                            edgequake_tasks::TaskError::Processing(format!(
                                "PDF conversion failed after EdgeParse fallback: {e}"
                            ))
                        })?
                    }
                    Err(_elapsed) => {
                        let error = edgequake_tasks::TaskError::Timeout(format!(
                            "Vision extraction timed out after {}s for PDF {}. Provider '{}' may be unresponsive.",
                            vision_timeout.as_secs(),
                            data.pdf_id,
                            data.vision_provider
                        ));
                        if !should_fallback_to_edgeparse(backend, &error) {
                            return Err(error);
                        }

                        let message =
                            build_edgeparse_fallback_message(&data.vision_provider, &error);
                        warn!(
                            pdf_id = %data.pdf_id,
                            timeout_secs = vision_timeout.as_secs(),
                            "Vision extraction timed out; falling back to EdgeParse"
                        );
                        let _ = self
                            .update_document_status(&early_doc_id, "processing", Some(&message))
                            .await;
                        fallback_warning = Some(message);
                        extraction_method = ExtractionMethod::EdgeParse;
                        vision_model = None;

                        edgequake_pdf::create_pdf_converter(
                            edgequake_pdf::PdfParserBackend::EdgeParse,
                            None,
                        )
                        .convert(&pdf_data, &edgeparse_config)
                        .await
                        .map_err(|e| {
                            edgequake_tasks::TaskError::Processing(format!(
                                "PDF conversion failed after EdgeParse fallback: {e}"
                            ))
                        })?
                    }
                }
            }
            ExtractionMethod::EdgeParse | ExtractionMethod::Text | ExtractionMethod::Hybrid => {
                info!(
                    pdf_id = %data.pdf_id,
                    page_count = page_count,
                    "Starting EdgeParse PDF conversion"
                );
                converter
                    .convert(&pdf_data, &edgeparse_config)
                    .await
                    .map_err(|e| {
                        edgequake_tasks::TaskError::Processing(format!(
                            "PDF conversion failed: {e}"
                        ))
                    })?
            }
        };

        let markdown = strip_nul_bytes(markdown);
        drop(pdf_data);

        let mut extraction_errors = if extraction_method == ExtractionMethod::EdgeParse {
            let avg_chars_per_page = markdown.len() / page_count.max(1);
            if avg_chars_per_page < 50 {
                warn!(
                    pdf_id = %data.pdf_id,
                    avg_chars_per_page,
                    "Low text content from EdgeParse — PDF may be scanned/image-only"
                );
                Some(json!({
                    "low_content_warning": {
                        "avg_chars_per_page": avg_chars_per_page,
                        "message": "Low text content detected. This PDF may be image-only. Consider using Vision extraction."
                    }
                }))
            } else {
                None
            }
        } else {
            None
        };
        if let Some(message) = fallback_warning.take() {
            merge_extraction_notice(&mut extraction_errors, "vision_fallback", message);
        }
        let extraction_warning = extraction_errors
            .as_ref()
            .and_then(|value| {
                value
                    .get("vision_fallback")
                    .or_else(|| value.get("low_content_warning"))
            })
            .and_then(|value| value.get("message"))
            .and_then(|value| value.as_str())
            .map(str::to_string);

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
            extraction_errors: extraction_errors.clone(),
            document_id: None, // Will be set after document creation
            vision_model: vision_model.clone(),
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
            file_source: filename.clone(),
            workspace_id: data.workspace_id.to_string(),
            metadata: Some(json!({
                "document_id": early_doc_id.clone(),  // Reuse early document ID
                "source": "pdf_upload",
                "source_type": "pdf",
                "document_type": "pdf",
                "pdf_id": data.pdf_id.to_string(),
                "filename": filename.clone(),
                "page_count": page_count_opt,
                "file_size_bytes": file_size_bytes,
                "sha256_checksum": sha256_checksum.clone(),
                "tenant_id": data.tenant_id.to_string(),
                "workspace_id": data.workspace_id.to_string(),
                // SPEC-040: Store PDF extraction lineage for document detail view
                // WHY: The lineage builder in documents.rs reads from this metadata JSON.
                // vision_model and extraction_method are stored in pdf_documents table but
                // not in the KV document metadata, making them invisible in the lineage view.
                "pdf_vision_model": vision_model,
                "pdf_extraction_method": extraction_method.as_str(),
                "pdf_extraction_warning": extraction_warning,
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
                    &filename,
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

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use super::*;
    use edgequake_pdf::PdfParserBackend;
    use edgequake_tasks::TaskError;

    #[test]
    fn vision_timeouts_trigger_edgeparse_fallback() {
        let error = TaskError::Timeout(
            "Vision extraction timed out after 480s for PDF abc. Provider 'ollama' may be unresponsive."
                .to_string(),
        );

        assert!(should_fallback_to_edgeparse(
            PdfParserBackend::Vision,
            &error
        ));
    }

    #[test]
    fn edgeparse_requests_do_not_self_fallback() {
        let error = TaskError::Timeout("EdgeParse timed out".to_string());

        assert!(!should_fallback_to_edgeparse(
            PdfParserBackend::EdgeParse,
            &error
        ));
    }

    #[test]
    fn existing_document_prefers_resume_by_default() {
        assert!(should_resume_pdf_conversion(true, false));
        assert!(!should_restart_pdf_conversion(true, false));
    }

    #[test]
    fn explicit_restart_starts_clean() {
        assert!(!should_resume_pdf_conversion(true, true));
        assert!(should_restart_pdf_conversion(true, true));
    }

    #[test]
    fn large_local_pdfs_are_throttled_aggressively() {
        let (concurrency, dpi) = compute_safe_pdf_resource_profile(250, 60 * 1024 * 1024, "ollama");
        assert_eq!(concurrency, 1);
        assert_eq!(dpi, 96);
    }

    #[test]
    fn small_cloud_pdfs_keep_reasonable_parallelism() {
        let (concurrency, dpi) = compute_safe_pdf_resource_profile(40, 4 * 1024 * 1024, "openai");
        assert_eq!(concurrency, 8);
        assert_eq!(dpi, 150);
    }

    // ── Resume-shortcut logic tests ──────────────────────────────────────────

    /// should_resume_pdf_conversion is the gate for the resume shortcut.
    /// Without an existing document there is nothing to resume.
    #[test]
    fn new_document_never_resumes() {
        assert!(!should_resume_pdf_conversion(false, false));
        assert!(!should_resume_pdf_conversion(false, true));
    }

    /// When an existing document is present AND restart is not requested, the
    /// shortcut should be taken so we never re-run PDF→Markdown conversion.
    #[test]
    fn retry_without_restart_flag_takes_shortcut() {
        // has_existing_document=true, restart_from_scratch=false
        assert!(should_resume_pdf_conversion(true, false));
        // No cleanup of old content should happen
        assert!(!should_restart_pdf_conversion(true, false));
    }

    /// An explicit "restart from scratch" request overrides the shortcut.
    #[test]
    fn explicit_restart_bypasses_resume_shortcut() {
        assert!(!should_resume_pdf_conversion(true, true));
        assert!(should_restart_pdf_conversion(true, true));
    }
}
