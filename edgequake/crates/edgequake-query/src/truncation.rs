//! Token-based truncation for context management.
//!
//! # Implements
//!
//! @implements FEAT0108 (Context Truncation)
//!
//! # Enforces
//!
//! - **BR0101**: Token budget must not exceed LLM context window
//! - **BR0102**: Graph context takes priority over naive chunks
//!
//! This module provides functions to truncate entities, relationships, and chunks
//! to fit within LLM token limits.
//!
//! # WHY Token Budgeting is Critical
//!
//! LLMs have fixed context window sizes (e.g., 128K tokens for GPT-4 Turbo).
//! Exceeding this limit causes:
//!
//! 1. **API errors**: Request rejected with "context length exceeded"
//! 2. **Truncation by API**: Important context silently dropped
//! 3. **Quality degradation**: Too much context dilutes attention
//!
//! ## The Token Budget Strategy
//!
//! We allocate tokens across context types (BR0102):
//!
//! ```text
//! Total Budget: 30,000 tokens (default, matching LightRAG)
//! ├── Entities:      10,000 tokens (33%)  ← Graph context (priority)
//! ├── Relationships: 10,000 tokens (33%)  ← Graph context (priority)
//! └── Chunks:        10,000 tokens (33%)  ← Primary evidence source
//! └── System prompt: ~500 tokens (separate)
//! ```
//!
//! ## WHY Entities and Relationships Get Equal Budget
//!
//! Entity descriptions provide factual grounding ("Sarah Chen is a PhD student").
//! Relationship descriptions provide connections ("Sarah works with Michael").
//! Both are equally important for comprehensive answers.
//!
//! ## Order Matters
//!
//! Items are already sorted by relevance (score/degree) before truncation.
//! Truncation preserves the most relevant items while respecting token limits.

use serde::{Deserialize, Serialize};

use crate::context::{RetrievedChunk, RetrievedEntity, RetrievedRelationship};
use crate::tokenizer::Tokenizer;

/// Configuration for token-based truncation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruncationConfig {
    /// Maximum tokens for entity descriptions.
    pub max_entity_tokens: usize,

    /// Maximum tokens for relationship descriptions.
    pub max_relation_tokens: usize,

    /// Maximum total tokens for all context.
    pub max_total_tokens: usize,
}

impl Default for TruncationConfig {
    fn default() -> Self {
        // WHY 30000: LightRAG uses max_total_tokens=30000. Entity and relationship
        // budgets are 1/3 each, leaving 1/3 for chunks (the primary evidence source).
        Self {
            max_entity_tokens: 10000,
            max_relation_tokens: 10000,
            max_total_tokens: 30000,
        }
    }
}

/// Truncate entities to fit within token limit.
pub fn truncate_entities(
    entities: Vec<RetrievedEntity>,
    max_tokens: usize,
    tokenizer: &dyn Tokenizer,
) -> Vec<RetrievedEntity> {
    let mut result = Vec::new();
    let mut total_tokens = 0;

    for entity in entities {
        // Format entity as it would appear in context
        let formatted = format!(
            "Entity: {} ({})\n{}\n",
            entity.name, entity.entity_type, entity.description
        );
        let entity_tokens = tokenizer.count_tokens(&formatted);

        if total_tokens + entity_tokens <= max_tokens {
            result.push(entity);
            total_tokens += entity_tokens;
        } else {
            // Stop when we exceed limit
            break;
        }
    }

    result
}

/// Truncate relationships to fit within token limit.
pub fn truncate_relationships(
    relationships: Vec<RetrievedRelationship>,
    max_tokens: usize,
    tokenizer: &dyn Tokenizer,
) -> Vec<RetrievedRelationship> {
    let mut result = Vec::new();
    let mut total_tokens = 0;

    for rel in relationships {
        // Format relationship as it would appear in context
        let formatted = format!(
            "Relationship: {} -> {} ({})\n",
            rel.source, rel.target, rel.relation_type
        );
        let rel_tokens = tokenizer.count_tokens(&formatted);

        if total_tokens + rel_tokens <= max_tokens {
            result.push(rel);
            total_tokens += rel_tokens;
        } else {
            break;
        }
    }

    result
}

/// Truncate chunks to fit within token limit.
pub fn truncate_chunks(
    chunks: Vec<RetrievedChunk>,
    max_tokens: usize,
    tokenizer: &dyn Tokenizer,
) -> Vec<RetrievedChunk> {
    let mut result = Vec::new();
    let mut total_tokens = 0;

    for chunk in chunks {
        let chunk_tokens = tokenizer.count_tokens(&chunk.content);

        if total_tokens + chunk_tokens <= max_tokens {
            result.push(chunk);
            total_tokens += chunk_tokens;
        } else {
            break;
        }
    }

    result
}

/// Balance context to fit within total token limit.
/// Proportionally reduces entities, relationships, and chunks.
pub fn balance_context(
    entities: Vec<RetrievedEntity>,
    relationships: Vec<RetrievedRelationship>,
    chunks: Vec<RetrievedChunk>,
    config: &TruncationConfig,
    tokenizer: &dyn Tokenizer,
) -> (
    Vec<RetrievedEntity>,
    Vec<RetrievedRelationship>,
    Vec<RetrievedChunk>,
) {
    let input_entity_count = entities.len();
    let input_rel_count = relationships.len();
    let input_chunk_count = chunks.len();

    // First pass: apply individual limits
    let mut entities = truncate_entities(entities, config.max_entity_tokens, tokenizer);
    let mut relationships =
        truncate_relationships(relationships, config.max_relation_tokens, tokenizer);
    // WHY: Chunk budget = total - entity - relationship (the remainder).
    // Previously used max_entity_tokens by mistake, which was correct by coincidence
    // when all 3 budgets are equal (10K each), but wrong in general.
    let max_chunk_tokens = config
        .max_total_tokens
        .saturating_sub(config.max_entity_tokens)
        .saturating_sub(config.max_relation_tokens);
    let mut chunks = truncate_chunks(chunks, max_chunk_tokens, tokenizer);

    tracing::debug!(
        input_entities = input_entity_count,
        input_relationships = input_rel_count,
        input_chunks = input_chunk_count,
        after_truncate_entities = entities.len(),
        after_truncate_rels = relationships.len(),
        after_truncate_chunks = chunks.len(),
        max_entity_tokens = config.max_entity_tokens,
        max_relation_tokens = config.max_relation_tokens,
        "OODA-231: balance_context first pass (individual limits)"
    );

    // Calculate current total
    let entity_tokens: usize = entities
        .iter()
        .map(|e| tokenizer.count_tokens(&format!("{} {}", e.name, e.description)))
        .sum();
    let rel_tokens: usize = relationships
        .iter()
        .map(|r| tokenizer.count_tokens(&format!("{} -> {}", r.source, r.target)))
        .sum();
    let chunk_tokens: usize = chunks
        .iter()
        .map(|c| tokenizer.count_tokens(&c.content))
        .sum();

    let total = entity_tokens + rel_tokens + chunk_tokens;

    tracing::debug!(
        entity_tokens = entity_tokens,
        rel_tokens = rel_tokens,
        chunk_tokens = chunk_tokens,
        total_tokens = total,
        max_total_tokens = config.max_total_tokens,
        "OODA-231: balance_context token counts"
    );

    // If within limit, return as-is
    if total <= config.max_total_tokens {
        tracing::debug!(
            final_entities = entities.len(),
            final_rels = relationships.len(),
            final_chunks = chunks.len(),
            "OODA-231: balance_context within limit, no reduction needed"
        );
        return (entities, relationships, chunks);
    }

    // Need to reduce: calculate proportional reduction
    let reduction_ratio = config.max_total_tokens as f32 / total as f32;

    // Apply reduction proportionally
    let new_entity_count = (entities.len() as f32 * reduction_ratio).ceil() as usize;
    let new_rel_count = (relationships.len() as f32 * reduction_ratio).ceil() as usize;
    let new_chunk_count = (chunks.len() as f32 * reduction_ratio).ceil() as usize;

    tracing::debug!(
        reduction_ratio = reduction_ratio,
        new_entity_count = new_entity_count,
        new_rel_count = new_rel_count,
        new_chunk_count = new_chunk_count,
        "OODA-231: balance_context proportional reduction"
    );

    entities.truncate(new_entity_count.max(1));
    relationships.truncate(new_rel_count);
    chunks.truncate(new_chunk_count);

    tracing::debug!(
        final_entities = entities.len(),
        final_rels = relationships.len(),
        final_chunks = chunks.len(),
        "OODA-231: balance_context after truncation"
    );

    (entities, relationships, chunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::MockTokenizer;

    fn create_test_entity(name: &str, description: &str) -> RetrievedEntity {
        RetrievedEntity {
            name: name.to_string(),
            entity_type: "TEST".to_string(),
            description: description.to_string(),
            score: 1.0,
            degree: 0,
            source_chunk_ids: Vec::new(),
            source_document_id: None,
            source_file_path: None,
        }
    }

    fn create_test_relationship(source: &str, target: &str) -> RetrievedRelationship {
        RetrievedRelationship {
            source: source.to_string(),
            target: target.to_string(),
            relation_type: "TEST".to_string(),
            description: "Test relationship".to_string(),
            score: 1.0,
            source_chunk_id: None,
            source_document_id: None,
            source_file_path: None,
        }
    }

    fn create_test_chunk(id: &str, content: &str) -> RetrievedChunk {
        RetrievedChunk::new(id, content, 1.0)
    }

    #[test]
    fn test_truncate_entities() {
        let tokenizer = MockTokenizer::with_rate(0.1); // 10 chars per token

        let entities = vec![
            create_test_entity("E1", "Short"),
            create_test_entity("E2", "A bit longer description"),
            create_test_entity("E3", "Another entity"),
        ];

        let truncated = truncate_entities(entities.clone(), 10, &tokenizer);

        // Should keep at least one entity
        assert!(truncated.len() > 0);
        assert!(truncated.len() <= entities.len());
    }

    #[test]
    fn test_truncate_relationships() {
        let tokenizer = MockTokenizer::with_rate(0.1);

        let rels = vec![
            create_test_relationship("A", "B"),
            create_test_relationship("C", "D"),
            create_test_relationship("E", "F"),
        ];

        let truncated = truncate_relationships(rels.clone(), 10, &tokenizer);

        assert!(truncated.len() > 0);
        assert!(truncated.len() <= rels.len());
    }

    #[test]
    fn test_truncate_chunks() {
        let tokenizer = MockTokenizer::with_rate(0.1);

        let chunks = vec![
            create_test_chunk("c1", "Short chunk"),
            create_test_chunk("c2", "This is a much longer chunk with more content"),
            create_test_chunk("c3", "Another chunk"),
        ];

        let truncated = truncate_chunks(chunks.clone(), 10, &tokenizer);

        assert!(truncated.len() > 0);
        assert!(truncated.len() <= chunks.len());
    }

    #[test]
    fn test_balance_context() {
        let tokenizer = MockTokenizer::with_rate(1.0); // 1 token per char
        let config = TruncationConfig {
            max_entity_tokens: 100,
            max_relation_tokens: 100,
            max_total_tokens: 10, // Very tight limit to force reduction
        };

        let entities = vec![
            create_test_entity("E1", "Description 1"),
            create_test_entity("E2", "Description 2"),
            create_test_entity("E3", "Description 3"),
        ];

        let rels = vec![
            create_test_relationship("A", "B"),
            create_test_relationship("C", "D"),
        ];

        let chunks = vec![
            create_test_chunk("c1", "Chunk 1"),
            create_test_chunk("c2", "Chunk 2"),
        ];

        let (balanced_entities, balanced_rels, balanced_chunks) = balance_context(
            entities.clone(),
            rels.clone(),
            chunks.clone(),
            &config,
            &tokenizer,
        );

        // Should reduce at least one category due to very small total limit
        assert!(
            balanced_entities.len() < entities.len()
                || balanced_rels.len() < rels.len()
                || balanced_chunks.len() < chunks.len()
        );
    }

    #[test]
    fn test_balance_context_within_limit() {
        let tokenizer = MockTokenizer::with_rate(0.01); // Very small tokens
        let config = TruncationConfig {
            max_entity_tokens: 1000,
            max_relation_tokens: 1000,
            max_total_tokens: 10000, // Large limit
        };

        let entities = vec![create_test_entity("E1", "Desc")];
        let rels = vec![create_test_relationship("A", "B")];
        let chunks = vec![create_test_chunk("c1", "Chunk")];

        let (balanced_entities, balanced_rels, balanced_chunks) = balance_context(
            entities.clone(),
            rels.clone(),
            chunks.clone(),
            &config,
            &tokenizer,
        );

        // Should keep all since within limit
        assert_eq!(balanced_entities.len(), entities.len());
        assert_eq!(balanced_rels.len(), rels.len());
        assert_eq!(balanced_chunks.len(), chunks.len());
    }

    #[test]
    fn test_truncation_config_default() {
        let config = TruncationConfig::default();

        assert_eq!(config.max_entity_tokens, 10000);
        assert_eq!(config.max_relation_tokens, 10000);
        assert_eq!(config.max_total_tokens, 30000);
    }
}
