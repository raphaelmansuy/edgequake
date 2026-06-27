//! Statistics aggregation and extraction post-processing.

use std::collections::HashSet;
use std::sync::Arc;

use crate::chunker::TextChunk;
use crate::extractor::ExtractionResult;

use super::super::{CostBreakdownStats, Pipeline, ProcessingStats};

/// Link extracted entities and relationships to their source chunks.
///
/// WHY: Without chunk linkage, Local/Global query modes cannot find
/// related chunks during retrieval — entities would be "orphaned" nodes
/// in the knowledge graph with no provenance trail.
pub(in crate::pipeline) fn link_extractions_to_chunks(extractions: &mut [ExtractionResult]) {
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
pub(in crate::pipeline) fn aggregate_extraction_stats(
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

impl Pipeline {
    /// Initialize processing stats from chunked document.
    ///
    /// Sets chunk_count, chunking_strategy (SPEC-026 enum), and avg_chunk_size.
    pub(in crate::pipeline) fn init_chunk_stats(&self, chunks: &[TextChunk]) -> ProcessingStats {
        let avg_chunk_size = if chunks.is_empty() {
            None
        } else {
            let total_chars: usize = chunks.iter().map(|c| c.content.len()).sum();
            Some(total_chars / chunks.len())
        };

        ProcessingStats {
            chunk_count: chunks.len(),
            chunking_strategy: Some(self.config.chunk_strategy.as_str().to_string()),
            avg_chunk_size,
            ..ProcessingStats::default()
        }
    }
}
