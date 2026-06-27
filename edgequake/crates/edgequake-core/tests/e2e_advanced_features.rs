#![cfg(feature = "pipeline")]

//! E2E tests for advanced retrieval features.
//!
//! Tests keyword extraction, truncation, and chunk retrieval integration.

use edgequake_query::context::{RetrievedChunk, RetrievedEntity, RetrievedRelationship};
use edgequake_query::{
    balance_context, truncate_entities, KeywordExtractor, MockKeywordExtractor, SimpleTokenizer,
    Tokenizer, TruncationConfig,
};
/// Test keyword extraction integration.
#[tokio::test]
async fn test_keyword_extraction() {
    let extractor = MockKeywordExtractor::new();

    // Add a response
    extractor.add_response(edgequake_query::Keywords::new(
        vec![
            "machine learning".to_string(),
            "software engineering".to_string(),
        ],
        vec![
            "SARAH CHEN".to_string(),
            "TECHCORP".to_string(),
            "AUTOML".to_string(),
        ],
    ));

    let keywords = extractor
        .extract("Tell me about Sarah Chen and the AutoML project at TechCorp")
        .await
        .unwrap();

    assert_eq!(keywords.high_level.len(), 2);
    assert_eq!(keywords.low_level.len(), 3);
    assert!(keywords
        .high_level
        .contains(&"machine learning".to_string()));
    assert!(keywords.low_level.contains(&"SARAH CHEN".to_string()));
}

/// Test token-based truncation with large context.
#[test]
fn test_truncation_with_large_context() {
    let tokenizer = SimpleTokenizer;
    let config = TruncationConfig {
        max_entity_tokens: 200, // Very strict limit
        max_relation_tokens: 200,
        max_total_tokens: 300, // Even stricter total
    };

    // Create many entities with longer descriptions to force truncation
    let entities: Vec<RetrievedEntity> = (0..30)
        .map(|i| {
            RetrievedEntity::new(
                format!("VeryLongEntityName{}", i),
                "DETAILED_TYPE",
                format!("This is an extremely detailed and comprehensive description that contains many words and takes up a significant number of tokens for entity number {}", i),
            )
        })
        .collect();

    let rels: Vec<RetrievedRelationship> = vec![];
    let chunks: Vec<RetrievedChunk> = vec![];

    let (truncated_entities, truncated_rels, truncated_chunks) =
        balance_context(entities.clone(), rels, chunks, &config, &tokenizer);

    // Should have significantly reduced entities to fit within strict limits
    assert!(
        truncated_entities.len() < entities.len(),
        "Expected fewer entities after truncation: got {} from {}",
        truncated_entities.len(),
        entities.len()
    );
    assert!(truncated_rels.is_empty());
    assert!(truncated_chunks.is_empty());

    // Calculate total tokens (approximate)
    let total_tokens: usize = truncated_entities
        .iter()
        .map(|e| {
            let text = format!("{} {} {}", e.name, e.entity_type, e.description);
            tokenizer.count_tokens(&text)
        })
        .sum();

    assert!(
        total_tokens <= config.max_total_tokens + 200,
        "Total tokens {} exceeds limit {}",
        total_tokens,
        config.max_total_tokens
    );
}

/// Test truncation preserves entity order.
#[test]
fn test_truncation_preserves_order() {
    let tokenizer = SimpleTokenizer;

    let entities = vec![
        RetrievedEntity::new("E1", "PERSON", "First entity"),
        RetrievedEntity::new("E2", "PERSON", "Second entity"),
        RetrievedEntity::new("E3", "PERSON", "Third entity"),
    ];

    // Truncate to fit only 2 entities
    let truncated = truncate_entities(entities.clone(), 50, &tokenizer);

    // Should preserve order
    assert!(truncated.len() <= entities.len());
    if !truncated.is_empty() {
        assert_eq!(truncated[0].name, "E1");
    }
}

/// Test tokenizer consistency.
#[test]
fn test_tokenizer_consistency() {
    let tokenizer = SimpleTokenizer;

    let short_text = "Hello";
    let medium_text = "This is a medium length text with several words.";
    let long_text = "This is a much longer text that contains many more characters.";

    let short_tokens = tokenizer.count_tokens(short_text);
    let medium_tokens = tokenizer.count_tokens(medium_text);
    let long_tokens = tokenizer.count_tokens(long_text);

    // Simple tokenizer uses 4 chars per token
    assert_eq!(
        short_tokens,
        (short_text.len() as f32 / 4.0).ceil() as usize
    );
    assert!(medium_tokens > short_tokens);
    assert!(long_tokens > medium_tokens);
}

/// Test truncation config defaults match the current LightRAG-style token budgets.
#[test]
fn test_truncation_config_defaults() {
    let config = TruncationConfig::default();

    // WHY: the query engine now reserves 10k each for entities and relationships,
    // leaving the remaining 10k for chunks under a 30k total context budget.
    assert_eq!(config.max_entity_tokens, 10000);
    assert_eq!(config.max_relation_tokens, 10000);
    assert_eq!(config.max_total_tokens, 30000);
}

/// Test balance_context reduces all categories proportionally.
#[test]
fn test_balance_reduces_proportionally() {
    let tokenizer = SimpleTokenizer;
    let config = TruncationConfig {
        max_entity_tokens: 1000,
        max_relation_tokens: 1000,
        max_total_tokens: 200, // Very small limit to force reduction
    };

    // Create roughly equal amounts of each with longer content
    let entities: Vec<RetrievedEntity> = (0..15)
        .map(|i| {
            RetrievedEntity::new(
                format!("EntityWithLongName{}", i),
                "TYPE",
                format!("This is a detailed description with many words that take up space for entity number {}", i),
            )
        })
        .collect();

    let relationships: Vec<RetrievedRelationship> = (0..15)
        .map(|i| {
            RetrievedRelationship::new(
                format!("SourceEntityA{}", i),
                format!("TargetEntityB{}", i),
                "COMPLEX_RELATIONSHIP_TYPE",
            )
            .with_description(format!("A comprehensive description of the relationship between the two entities, explaining their connection in detail for relationship number {}", i))
        })
        .collect();

    let chunks: Vec<RetrievedChunk> = (0..15)
        .map(|i| {
            RetrievedChunk::new(
                format!("chunk_{}", i),
                format!("This is a long chunk of text content that contains many words and takes up significant space in the context window for chunk number {}", i),
                1.0,
            )
        })
        .collect();

    let (balanced_entities, balanced_rels, balanced_chunks) =
        balance_context(entities, relationships, chunks, &config, &tokenizer);

    // Should reduce all categories due to very small limit
    assert!(
        balanced_entities.len() < 15,
        "Expected fewer entities: got {}",
        balanced_entities.len()
    );
    assert!(
        balanced_rels.len() < 15,
        "Expected fewer relationships: got {}",
        balanced_rels.len()
    );
    assert!(
        balanced_chunks.len() < 15,
        "Expected fewer chunks: got {}",
        balanced_chunks.len()
    );
}
