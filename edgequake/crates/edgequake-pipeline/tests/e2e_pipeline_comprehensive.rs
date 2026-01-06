//! Comprehensive E2E tests for edgequake-pipeline.
//!
//! Tests cover:
//! - Chunker configuration and chunking strategies
//! - Entity and relationship extraction
//! - Knowledge graph merging
//! - Description summarization
//! - Pipeline processing
//! - Error handling

use std::sync::Arc;

use edgequake_pipeline::{
    CharacterBasedChunking, Chunker, ChunkerConfig, ChunkingStrategy, DescriptionSummarizer,
    EntityExtractor, ExtractedEntity, ExtractedRelationship, ExtractionResult, GleaningConfig,
    KnowledgeGraphMerger, MergerConfig, Pipeline, PipelineConfig, SimpleSummarizer,
    SummarizerConfig,
};
use edgequake_storage::{GraphStorage, MemoryGraphStorage, MemoryVectorStorage, VectorStorage};

// Sample document for testing
const SAMPLE_DOCUMENT: &str = r#"
Dr. Sarah Chen is a renowned computer scientist at Stanford University. 
She specializes in artificial intelligence and machine learning.
Sarah has published over 100 papers and received the Turing Award in 2023.

EdgeQuake is a Rust-based RAG framework developed by the engineering team.
It uses knowledge graphs to improve retrieval accuracy.
The system supports multiple storage backends including PostgreSQL and in-memory storage.

John Smith works with Sarah Chen on the EdgeQuake project.
They collaborate on the entity extraction algorithms.
"#;

// =============================================================================
// Chunker Tests
// =============================================================================

mod chunker_tests {
    use super::*;

    #[test]
    fn test_chunker_config_default() {
        let config = ChunkerConfig::default();
        assert!(config.chunk_size > 0);
        assert!(config.chunk_overlap < config.chunk_size);
        assert!(config.min_chunk_size > 0);
        assert!(!config.separators.is_empty());
    }

    #[test]
    fn test_chunker_config_custom() {
        let config = ChunkerConfig {
            chunk_size: 500,
            chunk_overlap: 50,
            min_chunk_size: 20,
            separators: vec!["\n".to_string(), " ".to_string()],
            preserve_sentences: true,
            split_by_character: None,
            split_by_character_only: false,
        };

        assert_eq!(config.chunk_size, 500);
        assert_eq!(config.chunk_overlap, 50);
        assert_eq!(config.min_chunk_size, 20);
    }

    #[test]
    fn test_chunker_basic_chunking() {
        let config = ChunkerConfig::default();
        let chunker = Chunker::new(config);

        let chunks = chunker.chunk(SAMPLE_DOCUMENT, "test-doc").unwrap();
        assert!(!chunks.is_empty());

        // Verify each chunk has content
        for chunk in &chunks {
            assert!(!chunk.content.is_empty());
            assert!(chunk.token_count > 0);
        }
    }

    #[test]
    fn test_chunker_small_document() {
        let config = ChunkerConfig::default();
        let chunker = Chunker::new(config);

        let small_doc = "This is a small document.";
        let chunks = chunker.chunk(small_doc, "small-doc").unwrap();

        // Small document should produce single chunk
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].content.contains("small document"));
    }

    #[test]
    fn test_chunker_empty_document() {
        let config = ChunkerConfig::default();
        let chunker = Chunker::new(config);

        let chunks = chunker.chunk("", "empty-doc").unwrap();
        // Empty doc should return empty or single empty chunk
        assert!(chunks.is_empty() || chunks[0].content.trim().is_empty());
    }

    #[test]
    fn test_chunker_chunk_ids_are_unique() {
        let config = ChunkerConfig::default();
        let chunker = Chunker::new(config);

        let long_doc = "A ".repeat(1000);
        let chunks = chunker.chunk(&long_doc, "long-doc").unwrap();

        let ids: Vec<_> = chunks.iter().map(|c| &c.id).collect();
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique_ids.len(), "Chunk IDs should be unique");
    }

    #[test]
    fn test_chunker_preserves_order() {
        let config = ChunkerConfig::default();
        let chunker = Chunker::new(config);

        let chunks = chunker.chunk(SAMPLE_DOCUMENT, "test-doc").unwrap();

        // Indices should be sequential
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i);
        }
    }
}

// =============================================================================
// Chunking Strategy Tests
// =============================================================================

mod chunking_strategy_tests {
    use super::*;

    #[tokio::test]
    async fn test_character_based_chunking_by_newline() {
        let strategy = CharacterBasedChunking::by_newline();
        let config = ChunkerConfig::default();

        let text = "Line 1\nLine 2\nLine 3";
        let chunks = strategy.chunk(text, &config).await.unwrap();

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].content, "Line 1");
        assert_eq!(chunks[1].content, "Line 2");
        assert_eq!(chunks[2].content, "Line 3");
    }

    #[tokio::test]
    async fn test_character_based_chunking_by_paragraph() {
        let strategy = CharacterBasedChunking::by_paragraph();
        let config = ChunkerConfig::default();

        let text = "Paragraph 1\n\nParagraph 2\n\nParagraph 3";
        let chunks = strategy.chunk(text, &config).await.unwrap();

        assert_eq!(chunks.len(), 3);
        assert!(chunks[0].content.contains("Paragraph 1"));
    }

    #[tokio::test]
    async fn test_character_based_chunking_custom_separator() {
        let strategy = CharacterBasedChunking::new("|||");
        let config = ChunkerConfig::default();

        let text = "Part A|||Part B|||Part C";
        let chunks = strategy.chunk(text, &config).await.unwrap();

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].content, "Part A");
    }

    #[tokio::test]
    async fn test_character_based_chunking_empty_parts() {
        let strategy = CharacterBasedChunking::by_newline();
        let config = ChunkerConfig::default();

        let text = "Line 1\n\n\nLine 2"; // Multiple empty lines
        let chunks = strategy.chunk(text, &config).await.unwrap();

        // Empty parts should be filtered out
        assert_eq!(chunks.len(), 2);
    }

    #[tokio::test]
    async fn test_chunking_strategy_name() {
        let strategy = CharacterBasedChunking::by_newline();
        assert_eq!(strategy.name(), "character_based");
    }

    #[tokio::test]
    async fn test_chunk_result_token_estimation() {
        let strategy = CharacterBasedChunking::by_newline();
        let config = ChunkerConfig::default();

        let text = "A simple line of text";
        let chunks = strategy.chunk(text, &config).await.unwrap();

        // Token count should be roughly chars / 4
        assert!(chunks[0].tokens > 0);
        assert!(chunks[0].tokens <= text.len());
    }
}

// =============================================================================
// Entity Extraction Tests
// =============================================================================

mod extraction_tests {
    use super::*;
    use edgequake_pipeline::TextChunk;

    #[test]
    fn test_extracted_entity_creation() {
        let entity = ExtractedEntity::new("EdgeQuake", "SOFTWARE", "A RAG framework");

        assert_eq!(entity.name, "EdgeQuake");
        assert_eq!(entity.entity_type, "SOFTWARE");
        assert_eq!(entity.description, "A RAG framework");
        assert_eq!(entity.importance, 0.5); // Default importance
    }

    #[test]
    fn test_extracted_entity_with_importance() {
        let entity =
            ExtractedEntity::new("EdgeQuake", "SOFTWARE", "A RAG framework").with_importance(0.9);

        assert_eq!(entity.importance, 0.9);
    }

    #[test]
    fn test_extracted_entity_importance_clamping() {
        let entity = ExtractedEntity::new("Test", "TYPE", "Desc").with_importance(1.5);

        assert_eq!(entity.importance, 1.0);

        let entity2 = ExtractedEntity::new("Test", "TYPE", "Desc").with_importance(-0.5);

        assert_eq!(entity2.importance, 0.0);
    }

    #[test]
    fn test_extracted_entity_with_source_span() {
        let entity = ExtractedEntity::new("Test", "TYPE", "Desc").with_source_span("source text");

        assert_eq!(entity.source_spans.len(), 1);
        assert_eq!(entity.source_spans[0], "source text");
    }

    #[test]
    fn test_extracted_relationship_creation() {
        let rel = ExtractedRelationship::new("Sarah Chen", "EdgeQuake", "DESIGNED");

        assert_eq!(rel.source, "Sarah Chen");
        assert_eq!(rel.target, "EdgeQuake");
        assert_eq!(rel.relation_type, "DESIGNED");
        assert_eq!(rel.weight, 0.5);
    }

    #[test]
    fn test_extracted_relationship_with_description() {
        let rel =
            ExtractedRelationship::new("A", "B", "RELATES_TO").with_description("A relates to B");

        assert_eq!(rel.description, "A relates to B");
    }

    #[test]
    fn test_extraction_result_creation() {
        let result = ExtractionResult::new("chunk-1");

        assert_eq!(result.source_chunk_id, "chunk-1");
        assert!(result.entities.is_empty());
        assert!(result.relationships.is_empty());
    }

    #[test]
    fn test_extraction_result_add_entity() {
        let mut result = ExtractionResult::new("chunk-1");
        let entity = ExtractedEntity::new("Test", "TYPE", "Desc");
        result.add_entity(entity);

        assert_eq!(result.entities.len(), 1);
    }

    #[test]
    fn test_extraction_result_add_relationship() {
        let mut result = ExtractionResult::new("chunk-1");
        let rel = ExtractedRelationship::new("A", "B", "TYPE");
        result.add_relationship(rel);

        assert_eq!(result.relationships.len(), 1);
    }

    #[tokio::test]
    async fn test_simple_extractor_with_chunk() {
        use edgequake_pipeline::SimpleExtractor;

        let extractor = SimpleExtractor::default();

        let chunk = TextChunk::new("chunk-1", "Sarah Chen works on EdgeQuake.", 0, 0, 30);

        let result = extractor.extract(&chunk).await.unwrap();

        // SimpleExtractor should find the person name
        assert!(!result.source_chunk_id.is_empty());
        // May or may not find entities depending on regex patterns
    }
}

// =============================================================================
// Gleaning Config Tests
// =============================================================================

mod gleaning_tests {
    use super::*;

    #[test]
    fn test_gleaning_config_default() {
        let config = GleaningConfig::default();

        assert_eq!(config.max_gleaning, 1);
        assert!(!config.always_glean);
    }

    #[test]
    fn test_gleaning_config_custom() {
        let config = GleaningConfig {
            max_gleaning: 3,
            always_glean: true,
        };

        assert_eq!(config.max_gleaning, 3);
        assert!(config.always_glean);
    }
}

// =============================================================================
// Merger Tests
// =============================================================================

mod merger_tests {
    use super::*;

    async fn create_merger() -> KnowledgeGraphMerger<MemoryGraphStorage, MemoryVectorStorage> {
        let graph = MemoryGraphStorage::new("test");
        let vector = MemoryVectorStorage::new("test", 1536);
        graph.initialize().await.unwrap();
        vector.initialize().await.unwrap();

        let config = MergerConfig::default();
        KnowledgeGraphMerger::new(config, Arc::new(graph), Arc::new(vector))
    }

    #[test]
    fn test_merger_config_default() {
        let config = MergerConfig::default();

        assert!(config.max_description_length > 0);
        assert!(config.description_decay > 0.0 && config.description_decay <= 1.0);
        assert!(config.min_importance >= 0.0 && config.min_importance <= 1.0);
        assert!(config.max_sources > 0);
    }

    #[test]
    fn test_merger_config_custom() {
        let config = MergerConfig {
            max_description_length: 8192,
            description_decay: 0.8,
            min_importance: 0.2,
            max_sources: 20,
            use_llm_summarization: true,
        };

        assert_eq!(config.max_description_length, 8192);
        assert_eq!(config.max_sources, 20);
    }

    #[tokio::test]
    async fn test_merger_creation() {
        let _merger = create_merger().await;
        // Merger created successfully
    }

    #[tokio::test]
    async fn test_merger_with_tenant_context() {
        let merger = create_merger().await;
        let merger = merger.with_tenant_context(
            Some("tenant-1".to_string()),
            Some("workspace-1".to_string()),
        );
        // Merger configured with tenant context
        drop(merger);
    }

    #[tokio::test]
    async fn test_merge_empty_results() {
        let merger = create_merger().await;
        let stats = merger.merge(vec![]).await.unwrap();

        assert_eq!(stats.entities_created, 0);
        assert_eq!(stats.entities_updated, 0);
        assert_eq!(stats.relationships_created, 0);
    }

    #[tokio::test]
    async fn test_merge_single_entity() {
        let merger = create_merger().await;

        let mut result = ExtractionResult::new("chunk-1");
        result.add_entity(ExtractedEntity::new(
            "EdgeQuake",
            "SOFTWARE",
            "A RAG framework",
        ));

        let stats = merger.merge(vec![result]).await.unwrap();

        assert_eq!(stats.entities_created, 1);
    }

    #[tokio::test]
    async fn test_merge_multiple_entities() {
        let merger = create_merger().await;

        let mut result = ExtractionResult::new("chunk-1");
        result.add_entity(ExtractedEntity::new("EdgeQuake", "SOFTWARE", "Framework"));
        result.add_entity(ExtractedEntity::new("Sarah Chen", "PERSON", "Scientist"));
        result.add_entity(ExtractedEntity::new(
            "Stanford",
            "ORGANIZATION",
            "University",
        ));

        let stats = merger.merge(vec![result]).await.unwrap();

        assert_eq!(stats.entities_created, 3);
    }

    #[tokio::test]
    async fn test_merge_entity_with_relationship() {
        let merger = create_merger().await;

        let mut result = ExtractionResult::new("chunk-1");
        result.add_entity(ExtractedEntity::new("Sarah", "PERSON", "Scientist"));
        result.add_entity(ExtractedEntity::new("EdgeQuake", "SOFTWARE", "Framework"));
        result.add_relationship(
            ExtractedRelationship::new("Sarah", "EdgeQuake", "DESIGNED")
                .with_description("Sarah designed EdgeQuake"),
        );

        let stats = merger.merge(vec![result]).await.unwrap();

        assert_eq!(stats.entities_created, 2);
        assert_eq!(stats.relationships_created, 1);
    }

    #[tokio::test]
    async fn test_merge_multiple_results() {
        let merger = create_merger().await;

        let mut result1 = ExtractionResult::new("chunk-1");
        result1.add_entity(ExtractedEntity::new("Entity1", "TYPE1", "Desc1"));

        let mut result2 = ExtractionResult::new("chunk-2");
        result2.add_entity(ExtractedEntity::new("Entity2", "TYPE2", "Desc2"));

        let stats = merger.merge(vec![result1, result2]).await.unwrap();

        assert_eq!(stats.entities_created, 2);
    }
}

// =============================================================================
// Summarizer Tests
// =============================================================================

mod summarizer_tests {
    use super::*;

    #[test]
    fn test_summarizer_config_default() {
        let config = SummarizerConfig::default();

        assert!(config.max_input_length > 0);
        assert!(config.target_length > 0);
        assert!(config.max_tokens_per_chunk > 0);
    }

    #[test]
    fn test_summarizer_config_builder() {
        let config = SummarizerConfig::default()
            .with_target_length(256)
            .with_force_threshold(3);

        assert_eq!(config.target_length, 256);
        assert_eq!(config.force_llm_summary_threshold, 3);
    }

    #[tokio::test]
    async fn test_simple_summarizer_short_text() {
        let summarizer = SimpleSummarizer::default();

        let short_text = "This is a short text.";
        let result = summarizer.summarize(short_text).await.unwrap();

        // Short text should pass through unchanged
        assert_eq!(result, short_text);
    }

    #[tokio::test]
    async fn test_simple_summarizer_long_text() {
        let config = SummarizerConfig::default().with_target_length(50);
        let summarizer = SimpleSummarizer::new(config);

        let long_text = "This is a very long sentence that goes on and on. It has multiple parts. And it just keeps going with more and more words. This should be truncated.";
        let result = summarizer.summarize(long_text).await.unwrap();

        assert!(result.len() <= 60); // Allow some flexibility
    }

    #[tokio::test]
    async fn test_simple_summarizer_empty_text() {
        let summarizer = SimpleSummarizer::default();
        let result = summarizer.summarize("").await.unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_simple_summarizer_sentence_boundary() {
        let config = SummarizerConfig::default().with_target_length(100);
        let summarizer = SimpleSummarizer::new(config);

        let text =
            "First sentence. Second sentence. Third sentence. Fourth sentence. Fifth sentence.";
        let result = summarizer.summarize(text).await.unwrap();

        // Should end at a sentence boundary
        assert!(result.ends_with('.'));
    }

    #[tokio::test]
    async fn test_summarize_combined() {
        let summarizer = SimpleSummarizer::default();

        let descriptions = vec!["Description one.", "Description two."];
        let refs: Vec<&str> = descriptions.iter().map(|s| s.as_ref()).collect();
        let result = summarizer.summarize_combined(&refs).await.unwrap();

        assert!(result.contains("Description"));
    }
}

// =============================================================================
// Pipeline Tests
// =============================================================================

mod pipeline_tests {
    use super::*;

    #[test]
    fn test_pipeline_config_default() {
        let config = PipelineConfig::default();

        assert!(config.extraction_batch_size > 0);
        assert!(config.embedding_batch_size > 0);
        assert!(config.enable_entity_extraction);
        assert!(config.enable_relationship_extraction);
    }

    #[test]
    fn test_pipeline_creation() {
        let config = PipelineConfig::default();
        let _pipeline = Pipeline::new(config);
    }

    #[test]
    fn test_pipeline_default_creation() {
        let _pipeline = Pipeline::default_pipeline();
    }

    #[tokio::test]
    async fn test_pipeline_process_without_extractor() {
        let pipeline = Pipeline::default_pipeline();

        // Without an extractor, entities/relationships won't be extracted
        let result = pipeline.process("doc-1", SAMPLE_DOCUMENT).await.unwrap();

        assert_eq!(result.document_id, "doc-1");
        assert!(!result.chunks.is_empty());
        // No extractor means no entities
        assert_eq!(result.stats.entity_count, 0);
    }

    #[tokio::test]
    async fn test_pipeline_process_with_simple_extractor() {
        use edgequake_pipeline::SimpleExtractor;

        let extractor = Arc::new(SimpleExtractor::default());

        let pipeline = Pipeline::default_pipeline().with_extractor(extractor);

        let result = pipeline.process("doc-1", SAMPLE_DOCUMENT).await.unwrap();

        assert!(!result.chunks.is_empty());
        assert!(result.stats.llm_calls > 0);
    }

    #[tokio::test]
    async fn test_pipeline_process_empty_document() {
        let pipeline = Pipeline::default_pipeline();
        let result = pipeline.process("doc-empty", "").await.unwrap();

        assert_eq!(result.stats.chunk_count, 0);
    }

    #[tokio::test]
    async fn test_pipeline_process_small_document() {
        let pipeline = Pipeline::default_pipeline();
        let result = pipeline.process("doc-small", "Hello world.").await.unwrap();

        assert_eq!(result.stats.chunk_count, 1);
    }

    #[tokio::test]
    async fn test_pipeline_stats_tracking() {
        use edgequake_pipeline::SimpleExtractor;

        let extractor = Arc::new(SimpleExtractor::default());

        let pipeline = Pipeline::default_pipeline().with_extractor(extractor);

        let result = pipeline.process("doc-1", SAMPLE_DOCUMENT).await.unwrap();

        // Stats should be populated
        assert!(result.stats.chunking_strategy.is_some());
        if !result.chunks.is_empty() {
            assert!(result.stats.avg_chunk_size.is_some());
        }
    }

    #[tokio::test]
    async fn test_pipeline_multiple_documents() {
        let pipeline = Pipeline::default_pipeline();

        let docs = vec![
            ("doc-1", "First document content."),
            ("doc-2", "Second document content."),
            ("doc-3", "Third document content."),
        ];

        for (id, content) in docs {
            let result = pipeline.process(id, content).await.unwrap();
            assert_eq!(result.document_id, id);
        }
    }
}

// =============================================================================
// Error Handling Tests
// =============================================================================

mod error_tests {
    use edgequake_pipeline::PipelineError;

    #[test]
    fn test_pipeline_error_display() {
        let error = PipelineError::ExtractionError("test error".to_string());
        let display = format!("{}", error);
        assert!(display.contains("test error"));
    }

    #[test]
    fn test_pipeline_error_from_string() {
        let error: PipelineError = PipelineError::ExtractionError("custom error".to_string());
        assert!(matches!(error, PipelineError::ExtractionError(_)));
    }
}

// =============================================================================
// Concurrent Processing Tests
// =============================================================================

mod concurrent_tests {
    use super::*;
    use tokio::task::JoinSet;

    #[tokio::test]
    async fn test_concurrent_pipeline_processing() {
        let pipeline = Arc::new(Pipeline::default_pipeline());

        let mut join_set = JoinSet::new();

        for i in 0..5 {
            let p = pipeline.clone();
            let doc_id = format!("doc-{}", i);
            let content = format!("Document {} content for testing.", i);

            join_set.spawn(async move { p.process(&doc_id, &content).await });
        }

        let mut completed = 0;
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(_)) => completed += 1,
                Ok(Err(e)) => panic!("Processing failed: {}", e),
                Err(e) => panic!("Task panicked: {}", e),
            }
        }

        assert_eq!(completed, 5);
    }

    #[tokio::test]
    async fn test_concurrent_chunking() {
        let strategies: Vec<Arc<dyn ChunkingStrategy>> = vec![
            Arc::new(CharacterBasedChunking::by_newline()),
            Arc::new(CharacterBasedChunking::by_paragraph()),
        ];

        let config = ChunkerConfig::default();
        let text = "Line 1\nLine 2\n\nParagraph 2";

        let mut join_set = JoinSet::new();

        for strategy in strategies {
            let config = config.clone();
            let text = text.to_string();
            join_set.spawn(async move { strategy.chunk(&text, &config).await });
        }

        let mut results = 0;
        while let Some(result) = join_set.join_next().await {
            assert!(result.unwrap().is_ok());
            results += 1;
        }

        assert_eq!(results, 2);
    }

    #[tokio::test]
    async fn test_concurrent_summarization() {
        let summarizer = Arc::new(SimpleSummarizer::default());

        let texts = vec![
            "Text one for summarization.",
            "Text two for summarization.",
            "Text three for summarization.",
        ];

        let mut join_set = JoinSet::new();

        for text in texts {
            let s = summarizer.clone();
            let t = text.to_string();
            join_set.spawn(async move { s.summarize(&t).await });
        }

        let mut completed = 0;
        while let Some(result) = join_set.join_next().await {
            assert!(result.unwrap().is_ok());
            completed += 1;
        }

        assert_eq!(completed, 3);
    }
}

// =============================================================================
// Integration Tests
// =============================================================================

mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_extraction_to_merge_flow() {
        // Step 1: Create extraction results
        let mut result1 = ExtractionResult::new("chunk-1");
        result1.add_entity(
            ExtractedEntity::new("Sarah Chen", "PERSON", "A computer scientist")
                .with_importance(0.9),
        );
        result1.add_entity(ExtractedEntity::new(
            "Stanford",
            "ORGANIZATION",
            "University",
        ));

        let mut result2 = ExtractionResult::new("chunk-2");
        result2.add_entity(ExtractedEntity::new(
            "EdgeQuake",
            "SOFTWARE",
            "RAG framework",
        ));
        result2.add_relationship(
            ExtractedRelationship::new("Sarah Chen", "EdgeQuake", "DESIGNED")
                .with_description("Sarah designed EdgeQuake"),
        );

        // Step 2: Merge into graph
        let graph = Arc::new(MemoryGraphStorage::new("test"));
        let vector = Arc::new(MemoryVectorStorage::new("test", 1536));
        graph.initialize().await.unwrap();
        vector.initialize().await.unwrap();

        let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector);

        let stats = merger.merge(vec![result1, result2]).await.unwrap();

        assert_eq!(stats.entities_created, 3);
        assert_eq!(stats.relationships_created, 1);

        // Step 3: Verify graph state
        let nodes = graph.get_all_nodes().await.unwrap();
        assert_eq!(nodes.len(), 3);
    }

    #[tokio::test]
    async fn test_chunker_extractor_integration() {
        use edgequake_pipeline::SimpleExtractor;

        let extractor = SimpleExtractor::default();

        // Chunk document
        let chunker = Chunker::new(ChunkerConfig::default());
        let chunks = chunker.chunk(SAMPLE_DOCUMENT, "doc-1").unwrap();

        // Extract from each chunk
        for chunk in chunks {
            let result = extractor.extract(&chunk).await.unwrap();
            assert!(!result.source_chunk_id.is_empty());
        }
    }
}
