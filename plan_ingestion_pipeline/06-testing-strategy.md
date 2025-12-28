# Testing Strategy: SOTA Ingestion Pipeline

> Document ID: TEST-001
> Version: 1.0
> Created: 2024-12-28

## Table of Contents

1. [Testing Philosophy](#1-testing-philosophy)
2. [Unit Testing](#2-unit-testing)
3. [Integration Testing](#3-integration-testing)
4. [End-to-End Testing](#4-end-to-end-testing)
5. [Performance Testing](#5-performance-testing)
6. [Test Data Management](#6-test-data-management)
7. [Continuous Integration](#7-continuous-integration)
8. [Quality Gates](#8-quality-gates)

---

## 1. Testing Philosophy

### 1.1 Pyramid Strategy

```
                    ┌─────────┐
                    │   E2E   │     5% - Critical Paths
                    │  Tests  │     Manual + Automated
                    ├─────────┤
                    │ Integra-│    15% - Component Boundaries
                    │   tion  │     Mock LLMs
                    ├─────────┤
                    │  Unit   │    80% - Business Logic
                    │  Tests  │     Fast, Isolated
                    └─────────┘
```

### 1.2 Key Principles

1. **Fast Feedback** - Unit tests run in < 5 seconds
2. **Mock LLMs by Default** - Real LLM tests opt-in via feature flag
3. **Deterministic** - Same input, same output
4. **Coverage Targets** - 80% line coverage, 100% for critical paths
5. **Property-Based Testing** - For parsing and edge cases

---

## 2. Unit Testing

### 2.1 Chunker Tests

**File:** `edgequake-pipeline/tests/chunker_tests.rs`

```rust
use edgequake_pipeline::chunker::{Chunker, ChunkerConfig, TextChunk};

mod line_number_tests {
    use super::*;

    #[test]
    fn test_single_line_chunk() {
        let config = ChunkerConfig {
            chunk_size: 1000,
            overlap: 0,
            ..Default::default()
        };
        let chunker = Chunker::new(config);

        let text = "This is a single line.";
        let chunks = chunker.chunk_sync(text, "doc-1").unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start_line, 1);
        assert_eq!(chunks[0].end_line, 1);
    }

    #[test]
    fn test_multi_line_chunk() {
        let config = ChunkerConfig {
            chunk_size: 1000,
            overlap: 0,
            ..Default::default()
        };
        let chunker = Chunker::new(config);

        let text = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        let chunks = chunker.chunk_sync(text, "doc-1").unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start_line, 1);
        assert_eq!(chunks[0].end_line, 5);
    }

    #[test]
    fn test_multiple_chunks_line_numbers() {
        let config = ChunkerConfig {
            chunk_size: 20, // Force multiple chunks
            overlap: 0,
            ..Default::default()
        };
        let chunker = Chunker::new(config);

        let text = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        let chunks = chunker.chunk_sync(text, "doc-1").unwrap();

        // Verify each chunk has correct line numbers
        assert!(chunks.len() > 1);
        assert_eq!(chunks[0].start_line, 1);

        // Last chunk should end at line 5
        let last = chunks.last().unwrap();
        assert_eq!(last.end_line, 5);

        // Line numbers should be contiguous
        for i in 1..chunks.len() {
            assert!(chunks[i].start_line <= chunks[i - 1].end_line + 1);
        }
    }

    #[test]
    fn test_overlap_line_numbers() {
        let config = ChunkerConfig {
            chunk_size: 30,
            overlap: 10,
            ..Default::default()
        };
        let chunker = Chunker::new(config);

        let text = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        let chunks = chunker.chunk_sync(text, "doc-1").unwrap();

        // Overlapping chunks may have overlapping line numbers
        if chunks.len() > 1 {
            // Verify overlap creates correct line boundaries
            for chunk in &chunks {
                assert!(chunk.start_line <= chunk.end_line);
                assert!(chunk.start_line >= 1);
                assert!(chunk.end_line <= 5);
            }
        }
    }

    #[test]
    fn test_empty_lines() {
        let config = ChunkerConfig::default();
        let chunker = Chunker::new(config);

        let text = "Line 1\n\n\nLine 4";
        let chunks = chunker.chunk_sync(text, "doc-1").unwrap();

        // Empty lines should be counted
        assert_eq!(chunks[0].start_line, 1);
        assert_eq!(chunks[0].end_line, 4);
    }
}

mod offset_tests {
    use super::*;

    #[test]
    fn test_offsets_match_content() {
        let config = ChunkerConfig::default();
        let chunker = Chunker::new(config);

        let text = "Hello, world! This is a test.";
        let chunks = chunker.chunk_sync(text, "doc-1").unwrap();

        for chunk in &chunks {
            let extracted = &text[chunk.start_offset..chunk.end_offset];
            assert_eq!(extracted, chunk.content);
        }
    }
}

mod token_count_tests {
    use super::*;

    #[test]
    fn test_token_count_estimation() {
        let config = ChunkerConfig::default();
        let chunker = Chunker::new(config);

        let text = "One two three four five six seven eight nine ten.";
        let chunks = chunker.chunk_sync(text, "doc-1").unwrap();

        // Rough estimation: ~10-15 tokens for 10 words
        assert!(chunks[0].token_count >= 10);
        assert!(chunks[0].token_count <= 20);
    }
}
```

### 2.2 Extractor Tests

**File:** `edgequake-pipeline/tests/extractor_tests.rs`

```rust
use edgequake_pipeline::extractor::{
    ExtractedEntity, ExtractionResult, LLMExtractor,
    EntityExtractor, GleaningExtractor,
};
use edgequake_llm::mock::MockLLMProvider;

mod entity_parsing_tests {
    use super::*;

    #[test]
    fn test_parse_tuple_format() {
        let response = r#"("entity"<|#|>ALICE CHEN<|#|>person<|#|>A software engineer working at TechCorp)
("entity"<|#|>TECHCORP<|#|>organization<|#|>A technology company based in San Francisco)
("relationship"<|#|>ALICE CHEN<|#|>WORKS_AT<|#|>TECHCORP<|#|>Alice is employed by TechCorp<|#|>8)"#;

        let result = parse_lightrag_response(response, "chunk-1").unwrap();

        assert_eq!(result.entities.len(), 2);
        assert_eq!(result.relationships.len(), 1);

        assert_eq!(result.entities[0].name, "ALICE_CHEN");
        assert_eq!(result.entities[0].entity_type, "PERSON");

        assert_eq!(result.relationships[0].source, "ALICE_CHEN");
        assert_eq!(result.relationships[0].target, "TECHCORP");
        assert_eq!(result.relationships[0].weight, 8.0);
    }

    #[test]
    fn test_entity_name_normalization() {
        let cases = vec![
            ("Alice Chen", "ALICE_CHEN"),
            ("TechCorp Inc.", "TECHCORP_INC"),
            ("New York City", "NEW_YORK_CITY"),
            ("  John   Doe  ", "JOHN_DOE"),
            ("O'Brien", "O_BRIEN"),
        ];

        for (input, expected) in cases {
            assert_eq!(normalize_entity_name(input), expected);
        }
    }

    #[test]
    fn test_skip_generic_entities() {
        let entities = vec![
            ExtractedEntity::new("THE", "person", "Generic article"),
            ExtractedEntity::new("ALICE_CHEN", "person", "A developer"),
            ExtractedEntity::new("IT", "thing", "Pronoun"),
        ];

        let filtered: Vec<_> = entities
            .into_iter()
            .filter(|e| !is_generic_entity(&e.name))
            .collect();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "ALICE_CHEN");
    }
}

mod extraction_tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_extraction() {
        let mock = MockLLMProvider::new();
        let extractor = LLMExtractor::new(Arc::new(mock));

        let chunk = TextChunk {
            id: "chunk-1".to_string(),
            content: "Alice Chen works at TechCorp.".to_string(),
            ..Default::default()
        };

        let result = extractor.extract(&chunk).await.unwrap();

        // Mock provider returns deterministic entities
        assert!(!result.entities.is_empty());
    }

    #[tokio::test]
    async fn test_gleaning_adds_entities() {
        let mock = MockLLMProvider::new();
        let base_extractor = LLMExtractor::new(Arc::new(mock.clone()));
        let gleaning = GleaningExtractor::new(Arc::new(base_extractor), 2);

        let chunk = TextChunk {
            id: "chunk-1".to_string(),
            content: "Alice Chen and Bob Smith work at TechCorp. They collaborate with Carol.".to_string(),
            ..Default::default()
        };

        let result = gleaning.extract(&chunk).await.unwrap();

        // Gleaning should extract more entities than single pass
        // (depends on mock behavior)
        assert!(!result.entities.is_empty());
    }
}

mod token_tracking_tests {
    use super::*;

    #[tokio::test]
    async fn test_token_usage_tracking() {
        let mock = MockLLMProvider::new_with_token_tracking();
        let extractor = LLMExtractor::new(Arc::new(mock));

        let chunk = TextChunk {
            id: "chunk-1".to_string(),
            content: "Test content".to_string(),
            ..Default::default()
        };

        let result = extractor.extract(&chunk).await.unwrap();

        assert!(result.input_tokens > 0);
        assert!(result.output_tokens > 0);
        assert!(result.extraction_time_ms > 0);
    }
}
```

### 2.3 MapReduce Summarizer Tests

**File:** `edgequake-pipeline/tests/summarizer_tests.rs`

```rust
use edgequake_pipeline::summarizer::{MapReduceSummarizer, SummarizerConfig};
use edgequake_llm::mock::MockLLMProvider;

mod summarization_tests {
    use super::*;

    #[tokio::test]
    async fn test_single_description_passthrough() {
        let mock = MockLLMProvider::new();
        let summarizer = MapReduceSummarizer::new(
            Arc::new(mock),
            SummarizerConfig::default(),
        );

        let descriptions = vec!["Single description".to_string()];
        let (result, summarized) = summarizer.summarize(descriptions).await.unwrap();

        assert_eq!(result, "Single description");
        assert!(!summarized); // No summarization needed
    }

    #[tokio::test]
    async fn test_small_set_concatenation() {
        let mock = MockLLMProvider::new();
        let config = SummarizerConfig {
            force_llm_summary_on_merge: 10,
            context_size: 10000,
            ..Default::default()
        };
        let summarizer = MapReduceSummarizer::new(Arc::new(mock), config);

        let descriptions = vec![
            "Desc 1".to_string(),
            "Desc 2".to_string(),
            "Desc 3".to_string(),
        ];

        let (result, summarized) = summarizer.summarize(descriptions).await.unwrap();

        // Should concatenate without LLM
        assert!(result.contains("Desc 1"));
        assert!(result.contains("Desc 2"));
        assert!(result.contains("Desc 3"));
        assert!(!summarized);
    }

    #[tokio::test]
    async fn test_large_set_mapreduce() {
        let mock = MockLLMProvider::new();
        let config = SummarizerConfig {
            force_llm_summary_on_merge: 3, // Force LLM after 3
            context_size: 100,
            ..Default::default()
        };
        let summarizer = MapReduceSummarizer::new(Arc::new(mock), config);

        let descriptions = (0..10)
            .map(|i| format!("Description number {} with some text", i))
            .collect();

        let (result, summarized) = summarizer.summarize(descriptions).await.unwrap();

        // Should have triggered LLM summarization
        assert!(summarized);
        // Result should be shorter than concatenation
        assert!(result.len() < 500);
    }

    #[tokio::test]
    async fn test_context_size_splitting() {
        let mock = MockLLMProvider::new();
        let config = SummarizerConfig {
            context_size: 50, // Very small to force splitting
            force_llm_summary_on_merge: 100,
            ..Default::default()
        };
        let summarizer = MapReduceSummarizer::new(Arc::new(mock), config);

        let descriptions = vec![
            "A".repeat(30),
            "B".repeat(30),
            "C".repeat(30),
        ];

        let (_, summarized) = summarizer.summarize(descriptions).await.unwrap();

        // Should split into chunks and summarize
        assert!(summarized);
    }
}
```

### 2.4 Cache Tests

**File:** `edgequake-pipeline/tests/cache_tests.rs`

```rust
use edgequake_pipeline::cache::{
    CacheEntry, CacheType, LLMCache, MemoryLLMCache,
};
use chrono::Utc;

mod memory_cache_tests {
    use super::*;

    #[tokio::test]
    async fn test_set_and_get() {
        let cache = MemoryLLMCache::new();

        let entry = CacheEntry {
            id: "entry-1".to_string(),
            cache_type: CacheType::Extract,
            chunk_id: Some("chunk-1".to_string()),
            prompt_hash: "hash-abc".to_string(),
            response: "LLM response".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            model: "gpt-4o-mini".to_string(),
            created_at: Utc::now(),
        };

        cache.set(entry.clone()).await.unwrap();

        let retrieved = cache.get("hash-abc").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().response, "LLM response");
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let cache = MemoryLLMCache::new();

        let result = cache.get("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_by_chunk() {
        let cache = MemoryLLMCache::new();

        // Add multiple entries for same chunk
        for i in 0..3 {
            let entry = CacheEntry {
                id: format!("entry-{}", i),
                cache_type: CacheType::Extract,
                chunk_id: Some("chunk-1".to_string()),
                prompt_hash: format!("hash-{}", i),
                response: format!("Response {}", i),
                input_tokens: 100,
                output_tokens: 50,
                model: "gpt-4o-mini".to_string(),
                created_at: Utc::now(),
            };
            cache.set(entry).await.unwrap();
        }

        let entries = cache.get_by_chunk("chunk-1").await.unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[tokio::test]
    async fn test_delete_by_chunk() {
        let cache = MemoryLLMCache::new();

        // Add entries for two chunks
        for i in 0..2 {
            cache.set(CacheEntry {
                id: format!("chunk1-entry-{}", i),
                cache_type: CacheType::Extract,
                chunk_id: Some("chunk-1".to_string()),
                prompt_hash: format!("hash-1-{}", i),
                response: "Response".to_string(),
                input_tokens: 100,
                output_tokens: 50,
                model: "gpt-4o-mini".to_string(),
                created_at: Utc::now(),
            }).await.unwrap();
        }

        cache.set(CacheEntry {
            id: "chunk2-entry".to_string(),
            cache_type: CacheType::Extract,
            chunk_id: Some("chunk-2".to_string()),
            prompt_hash: "hash-2".to_string(),
            response: "Response".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            model: "gpt-4o-mini".to_string(),
            created_at: Utc::now(),
        }).await.unwrap();

        let deleted = cache.delete_by_chunk("chunk-1").await.unwrap();
        assert_eq!(deleted, 2);

        // chunk-2 should still exist
        let remaining = cache.get_by_chunk("chunk-2").await.unwrap();
        assert_eq!(remaining.len(), 1);
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = MemoryLLMCache::new();

        // Add some entries
        cache.set(CacheEntry {
            id: "entry-1".to_string(),
            cache_type: CacheType::Extract,
            chunk_id: Some("chunk-1".to_string()),
            prompt_hash: "hash-1".to_string(),
            response: "Response".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            model: "gpt-4o-mini".to_string(),
            created_at: Utc::now(),
        }).await.unwrap();

        cache.clear().await.unwrap();

        let result = cache.get("hash-1").await.unwrap();
        assert!(result.is_none());
    }
}
```

### 2.5 Cost Calculator Tests

**File:** `edgequake-core/tests/cost_tests.rs`

```rust
use edgequake_core::cost::{CostCalculator, ModelCost};

mod cost_calculation_tests {
    use super::*;

    #[test]
    fn test_gpt4o_mini_cost() {
        let calc = CostCalculator::new();

        // 1000 input + 1000 output
        let cost = calc.calculate("gpt-4o-mini", 1000, 1000);

        // $0.00015/1K input + $0.0006/1K output
        let expected = 0.00015 + 0.0006;
        assert!((cost - expected).abs() < 0.0001);
    }

    #[test]
    fn test_gpt4_cost() {
        let calc = CostCalculator::new();

        let cost = calc.calculate("gpt-4", 1000, 1000);

        // $0.03/1K input + $0.06/1K output
        let expected = 0.03 + 0.06;
        assert!((cost - expected).abs() < 0.0001);
    }

    #[test]
    fn test_embedding_cost() {
        let calc = CostCalculator::new();

        let cost = calc.calculate("text-embedding-3-small", 10000, 0);

        // $0.00002/1K tokens * 10
        let expected = 0.0002;
        assert!((cost - expected).abs() < 0.0001);
    }

    #[test]
    fn test_unknown_model() {
        let calc = CostCalculator::new();

        let cost = calc.calculate("unknown-model", 1000, 1000);

        // Unknown model = $0
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_custom_cost() {
        let calc = CostCalculator::new()
            .with_custom_cost("custom-model", ModelCost {
                input_per_1k: 0.01,
                output_per_1k: 0.02,
            });

        let cost = calc.calculate("custom-model", 1000, 1000);

        assert!((cost - 0.03).abs() < 0.0001);
    }

    #[test]
    fn test_breakdown_creation() {
        let calc = CostCalculator::new();

        let stats = ProcessingStats {
            extraction_calls: 10,
            extraction_input_tokens: 5000,
            extraction_output_tokens: 2000,
            gleaning_calls: 5,
            gleaning_input_tokens: 2500,
            gleaning_output_tokens: 1000,
            summarization_calls: 2,
            summarization_input_tokens: 1000,
            summarization_output_tokens: 500,
            embedding_calls: 15,
            embedding_tokens: 10000,
        };

        let breakdown = calc.create_breakdown(
            "gpt-4o-mini",
            "text-embedding-3-small",
            &stats,
        );

        assert!(breakdown.total_usd > 0.0);
        assert_eq!(breakdown.extraction.api_calls, 10);
        assert_eq!(breakdown.embedding.api_calls, 15);
    }
}
```

---

## 3. Integration Testing

### 3.1 Pipeline Integration Tests

**File:** `edgequake-core/tests/pipeline_integration.rs`

```rust
use edgequake_core::pipeline::{Pipeline, PipelineConfig};
use edgequake_llm::mock::MockLLMProvider;
use edgequake_storage::memory::MemoryStorage;

mod pipeline_tests {
    use super::*;

    async fn create_test_pipeline() -> Pipeline {
        let config = PipelineConfig {
            enable_entity_extraction: true,
            enable_relationship_extraction: true,
            max_concurrent_extractions: 4,
            ..Default::default()
        };

        let llm = Arc::new(MockLLMProvider::new());
        let storage = Arc::new(MemoryStorage::new());

        Pipeline::new(config, llm, storage)
    }

    #[tokio::test]
    async fn test_full_pipeline_processing() {
        let pipeline = create_test_pipeline().await;

        let content = r#"
Alice Chen is a software engineer at TechCorp.
She works with Bob Smith on the EdgeQuake project.
TechCorp is based in San Francisco, California.
The project uses Rust and PostgreSQL.
"#;

        let result = pipeline.process("doc-1", content).await.unwrap();

        assert!(!result.chunks.is_empty());
        assert!(!result.entities.is_empty());
        assert!(result.stats.llm_calls > 0);
    }

    #[tokio::test]
    async fn test_pipeline_with_progress_tracking() {
        let pipeline = create_test_pipeline().await;
        let (tx, mut rx) = tokio::sync::broadcast::channel(100);

        let progress = ProgressTracker::new("job-1".to_string(), "doc-1".to_string())
            .with_event_channel(tx);

        let content = "Alice Chen works at TechCorp.";

        // Process in background
        let pipeline_clone = pipeline.clone();
        let handle = tokio::spawn(async move {
            pipeline_clone.process_with_progress("doc-1", content, &progress).await
        });

        // Collect events
        let mut events = Vec::new();
        while let Ok(event) = rx.recv().await {
            events.push(event);
            if matches!(event, ProgressEvent::Completed { .. }) {
                break;
            }
        }

        let result = handle.await.unwrap().unwrap();

        assert!(!events.is_empty());
        assert!(events.iter().any(|e| matches!(e, ProgressEvent::Started { .. })));
        assert!(events.iter().any(|e| matches!(e, ProgressEvent::Completed { .. })));
    }

    #[tokio::test]
    async fn test_parallel_chunk_processing() {
        let config = PipelineConfig {
            enable_entity_extraction: true,
            max_concurrent_extractions: 4,
            ..Default::default()
        };

        let llm = Arc::new(MockLLMProvider::new_with_delay(50)); // 50ms per call
        let storage = Arc::new(MemoryStorage::new());
        let pipeline = Pipeline::new(config, llm, storage);

        // Content that will create multiple chunks
        let content = (0..20)
            .map(|i| format!("Person {} works at Company {}.", i, i))
            .collect::<Vec<_>>()
            .join("\n");

        let start = std::time::Instant::now();
        let result = pipeline.process("doc-1", &content).await.unwrap();
        let elapsed = start.elapsed();

        // With 4 concurrent extractions and 50ms delay:
        // Sequential: 20 * 50ms = 1000ms
        // Parallel (4): ~250-350ms
        assert!(elapsed.as_millis() < 600);
        assert!(result.stats.chunk_count > 1);
    }
}
```

### 3.2 Storage Integration Tests

**File:** `edgequake-storage/tests/storage_integration.rs`

```rust
use edgequake_storage::{GraphStorage, LineageStorage, LLMCacheStorage};

mod lineage_tests {
    use super::*;

    #[tokio::test]
    async fn test_lineage_storage_chain() {
        let storage = create_test_storage().await;

        // Create document
        let doc_lineage = DocumentLineage {
            document_id: "doc-1".to_string(),
            filename: "test.txt".to_string(),
            ..Default::default()
        };
        storage.store_document_lineage(doc_lineage).await.unwrap();

        // Create chunks
        let chunk_lineage = ChunkLineage {
            chunk_id: "chunk-1".to_string(),
            document_id: "doc-1".to_string(),
            start_line: 1,
            end_line: 10,
            ..Default::default()
        };
        storage.store_chunk_lineage(chunk_lineage).await.unwrap();

        // Create entity
        let entity_lineage = EntityLineage {
            entity_id: "entity-1".to_string(),
            chunk_id: "chunk-1".to_string(),
            ..Default::default()
        };
        storage.store_entity_lineage(entity_lineage).await.unwrap();

        // Query lineage chain
        let chain = storage.get_entity_lineage_chain("entity-1").await.unwrap();

        assert_eq!(chain.entity_id, "entity-1");
        assert_eq!(chain.chunk_id, "chunk-1");
        assert_eq!(chain.document_id, "doc-1");
        assert_eq!(chain.start_line, 1);
        assert_eq!(chain.end_line, 10);
    }

    #[tokio::test]
    async fn test_cascade_delete() {
        let storage = create_test_storage().await;

        // Setup lineage chain
        setup_lineage_chain(&storage, "doc-1").await;

        // Delete document
        let impact = storage.calculate_deletion_impact("doc-1").await.unwrap();

        assert!(impact.chunks_affected > 0);
        assert!(impact.entities_affected > 0);
        assert!(impact.relationships_affected > 0);

        // Perform deletion
        storage.cascade_delete_document("doc-1").await.unwrap();

        // Verify everything is deleted
        let doc = storage.get_document_lineage("doc-1").await.unwrap();
        assert!(doc.is_none());

        let entities = storage.get_entities_by_document("doc-1").await.unwrap();
        assert!(entities.is_empty());
    }
}

mod cache_storage_tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_persistence() {
        let storage = create_test_storage().await;

        let entry = CacheEntry {
            id: "entry-1".to_string(),
            cache_type: CacheType::Extract,
            chunk_id: Some("chunk-1".to_string()),
            prompt_hash: "hash-abc".to_string(),
            response: "Response".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            model: "gpt-4o-mini".to_string(),
            created_at: Utc::now(),
        };

        storage.store_cache_entry(entry.clone()).await.unwrap();

        let retrieved = storage.get_cache_entry("hash-abc").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().response, "Response");
    }
}
```

---

## 4. End-to-End Testing

### 4.1 Full Ingestion E2E

**File:** `edgequake-core/tests/e2e/ingestion_e2e.rs`

```rust
use edgequake_core::EdgeQuake;

mod e2e_tests {
    use super::*;

    #[tokio::test]
    #[cfg(feature = "integration")]
    async fn test_full_ingestion_flow() {
        let config = EdgeQuakeConfig::from_env();
        let eq = EdgeQuake::new(config).await.unwrap();

        let content = std::fs::read_to_string("tests/fixtures/sample_document.txt")
            .unwrap();

        let result = eq.ingest_document("test-doc", &content).await.unwrap();

        // Verify processing completed
        assert!(result.stats.chunk_count > 0);
        assert!(result.stats.entity_count > 0);

        // Verify entities stored
        let entities = eq.query_entities("alice").await.unwrap();
        assert!(!entities.is_empty());

        // Verify lineage
        let lineage = eq.get_document_lineage("test-doc").await.unwrap();
        assert!(!lineage.chunks.is_empty());
    }

    #[tokio::test]
    #[cfg(feature = "integration")]
    async fn test_ingestion_with_real_llm() {
        // Only run if OPENAI_API_KEY is set
        if std::env::var("OPENAI_API_KEY").is_err() {
            return;
        }

        let config = EdgeQuakeConfig::from_env();
        let eq = EdgeQuake::new(config).await.unwrap();

        let content = "Dr. Sarah Chen is a renowned physicist at MIT. \
                       She collaborates with Professor James Wilson on quantum computing research.";

        let result = eq.ingest_document("real-llm-test", content).await.unwrap();

        // Real LLM should extract more entities
        assert!(result.stats.entity_count >= 4); // Sarah Chen, MIT, James Wilson, quantum computing

        // Verify costs tracked
        assert!(result.cost.total_usd > 0.0);
    }

    #[tokio::test]
    #[cfg(feature = "integration")]
    async fn test_document_suppression() {
        let eq = create_test_edgequake().await;

        // Ingest document
        let content = "Alice works at TechCorp.";
        eq.ingest_document("suppress-test", content).await.unwrap();

        // Verify entities exist
        let entities_before = eq.query_entities("alice").await.unwrap();
        assert!(!entities_before.is_empty());

        // Suppress document
        let impact = eq.suppress_document("suppress-test").await.unwrap();
        assert!(impact.entities_removed > 0);

        // Verify entities removed
        let entities_after = eq.query_entities("alice").await.unwrap();
        assert!(entities_after.is_empty());
    }
}
```

### 4.2 API E2E Tests

**File:** `edgequake-api/tests/e2e/api_e2e.rs`

```rust
use axum_test::TestServer;
use edgequake_api::create_app;

mod api_e2e_tests {
    use super::*;

    async fn create_test_server() -> TestServer {
        let app = create_app(TestConfig::default()).await;
        TestServer::new(app).unwrap()
    }

    #[tokio::test]
    async fn test_document_upload_and_track() {
        let server = create_test_server().await;

        // Upload document
        let response = server
            .post("/api/v1/documents")
            .json(&json!({
                "name": "test.txt",
                "content": "Alice works at TechCorp.",
                "metadata": {}
            }))
            .await;

        assert_eq!(response.status_code(), 202);

        let body: IngestionResponse = response.json();
        let track_id = body.track_id;

        // Poll for completion
        loop {
            let status = server
                .get(&format!("/api/v1/documents/track/{}", track_id))
                .await;

            let progress: IngestionProgress = status.json();

            if progress.status == "completed" {
                assert!(progress.result.is_some());
                break;
            }

            if progress.status == "failed" {
                panic!("Ingestion failed: {:?}", progress.errors);
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    #[tokio::test]
    async fn test_lineage_endpoints() {
        let server = create_test_server().await;

        // Setup: ingest a document first
        let upload = server
            .post("/api/v1/documents")
            .json(&json!({
                "name": "test.txt",
                "content": "Alice works at TechCorp."
            }))
            .await;
        let body: IngestionResponse = upload.json();
        wait_for_completion(&server, &body.track_id).await;

        // Get document lineage
        let lineage = server
            .get("/api/v1/documents/test.txt/lineage")
            .await;

        assert_eq!(lineage.status_code(), 200);
        let body: DocumentLineageResponse = lineage.json();
        assert!(!body.chunks.is_empty());

        // Get entity lineage
        let entity_id = body.chunks[0].entities[0].clone();
        let entity_lineage = server
            .get(&format!("/api/v1/entities/{}/lineage", entity_id))
            .await;

        assert_eq!(entity_lineage.status_code(), 200);
    }

    #[tokio::test]
    async fn test_cost_endpoints() {
        let server = create_test_server().await;

        // Get cost summary
        let response = server
            .get("/api/v1/costs/summary")
            .await;

        assert_eq!(response.status_code(), 200);

        // Get cost for date range
        let response = server
            .get("/api/v1/costs/breakdown?start_date=2024-01-01&end_date=2024-12-31")
            .await;

        assert_eq!(response.status_code(), 200);
    }
}
```

---

## 5. Performance Testing

### 5.1 Benchmarks

**File:** `edgequake/benches/pipeline_bench.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use edgequake_pipeline::{Chunker, ChunkerConfig};

fn chunker_benchmark(c: &mut Criterion) {
    let config = ChunkerConfig::default();
    let chunker = Chunker::new(config);

    // Generate test data
    let small_doc = "Hello world.".repeat(100);
    let medium_doc = "Hello world.".repeat(1000);
    let large_doc = "Hello world.".repeat(10000);

    c.bench_function("chunk_small_doc", |b| {
        b.iter(|| chunker.chunk_sync(black_box(&small_doc), "doc"))
    });

    c.bench_function("chunk_medium_doc", |b| {
        b.iter(|| chunker.chunk_sync(black_box(&medium_doc), "doc"))
    });

    c.bench_function("chunk_large_doc", |b| {
        b.iter(|| chunker.chunk_sync(black_box(&large_doc), "doc"))
    });
}

fn line_number_calculation_benchmark(c: &mut Criterion) {
    let text_with_lines: String = (0..1000)
        .map(|i| format!("Line {}", i))
        .collect::<Vec<_>>()
        .join("\n");

    c.bench_function("calculate_line_numbers", |b| {
        b.iter(|| {
            calculate_line_numbers(
                black_box(&text_with_lines),
                black_box(500),
                black_box(1000),
            )
        })
    });
}

fn entity_normalization_benchmark(c: &mut Criterion) {
    let names = vec![
        "Alice Chen",
        "TechCorp Inc.",
        "New York City",
        "  John   Doe  ",
        "O'Brien & Associates",
    ];

    c.bench_function("normalize_entity_names", |b| {
        b.iter(|| {
            for name in &names {
                black_box(normalize_entity_name(name));
            }
        })
    });
}

criterion_group!(
    benches,
    chunker_benchmark,
    line_number_calculation_benchmark,
    entity_normalization_benchmark
);
criterion_main!(benches);
```

### 5.2 Load Tests

**File:** `edgequake/tests/load_test.rs`

```rust
#[tokio::test]
#[cfg(feature = "load-test")]
async fn test_concurrent_ingestion() {
    let eq = create_test_edgequake().await;

    // Simulate 100 concurrent document uploads
    let handles: Vec<_> = (0..100)
        .map(|i| {
            let eq = eq.clone();
            tokio::spawn(async move {
                let content = format!("Document {} with some content.", i);
                eq.ingest_document(&format!("doc-{}", i), &content).await
            })
        })
        .collect();

    let results: Vec<_> = futures::future::join_all(handles).await;

    let successes = results.iter().filter(|r| r.is_ok()).count();
    assert!(successes >= 95); // Allow some failures under load
}

#[tokio::test]
#[cfg(feature = "load-test")]
async fn test_memory_usage() {
    use memory_stats::memory_stats;

    let eq = create_test_edgequake().await;

    let initial_memory = memory_stats().unwrap().physical_mem;

    // Ingest 1000 documents
    for i in 0..1000 {
        let content = format!("Document {} with some content.", i);
        eq.ingest_document(&format!("doc-{}", i), &content).await.unwrap();
    }

    let final_memory = memory_stats().unwrap().physical_mem;
    let memory_increase = final_memory - initial_memory;

    // Should not exceed 1GB for 1000 small documents
    assert!(memory_increase < 1024 * 1024 * 1024);
}
```

---

## 6. Test Data Management

### 6.1 Fixtures

```
tests/
├── fixtures/
│   ├── sample_document.txt        # Standard test document
│   ├── multi_entity_doc.txt       # Document with many entities
│   ├── edge_cases/
│   │   ├── empty.txt              # Empty document
│   │   ├── single_line.txt        # Single line document
│   │   ├── unicode.txt            # Unicode characters
│   │   ├── very_long_lines.txt    # Lines > 10KB
│   │   └── special_chars.txt      # Special characters
│   └── expected_outputs/
│       ├── sample_entities.json   # Expected entities for sample_document
│       └── sample_lineage.json    # Expected lineage structure
```

### 6.2 Test Data Generation

```rust
/// Generate test documents with known entity counts
pub fn generate_test_document(entity_count: usize) -> String {
    let names = ["Alice", "Bob", "Carol", "David", "Eve"];
    let companies = ["TechCorp", "DataCo", "CloudInc", "AILabs", "CodeBase"];

    (0..entity_count)
        .map(|i| {
            let name = names[i % names.len()];
            let company = companies[(i / names.len()) % companies.len()];
            format!("{} works at {}.", name, company)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```

---

## 7. Continuous Integration

### 7.1 CI Pipeline

```yaml
# .github/workflows/test.yml
name: Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run unit tests
        run: cargo test --lib --no-default-features

  integration-tests:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: apache/age:v1.5.0-pg15
        ports:
          - 5432:5432
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run integration tests
        run: cargo test --features integration
        env:
          DATABASE_URL: postgres://postgres:postgres@localhost:5432/test

  e2e-tests:
    runs-on: ubuntu-latest
    needs: [unit-tests, integration-tests]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run E2E tests
        run: cargo test --features e2e
        env:
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}

  benchmarks:
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run benchmarks
        run: cargo bench --no-run
```

### 7.2 Test Commands

```bash
# Run all unit tests (fast, no external deps)
cargo test --lib

# Run with mock LLM provider (default)
cargo test

# Run with real LLM provider
OPENAI_API_KEY=sk-... cargo test --features integration

# Run specific test
cargo test --package edgequake-pipeline --test chunker_tests

# Run with coverage
cargo llvm-cov --all-features

# Run benchmarks
cargo bench
```

---

## 8. Quality Gates

### 8.1 Coverage Requirements

| Module             | Minimum Coverage |
| ------------------ | ---------------- |
| edgequake-pipeline | 85%              |
| edgequake-core     | 80%              |
| edgequake-storage  | 75%              |
| edgequake-api      | 70%              |
| edgequake-llm      | 70%              |

### 8.2 Performance Requirements

| Metric                  | Threshold        |
| ----------------------- | ---------------- |
| Unit test suite         | < 30 seconds     |
| Integration tests       | < 5 minutes      |
| Chunking 1MB document   | < 100ms          |
| Entity normalization    | < 1μs per entity |
| Line number calculation | < 10μs per chunk |

### 8.3 Acceptance Checklist

- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Coverage meets thresholds
- [ ] No new clippy warnings
- [ ] Benchmarks don't regress > 10%
- [ ] Documentation updated
- [ ] CHANGELOG updated

---
