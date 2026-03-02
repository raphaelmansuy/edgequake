//! Document processing entry points.
//!
//! Three processing modes with increasing resilience:
//! - [`Pipeline::process`]: Fail-fast on first extraction error
//! - [`Pipeline::process_with_progress`]: Fail-fast with progress callbacks
//! - [`Pipeline::process_with_resilience`]: Continue on chunk failures
//!
//! All three share common logic via helpers for embedding generation,
//! stats aggregation, and lineage building (DRY).

use futures::stream::{self, StreamExt};
use tokio_util::sync::CancellationToken;

use crate::error::Result;

use super::helpers::{aggregate_extraction_stats, link_extractions_to_chunks};
use super::{ChunkErrorInfo, ChunkProgressCallback, Pipeline, ProcessingResult};

impl Pipeline {
    /// Process a document through the pipeline.
    ///
    /// Uses fail-fast extraction: the first chunk error aborts all processing.
    pub async fn process(&self, document_id: &str, content: &str) -> Result<ProcessingResult> {
        let start = std::time::Instant::now();

        // Step 1: Chunk the document
        let mut chunks = self.chunker.chunk(content, document_id)?;
        let mut stats = self.init_chunk_stats(&chunks);

        // Step 2: Extract entities and relationships
        let mut extractions = Vec::new();
        if self.config.enable_entity_extraction || self.config.enable_relationship_extraction {
            if let Some(extractor) = &self.extractor {
                extractions = self.extract_parallel(&chunks, extractor).await?;
                link_extractions_to_chunks(&mut extractions);
                aggregate_extraction_stats(&extractions, extractor, &mut stats);
            }
        }

        // Step 3: Generate embeddings
        self.generate_all_embeddings(&mut chunks, &mut extractions, &mut stats)
            .await?;

        stats.processing_time_ms = start.elapsed().as_millis() as u64;

        // Step 4: Build lineage if enabled
        let lineage = self.build_lineage(document_id, &chunks, &extractions, &stats);

        Ok(ProcessingResult {
            document_id: document_id.to_string(),
            chunks,
            extractions,
            stats,
            lineage,
        })
    }

    /// Process a document with chunk-level progress callbacks.
    ///
    /// Identical to `process` but invokes the provided callback after each
    /// chunk is processed during entity extraction.
    ///
    /// ## Implements
    /// - **FEAT0019**: Chunk-level progress tracking
    /// - **UC2304**: System reports per-chunk progress during extraction
    ///
    /// ## Example
    /// ```ignore
    /// let callback = Arc::new(|update: ChunkProgressUpdate| {
    ///     println!("Chunk {}/{}: ETA {}s",
    ///         update.chunk_index + 1, update.total_chunks, update.eta_seconds);
    /// });
    /// let result = pipeline.process_with_progress("doc1", content, Some(callback)).await?;
    /// ```
    pub async fn process_with_progress(
        &self,
        document_id: &str,
        content: &str,
        progress_callback: Option<ChunkProgressCallback>,
    ) -> Result<ProcessingResult> {
        let start = std::time::Instant::now();

        // Step 1: Chunk the document
        let mut chunks = self.chunker.chunk(content, document_id)?;
        let mut stats = self.init_chunk_stats(&chunks);

        // Step 2: Extract entities and relationships WITH PROGRESS CALLBACK
        let mut extractions = Vec::new();
        if self.config.enable_entity_extraction || self.config.enable_relationship_extraction {
            if let Some(extractor) = &self.extractor {
                extractions = self
                    .extract_parallel_with_progress(&chunks, extractor, progress_callback)
                    .await?;
                link_extractions_to_chunks(&mut extractions);
                aggregate_extraction_stats(&extractions, extractor, &mut stats);
            }
        }

        // Step 3: Generate embeddings
        self.generate_all_embeddings(&mut chunks, &mut extractions, &mut stats)
            .await?;

        stats.processing_time_ms = start.elapsed().as_millis() as u64;

        // Step 4: Build lineage if enabled
        let lineage = self.build_lineage(document_id, &chunks, &extractions, &stats);

        Ok(ProcessingResult {
            document_id: document_id.to_string(),
            chunks,
            extractions,
            stats,
            lineage,
        })
    }

    // ═══════════════════════════════════════════════════════════════════════════
    //                    RESILIENT DOCUMENT PROCESSING
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // WHY PROCESS_WITH_RESILIENCE?
    // ────────────────────────────────
    // This method uses the resilient extraction strategy to ensure:
    // - Partial failures don't discard successful extractions
    // - Failed chunks are tracked for reporting and potential retry
    // - Users can see exactly which parts of their document were processed
    //
    // DECISION TREE:
    //   100% success → normal ProcessingResult
    //   Partial success → ProcessingResult with stats.chunk_errors populated
    //   0% success → Err(PipelineError::ExtractionError)

    /// Process a document with resilient chunk-level error handling.
    ///
    /// Unlike `process` and `process_with_progress`, this method does NOT fail
    /// the entire document if individual chunks fail. Instead:
    /// - If ALL chunks fail → returns `Err(PipelineError::ExtractionError)`
    /// - If SOME chunks fail → returns `Ok` with `stats.chunk_errors` populated
    /// - If ALL succeed → returns normal `Ok(ProcessingResult)`
    ///
    /// ## Implements
    /// - **FEAT0020**: Chunk-level resilience and error isolation
    /// - **UC2305**: System continues processing when individual chunks fail
    pub async fn process_with_resilience(
        &self,
        document_id: &str,
        content: &str,
        progress_callback: Option<ChunkProgressCallback>,
    ) -> Result<ProcessingResult> {
        self.process_with_resilience_cancellable(document_id, content, progress_callback, None)
            .await
    }

    /// Process a document with resilient chunk-level error handling and
    /// cooperative cancellation support.
    ///
    /// When a `cancel_token` is provided, new chunk extractions are skipped
    /// once the token is cancelled. Already in-flight LLM calls finish
    /// naturally  (cooperative, not preemptive).
    pub async fn process_with_resilience_cancellable(
        &self,
        document_id: &str,
        content: &str,
        progress_callback: Option<ChunkProgressCallback>,
        cancel_token: Option<CancellationToken>,
    ) -> Result<ProcessingResult> {
        let start = std::time::Instant::now();

        // Step 1: Chunk the document
        let mut chunks = self.chunker.chunk(content, document_id)?;
        let mut stats = self.init_chunk_stats(&chunks);

        // Step 2: Extract entities and relationships WITH RESILIENCE
        let mut extractions = Vec::new();
        if self.config.enable_entity_extraction || self.config.enable_relationship_extraction {
            if let Some(extractor) = &self.extractor {
                let resilient_result = self
                    .resilient_extract_parallel(
                        &chunks,
                        extractor,
                        progress_callback,
                        cancel_token.clone(),
                    )
                    .await;

                tracing::info!(
                    document_id = %document_id,
                    total_chunks = resilient_result.total_chunks,
                    successful = resilient_result.successful_extractions.len(),
                    failed = resilient_result.failed_chunks.len(),
                    success_rate = %format!("{:.1}%", resilient_result.success_rate() * 100.0),
                    "Resilient extraction completed"
                );

                // Handle complete failure
                if resilient_result.is_complete_failure() {
                    let failure_summary: Vec<String> = resilient_result
                        .failed_chunks
                        .iter()
                        .map(|f| format!("Chunk {}: {}", f.chunk_index, f.error))
                        .collect();

                    return Err(crate::error::PipelineError::ExtractionError(format!(
                        "All {} chunks failed extraction. Failures: {}",
                        resilient_result.total_chunks,
                        failure_summary.join("; ")
                    )));
                }

                // Populate failure stats
                stats.successful_chunks = resilient_result.successful_extractions.len();
                stats.failed_chunks = resilient_result.failed_chunks.len();

                if !resilient_result.failed_chunks.is_empty() {
                    stats.chunk_errors = Some(
                        resilient_result
                            .failed_chunks
                            .iter()
                            .map(|f| ChunkErrorInfo {
                                chunk_id: f.chunk_id.clone(),
                                chunk_index: f.chunk_index,
                                error_message: f.error.clone(),
                                was_timeout: f.was_timeout,
                                retry_attempts: f.retry_attempts,
                            })
                            .collect(),
                    );

                    tracing::warn!(
                        document_id = %document_id,
                        failed_count = resilient_result.failed_chunks.len(),
                        "Some chunks failed extraction, continuing with partial results"
                    );
                }

                extractions = resilient_result.successful_extractions;

                link_extractions_to_chunks(&mut extractions);
                aggregate_extraction_stats(&extractions, extractor, &mut stats);
            }
        }

        // Step 3: Generate embeddings
        self.generate_all_embeddings(&mut chunks, &mut extractions, &mut stats)
            .await?;

        stats.processing_time_ms = start.elapsed().as_millis() as u64;

        // Step 4: Build lineage if enabled
        let lineage = self.build_lineage(document_id, &chunks, &extractions, &stats);

        // ── Validation ──
        // FIX-2: Validate processing results before returning Ok
        if stats.chunk_count == 0 {
            return Err(crate::error::PipelineError::ChunkingError(
                "Document chunking produced 0 chunks - content may be empty or malformed"
                    .to_string(),
            ));
        }

        // FIX-RELIABILITY: Changed from hard error to warning.
        // WHY: 0 entities is NOT always a failure:
        //   1. Pipeline may have no extractor (test/mock mode)
        //   2. Document content may have no named entities
        //   3. Chunks are still valuable for semantic search via embeddings
        if stats.entity_count == 0 && stats.chunk_count > 0 {
            tracing::warn!(
                document_id = document_id,
                chunk_count = stats.chunk_count,
                successful_chunks = stats.successful_chunks,
                failed_chunks = stats.failed_chunks,
                has_extractor = self.extractor.is_some(),
                "Pipeline processed {} chunks but extracted 0 entities - document accepted with zero entities",
                stats.chunk_count
            );
            stats.error_details = Some(format!(
                "Extracted 0 entities from {} chunks ({} succeeded, {} failed). \
                 Document chunks are stored for semantic search.",
                stats.chunk_count, stats.successful_chunks, stats.failed_chunks
            ));
        }

        Ok(ProcessingResult {
            document_id: document_id.to_string(),
            chunks,
            extractions,
            stats,
            lineage,
        })
    }

    /// Process multiple documents in parallel.
    ///
    /// Uses concurrent processing with a configurable limit based on
    /// `max_concurrent_extractions` to process multiple documents simultaneously.
    pub async fn process_batch(
        &self,
        documents: &[(String, String)],
    ) -> Result<Vec<ProcessingResult>> {
        let max_concurrent_docs = self.config.max_concurrent_extractions.max(4);

        let futures: Vec<_> = documents
            .iter()
            .map(|(doc_id, content)| self.process(doc_id, content))
            .collect();

        let results: Vec<Result<ProcessingResult>> = stream::iter(futures)
            .buffer_unordered(max_concurrent_docs)
            .collect()
            .await;

        results.into_iter().collect()
    }
}
