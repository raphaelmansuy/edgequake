//! Parallel and resilient chunk extraction.
//!
//! ## Implements
//! - **FEAT0019**: Chunk-level progress tracking with callbacks
//! - **FEAT0020**: Chunk-level resilience and error isolation

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use futures::stream::{self, StreamExt};
use tokio_util::sync::CancellationToken;

use crate::error::Result;
use crate::extractor::EntityExtractor;

use super::{ChunkProgressCallback, ChunkProgressUpdate, Pipeline};

impl Pipeline {
    /// Extract entities from chunks in parallel using a semaphore.
    pub(super) async fn extract_parallel(
        &self,
        chunks: &[crate::chunker::TextChunk],
        extractor: &Arc<dyn EntityExtractor>,
    ) -> Result<Vec<crate::extractor::ExtractionResult>> {
        // Delegate to extract_parallel_with_progress with no callback
        self.extract_parallel_with_progress(chunks, extractor, None)
            .await
    }

    /// Extract entities from chunks in parallel with optional progress callback.
    ///
    /// ## Implements
    /// - **FEAT0019**: Chunk-level progress tracking
    /// - **UC2304**: System reports per-chunk progress during extraction
    pub(super) async fn extract_parallel_with_progress(
        &self,
        chunks: &[crate::chunker::TextChunk],
        extractor: &Arc<dyn EntityExtractor>,
        progress_callback: Option<ChunkProgressCallback>,
    ) -> Result<Vec<crate::extractor::ExtractionResult>> {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(
            self.config.max_concurrent_extractions,
        ));

        let total_chunks = chunks.len();

        // Atomic counters for cumulative tracking across concurrent extractions
        let cumulative_time_ms = Arc::new(AtomicU64::new(0));
        let cumulative_input_tokens = Arc::new(AtomicU64::new(0));
        let cumulative_output_tokens = Arc::new(AtomicU64::new(0));
        let completed_chunks = Arc::new(AtomicU32::new(0));

        // Get model pricing for cost calculation
        let pricing = crate::progress::default_model_pricing();
        let model_name = extractor.model_name();
        let model_pricing = pricing
            .get(model_name)
            .cloned()
            .unwrap_or_else(|| crate::progress::ModelPricing::new("gpt-4.1-nano", 0.00015, 0.0006));
        let model_pricing = Arc::new(model_pricing);

        // Create futures for all chunks with progress tracking
        let futures: Vec<_> = chunks
            .iter()
            .enumerate()
            .map(|(chunk_index, chunk)| {
                let semaphore = semaphore.clone();
                let extractor = extractor.clone();
                let chunk = chunk.clone();
                let progress_callback = progress_callback.clone();
                let cumulative_time_ms = cumulative_time_ms.clone();
                let cumulative_input_tokens = cumulative_input_tokens.clone();
                let cumulative_output_tokens = cumulative_output_tokens.clone();
                let completed_chunks = completed_chunks.clone();
                let model_pricing = model_pricing.clone();

                async move {
                    // Acquire permit (released on drop)
                    let _permit = semaphore
                        .acquire()
                        .await
                        .map_err(|e| crate::error::PipelineError::ExtractionError(e.to_string()))?;

                    // Extract entities from this chunk
                    let result = extractor.extract(&chunk).await?;

                    // Update cumulative counters
                    let time_ms = result.extraction_time_ms;
                    let in_tokens = result.input_tokens;
                    let out_tokens = result.output_tokens;

                    cumulative_time_ms.fetch_add(time_ms, Ordering::Relaxed);
                    cumulative_input_tokens.fetch_add(in_tokens as u64, Ordering::Relaxed);
                    cumulative_output_tokens.fetch_add(out_tokens as u64, Ordering::Relaxed);
                    let completed = completed_chunks.fetch_add(1, Ordering::Relaxed) + 1;

                    // Calculate cost for this chunk
                    let chunk_cost = model_pricing.calculate_cost(in_tokens, out_tokens);

                    // Emit progress update if callback is provided
                    if let Some(ref callback) = progress_callback {
                        let total_time = cumulative_time_ms.load(Ordering::Relaxed);
                        let total_in = cumulative_input_tokens.load(Ordering::Relaxed);
                        let total_out = cumulative_output_tokens.load(Ordering::Relaxed);

                        // Calculate average time per chunk and ETA
                        let avg_time_ms = if completed > 0 {
                            total_time as f64 / completed as f64
                        } else {
                            0.0
                        };
                        let remaining = total_chunks.saturating_sub(completed as usize);
                        let eta_seconds = ((avg_time_ms * remaining as f64) / 1000.0) as u64;

                        // Calculate cumulative cost
                        let cumulative_cost =
                            model_pricing.calculate_cost(total_in as usize, total_out as usize);

                        // Truncate chunk preview to 100 chars (OODA-02: Fixed UTF-8 char boundary panic)
                        let chunk_preview = if chunk.content.len() > 100 {
                            // Use char_indices() to ensure we don't split multi-byte UTF-8 characters
                            let truncate_at = chunk
                                .content
                                .char_indices()
                                .nth(97)
                                .map(|(idx, _)| idx)
                                .unwrap_or(chunk.content.len());
                            format!("{}...", &chunk.content[..truncate_at])
                        } else {
                            chunk.content.clone()
                        };

                        let update = ChunkProgressUpdate {
                            chunk_index,
                            total_chunks,
                            chunk_preview,
                            processing_time_ms: time_ms,
                            input_tokens: in_tokens,
                            output_tokens: out_tokens,
                            chunk_cost_usd: chunk_cost,
                            cumulative_input_tokens: total_in,
                            cumulative_output_tokens: total_out,
                            cumulative_cost_usd: cumulative_cost,
                            avg_time_per_chunk_ms: avg_time_ms,
                            eta_seconds,
                        };

                        callback(update);
                    }

                    Ok(result)
                }
            })
            .collect();

        // Execute concurrently with buffer to respect semaphore
        let results: Vec<Result<crate::extractor::ExtractionResult>> = stream::iter(futures)
            .buffer_unordered(self.config.max_concurrent_extractions)
            .collect()
            .await;

        // Collect results, propagating first error
        results.into_iter().collect()
    }

    // ═══════════════════════════════════════════════════════════════════════════
    //                    RESILIENT PARALLEL EXTRACTION (MAP-REDUCE)
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // WHY RESILIENT EXTRACTION?
    // ────────────────────────────
    // The original extract_parallel_with_progress fails fast on the first error.
    // This is problematic for large documents where:
    // - A single chunk timeout shouldn't discard 99 successful extractions
    // - Users expect partial results with clear reporting of failures
    // - Retry logic should be at chunk level, not document level
    //
    // ARCHITECTURE (MAP-REDUCE PATTERN):
    //
    //   Document (N chunks)
    //        │
    //        ▼
    //   ┌────┬────┬────┬────┬────┐
    //   │ C1 │ C2 │ C3 │ C4 │ CN │   (parallel LLM calls with semaphore)
    //   └─┬──┴─┬──┴─┬──┴─┬──┴─┬──┘
    //     ▼    ▼    ▼    ▼    ▼
    //   ┌───┐┌───┐┌───┐┌───┐┌───┐
    //   │ ✓ ││ ✗ ││ ✓ ││ ✓ ││ ✓ │   (each with per-chunk retry + timeout)
    //   └───┘└───┘└───┘└───┘└───┘
    //     │                   │
    //     ▼                   ▼
    //   Successes: [C1,C3,C4,CN]   Failures: [C2]
    //
    // RETRY STRATEGY (PER CHUNK):
    //   Attempt 1 → Attempt 2 (2x delay) → Attempt 3 (4x delay) → Failed

    /// Extract entities from chunks with resilient error handling.
    ///
    /// Unlike `extract_parallel`, this method does NOT fail fast on errors.
    /// Instead, it processes all chunks and returns both successes and failures,
    /// allowing partial results to be used.
    ///
    /// ## Implements
    /// - **FEAT0020**: Chunk-level resilience and error isolation
    /// - **UC2305**: System continues processing when individual chunks fail
    pub(super) async fn resilient_extract_parallel(
        &self,
        chunks: &[crate::chunker::TextChunk],
        extractor: &Arc<dyn EntityExtractor>,
        progress_callback: Option<ChunkProgressCallback>,
        cancel_token: Option<CancellationToken>,
    ) -> crate::error::ResilientExtractionResult {
        use crate::error::{ChunkExtractionOutcome, ChunkFailure, ResilientExtractionResult};

        let semaphore = Arc::new(tokio::sync::Semaphore::new(
            self.config.max_concurrent_extractions,
        ));

        let total_chunks = chunks.len();
        let timeout_secs = self.config.chunk_extraction_timeout_secs;
        let max_retries = self.config.chunk_max_retries;
        let initial_delay_ms = self.config.initial_retry_delay_ms;

        // Atomic counters for cumulative tracking
        let cumulative_time_ms = Arc::new(AtomicU64::new(0));
        let cumulative_input_tokens = Arc::new(AtomicU64::new(0));
        let cumulative_output_tokens = Arc::new(AtomicU64::new(0));
        let completed_chunks = Arc::new(AtomicU32::new(0));

        // Get model pricing for cost calculation
        let pricing = crate::progress::default_model_pricing();
        let model_name = extractor.model_name();
        let model_pricing = pricing
            .get(model_name)
            .cloned()
            .unwrap_or_else(|| crate::progress::ModelPricing::new("gpt-4.1-nano", 0.00015, 0.0006));
        let model_pricing = Arc::new(model_pricing);

        // ═══════════════════════════════════════════════════════════════════════
        //                           MAP PHASE
        // ═══════════════════════════════════════════════════════════════════════

        let futures: Vec<_> = chunks
            .iter()
            .enumerate()
            .map(|(chunk_index, chunk)| {
                let semaphore = semaphore.clone();
                let extractor = extractor.clone();
                let chunk = chunk.clone();
                let progress_callback = progress_callback.clone();
                let cumulative_time_ms = cumulative_time_ms.clone();
                let cumulative_input_tokens = cumulative_input_tokens.clone();
                let cumulative_output_tokens = cumulative_output_tokens.clone();
                let completed_chunks = completed_chunks.clone();
                let model_pricing = model_pricing.clone();
                let cancel_token = cancel_token.clone();

                async move {
                    let chunk_start = std::time::Instant::now();

                    // ── CANCELLATION CHECK: skip pending chunks when cancelled ──
                    if let Some(ref token) = cancel_token {
                        if token.is_cancelled() {
                            return ChunkExtractionOutcome::Failed(ChunkFailure {
                                chunk_index,
                                chunk_id: chunk.id.clone(),
                                error: "Task cancelled".to_string(),
                                retry_attempts: 0,
                                was_timeout: false,
                                processing_time_ms: 0,
                            });
                        }
                    }

                    // Acquire permit (released on drop)
                    let _permit = match semaphore.acquire().await {
                        Ok(p) => p,
                        Err(e) => {
                            return ChunkExtractionOutcome::Failed(ChunkFailure {
                                chunk_index,
                                chunk_id: chunk.id.clone(),
                                error: format!("Semaphore acquisition failed: {}", e),
                                retry_attempts: 0,
                                was_timeout: false,
                                processing_time_ms: chunk_start.elapsed().as_millis() as u64,
                            });
                        }
                    };

                    // ─────────────────────────────────────────────────────────
                    // PER-CHUNK RETRY LOOP
                    // ─────────────────────────────────────────────────────────
                    let mut last_error = String::new();
                    let mut was_timeout = false;

                    for attempt in 1..=max_retries {
                        // ── CANCELLATION CHECK: abort retry loop when cancelled ──
                        if let Some(ref token) = cancel_token {
                            if token.is_cancelled() {
                                last_error = "Task cancelled".to_string();
                                break;
                            }
                        }

                        let extraction_future = extractor.extract(&chunk);
                        let timeout_duration = tokio::time::Duration::from_secs(timeout_secs);

                        match tokio::time::timeout(timeout_duration, extraction_future).await {
                            Ok(Ok(result)) => {
                                // SUCCESS PATH
                                let time_ms = result.extraction_time_ms;
                                let in_tokens = result.input_tokens;
                                let out_tokens = result.output_tokens;

                                cumulative_time_ms.fetch_add(time_ms, Ordering::Relaxed);
                                cumulative_input_tokens
                                    .fetch_add(in_tokens as u64, Ordering::Relaxed);
                                cumulative_output_tokens
                                    .fetch_add(out_tokens as u64, Ordering::Relaxed);
                                let completed =
                                    completed_chunks.fetch_add(1, Ordering::Relaxed) + 1;

                                // Emit progress update if callback is provided
                                if let Some(ref callback) = progress_callback {
                                    let total_time = cumulative_time_ms.load(Ordering::Relaxed);
                                    let total_in = cumulative_input_tokens.load(Ordering::Relaxed);
                                    let total_out =
                                        cumulative_output_tokens.load(Ordering::Relaxed);

                                    let avg_time_ms = if completed > 0 {
                                        total_time as f64 / completed as f64
                                    } else {
                                        0.0
                                    };
                                    let remaining = total_chunks.saturating_sub(completed as usize);
                                    let eta_seconds =
                                        ((avg_time_ms * remaining as f64) / 1000.0) as u64;

                                    let cumulative_cost = model_pricing
                                        .calculate_cost(total_in as usize, total_out as usize);

                                    let chunk_preview = if chunk.content.len() > 100 {
                                        let truncate_at = chunk
                                            .content
                                            .char_indices()
                                            .nth(97)
                                            .map(|(idx, _)| idx)
                                            .unwrap_or(chunk.content.len());
                                        format!("{}...", &chunk.content[..truncate_at])
                                    } else {
                                        chunk.content.clone()
                                    };

                                    let chunk_cost =
                                        model_pricing.calculate_cost(in_tokens, out_tokens);

                                    callback(ChunkProgressUpdate {
                                        chunk_index,
                                        total_chunks,
                                        chunk_preview,
                                        processing_time_ms: time_ms,
                                        input_tokens: in_tokens,
                                        output_tokens: out_tokens,
                                        chunk_cost_usd: chunk_cost,
                                        cumulative_input_tokens: total_in,
                                        cumulative_output_tokens: total_out,
                                        cumulative_cost_usd: cumulative_cost,
                                        avg_time_per_chunk_ms: avg_time_ms,
                                        eta_seconds,
                                    });
                                }

                                return ChunkExtractionOutcome::Success {
                                    chunk_index,
                                    result,
                                };
                            }
                            Ok(Err(e)) => {
                                // Extraction error (not timeout)
                                last_error = format!("{}", e);
                                was_timeout = false;
                                tracing::warn!(
                                    chunk_index = chunk_index,
                                    chunk_id = %chunk.id,
                                    attempt = attempt,
                                    max_retries = max_retries,
                                    error = %e,
                                    "Chunk extraction failed, will retry"
                                );
                            }
                            Err(_) => {
                                // Timeout
                                last_error = format!(
                                    "Timeout after {}s (attempt {}/{})",
                                    timeout_secs, attempt, max_retries
                                );
                                was_timeout = true;
                                tracing::warn!(
                                    chunk_index = chunk_index,
                                    chunk_id = %chunk.id,
                                    attempt = attempt,
                                    max_retries = max_retries,
                                    timeout_secs = timeout_secs,
                                    "Chunk extraction timed out, will retry"
                                );
                            }
                        }

                        // Exponential backoff before retry
                        if attempt < max_retries {
                            let delay_ms = initial_delay_ms * 2_u64.pow(attempt - 1);
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        }
                    }

                    // FAILURE PATH (all retries exhausted)
                    completed_chunks.fetch_add(1, Ordering::Relaxed);

                    ChunkExtractionOutcome::Failed(ChunkFailure {
                        chunk_index,
                        chunk_id: chunk.id.clone(),
                        error: last_error,
                        retry_attempts: max_retries,
                        was_timeout,
                        processing_time_ms: chunk_start.elapsed().as_millis() as u64,
                    })
                }
            })
            .collect();

        // ═══════════════════════════════════════════════════════════════════════
        //                          REDUCE PHASE
        // ═══════════════════════════════════════════════════════════════════════

        let outcomes: Vec<ChunkExtractionOutcome> = stream::iter(futures)
            .buffer_unordered(self.config.max_concurrent_extractions)
            .collect()
            .await;

        ResilientExtractionResult::from_outcomes(outcomes)
    }
}
