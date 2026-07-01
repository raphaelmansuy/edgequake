//! Document ingestion operations for EdgeQuake.
//!
//! Contains `insert()`, `insert_batch()`, and adaptive chunk size calculation.

use edgequake_pipeline::prompts::EntityExtractionSchema;
use edgequake_pipeline::{
    build_ingestion_pipeline, ChunkVectorBuildOptions, DefaultIngestionPersister,
    IngestionPersistContext, IngestionPersistSettings, IngestionPersister,
    IngestionPipelineOptions,
};

use crate::error::{Error, Result};
use crate::types::InsertResult;

use super::EdgeQuake;

/// Calculate adaptive chunk size based on document length.
///
/// WHY: Large documents need smaller chunks to avoid LLM timeouts and ensure reliable processing.
///
/// Based on LightRAG research:
/// - Default: 1200 tokens for normal documents
/// - Quality mode: 1500 tokens (maximum)
/// - Large documents: 600-800 tokens for better reliability
///
/// # Arguments
///
/// * `document_size_bytes` - Size of the document in bytes
///
/// # Returns
///
/// Recommended chunk size in tokens
///
/// # Examples
///
/// ```ignore
/// // Internal function - not part of public API
/// let chunk_size = calculate_adaptive_chunk_size(30_000);  // 30KB → 1200 tokens
/// let chunk_size = calculate_adaptive_chunk_size(80_000);  // 80KB → 800 tokens
/// let chunk_size = calculate_adaptive_chunk_size(200_000); // 200KB → 600 tokens
/// ```
/// @deprecated Use [`edgequake_pipeline::calculate_adaptive_chunk_size`] (SSOT).
#[allow(dead_code)]
fn calculate_adaptive_chunk_size(document_size_bytes: usize) -> usize {
    edgequake_pipeline::calculate_adaptive_chunk_size(document_size_bytes)
}

impl EdgeQuake {
    /// Insert a document for processing.
    ///
    /// # Implements
    ///
    /// - **FEAT0001**: Document Ingestion
    /// - **FEAT0002**: Text Chunking with Overlap
    /// - **FEAT0003**: LLM-Based Entity Extraction
    /// - **FEAT0005**: Knowledge Graph Construction
    /// - **FEAT0006**: Vector Embedding Generation
    ///
    /// # Enforces
    ///
    /// - **BR0001**: Document ID must be unique (error on duplicate)
    /// - **BR0002**: Chunk overlap < chunk size (validated in pipeline)
    /// - **BR0003**: Entity names normalized to UPPERCASE_UNDERSCORED
    ///
    /// # WHY: 3-Stage Pipeline Architecture
    ///
    /// The insert flow follows a 3-stage architecture (similar to LightRAG):
    ///
    /// 1. **Pipeline Processing** - Chunking → Entity Extraction → Embedding
    ///    - WHY chunks: LLM context windows are limited; chunks enable parallel processing
    ///    - WHY overlapping chunks: Entities spanning chunk boundaries are captured
    ///
    /// 2. **Vector Storage** - Store chunk embeddings for semantic search
    ///    - WHY first: this write is internally atomic (QW2 single-transaction
    ///      UNNEST), so writing it before the graph merge lets us cleanly roll it
    ///      back if the merge fails (see SC2 saga compensation below).
    ///    - WHY type metadata: Distinguish entity vectors from chunk vectors
    ///    - WHY tenant isolation: Multi-tenancy requires vector filtering
    ///
    /// 3. **Knowledge Graph Merge** - Deduplicate and merge into graph storage
    ///    - WHY last: the merge is the only remaining fallible step and is
    ///      idempotent/source-tracked, so a failed merge is safe to re-run.
    ///    - WHY merge instead of insert: Same entity may appear in multiple documents
    ///    - WHY LLM summarization: Merge conflicting descriptions intelligently
    ///    - WHY source tracking: Enable cascade delete when documents are removed
    ///
    /// # WHY: SC2 cross-store saga (no 2PC available)
    ///
    /// Vector and graph stores are independent `Arc<dyn _>` backends with no
    /// shared transaction. A true ACID boundary is impossible, so on graph-merge
    /// failure we COMPENSATE by deleting this document's chunk vectors,
    /// guaranteeing no orphaned, unreachable embeddings survive a partial write.
    ///
    /// # Arguments
    ///
    /// * `content` - Raw text content to process
    /// * `document_id` - Optional document ID; auto-generated UUID if not provided
    ///
    /// # Returns
    ///
    /// [`InsertResult`] with processing statistics (chunks, entities, relationships)
    ///
    /// # Errors
    ///
    /// - `Error::not_initialized` if EdgeQuake not initialized
    /// - `Error::internal` if pipeline or storage operations fail
    pub async fn insert(&self, content: &str, document_id: Option<&str>) -> Result<InsertResult> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        // Edge case: Empty document
        // WHY: Skip processing empty content to save resources
        let content_trimmed = content.trim();
        if content_trimmed.is_empty() {
            let doc_id = document_id
                .map(String::from)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            tracing::warn!(
                doc_id = %doc_id,
                "Skipping empty document - no content to process"
            );

            return Ok(InsertResult {
                document_id: doc_id,
                success: true,
                chunks_created: 0,
                entities_extracted: 0,
                relationships_extracted: 0,
                processing_time_ms: 0,
                error: None,
            });
        }

        // Edge case: Extremely large document — align with upload SSOT (SPEC-038 REQ-038-08)
        let max_document_size_bytes = crate::MAX_UPLOAD_BYTES;
        if content.len() > max_document_size_bytes {
            let doc_id = document_id
                .map(String::from)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            let size_mb = content.len() as f64 / (1024.0 * 1024.0);
            tracing::error!(
                doc_id = %doc_id,
                size_bytes = content.len(),
                size_mb = %format!("{:.2}", size_mb),
                max_size_mb = max_document_size_bytes / (1024 * 1024),
                "Document exceeds maximum size limit"
            );

            return Err(Error::validation(format!(
                "Document too large: {:.2}MB. Maximum allowed: {}MB. \
                Please split the document into smaller files.",
                size_mb,
                max_document_size_bytes / (1024 * 1024)
            )));
        }

        let doc_id = document_id
            .map(String::from)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let start = std::time::Instant::now();

        let doc_size_bytes = content.len();

        let llm = self
            .llm_provider
            .as_ref()
            .ok_or_else(|| Error::config("LLM provider not set"))?;

        let embedding = self
            .embedding_provider
            .as_ref()
            .ok_or_else(|| Error::config("Embedding provider not set"))?;

        let entity_schema = EntityExtractionSchema::with_types(self.config.entity_types.clone());
        let pipeline = build_ingestion_pipeline(
            llm.clone(),
            embedding.clone(),
            entity_schema,
            IngestionPipelineOptions::from_document_size(doc_size_bytes)
                .with_gleaning(self.config.enable_gleaning, self.config.max_gleaning),
        );

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        let vector_storage = self.resolve_ingestion_vector_storage().await?;

        // Stage 1: Process document through pipeline (Chunking → Extraction → Embedding)
        // WHY: Transforms raw text into structured knowledge graph elements
        let processing_result = pipeline
            .process(&doc_id, content)
            .await
            .map_err(|e| Error::internal(format!("Pipeline error: {}", e)))?;

        // ── Cross-store write ordering & saga compensation (SC2 / finding F4) ──
        //
        // WHY this matters: a single document is persisted across TWO independent
        // backends — the vector store (chunk embeddings) and the graph store
        // (entities + relationships). They are separate `Arc<dyn _>` trait objects
        // with NO shared connection or transaction handle, and at least one
        // backend (the in-memory adapter) has no transactional semantics at all.
        // A true cross-store ACID/2PC boundary is therefore architecturally
        // impossible here without a distributed transaction coordinator we do not
        // have.
        //
        // WHAT we do instead — a bounded, deterministic SAGA:
        //   1. Write chunk vectors FIRST. This write is internally atomic (QW2
        //      performs a single chunked-UNNEST transaction: all chunk rows for
        //      the document commit together, or none do).
        //   2. Run the graph merge LAST. The graph MERGE is idempotent and
        //      source-tracked, so a failed/partial merge is safe to re-run or to
        //      clean up via normal document deletion.
        //   3. If the merge fails, COMPENSATE by deleting exactly this document's
        //      chunk vectors (uniquely owned by `doc_id`) and emit a structured
        //      quarantine log, then surface the error.
        //
        // Net effect vs. the previous "graph-first, vectors-second, no cleanup"
        // ordering: orphaned chunk vectors become impossible (they are written
        // immediately before the only remaining fallible step and are actively
        // removed on failure), shrinking the orphan class to the idempotent,
        // re-runnable graph side.

        // Stage 2+3: SPEC-021 P-G2 — delegate chunk vectors + graph merge to the
        // shared persister (same sequence as processor `text_insert`).
        let llm = self
            .llm_provider
            .as_ref()
            .ok_or_else(|| Error::not_initialized("LLM provider not initialized"))?;

        let persist_config = IngestionPersistSettings {
            use_llm_summarization: self.config.use_llm_summarization,
        };

        let persist_ctx = IngestionPersistContext::new(
            doc_id.clone(),
            self.config.tenant_id.clone(),
            self.config.workspace_id.clone(),
        );

        let kv_storage = self
            .kv_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("KV storage not initialized"))?
            .clone();

        let persister = DefaultIngestionPersister::from_settings(
            graph_storage.clone(),
            vector_storage,
            persist_config,
            self.relational_sink.clone(),
            Some(llm.clone()),
            Some(kv_storage),
        );

        let persist_out = persister
            .persist(
                &persist_ctx,
                &processing_result,
                ChunkVectorBuildOptions::STANDARD,
            )
            .await
            .map_err(|e| Error::internal(format!("Persistence failed: {}", e)))?;

        if let Some(engine) = &self.query_engine {
            use edgequake_query::QueryResultCacheInvalidator;
            if let Some(workspace_id) = &self.config.workspace_id {
                engine.invalidate_query_result_cache_for_workspace(workspace_id);
            } else {
                engine.invalidate_query_result_cache();
            }
        }

        let merge_stats = persist_out.merge_stats;

        let processing_time_ms = start.elapsed().as_millis() as u64;

        Ok(InsertResult {
            document_id: doc_id,
            success: true,
            chunks_created: processing_result.stats.chunk_count,
            entities_extracted: merge_stats.entities_created + merge_stats.entities_updated,
            relationships_extracted: merge_stats.relationships_created
                + merge_stats.relationships_updated,
            processing_time_ms,
            error: None,
        })
    }

    /// Insert multiple documents.
    ///
    /// SC5: documents are ingested with bounded concurrency while results are
    /// returned in input order. A single failing document no longer aborts the
    /// whole batch (previously a `?` would bubble the first error and discard
    /// every other document's work); instead each failure is captured as an
    /// `InsertResult { success: false, error: Some(..) }` so callers can do
    /// partial-success accounting.
    ///
    /// WHY bounded concurrency: ingestion is dominated by LLM/embedding round
    /// trips, so processing a few documents in parallel hides that latency, but
    /// an unbounded fan-out would overwhelm provider rate limits and the DB
    /// pool. The limit is read from `EDGEQUAKE_INGEST_CONCURRENCY` (default 4).
    pub async fn insert_batch(
        &self,
        documents: Vec<(&str, Option<&str>)>,
    ) -> Result<Vec<InsertResult>> {
        use futures::stream::{self, StreamExt};

        let concurrency = std::env::var("EDGEQUAKE_INGEST_CONCURRENCY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|&n| n > 0)
            .unwrap_or(4);

        // `buffered` polls up to `concurrency` futures at once but yields their
        // outputs in the original stream order, so result[i] always maps to
        // documents[i] regardless of which finished first.
        let results = stream::iter(documents)
            .map(|(content, doc_id)| async move {
                match self.insert(content, doc_id).await {
                    Ok(result) => result,
                    Err(e) => InsertResult {
                        // Preserve the caller-supplied id when known so the
                        // failure can be correlated to its source document.
                        document_id: doc_id.map(|s| s.to_string()).unwrap_or_default(),
                        success: false,
                        chunks_created: 0,
                        entities_extracted: 0,
                        relationships_extracted: 0,
                        processing_time_ms: 0,
                        error: Some(e.to_string()),
                    },
                }
            })
            .buffered(concurrency)
            .collect::<Vec<_>>()
            .await;

        Ok(results)
    }
}
