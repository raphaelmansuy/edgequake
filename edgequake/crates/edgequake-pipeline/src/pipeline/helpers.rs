//! Shared helpers for pipeline processing stages.
//!
//! These functions eliminate duplication across `process`, `process_with_progress`,
//! and `process_with_resilience` by extracting common logic for:
//! - Linking entities/relationships to source chunks
//! - Aggregating extraction statistics
//! - Generating embeddings (chunk, entity, relationship)
//! - Building document lineage

use std::collections::HashSet;
use std::sync::Arc;

use crate::chunker::TextChunk;
use crate::error::Result;
use crate::extractor::ExtractionResult;
use crate::lineage::{DocumentLineage, ExtractionMetadata, LineageBuilder, SourceSpan};

use super::{
    CostBreakdownStats, EmbedProgressCallback, EmbedProgressUpdate, Pipeline, ProcessingStats,
};

// ─────────────────────────────────────────────────────────────────────────────
//                       EXTRACTION POST-PROCESSING
// ─────────────────────────────────────────────────────────────────────────────

/// Link extracted entities and relationships to their source chunks.
///
/// WHY: Without chunk linkage, Local/Global query modes cannot find
/// related chunks during retrieval — entities would be "orphaned" nodes
/// in the knowledge graph with no provenance trail.
pub(super) fn link_extractions_to_chunks(extractions: &mut [ExtractionResult]) {
    for extraction in extractions.iter_mut() {
        let chunk_id = extraction.source_chunk_id.clone();
        tracing::debug!(
            "Linking {} entities and {} relationships to chunk {}",
            extraction.entities.len(),
            extraction.relationships.len(),
            chunk_id
        );
        for entity in &mut extraction.entities {
            entity.add_source_chunk_id(&chunk_id);
        }
        for rel in &mut extraction.relationships {
            if rel.source_chunk_id.is_none() {
                rel.source_chunk_id = Some(chunk_id.clone());
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//                       STATISTICS AGGREGATION
// ─────────────────────────────────────────────────────────────────────────────

/// Aggregate extraction statistics from all successful extractions.
///
/// Populates entity/relationship counts, token usage, unique types/keywords,
/// and extraction cost in the provided `ProcessingStats`.
///
/// WHY UNIFIED: This logic was duplicated verbatim across `process`,
/// `process_with_progress`, and `process_with_resilience`. Extracting it
/// ensures consistent cost calculation and keyword collection.
pub(super) fn aggregate_extraction_stats(
    extractions: &[ExtractionResult],
    extractor: &Arc<dyn crate::extractor::EntityExtractor>,
    stats: &mut ProcessingStats,
) {
    let mut entity_types_set = HashSet::new();
    let mut relationship_types_set = HashSet::new();
    let mut keywords_set = HashSet::new();
    let mut total_input_tokens = 0usize;
    let mut total_output_tokens = 0usize;

    // Capture LLM model and provider names
    // @implements SPEC-032/OODA-226: Provider tracking in ProcessingStats
    stats.llm_model = Some(extractor.model_name().to_string());
    stats.llm_provider = Some(extractor.provider_name().to_string());

    for extraction in extractions {
        stats.entity_count += extraction.entities.len();
        stats.relationship_count += extraction.relationships.len();
        stats.llm_calls += 1;
        total_input_tokens += extraction.input_tokens;
        total_output_tokens += extraction.output_tokens;

        for entity in &extraction.entities {
            entity_types_set.insert(entity.entity_type.clone());
        }
        for rel in &extraction.relationships {
            relationship_types_set.insert(rel.relation_type.clone());
            for keyword in &rel.keywords {
                keywords_set.insert(keyword.clone());
            }
        }
    }

    stats.total_tokens = total_input_tokens + total_output_tokens;
    stats.input_tokens = total_input_tokens;
    stats.output_tokens = total_output_tokens;

    // Store collected types and keywords
    if !entity_types_set.is_empty() {
        stats.entity_types = Some(entity_types_set.into_iter().collect());
    }
    if !relationship_types_set.is_empty() {
        stats.relationship_types = Some(relationship_types_set.into_iter().collect());
    }
    if !keywords_set.is_empty() {
        let mut keywords: Vec<String> = keywords_set.into_iter().collect();
        keywords.sort();
        // Limit to top 50 keywords
        keywords.truncate(50);
        stats.keywords = Some(keywords);
    }

    // Calculate extraction cost using model pricing
    let model_name = extractor.model_name();
    let pricing = crate::progress::default_model_pricing();
    let model_pricing = pricing
        .get(model_name)
        .cloned()
        .unwrap_or_else(|| crate::progress::ModelPricing::new("gpt-4.1-nano", 0.00015, 0.0006));

    let extraction_cost = model_pricing.calculate_cost(total_input_tokens, total_output_tokens);
    stats.cost_usd += extraction_cost;

    let cost_breakdown = CostBreakdownStats {
        extraction_cost_usd: extraction_cost,
        extraction_input_tokens: total_input_tokens,
        extraction_output_tokens: total_output_tokens,
        ..CostBreakdownStats::default()
    };
    stats.cost_breakdown = Some(cost_breakdown);
}

// ─────────────────────────────────────────────────────────────────────────────
//                       EMBEDDING GENERATION HELPERS
// ─────────────────────────────────────────────────────────────────────────────

/// Conservative chars-per-true-token for dense technical content.
///
/// WHY 2.5: The chunker uses 4 chars/token (English prose).
/// Scientific PDFs contain tables with numbers, gene IDs, p-values, and
/// formulas where tokenizers split aggressively — real density can reach
/// 1.5–2.0 chars/token. Using 2.5 provides a safe intermediate buffer.
const EMBED_CHARS_PER_TOKEN: f64 = 2.5;

/// Safety headroom factor applied to the embedding context limit.
///
/// WHY 0.85: Leaves 15% slack for tokenizer variance, whitespace tokens,
/// and any prompt overhead the embedding endpoint may add.
const EMBED_SAFETY_FACTOR: f64 = 0.85;

/// Fallback maximum characters when `provider.max_tokens()` returns 0 (unknown).
///
/// 6 000 chars ≈ 2 400 tokens at 2.5 chars/token, keeping chunks well within
/// the 2 048-token limit of models like embeddinggemma.
const EMBED_FALLBACK_MAX_CHARS: usize = 6_000;

/// Compute the maximum safe character count for a single embedding input.
///
/// When the provider exposes its context limit, we derive the char cap from it.
/// When the limit is unknown (0), we fall back to `EMBED_FALLBACK_MAX_CHARS`.
fn embed_max_chars(max_tokens: usize) -> usize {
    if max_tokens == 0 {
        EMBED_FALLBACK_MAX_CHARS
    } else {
        (max_tokens as f64 * EMBED_CHARS_PER_TOKEN * EMBED_SAFETY_FACTOR) as usize
    }
}

/// Truncate `s` to at most `max_bytes`, preserving UTF-8 character boundaries.
fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    // Walk back to the nearest valid UTF-8 boundary.
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Guard a text batch before sending to the embedding provider.
///
/// Truncates any string that exceeds `max_chars` and logs a WARNING so
/// operators know chunks are being trimmed and can tune `chunk_size` or
/// switch to an embedding model with a larger context window.
///
/// WHY: A partial embedding is more useful than a pipeline failure.
/// The 400 "input length exceeds context length" error from Ollama would
/// otherwise abort the entire document ingestion.
fn guard_for_embedding(texts: &[String], max_chars: usize) -> Vec<String> {
    texts
        .iter()
        .enumerate()
        .map(|(i, text)| {
            if text.len() > max_chars {
                tracing::warn!(
                    input_index = i,
                    original_chars = text.len(),
                    cap_chars = max_chars,
                    "Embedding input truncated: text exceeds the safe token limit for the \
                     embedding model. Consider reducing chunk_size in PipelineConfig or \
                     switching to an embedding model with a larger context window."
                );
                truncate_at_char_boundary(text, max_chars).to_string()
            } else {
                text.clone()
            }
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
//                   TOKEN-AWARE EMBEDDING BATCH HELPER
// ─────────────────────────────────────────────────────────────────────────────

/// Embed `texts` with automatic token-aware sub-batching.
///
/// ## Problem
///
/// `EmbeddingProvider::embed_batched` splits inputs only by **count** (default
/// 2 048 texts per API call). This is insufficient for providers like Mistral
/// whose API enforces an **8 192-token TOTAL budget per request** regardless of
/// the number of individual texts. 142 entity descriptions from a dense
/// technical PDF can easily exceed 8 192 total tokens, producing:
///
/// ```text
/// 400 Bad Request: "Too many tokens overall, split into more batches." (code 3210)
/// ```
///
/// ## Fix
///
/// Before delegating to `embed_batched`, accumulate texts into sub-batches
/// whose estimated total token count stays within
/// `provider.max_tokens() * EMBED_SAFETY_FACTOR`. When the budget would be
/// exceeded by the next text, flush the current sub-batch first.
///
/// Token estimation uses `EMBED_CHARS_PER_TOKEN = 2.5` (conservative for
/// dense technical content) — the same constant as `guard_for_embedding`.
///
/// When `provider.max_tokens()` returns 0 (limit unknown) we fall back to
/// `embed_batched` directly because there is no budget to split against.
async fn embed_with_token_budget(
    provider: &Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
    texts: &[String],
) -> crate::error::Result<Vec<Vec<f32>>> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    let max_tokens = provider.max_tokens();
    if max_tokens == 0 {
        // Limit unknown — standard count-based batching
        return provider
            .embed_batched(texts)
            .await
            .map_err(|e| crate::error::PipelineError::EmbeddingError(e.to_string()));
    }

    // Effective token budget per sub-batch (apply safety headroom)
    let token_budget = (max_tokens as f64 * EMBED_SAFETY_FACTOR) as usize;

    // Maximum input count per sub-batch from the provider (e.g. 512 for Mistral).
    //
    // WHY DUAL-DIMENSION SPLIT (spec-011):
    // Mistral enforces TWO independent limits per embedding request:
    //   (A) total tokens ≤ 8 192  → guarded by `token_budget` above
    //   (B) input count  ≤ 512    → guarded by `max_batch_count` here
    //
    // For dense legal/regulatory documents (EU AI Act, GDPR) many short entity
    // names (~10 tokens each) fit within the token budget but may exceed the
    // input count limit (e.g. 700 items × 10 tokens = 7 000 ≤ 8 192, but
    // 700 > 512).  The previous token-only split produced sub-batches of 700
    // items, triggering HTTP 400 "Too many inputs in request" (code 3210).
    //
    // By flushing on EITHER dimension we satisfy both limits simultaneously.
    let max_batch_count = provider.max_batch_size();

    let mut all_embeddings: Vec<Vec<f32>> = Vec::with_capacity(texts.len());
    let mut batch_start: usize = 0;
    let mut batch_tokens: usize = 0;

    for (i, text) in texts.iter().enumerate() {
        // Estimate token count for this text
        let text_tokens = ((text.len() as f64) / EMBED_CHARS_PER_TOKEN).ceil() as usize;

        let current_count = i - batch_start;
        let token_overflow = batch_tokens + text_tokens > token_budget;
        let count_overflow = current_count >= max_batch_count;

        // Flush if EITHER the token budget OR the input count limit would be exceeded.
        // Always include at least one text even if it alone exceeds the budget (single-text
        // requests are the smallest possible unit; the provider must handle them).
        if (token_overflow || count_overflow) && i > batch_start {
            let flush_reason = if count_overflow {
                "count limit"
            } else {
                "token budget"
            };
            tracing::debug!(
                sub_batch_texts = current_count,
                estimated_tokens = batch_tokens,
                token_budget = token_budget,
                max_batch_count = max_batch_count,
                reason = flush_reason,
                "Flushing embedding sub-batch"
            );
            let batch_result = provider
                .embed_batched(&texts[batch_start..i])
                .await
                .map_err(|e| crate::error::PipelineError::EmbeddingError(e.to_string()))?;
            all_embeddings.extend(batch_result);
            batch_start = i;
            batch_tokens = 0;
        }
        batch_tokens += text_tokens;
    }

    // Flush the final sub-batch
    if batch_start < texts.len() {
        let batch_result = provider
            .embed_batched(&texts[batch_start..])
            .await
            .map_err(|e| crate::error::PipelineError::EmbeddingError(e.to_string()))?;
        all_embeddings.extend(batch_result);
    }

    Ok(all_embeddings)
}

// ─────────────────────────────────────────────────────────────────────────────
//                       SAFE EMBEDDING HELPER
// ─────────────────────────────────────────────────────────────────────────────

/// Guard `texts`, embed with token-budget batching, and validate result count.
///
/// Encapsulates the three-step pattern repeated for every embeddable item kind
/// (chunks, entities, relationships):
/// 1. `guard_for_embedding` — truncate inputs that exceed the provider's
///    character limit to avoid 400 "input too long" errors.
/// 2. `embed_with_token_budget` — split into sub-batches respecting BOTH the
///    token budget and the provider's input-count limit.
/// 3. Count mismatch warning — if the provider silently drops embeddings,
///    log a warning so that operators notice orphaned graph nodes.
///
/// Returns the embeddings in the same order as `texts`, or an empty `Vec` when
/// `texts` is empty (short-circuit avoids a provider round-trip).
async fn safe_embed(
    provider: &Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
    texts: &[String],
    max_chars: usize,
    kind: &str,
) -> crate::error::Result<Vec<Vec<f32>>> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }
    let safe_texts = guard_for_embedding(texts, max_chars);
    let embeddings = embed_with_token_budget(provider, &safe_texts).await?;
    if embeddings.len() != texts.len() {
        tracing::warn!(
            expected = texts.len(),
            actual = embeddings.len(),
            kind,
            "{kind} embedding count mismatch - some items may lack embeddings"
        );
    }
    Ok(embeddings)
}

/// Estimate the number of embedding tokens for a character count.
///
/// Uses `EMBED_CHARS_PER_TOKEN` — the same conservative constant as the
/// batch-splitting logic — so that cost estimates and batch boundaries share a
/// single denominator (DRY). Previously the cost loops used a hardcoded `/ 4`
/// (4 chars/token) which diverged from the `2.5` used in sub-batch sizing.
fn estimate_embed_tokens(char_count: usize) -> usize {
    (char_count as f64 / EMBED_CHARS_PER_TOKEN).ceil() as usize
}

// ─────────────────────────────────────────────────────────────────────────────
//                       EMBEDDING GENERATION
// ─────────────────────────────────────────────────────────────────────────────

impl Pipeline {
    /// Generate embeddings for chunks, entities, and relationships.
    ///
    /// WHY UNIFIED: All three processing methods shared identical embedding
    /// logic (~120 lines each). This single implementation handles:
    /// - Chunk embeddings (content → vector)
    /// - Entity embeddings (name: description → vector)
    /// - Relationship embeddings (keywords + source→target + description → vector)
    /// - Embedding cost calculation
    ///
    /// Fire an `EmbedProgressUpdate` via the optional callback.
    ///
    /// WHY helper fn: keeps the embedding loop bodies readable — avoids
    /// repeating the `if let Some(cb) = progress { cb(...) }` guard at every
    /// call-site (DRY).
    fn emit_embed_progress(
        progress: Option<&EmbedProgressCallback>,
        stage: &'static str,
        current: usize,
        total: usize,
    ) {
        if let Some(cb) = progress {
            cb(EmbedProgressUpdate {
                stage,
                current,
                total,
            });
        }
    }

    /// Generate embeddings for chunks, entities, and relationships.
    ///
    /// WHY UNIFIED: All three processing methods shared identical embedding
    /// logic (~120 lines each). This single implementation handles:
    /// - Chunk embeddings (content → vector)
    /// - Entity embeddings (name: description → vector)
    /// - Relationship embeddings (keywords + source→target + description → vector)
    /// - Embedding cost calculation
    ///
    /// `progress` is invoked **at the start and end of each sub-stage** so
    /// callers can surface real-time progress while embeddings are generated.
    /// Pass `None` when no progress reporting is needed.
    pub(super) async fn generate_all_embeddings(
        &self,
        chunks: &mut [TextChunk],
        extractions: &mut [ExtractionResult],
        stats: &mut ProcessingStats,
        progress: Option<&EmbedProgressCallback>,
    ) -> Result<()> {
        let provider = match &self.embedding_provider {
            Some(p) => p,
            None => return Ok(()),
        };

        // Capture embedding model and provider info
        // @implements SPEC-032/OODA-226: Provider tracking in ProcessingStats
        stats.embedding_model = Some(provider.model().to_string());
        stats.embedding_provider = Some(provider.name().to_string());
        stats.embedding_dimensions = Some(provider.dimension());

        // Pre-compute the safe character limit for this provider once.
        // WHY: Avoids repeated calls to max_tokens() in tight loops and keeps
        // the guard logic in a single reusable helper (DRY).
        let max_chars = embed_max_chars(provider.max_tokens());

        // ── Chunk embeddings ──
        if self.config.enable_chunk_embeddings {
            let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
            // Notify: starting chunk embeddings
            Self::emit_embed_progress(progress, "chunks", 0, texts.len());
            let embeddings = safe_embed(provider, &texts, max_chars, "Chunk").await?;
            for (chunk, embedding) in chunks.iter_mut().zip(embeddings) {
                chunk.embedding = Some(embedding);
            }
            // Notify: chunk embeddings complete
            Self::emit_embed_progress(progress, "chunks", texts.len(), texts.len());
        }

        // ── Entity embeddings (batched) ──
        if self.config.enable_entity_embeddings {
            let mut all_entity_texts: Vec<String> = Vec::new();
            let mut entity_indices: Vec<(usize, usize)> = Vec::new(); // (extraction_idx, entity_idx)

            for (ext_idx, extraction) in extractions.iter().enumerate() {
                for (ent_idx, entity) in extraction.entities.iter().enumerate() {
                    all_entity_texts.push(format!("{}: {}", entity.name, entity.description));
                    entity_indices.push((ext_idx, ent_idx));
                }
            }

            // Notify: starting entity embeddings
            Self::emit_embed_progress(progress, "entities", 0, all_entity_texts.len());

            // WHY safe_embed: guards length, embeds in compliant sub-batches, and
            // warns if the provider silently drops results (zip() would otherwise
            // create orphaned graph nodes with no embedding vector).
            let all_embeddings =
                safe_embed(provider, &all_entity_texts, max_chars, "Entity").await?;
            for (embedding, (ext_idx, ent_idx)) in all_embeddings.into_iter().zip(entity_indices) {
                extractions[ext_idx].entities[ent_idx].embedding = Some(embedding);
            }

            // Notify: entity embeddings complete
            Self::emit_embed_progress(
                progress,
                "entities",
                all_entity_texts.len(),
                all_entity_texts.len(),
            );
        }

        // ── Relationship embeddings (batched) ──
        if self.config.enable_relationship_embeddings {
            let mut all_relationship_texts: Vec<String> = Vec::new();
            let mut relationship_indices: Vec<(usize, usize)> = Vec::new();

            for (ext_idx, extraction) in extractions.iter().enumerate() {
                for (rel_idx, r) in extraction.relationships.iter().enumerate() {
                    // Format: "keywords\tsource->target\ndescription"
                    // Matches LightRAG's relationship embedding format
                    all_relationship_texts.push(format!(
                        "{}\t{}->{}\n{}",
                        r.keywords.join(", "),
                        r.source,
                        r.target,
                        r.description
                    ));
                    relationship_indices.push((ext_idx, rel_idx));
                }
            }

            // Notify: starting relationship embeddings
            Self::emit_embed_progress(progress, "relationships", 0, all_relationship_texts.len());

            let all_embeddings =
                safe_embed(provider, &all_relationship_texts, max_chars, "Relationship").await?;
            for (embedding, (ext_idx, rel_idx)) in
                all_embeddings.into_iter().zip(relationship_indices)
            {
                extractions[ext_idx].relationships[rel_idx].embedding = Some(embedding);
            }

            // Notify: relationship embeddings complete
            Self::emit_embed_progress(
                progress,
                "relationships",
                all_relationship_texts.len(),
                all_relationship_texts.len(),
            );
        }

        // ── Embedding cost calculation ──
        // WHY estimate_embed_tokens: uses the same EMBED_CHARS_PER_TOKEN denominator
        // as sub-batch sizing — one constant, one formula (DRY). The previous code
        // used a hardcoded / 4 (4 chars/token) which diverged from the 2.5 used in
        // the batch splitter, giving inconsistent cost estimates for the same content.
        let mut total_embed_tokens = 0usize;

        if self.config.enable_chunk_embeddings {
            let chunk_text_len: usize = chunks.iter().map(|c| c.content.len()).sum();
            total_embed_tokens += estimate_embed_tokens(chunk_text_len);
        }
        if self.config.enable_entity_embeddings {
            for extraction in extractions.iter() {
                for entity in &extraction.entities {
                    total_embed_tokens +=
                        estimate_embed_tokens(entity.name.len() + entity.description.len());
                }
            }
        }
        if self.config.enable_relationship_embeddings {
            for extraction in extractions.iter() {
                for rel in &extraction.relationships {
                    total_embed_tokens += estimate_embed_tokens(
                        rel.source.len() + rel.target.len() + rel.description.len(),
                    );
                }
            }
        }

        let embed_model_name = provider.model();
        let pricing = crate::progress::default_model_pricing();
        let embed_pricing = pricing.get(embed_model_name).cloned().unwrap_or_else(|| {
            crate::progress::ModelPricing::new("text-embedding-3-small", 0.00002, 0.0)
        });

        let embedding_cost = embed_pricing.calculate_cost(total_embed_tokens, 0);
        stats.cost_usd += embedding_cost;

        if let Some(ref mut breakdown) = stats.cost_breakdown {
            breakdown.embedding_cost_usd = embedding_cost;
            breakdown.embedding_tokens = total_embed_tokens;
        } else {
            let breakdown = CostBreakdownStats {
                embedding_cost_usd: embedding_cost,
                embedding_tokens: total_embed_tokens,
                ..CostBreakdownStats::default()
            };
            stats.cost_breakdown = Some(breakdown);
        }

        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    //                       LINEAGE BUILDING
    // ─────────────────────────────────────────────────────────────────────────

    /// Build document lineage from chunks and extractions.
    ///
    /// Returns `None` if lineage tracking is disabled in config.
    ///
    /// WHY UNIFIED: All three processing methods had identical lineage
    /// building code (~40 lines each). This single implementation ensures
    /// consistent entity/relationship ID generation and span recording.
    pub(super) fn build_lineage(
        &self,
        document_id: &str,
        chunks: &[TextChunk],
        extractions: &[ExtractionResult],
        stats: &ProcessingStats,
    ) -> Option<DocumentLineage> {
        if !self.config.enable_lineage_tracking {
            return None;
        }

        let job_id = uuid::Uuid::new_v4().to_string();
        let mut builder = LineageBuilder::new(document_id, document_id, &job_id);

        // Record chunks with their line numbers
        for chunk in chunks {
            let metadata = ExtractionMetadata::new(stats.llm_model.as_deref().unwrap_or("unknown"));
            builder.record_chunk(
                &chunk.id,
                chunk.index,
                chunk.start_line,
                chunk.end_line,
                chunk.start_offset,
                chunk.end_offset,
                metadata,
            );
        }

        // Record entities and relationships from extractions
        for extraction in extractions {
            for entity in &extraction.entities {
                let entity_id = format!("{}_{}", extraction.source_chunk_id, entity.name);
                let span = SourceSpan::new(0, 0, 0, 0);
                builder.record_entity(
                    &entity_id,
                    &entity.name,
                    &extraction.source_chunk_id,
                    span,
                    &entity.description,
                );
            }

            for rel in &extraction.relationships {
                let rel_id = format!(
                    "{}_{}_{}",
                    extraction.source_chunk_id, rel.source, rel.target
                );
                let span = SourceSpan::new(0, 0, 0, 0);
                builder.record_relationship(
                    &rel_id,
                    &rel.source,
                    &rel.target,
                    &rel.relation_type,
                    &extraction.source_chunk_id,
                    span,
                    &rel.description,
                );
            }
        }

        Some(builder.build())
    }

    /// Initialize processing stats from chunked document.
    ///
    /// Sets chunk_count, chunking_strategy, and avg_chunk_size.
    pub(super) fn init_chunk_stats(&self, chunks: &[TextChunk]) -> ProcessingStats {
        let avg_chunk_size = if chunks.is_empty() {
            None
        } else {
            let total_chars: usize = chunks.iter().map(|c| c.content.len()).sum();
            Some(total_chars / chunks.len())
        };

        ProcessingStats {
            chunk_count: chunks.len(),
            chunking_strategy: Some(format!("sliding_window_{}", self.config.chunker.chunk_size)),
            avg_chunk_size,
            ..ProcessingStats::default()
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//                               TESTS
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── embed_max_chars ────────────────────────────────────────────────────

    #[test]
    fn test_embed_max_chars_with_known_limit() {
        // 8192 tokens × 2.5 chars/token × 0.85 safety = 17 408 chars
        let expected = (8192_f64 * EMBED_CHARS_PER_TOKEN * EMBED_SAFETY_FACTOR) as usize;
        assert_eq!(embed_max_chars(8192), expected);
    }

    #[test]
    fn test_embed_max_chars_fallback_when_zero() {
        assert_eq!(embed_max_chars(0), EMBED_FALLBACK_MAX_CHARS);
    }

    // ── truncate_at_char_boundary ─────────────────────────────────────────

    #[test]
    fn test_truncate_exact_boundary() {
        let s = "hello world";
        assert_eq!(truncate_at_char_boundary(s, 5), "hello");
    }

    #[test]
    fn test_truncate_within_multibyte_char() {
        // "é" is 2 bytes (U+00E9). Truncating at byte 1 must walk back to byte 0.
        let s = "aéb";
        let truncated = truncate_at_char_boundary(s, 2);
        assert!(s.is_char_boundary(truncated.len()));
        assert_eq!(truncated, "a");
    }

    #[test]
    fn test_truncate_no_op_when_within_limit() {
        let s = "short";
        assert_eq!(truncate_at_char_boundary(s, 100), "short");
    }

    // ── guard_for_embedding ──────────────────────────────────────────────

    #[test]
    fn test_guard_preserves_short_texts() {
        let texts = vec!["hello".to_string(), "world".to_string()];
        let result = guard_for_embedding(&texts, 100);
        assert_eq!(result, texts);
    }

    #[test]
    fn test_guard_truncates_long_texts() {
        let long_text = "a".repeat(200);
        let result = guard_for_embedding(&[long_text.clone()], 50);
        assert_eq!(result.len(), 1);
        assert!(result[0].len() <= 50);
    }

    // ── embed_with_token_budget ────────────────────────────────────────────

    /// Counts how many times `embed()` is called by accumulating batch sizes.
    use std::sync::{Arc, Mutex};

    struct CountingEmbedProvider {
        /// Each element records the number of texts in that sub-batch call.
        call_sizes: Arc<Mutex<Vec<usize>>>,
        /// Simulated max_tokens limit.
        max_tokens: usize,
        /// Simulated max_batch_size (input count limit).
        max_batch: usize,
    }

    impl CountingEmbedProvider {
        fn new(max_tokens: usize) -> (Self, Arc<Mutex<Vec<usize>>>) {
            Self::new_with_batch(max_tokens, 2048)
        }

        fn new_with_batch(max_tokens: usize, max_batch: usize) -> (Self, Arc<Mutex<Vec<usize>>>) {
            let call_sizes = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    call_sizes: call_sizes.clone(),
                    max_tokens,
                    max_batch,
                },
                call_sizes,
            )
        }
    }

    #[async_trait::async_trait]
    impl edgequake_llm::traits::EmbeddingProvider for CountingEmbedProvider {
        fn name(&self) -> &str {
            "counting"
        }
        fn model(&self) -> &str {
            "counting-embed"
        }
        fn dimension(&self) -> usize {
            4
        }
        fn max_tokens(&self) -> usize {
            self.max_tokens
        }
        fn max_batch_size(&self) -> usize {
            self.max_batch
        }

        async fn embed(&self, texts: &[String]) -> edgequake_llm::Result<Vec<Vec<f32>>> {
            self.call_sizes.lock().unwrap().push(texts.len());
            // Return a dummy 4-dim vector per text
            Ok(texts.iter().map(|_| vec![0.1, 0.2, 0.3, 0.4]).collect())
        }
    }

    /// When total estimated tokens fit within the budget, exactly ONE call is made.
    #[tokio::test]
    async fn test_embed_budget_single_batch_when_within_limit() {
        // 10 texts × 10 chars / 2.5 chars/token = 40 tokens, budget = 8192 * 0.85 ≈ 6963
        let texts: Vec<String> = (0..10).map(|i| format!("entity_{:04}", i)).collect(); // ~13 chars each
        let (provider, call_sizes) = CountingEmbedProvider::new(8192);
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &texts).await.unwrap();
        assert_eq!(result.len(), 10, "All 10 embeddings must be returned");
        let sizes = call_sizes.lock().unwrap();
        assert_eq!(sizes.len(), 1, "Should have made exactly 1 embed call");
    }

    /// When texts are large enough to exceed a tiny budget, they are split across
    /// multiple sub-batch calls and all embeddings are reassembled in order.
    #[tokio::test]
    async fn test_embed_budget_splits_batches_correctly() {
        // 20 texts of 100 chars each:
        //   100 / 2.5 = 40 tokens per text
        //   budget = 80 * 0.85 = 68 tokens ≈ 1 text per batch
        let texts: Vec<String> = (0..20).map(|_| "x".repeat(100)).collect();
        let (provider, call_sizes) = CountingEmbedProvider::new(80); // tiny budget forces splits
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &texts).await.unwrap();
        assert_eq!(result.len(), 20, "All 20 embeddings must be returned");
        // With max_tokens=80 and SAFETY_FACTOR=0.85, budget = 68 tokens.
        // Each text costs ceil(100/2.5)=40 tokens. Two texts = 80 > 68 → at least 2 calls.
        let sizes = call_sizes.lock().unwrap();
        assert!(
            sizes.len() >= 2,
            "Expected multiple batches, got {} call(s)",
            sizes.len()
        );
        // Total texts across all calls must equal 20 (no duplicates, no drops)
        let total: usize = sizes.iter().sum();
        assert_eq!(total, 20, "Total texts across batches must be 20");
    }

    /// Empty input returns empty output without calling the provider at all.
    #[tokio::test]
    async fn test_embed_budget_empty_input() {
        let (provider, call_sizes) = CountingEmbedProvider::new(8192);
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &[]).await.unwrap();
        assert!(result.is_empty());
        assert!(
            call_sizes.lock().unwrap().is_empty(),
            "No calls for empty input"
        );
    }

    /// When `max_tokens == 0` (limit unknown), fall back to `embed_batched` — one call.
    #[tokio::test]
    async fn test_embed_budget_zero_max_tokens_fallback() {
        let texts: Vec<String> = (0..5).map(|i| format!("text_{}", i)).collect();
        let (provider, call_sizes) = CountingEmbedProvider::new(0); // 0 = unknown limit
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &texts).await.unwrap();
        assert_eq!(result.len(), 5);
        let sizes = call_sizes.lock().unwrap();
        assert_eq!(
            sizes.len(),
            1,
            "Should fall back to a single embed_batched call"
        );
    }

    // ── count-limit splitting (spec-011) ──────────────────────────────────

    /// When input count exceeds max_batch_size, splits into multiple calls.
    ///
    /// This is the regression test for the EU AI Act failure:
    /// many short entities fit within the token budget but exceed the
    /// Mistral input count limit (512).
    #[tokio::test]
    async fn test_embed_count_limit_splits_batches() {
        // 600 short texts (each 10 chars, ~4 tokens) with max_batch=512
        // Token budget = 8192 * 0.85 = 6963, 600 × 4 = 2400 tokens → fits budget
        // But 600 > 512 → must split
        let texts: Vec<String> = (0..600_usize).map(|i| format!("entity_{:04}", i)).collect();
        let (provider, call_sizes) = CountingEmbedProvider::new_with_batch(8192, 512);
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &texts).await.unwrap();
        assert_eq!(result.len(), 600, "All 600 embeddings returned");

        let sizes = call_sizes.lock().unwrap();
        assert!(
            sizes.len() >= 2,
            "600 texts with max_batch=512 must produce ≥ 2 calls, got {}",
            sizes.len()
        );
        // Each individual call must respect the count limit
        for (idx, &size) in sizes.iter().enumerate() {
            assert!(
                size <= 512,
                "Call {} sent {} items, exceeds max_batch_size 512",
                idx,
                size
            );
        }
        // No items lost or duplicated
        let total: usize = sizes.iter().sum();
        assert_eq!(total, 600, "Total items across all calls must be 600");
    }

    /// Exactly max_batch_size items → exactly ONE call (boundary: not exceeded).
    #[tokio::test]
    async fn test_embed_count_exactly_at_limit_is_one_call() {
        let texts: Vec<String> = (0..512_usize).map(|i| format!("e_{}", i)).collect();
        let (provider, call_sizes) = CountingEmbedProvider::new_with_batch(8192, 512);
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &texts).await.unwrap();
        assert_eq!(result.len(), 512);

        let sizes = call_sizes.lock().unwrap();
        assert_eq!(sizes.len(), 1, "Exactly 512 texts (== limit) → 1 call");
        assert_eq!(sizes[0], 512);
    }

    /// One more than max_batch_size → exactly TWO calls.
    #[tokio::test]
    async fn test_embed_count_one_over_limit_is_two_calls() {
        let texts: Vec<String> = (0..513_usize).map(|i| format!("e_{}", i)).collect();
        let (provider, call_sizes) = CountingEmbedProvider::new_with_batch(8192, 512);
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &texts).await.unwrap();
        assert_eq!(result.len(), 513);

        let sizes = call_sizes.lock().unwrap();
        assert_eq!(sizes.len(), 2, "513 texts with limit 512 → 2 calls");
        assert_eq!(sizes[0], 512, "First call: 512");
        assert_eq!(sizes[1], 1, "Second call: 1 remainder");
    }

    /// Both limits active simultaneously: token budget AND count limit both trigger.
    /// Verifies the flush uses the more restrictive limit.
    #[tokio::test]
    async fn test_embed_dual_limit_count_wins_over_token() {
        // 20 texts × 100 chars each = 40 tokens each
        // Token budget = 8192 * 0.85 = 6963 → 20×40=800 tokens fits budget
        // max_batch_count = 5 → must split on count, not tokens
        let texts: Vec<String> = (0..20).map(|_| "x".repeat(100)).collect();
        let (provider, call_sizes) = CountingEmbedProvider::new_with_batch(8192, 5);
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &texts).await.unwrap();
        assert_eq!(result.len(), 20);

        let sizes = call_sizes.lock().unwrap();
        assert!(
            sizes.len() >= 4,
            "20 texts with max_batch=5 → ≥ 4 calls, got {}",
            sizes.len()
        );
        for (idx, &size) in sizes.iter().enumerate() {
            assert!(
                size <= 5,
                "Call {} sent {} items, exceeds max_batch 5",
                idx,
                size
            );
        }
        let total: usize = sizes.iter().sum();
        assert_eq!(total, 20);
    }

    /// Token budget wins over count when texts are very large.
    #[tokio::test]
    async fn test_embed_dual_limit_token_wins_over_count() {
        // 20 texts × 1000 chars each = 400 tokens each
        // max_batch_count = 2048 (large) → count won't trigger
        // Token budget = 100 * 0.85 = 85 tokens → one 400-token text already exceeds budget
        // → splits on token budget (1 text per call since each > budget)
        let texts: Vec<String> = (0..5).map(|_| "y".repeat(1000)).collect();
        let (provider, call_sizes) = CountingEmbedProvider::new_with_batch(100, 2048);
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &texts).await.unwrap();
        assert_eq!(result.len(), 5);

        // Each text = ceil(1000/2.5) = 400 tokens; budget = 100*0.85 = 85 tokens
        // First text: 400 > 85 but i == batch_start (can't flush empty batch) → sent alone
        // Second text: 400 > 85 → flush first → each text in its own call
        let sizes = call_sizes.lock().unwrap();
        assert_eq!(
            sizes.len(),
            5,
            "Each text should be its own call due to tiny budget"
        );
        let total: usize = sizes.iter().sum();
        assert_eq!(total, 5);
    }
}
