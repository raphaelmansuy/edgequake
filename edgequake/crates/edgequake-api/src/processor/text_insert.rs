use super::*;

impl DocumentTaskProcessor {
    /// Process a text insert task.
    pub(super) async fn process_text_insert(
        &self,
        task: &mut Task,
        data: TextInsertData,
    ) -> TaskResult<serde_json::Value> {
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

            let metadata_key = format!("{}-metadata", document_id);
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
                        let metadata_key = format!("{}-metadata", doc_id_clone);
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

        let result = match pipeline
            .process_with_resilience(&document_id, &processed_text, Some(chunk_progress_callback))
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
                        "Document processed with partial success - some chunks failed extraction"
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
                    "CRITICAL: Pipeline processing failed - document marked as failed"
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

        // Update task progress - embedding
        task.update_progress("embedding".to_string(), 4, 30);
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
            error!("{}", error_msg);

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
                error!("{}", error_msg);
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

        // Store chunk embeddings in vector storage for semantic search
        // OODA-05: Include position metadata for lineage-aware retrieval
        // WHY: Semantic search results should carry source position so callers
        // can display "found in lines 42-58" without extra KV lookups.
        let mut chunk_embeddings_stored = 0;
        for chunk in &result.chunks {
            if let Some(embedding) = &chunk.embedding {
                let mut metadata = json!({
                    "type": "chunk",
                    "document_id": document_id,
                    "index": chunk.index,
                    "content": chunk.content,
                    "start_line": chunk.start_line,
                    "end_line": chunk.end_line,
                    "start_offset": chunk.start_offset,
                    "end_offset": chunk.end_offset,
                    "token_count": chunk.token_count,
                });

                // Add tenant and workspace IDs if present
                if let Some(ref tid) = tenant_id {
                    metadata["tenant_id"] = json!(tid);
                }
                metadata["workspace_id"] = json!(&workspace_id_meta);

                if workspace_vector_storage
                    .upsert(&[(chunk.id.clone(), embedding.clone(), metadata)])
                    .await
                    .is_ok()
                {
                    chunk_embeddings_stored += 1;
                }
            }
        }
        info!(
            "Stored {} chunk embeddings in vector storage for document {}",
            chunk_embeddings_stored, document_id
        );

        // Update task progress - extraction
        task.update_progress("extraction".to_string(), 4, 60);
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

        // Store entities and relationships in graph storage using batch operations
        // Collect all nodes for batch upsert
        let mut nodes_batch: Vec<(String, std::collections::HashMap<String, serde_json::Value>)> =
            Vec::new();
        let mut edges_batch: Vec<(
            String,
            String,
            std::collections::HashMap<String, serde_json::Value>,
        )> = Vec::new();

        // OODA-07: Pre-fetch existing entities to merge source_ids (GAP-07 fix for async path)
        // WHY: Without merge, second document overwrites first's source_ids, breaking reference counting
        let entity_names: Vec<String> = result
            .extractions
            .iter()
            .flat_map(|e| e.entities.iter().map(|ent| ent.name.clone()))
            .collect();

        let existing_entity_source_ids: std::collections::HashMap<
            String,
            std::collections::HashSet<String>,
        > = if !entity_names.is_empty() {
            match self.graph_storage.get_nodes_by_ids(&entity_names).await {
                Ok(nodes) => nodes
                    .into_iter()
                    .map(|node| {
                        let sources: std::collections::HashSet<String> = node
                            .properties
                            .get("source_ids")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();
                        (node.id, sources)
                    })
                    .collect(),
                Err(e) => {
                    warn!(
                        "Failed to fetch existing entities for source_ids merge: {}",
                        e
                    );
                    std::collections::HashMap::new()
                }
            }
        } else {
            std::collections::HashMap::new()
        };

        // OODA-07: Pre-fetch existing edges to merge source_ids
        // WHY: Same issue as entities - edges need reference counting for correct deletion
        let edge_keys: Vec<(String, String)> = result
            .extractions
            .iter()
            .flat_map(|e| {
                e.relationships
                    .iter()
                    .map(|r| (r.source.clone(), r.target.clone()))
            })
            .collect();

        let mut existing_edge_source_ids: std::collections::HashMap<
            (String, String),
            std::collections::HashSet<String>,
        > = std::collections::HashMap::new();
        for (source, target) in &edge_keys {
            if let Ok(Some(edge)) = self.graph_storage.get_edge(source, target).await {
                let sources: std::collections::HashSet<String> = edge
                    .properties
                    .get("source_ids")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                existing_edge_source_ids.insert((source.clone(), target.clone()), sources);
            }
        }

        for extraction in &result.extractions {
            for entity in &extraction.entities {
                let mut properties = std::collections::HashMap::new();
                properties.insert("entity_type".to_string(), json!(entity.entity_type));
                properties.insert("description".to_string(), json!(entity.description));
                properties.insert("importance".to_string(), json!(entity.importance));

                // OODA-07: Merge source_ids with existing entity (GAP-07 fix)
                let mut merged_sources: std::collections::HashSet<String> =
                    existing_entity_source_ids
                        .get(&entity.name)
                        .cloned()
                        .unwrap_or_default();
                merged_sources.insert(document_id.clone());
                let source_ids_vec: Vec<String> = merged_sources.into_iter().collect();
                properties.insert("source_ids".to_string(), json!(source_ids_vec));

                // CRITICAL: Store source_chunk_ids for Local/Global query mode chunk retrieval
                properties.insert(
                    "source_chunk_ids".to_string(),
                    json!(&entity.source_chunk_ids),
                );
                // Add tenant scoping
                if let Some(ref tid) = tenant_id {
                    properties.insert("tenant_id".to_string(), json!(tid));
                }
                properties.insert("workspace_id".to_string(), json!(&workspace_id_meta));

                nodes_batch.push((entity.name.clone(), properties));
            }

            for relationship in &extraction.relationships {
                let mut properties = std::collections::HashMap::new();
                properties.insert(
                    "relation_type".to_string(),
                    json!(relationship.relation_type),
                );
                properties.insert("description".to_string(), json!(relationship.description));
                properties.insert("weight".to_string(), json!(relationship.weight));
                properties.insert("keywords".to_string(), json!(relationship.keywords));

                // OODA-07: Merge source_ids with existing edge (GAP-07 fix)
                let edge_key = (relationship.source.clone(), relationship.target.clone());
                let mut merged_sources: std::collections::HashSet<String> =
                    existing_edge_source_ids
                        .get(&edge_key)
                        .cloned()
                        .unwrap_or_default();
                merged_sources.insert(document_id.clone());
                let source_ids_vec: Vec<String> = merged_sources.into_iter().collect();
                properties.insert("source_ids".to_string(), json!(source_ids_vec));

                // CRITICAL: Store source_chunk_id for relationship chunk linkage
                if let Some(ref chunk_id) = relationship.source_chunk_id {
                    properties.insert("source_chunk_ids".to_string(), json!(vec![chunk_id]));
                }
                // Add tenant scoping
                if let Some(ref tid) = tenant_id {
                    properties.insert("tenant_id".to_string(), json!(tid));
                }
                properties.insert("workspace_id".to_string(), json!(&workspace_id_meta));

                edges_batch.push((
                    relationship.source.clone(),
                    relationship.target.clone(),
                    properties,
                ));
            }
        }

        // Track storage errors for reliable status reporting.
        // WHY: Previously these were warn-and-continue, causing silent data loss where
        // the document shows "completed" but entities/relationships are missing.
        let mut storage_errors: Vec<String> = Vec::new();

        // Batch upsert nodes
        if !nodes_batch.is_empty() {
            if let Err(e) = self.graph_storage.upsert_nodes_batch(&nodes_batch).await {
                let err_msg = format!(
                    "Failed to store {} entities in graph: {}",
                    nodes_batch.len(),
                    e
                );
                error!(document_id = %document_id, "{}", err_msg);
                storage_errors.push(err_msg);
            } else {
                info!("Batch stored {} entities", nodes_batch.len());
            }
        }

        // CRITICAL: Store entity embeddings in vector storage for query_local retrieval
        // FIX: Use workspace_vector_storage instead of self.vector_storage to avoid
        // dimension mismatch (768 vs 1536) when workspace uses different embedding model
        let mut entity_embedding_failures = 0u32;
        for extraction in &result.extractions {
            for entity in &extraction.entities {
                if let Some(embedding) = &entity.embedding {
                    let mut metadata = json!({
                        "type": "entity",
                        "entity_name": entity.name,
                        "entity_type": entity.entity_type,
                        "description": entity.description,
                        "document_id": document_id,
                        "source_chunk_ids": entity.source_chunk_ids,
                    });
                    if let Some(ref tid) = tenant_id {
                        metadata["tenant_id"] = json!(tid);
                    }
                    metadata["workspace_id"] = json!(&workspace_id_meta);

                    let entity_id = format!("entity:{}", entity.name);
                    if let Err(e) = workspace_vector_storage
                        .upsert(&[(entity_id.clone(), embedding.clone(), metadata)])
                        .await
                    {
                        warn!("Failed to store entity embedding {}: {}", entity_id, e);
                        entity_embedding_failures += 1;
                    }
                }
            }
        }
        if entity_embedding_failures > 0 {
            let err_msg = format!(
                "{} entity embeddings failed to store in vector DB",
                entity_embedding_failures
            );
            error!(document_id = %document_id, "{}", err_msg);
            storage_errors.push(err_msg);
        }

        // Batch upsert edges
        if !edges_batch.is_empty() {
            if let Err(e) = self.graph_storage.upsert_edges_batch(&edges_batch).await {
                let err_msg = format!(
                    "Failed to store {} relationships in graph: {}",
                    edges_batch.len(),
                    e
                );
                error!(document_id = %document_id, "{}", err_msg);
                storage_errors.push(err_msg);
            } else {
                info!("Batch stored {} relationships", edges_batch.len());
            }
        }

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
            // Extraction succeeded but storage partially failed
            let combined = storage_errors.join("; ");
            warn!(
                document_id = %document_id,
                storage_error_count = storage_errors.len(),
                "Storage errors during indexing — marking as partial_failure: {}",
                combined
            );
            // Append storage errors to stats so they are visible via API
            stats_with_lineage.error_details = Some(combined);
            "partial_failure"
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

        // Log success
        self.pipeline_state
            .document_processed(&document_id, result.stats.entity_count)
            .await;

        info!(
            "Document {} processed: {} chunks, {} entities, {} relationships",
            document_id,
            result.stats.chunk_count,
            result.stats.entity_count,
            result.stats.relationship_count
        );

        Ok(json!({
            "document_id": document_id,
            "chunk_count": result.stats.chunk_count,
            "entity_count": result.stats.entity_count,
            "relationship_count": result.stats.relationship_count,
        }))
    }
}
