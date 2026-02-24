use super::*;

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
        let metadata_key = format!("{}-metadata", document_id);

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

                if let Some(msg) = error_message {
                    updated.insert("error_message".to_string(), json!(msg));
                    updated.insert("stage_message".to_string(), json!(msg));
                }

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

            if let Some(msg) = error_message {
                new_metadata.insert("error_message".to_string(), json!(msg));
                new_metadata.insert("stage_message".to_string(), json!(msg));
            }

            json!(new_metadata)
        };

        self.kv_storage
            .upsert(&[(metadata_key, updated_json)])
            .await
            .map_err(|e| edgequake_tasks::TaskError::Storage(e.to_string()))?;

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
        let metadata_key = format!("{}-metadata", document_id);

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
            self.kv_storage
                .upsert(&[(metadata_key, json)])
                .await
                .map_err(|e| edgequake_tasks::TaskError::Storage(e.to_string()))?;
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
        let metadata_key = format!("{}-metadata", document_id);

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

                updated.remove("error_message");

                self.kv_storage
                    .upsert(&[(metadata_key, json!(updated))])
                    .await
                    .map_err(|e| edgequake_tasks::TaskError::Storage(e.to_string()))?;
            }
        }

        Ok(())
    }
}
