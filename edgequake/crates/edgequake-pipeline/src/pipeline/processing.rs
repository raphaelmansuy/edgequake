//! Document processing entry points.
//!
//! Three processing modes with increasing resilience:
//! - [`Pipeline::process`]: Fail-fast on first extraction error
//! - [`Pipeline::process_with_progress`]: Fail-fast with progress callbacks
//! - [`Pipeline::process_with_resilience`]: Continue on chunk failures
//!
//! All three share common logic via helpers for embedding generation,
//! stats aggregation, and lineage building (DRY).

use std::future::Future;
use std::time::Instant;

use futures::stream::{self, StreamExt};
use tokio_util::sync::CancellationToken;

use crate::chunker::TextChunk;
use crate::error::Result;
use crate::extractor::ExtractionResult;

use super::helpers::{aggregate_extraction_stats, link_extractions_to_chunks};
use super::{
    ChunkErrorInfo, ChunkProgressCallback, EmbedProgressCallback, Pipeline, ProcessingResult,
    ProcessingStats,
};

impl Pipeline {
    /// SPEC-124: one ingest.document root + I/O for all process* entry points.
    async fn run_under_ingest_root<F, Fut>(
        document_id: &str,
        content: &str,
        work: F,
    ) -> Result<ProcessingResult>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<ProcessingResult>>,
    {
        edgequake_observability::with_ingest_document_span(async {
            edgequake_observability::record_ingest_document_input(document_id, content);
            let result = work().await?;
            edgequake_observability::record_ingest_document_output(
                result.stats.chunk_count,
                result.stats.entity_count,
                result.stats.relationship_count,
                result.stats.successful_chunks,
                result.stats.failed_chunks,
            );
            Ok(result)
        })
        .await
    }

    /// LAW-124-22: chunking observation + KG meta (strategy/size/overlap).
    async fn chunk_under_span(&self, content: &str, document_id: &str) -> Result<Vec<TextChunk>> {
        edgequake_observability::with_pipeline_stage_span("ingest.chunking", async {
            let chunks = self.chunker.chunk_async(content, document_id).await?;
            let mm_sidecar_appended = content.contains("<!-- multimodal-chunks -->");
            let budget = self.config.chunker.chunk_size;
            let (input, output, dist) = crate::ingest_chunking_observation_full(
                content.len(),
                chunks.iter().map(|c| (c.token_count, c.content.as_str())),
                Some(budget),
                mm_sidecar_appended,
            );
            if budget > 0 {
                let fill_p50 = dist.token_p50 as f64 / budget as f64;
                if fill_p50 < 0.4 {
                    let doc_tokens = crate::token_estimator::count_tokens(content);
                    if doc_tokens >= 8000 {
                        tracing::warn!(
                            fill_p50,
                            doc_tokens,
                            budget,
                            "SPEC-135 underfill (fail-open): fill_p50 < 0.4 on large doc"
                        );
                    }
                }
            }
            edgequake_observability::record_structured_io(Some(&input), Some(&output));
            edgequake_observability::record_ingest_kg_meta(edgequake_observability::IngestKgMeta {
                chunk_strategy: Some(self.config.chunk_strategy.as_str().to_string()),
                chunk_size: Some(self.config.chunker.chunk_size),
                overlap: Some(self.config.chunker.chunk_overlap),
                gleaning_max: None,
                embed_model: self
                    .embedding_provider
                    .as_ref()
                    .map(|p| p.model().to_string()),
                embed_dim: None,
                extract_entity_cap: None,
                token_min: Some(dist.token_min),
                token_p50: Some(dist.token_p50),
                token_max: Some(dist.token_max),
                orphan_heading_chunks: Some(dist.orphan_heading_chunks),
                fill_p50: (budget > 0).then_some(dist.token_p50 as f64 / budget as f64),
                mm_sidecar_appended: Some(mm_sidecar_appended),
            });
            Ok(chunks)
        })
        .await
    }

    /// Shared tail: link extractions, embed, build lineage (SPEC-017 ISP dedupe).
    #[allow(clippy::too_many_arguments)]
    async fn finish_document_processing(
        &self,
        document_id: &str,
        start: Instant,
        mut chunks: Vec<TextChunk>,
        mut extractions: Vec<ExtractionResult>,
        mut stats: ProcessingStats,
        embed_progress: Option<&EmbedProgressCallback>,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<ProcessingResult> {
        if self.config.enable_entity_extraction || self.config.enable_relationship_extraction {
            if let Some(extractor) = &self.extractor {
                link_extractions_to_chunks(&mut extractions, document_id);
                aggregate_extraction_stats(&extractions, extractor, &mut stats);
            }
        }

        self.generate_all_embeddings(
            &mut chunks,
            &mut extractions,
            &mut stats,
            embed_progress,
            cancel_token,
        )
        .await?;

        stats.processing_time_ms = start.elapsed().as_millis() as u64;
        let lineage = self.build_lineage(document_id, &chunks, &extractions, &stats);

        Ok(ProcessingResult {
            document_id: document_id.to_string(),
            chunks,
            extractions,
            stats,
            lineage,
        })
    }

    /// Process a document through the pipeline.
    ///
    /// Uses fail-fast extraction: the first chunk error aborts all processing.
    pub async fn process(&self, document_id: &str, content: &str) -> Result<ProcessingResult> {
        Self::run_under_ingest_root(document_id, content, || async {
            let start = Instant::now();

            let chunks = self.chunk_under_span(content, document_id).await?;
            let stats = self.init_chunk_stats(&chunks);

            let mut extractions = Vec::new();
            if self.config.enable_entity_extraction || self.config.enable_relationship_extraction {
                if let Some(extractor) = &self.extractor {
                    extractions = self.extract_parallel(&chunks, extractor).await?;
                }
            }

            self.finish_document_processing(
                document_id,
                start,
                chunks,
                extractions,
                stats,
                None,
                None,
            )
            .await
        })
        .await
    }

    /// Process a document with chunk-level progress callbacks.
    pub async fn process_with_progress(
        &self,
        document_id: &str,
        content: &str,
        progress_callback: Option<ChunkProgressCallback>,
    ) -> Result<ProcessingResult> {
        Self::run_under_ingest_root(document_id, content, || async {
            let start = Instant::now();

            let chunks = self.chunk_under_span(content, document_id).await?;
            let stats = self.init_chunk_stats(&chunks);

            let mut extractions = Vec::new();
            if self.config.enable_entity_extraction || self.config.enable_relationship_extraction {
                if let Some(extractor) = &self.extractor {
                    extractions = self
                        .extract_parallel_with_progress(&chunks, extractor, progress_callback)
                        .await?;
                }
            }

            self.finish_document_processing(
                document_id,
                start,
                chunks,
                extractions,
                stats,
                None,
                None,
            )
            .await
        })
        .await
    }

    /// Process a document with resilient chunk-level error handling.
    pub async fn process_with_resilience(
        &self,
        document_id: &str,
        content: &str,
        progress_callback: Option<ChunkProgressCallback>,
    ) -> Result<ProcessingResult> {
        self.process_with_resilience_cancellable(
            document_id,
            content,
            progress_callback,
            None,
            None,
            None,
            None,
        )
        .await
    }

    /// Process a document with resilient chunk-level error handling and
    /// cooperative cancellation support.
    ///
    /// `resume_by_chunk_id`: skip LLM for chunks already extracted (mid-doc resume).
    /// `on_chunk_extracted`: durable per-chunk checkpoint hook after each success.
    #[allow(clippy::too_many_arguments)]
    pub async fn process_with_resilience_cancellable(
        &self,
        document_id: &str,
        content: &str,
        progress_callback: Option<ChunkProgressCallback>,
        cancel_token: Option<CancellationToken>,
        embed_progress: Option<EmbedProgressCallback>,
        resume_by_chunk_id: Option<
            std::collections::HashMap<String, crate::extractor::ExtractionResult>,
        >,
        on_chunk_extracted: Option<crate::pipeline::types::ChunkExtractedCallback>,
    ) -> Result<ProcessingResult> {
        Self::run_under_ingest_root(document_id, content, || async {
            let start = Instant::now();

            let chunks = self.chunk_under_span(content, document_id).await?;
            let mut stats = self.init_chunk_stats(&chunks);

            let mut extractions = Vec::new();
            if self.config.enable_entity_extraction || self.config.enable_relationship_extraction {
                if let Some(extractor) = &self.extractor {
                    let resilient_result = self
                        .resilient_extract_parallel(
                            &chunks,
                            extractor,
                            progress_callback,
                            cancel_token.clone(),
                            resume_by_chunk_id,
                            on_chunk_extracted,
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
                }
            }

            let mut result = self
                .finish_document_processing(
                    document_id,
                    start,
                    chunks,
                    extractions,
                    stats,
                    embed_progress.as_ref(),
                    cancel_token.as_ref(),
                )
                .await?;

            if result.stats.chunk_count == 0 {
                return Err(crate::error::PipelineError::ChunkingError(
                    "Document chunking produced 0 chunks - content may be empty or malformed"
                        .to_string(),
                ));
            }

            if result.stats.entity_count == 0 && result.stats.chunk_count > 0 {
                tracing::warn!(
                    document_id = document_id,
                    chunk_count = result.stats.chunk_count,
                    successful_chunks = result.stats.successful_chunks,
                    failed_chunks = result.stats.failed_chunks,
                    has_extractor = self.extractor.is_some(),
                    "Pipeline processed {} chunks but extracted 0 entities - document accepted with zero entities",
                    result.stats.chunk_count
                );
                result.stats.error_details = Some(format!(
                    "Extracted 0 entities from {} chunks ({} succeeded, {} failed). \
                     Document chunks are stored for semantic search.",
                    result.stats.chunk_count,
                    result.stats.successful_chunks,
                    result.stats.failed_chunks
                ));
            }

            Ok(result)
        })
        .await
    }

    /// Process multiple documents in parallel.
    pub async fn process_batch(
        &self,
        documents: &[(String, String)],
    ) -> Result<Vec<ProcessingResult>> {
        // Use clamped extract concurrency as-is — no artificial floor-of-4
        // (local Ollama profiles set 1–2; a floor of 4 re-introduced fan-out storms).
        let max_concurrent_docs = self.config.max_concurrent_extractions.max(1);

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

#[cfg(test)]
mod spec124_ingest_stages {
    #[test]
    fn processing_source_wraps_chunking_and_kg_meta() {
        let src = include_str!("processing.rs");
        let prod = src
            .split("mod spec124_ingest_stages")
            .next()
            .expect("production source");
        assert!(
            prod.contains("ingest.chunking"),
            "processing.rs must emit ingest.chunking"
        );
        assert!(
            prod.contains("record_ingest_kg_meta"),
            "processing.rs must record IngestKgMeta via SSOT"
        );
        assert!(
            prod.contains("chunk_under_span"),
            "chunk_async must go through chunk_under_span"
        );
        assert!(
            prod.contains("ingest_chunking_observation"),
            "ingest.chunking I/O must use ChunkTokenStats SSOT"
        );
    }
}
