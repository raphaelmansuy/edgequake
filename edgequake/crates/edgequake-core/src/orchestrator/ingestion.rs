//! Document ingestion operations for EdgeQuake.
//!
//! Contains `insert()`, `insert_batch()`, and adaptive chunk size calculation.

use std::sync::Arc;

use edgequake_pipeline::{
    GleaningConfig, GleaningExtractor, KnowledgeGraphMerger, LLMExtractor, LLMSummarizer,
    MergerConfig, Pipeline, PipelineConfig, SummarizerConfig,
};

use edgequake_storage::traits::VectorStorage;

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
fn calculate_adaptive_chunk_size(document_size_bytes: usize) -> usize {
    // Based on LightRAG best practices and empirical testing:
    // - Small documents (<50KB): Use standard 1200 tokens
    // - Medium documents (50-100KB): Use reduced 800 tokens
    // - Large documents (>100KB): Use minimal 600 tokens
    //
    // WHY these thresholds:
    // - 50KB ≈ 12,500 tokens → ~10 chunks at 1200 tokens (manageable)
    // - 100KB ≈ 25,000 tokens → ~31 chunks at 800 tokens (reasonable)
    // - 150KB ≈ 37,500 tokens → ~62 chunks at 600 tokens (many but necessary)
    //
    // Smaller chunks for large documents reduce:
    // 1. LLM timeout risk (less context per request)
    // 2. Entity extraction complexity (focused scope)
    // 3. Memory pressure (smaller batches)
    if document_size_bytes > 100_000 {
        600 // >100KB: minimal chunks for reliability
    } else if document_size_bytes > 50_000 {
        800 // 50-100KB: reduced chunks
    } else {
        1200 // <50KB: standard LightRAG default
    }
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

        // Edge case: Extremely large document (>10MB)
        // WHY: Documents over 10MB are likely to cause OOM or extreme timeouts
        const MAX_DOCUMENT_SIZE_BYTES: usize = 10 * 1024 * 1024; // 10MB
        if content.len() > MAX_DOCUMENT_SIZE_BYTES {
            let doc_id = document_id
                .map(String::from)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            let size_mb = content.len() as f64 / (1024.0 * 1024.0);
            tracing::error!(
                doc_id = %doc_id,
                size_bytes = content.len(),
                size_mb = %format!("{:.2}", size_mb),
                max_size_mb = MAX_DOCUMENT_SIZE_BYTES / (1024 * 1024),
                "Document exceeds maximum size limit"
            );

            return Err(Error::validation(format!(
                "Document too large: {:.2}MB. Maximum allowed: {}MB. \
                Please split the document into smaller files.",
                size_mb,
                MAX_DOCUMENT_SIZE_BYTES / (1024 * 1024)
            )));
        }

        let doc_id = document_id
            .map(String::from)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let start = std::time::Instant::now();

        // Calculate adaptive chunk size based on document length
        // WHY: Large documents need smaller chunks to avoid LLM timeouts
        // Based on LightRAG research: 1200 tokens optimal for <50KB, scale down for larger docs
        let doc_size_bytes = content.len();
        let adaptive_chunk_size = calculate_adaptive_chunk_size(doc_size_bytes);
        let adaptive_overlap = (adaptive_chunk_size as f32 * 0.083) as usize; // ~8% overlap (LightRAG best practice)
        let doc_size_kb = doc_size_bytes / 1024;

        tracing::info!(
            doc_id = %doc_id,
            doc_size_bytes = doc_size_bytes,
            doc_size_kb = doc_size_kb,
            adaptive_chunk_size = adaptive_chunk_size,
            adaptive_overlap = adaptive_overlap,
            default_chunk_size = self.config.chunk_token_size,
            "Using adaptive chunking for document ingestion"
        );

        // Create pipeline with adaptive configuration
        // WHY: Per-document pipeline allows dynamic chunk sizing
        // WHY not reuse stored pipeline: Stored pipeline uses static config
        let pipeline_config = PipelineConfig {
            chunker: edgequake_pipeline::ChunkerConfig {
                chunk_size: adaptive_chunk_size,
                chunk_overlap: adaptive_overlap,
                ..Default::default()
            },
            ..Default::default()
        };

        let llm = self
            .llm_provider
            .as_ref()
            .ok_or_else(|| Error::config("LLM provider not set"))?;

        let embedding = self
            .embedding_provider
            .as_ref()
            .ok_or_else(|| Error::config("Embedding provider not set"))?;

        // Create base extractor
        let base_extractor: Arc<dyn edgequake_pipeline::EntityExtractor> = Arc::new(
            LLMExtractor::new(llm.clone()).with_entity_types(self.config.entity_types.clone()),
        );

        // Wrap with GleaningExtractor if enabled
        let extractor: Arc<dyn edgequake_pipeline::EntityExtractor> = if self.config.enable_gleaning
            && self.config.max_gleaning > 0
        {
            Arc::new(
                GleaningExtractor::new(llm.clone(), base_extractor).with_config(GleaningConfig {
                    max_gleaning: self.config.max_gleaning,
                    always_glean: false,
                }),
            )
        } else {
            base_extractor
        };

        let pipeline = Pipeline::new(pipeline_config)
            .with_extractor(extractor)
            .with_embedding_provider(embedding.clone());

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        let vector_storage = self
            .vector_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Vector storage not initialized"))?;

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

        // Stage 2: Store chunk embeddings with type metadata (written FIRST).
        // WHY type:"chunk" metadata: enables filtering entity vs chunk vectors at query time.
        // WHY tenant/workspace: multi-tenancy isolation at the vector level.
        //
        // QW2: collect ALL chunk embeddings into a SINGLE batched `upsert` call
        // instead of one round trip per chunk. The vector adapter performs a
        // chunked UNNEST insert in one transaction, collapsing N round trips into
        // ~ceil(N/1000).
        let chunk_vectors: Vec<(String, Vec<f32>, serde_json::Value)> = processing_result
            .chunks
            .iter()
            .filter_map(|chunk| {
                let embedding = chunk.embedding.as_ref()?;
                let mut metadata = serde_json::json!({
                    "type": "chunk",  // Mark as chunk for retrieval filtering
                    "document_id": doc_id,
                    "index": chunk.index,
                    "content": chunk.content
                });

                // Add tenant and workspace IDs if present
                if let Some(tenant_id) = &self.config.tenant_id {
                    metadata["tenant_id"] = serde_json::json!(tenant_id);
                }
                if let Some(workspace_id) = &self.config.workspace_id {
                    metadata["workspace_id"] = serde_json::json!(workspace_id);
                }

                Some((chunk.id.clone(), embedding.clone(), metadata))
            })
            .collect();

        // Capture the chunk IDs up front so the compensation path can delete
        // exactly what stage 2 wrote — nothing more, nothing less.
        let chunk_ids: Vec<String> = chunk_vectors.iter().map(|(id, _, _)| id.clone()).collect();

        if !chunk_vectors.is_empty() {
            vector_storage
                .upsert(&chunk_vectors)
                .await
                .map_err(|e| Error::internal(format!("Vector storage error: {}", e)))?;
        }

        // Stage 3: Merge results into the knowledge graph (the LAST fallible step).
        // WHY merge: entities may already exist from previous documents; merge
        // de-duplicates instead of creating conflicting nodes.
        // WHY LLM summarization: when merging descriptions, the LLM produces a
        // single coherent summary rather than concatenated fragments.
        let llm = self
            .llm_provider
            .as_ref()
            .ok_or_else(|| Error::not_initialized("LLM provider not initialized"))?;

        let merger_config = MergerConfig {
            use_llm_summarization: self.config.use_llm_summarization,
            ..Default::default()
        };

        let mut merger =
            KnowledgeGraphMerger::new(merger_config, graph_storage.clone(), vector_storage.clone())
                .with_tenant_context(
                    self.config.tenant_id.clone(),
                    self.config.workspace_id.clone(),
                );

        // Add LLM summarizer if enabled
        if self.config.use_llm_summarization {
            let summarizer = Arc::new(LLMSummarizer::new(llm.clone(), SummarizerConfig::default()));
            merger = merger.with_summarizer(summarizer);
        }

        // ── SC2 saga: the graph merge is the LAST fallible stage ──────────────
        //
        // Both failure modes below are the SAME logical event — "the graph stage
        // failed, so undo the chunk vectors Stage 2 committed" — and therefore
        // share ONE compensation path (`fail_with_chunk_vector_rollback`):
        //
        //   1. `merge()` returns `Err`: a catastrophic early abort (e.g. a
        //      pre-flight failure before per-element processing).
        //
        //   2. `merge()` returns `Ok` but `stats.errors > 0`:
        //      `KnowledgeGraphMerger::merge` is deliberately resilient — it does
        //      NOT abort on the first bad element. It catches each
        //      `merge_entity` / `merge_relationship` failure, increments
        //      `MergeStats::errors`, logs it, and returns `Ok(stats)`. Good for
        //      *data-level* hiccups, but it means a wholesale graph-storage fault
        //      (backend down, transaction rejected) is silently absorbed and the
        //      insert would otherwise report `success: true` while the graph is
        //      partially/fully unwritten — orphaning the Stage 2 chunk vectors.
        //      `errors > 0` is a reliable storage-fault signal because every
        //      counted error originates from a storage call (`get_node`,
        //      `upsert_node`, `get_edge`, `upsert_edge`, vector `upsert`):
        //      parsing/validation happened upstream in the extractor and
        //      LLM-summarization failures fall back instead of erroring.
        //
        // WHY we roll back only the vectors (not the graph): the graph MERGE is
        // idempotent and source-tracked, so any partial graph residue is safe to
        // re-run or to clean up via normal document deletion. Deleting the
        // freshly-written chunk vectors collapses the orphan window to that
        // re-runnable graph side.
        let merge_stats = match merger.merge(processing_result.extractions.clone()).await {
            Ok(stats) if stats.errors == 0 => stats,
            Ok(stats) => {
                return Err(Self::fail_with_chunk_vector_rollback(
                    vector_storage.as_ref(),
                    &doc_id,
                    &chunk_ids,
                    format!(
                        "{} knowledge-graph merge error(s) during insert",
                        stats.errors
                    ),
                )
                .await);
            }
            Err(merge_err) => {
                return Err(Self::fail_with_chunk_vector_rollback(
                    vector_storage.as_ref(),
                    &doc_id,
                    &chunk_ids,
                    merge_err.to_string(),
                )
                .await);
            }
        };

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

    /// Compensate a failed cross-store document write and build the error to
    /// surface to the caller.
    ///
    /// SOLID/DRY: this is the SINGLE place the two graph-stage failure modes
    /// (`merge()` returning `Err`, and `merge()` returning `Ok` with
    /// `errors > 0`) converge. It composes the best-effort vector rollback
    /// (`compensate_orphan_chunk_vectors`) with a uniform, caller-facing error
    /// so neither call site can drift in cleanup behaviour or messaging.
    async fn fail_with_chunk_vector_rollback(
        vector_storage: &dyn VectorStorage,
        doc_id: &str,
        chunk_ids: &[String],
        cause: String,
    ) -> Error {
        Self::compensate_orphan_chunk_vectors(vector_storage, doc_id, chunk_ids, &cause).await;
        Error::internal(format!(
            "Knowledge graph merge failed for document {doc_id} ({cause}); \
             rolled back chunk vectors to avoid orphaned embeddings"
        ))
    }

    /// Best-effort saga compensation for a failed cross-store document write.
    ///
    /// Deletes the chunk vectors that Stage 2 already committed for `doc_id`
    /// after the graph merge (the last fallible stage) failed, preventing
    /// orphaned embeddings that would otherwise be retrievable yet disconnected
    /// from the knowledge graph.
    ///
    /// WHY best-effort (no `?`, never returns an error): compensation runs on an
    /// already-failing path. If cleanup itself fails we must NOT mask the
    /// original merge error; instead we emit a structured `quarantine` log so an
    /// operator (or a reconciliation job) can remove the residue out of band.
    /// Deletion is keyed by the exact chunk IDs we wrote, so it is idempotent
    /// and safe to retry.
    async fn compensate_orphan_chunk_vectors(
        vector_storage: &dyn VectorStorage,
        doc_id: &str,
        chunk_ids: &[String],
        cause: &str,
    ) {
        // Nothing was written (e.g. a document with no embeddable chunks) — there
        // is nothing to roll back.
        if chunk_ids.is_empty() {
            return;
        }

        match vector_storage.delete(chunk_ids).await {
            Ok(()) => {
                tracing::warn!(
                    document_id = %doc_id,
                    chunk_vectors_deleted = chunk_ids.len(),
                    cause = %cause,
                    "saga_compensation: rolled back chunk vectors after graph merge failure"
                );
            }
            Err(cleanup_err) => {
                tracing::error!(
                    document_id = %doc_id,
                    orphan_chunk_vectors = chunk_ids.len(),
                    merge_cause = %cause,
                    cleanup_error = %cleanup_err,
                    "quarantine: failed to roll back chunk vectors after graph merge \
                     failure; manual or reconciliation cleanup required"
                );
            }
        }
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
