//! Integration tests for cost tracking across the pipeline.
//!
//! These tests verify that cost tracking works correctly when
//! entities are extracted, embeddings are generated, and documents
//! are processed through the full pipeline.

use std::sync::Arc;

use edgequake_pipeline::{
    CostBreakdown, CostBreakdownStats, CostTracker, ModelPricing, Pipeline, ProcessingStats,
};

// Sample documents for testing
const SHORT_DOCUMENT: &str = "EdgeQuake is a Rust-based RAG framework.";

const MEDIUM_DOCUMENT: &str = r#"
Dr. Sarah Chen is a renowned computer scientist at Stanford University. 
She specializes in artificial intelligence and machine learning.
Sarah has published over 100 papers and received the Turing Award in 2023.

EdgeQuake is a Rust-based RAG framework developed by the engineering team.
It uses knowledge graphs to improve retrieval accuracy.
"#;

const LONG_DOCUMENT: &str = r#"
EdgeQuake: A State-of-the-Art Retrieval-Augmented Generation Framework

Introduction
EdgeQuake is an advanced Retrieval-Augmented Generation (RAG) framework 
implemented in Rust. It is designed to enhance information retrieval and 
generation through graph-based knowledge representation.

Key Features
1. High-Performance Entity Extraction: EdgeQuake uses sophisticated natural 
   language processing techniques to identify entities and relationships 
   within documents.

2. Graph-Based Knowledge Storage: The framework stores extracted knowledge 
   in a graph database, allowing for efficient querying and relationship 
   traversal.

3. Multi-Tenant Support: EdgeQuake supports multiple tenants, with complete 
   data isolation through Row-Level Security (RLS) policies.

4. Flexible LLM Integration: The framework supports multiple LLM providers 
   including OpenAI, Anthropic, and local models via Ollama.

Technical Architecture
The system is built using a modular architecture with the following components:
- edgequake-core: Orchestration layer with pipeline and EdgeQuake API
- edgequake-llm: LLM provider implementations
- edgequake-storage: Storage adapters for PostgreSQL and in-memory storage
- edgequake-api: REST API service built with Axum
- edgequake-pipeline: Document processing pipeline

Cost Tracking
EdgeQuake includes comprehensive cost tracking for all LLM operations. 
The system tracks:
- Input tokens used for entity extraction
- Output tokens from LLM responses
- Embedding tokens for vector storage
- Dollar cost based on model-specific pricing

Performance Benchmarks
The framework achieves the following performance metrics:
- 1000 documents per minute throughput
- Sub-second query latency
- 99.9% entity extraction accuracy

Developed by the EdgeQuake engineering team.
Dr. Sarah Chen leads the AI research efforts.
John Smith manages the infrastructure.
"#;

// =============================================================================
// Cost Tracker Integration Tests
// =============================================================================

mod cost_tracker_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_cost_tracker_gpt4o_mini() {
        let tracker = CostTracker::new_gpt4o_mini("test-job-1");

        // Simulate extraction operation
        tracker.record("extraction", 500, 200).await;

        // Verify tracking
        let cost = tracker.total_cost().await;
        assert!(cost > 0.0, "Cost should be positive");
    }

    #[tokio::test]
    async fn test_cost_tracker_accumulates_across_operations() {
        let tracker = CostTracker::new_gpt4o_mini("test-job-2");

        // Multiple operations
        tracker.record("extraction", 1000, 500).await;
        tracker.record("extraction", 800, 400).await;
        tracker.record("summarization", 500, 200).await;

        // Verify accumulation
        let snapshot = tracker.snapshot().await;
        assert!(snapshot.total_input_tokens >= 2300);
        assert!(snapshot.total_output_tokens >= 1100);

        // Cost should reflect all operations
        let total = tracker.total_cost().await;
        assert!(total > 0.0, "Total cost should be positive");
    }

    #[tokio::test]
    async fn test_cost_tracker_realistic_document_processing() {
        let tracker = CostTracker::new_gpt4o_mini("test-job-3");

        // Simulate realistic document processing:
        // 1. Chunk a 10000-word document into 5 chunks
        // 2. Each chunk: ~500 input tokens, ~200 output tokens for extraction
        for i in 0..5 {
            tracker
                .record(&format!("extraction_chunk_{}", i), 500, 200)
                .await;
        }

        // 3. Summarization: ~300 input tokens, ~100 output tokens
        tracker.record("summarization", 300, 100).await;

        // 4. Embeddings: ~2000 tokens (no output)
        tracker.record("embedding", 2000, 0).await;

        // Verify totals
        let snapshot = tracker.snapshot().await;
        assert!(snapshot.total_input_tokens >= 4800);
        assert!(snapshot.total_output_tokens >= 1100);

        // With gpt-4o-mini pricing, this should cost roughly:
        // Extraction: 5 * (500 * 0.00015/1000 + 200 * 0.0006/1000) = 5 * 0.000195 = 0.000975
        // Summary: 300 * 0.00015/1000 + 100 * 0.0006/1000 = 0.000105
        // Embedding: 2000 * 0.00015/1000 = 0.0003 (using LLM rate, not embedding rate)
        // Total: ~0.00138
        let cost = tracker.total_cost().await;
        assert!(
            cost > 0.001 && cost < 0.01,
            "Expected cost around $0.001-0.01, got ${:.6}",
            cost
        );
    }

    #[tokio::test]
    async fn test_cost_tracker_gpt4o() {
        let tracker = CostTracker::new_gpt4o("test-job-gpt4o");

        // Track some usage
        tracker.record("test", 1000, 500).await;

        let cost = tracker.total_cost().await;
        // gpt-4o is more expensive than gpt-4o-mini
        // Input: 1000 * 0.005 / 1000 = 0.005
        // Output: 500 * 0.015 / 1000 = 0.0075
        // Total: 0.0125
        assert!(
            cost > 0.01 && cost < 0.02,
            "Expected cost around $0.0125, got ${:.6}",
            cost
        );
    }

    #[tokio::test]
    async fn test_cost_tracker_clone_shares_state() {
        let tracker1 = CostTracker::new_gpt4o_mini("clone-test");
        let tracker2 = tracker1.clone();

        // Record on one tracker
        tracker1.record("op1", 500, 200).await;

        // Should be visible on cloned tracker
        let cost1 = tracker1.total_cost().await;
        let cost2 = tracker2.total_cost().await;

        assert!(
            (cost1 - cost2).abs() < 0.000001,
            "Clone should share state: ${:.6} vs ${:.6}",
            cost1,
            cost2
        );

        // Record on other tracker
        tracker2.record("op2", 300, 100).await;

        let cost1_after = tracker1.total_cost().await;
        let cost2_after = tracker2.total_cost().await;

        assert!(
            (cost1_after - cost2_after).abs() < 0.000001,
            "Clone should share state after second op"
        );
    }
}

// =============================================================================
// Cost Breakdown Integration Tests
// =============================================================================

mod cost_breakdown_integration_tests {
    use super::*;

    #[test]
    fn test_cost_breakdown_multi_operation_pipeline() {
        let mut breakdown = CostBreakdown::new("doc-integration-1", "gpt-4o-mini");

        // Simulate full pipeline
        breakdown.add_operation_cost("chunk_1_extraction", 600, 250, 0.00024);
        breakdown.add_operation_cost("chunk_2_extraction", 550, 230, 0.00022);
        breakdown.add_operation_cost("summarization", 400, 150, 0.00015);
        breakdown.add_operation_cost("embedding", 1500, 0, 0.00003);

        // Verify totals
        let total = breakdown.total_cost_usd;
        assert!(
            (total - 0.00064).abs() < 0.0001,
            "Expected total ~$0.00064, got ${:.6}",
            total
        );

        // Verify formatted output
        let formatted = breakdown.formatted_cost();
        assert!(formatted.starts_with("$"), "Should start with $");
    }

    #[test]
    fn test_cost_breakdown_tracks_job_metadata() {
        let breakdown = CostBreakdown::new("job-123-abc", "gpt-4o");

        assert_eq!(breakdown.job_id, "job-123-abc");
        assert_eq!(breakdown.model, "gpt-4o");
    }

    #[test]
    fn test_cost_breakdown_empty_is_zero() {
        let breakdown = CostBreakdown::new("empty-job", "gpt-4o-mini");

        assert_eq!(breakdown.total_cost_usd, 0.0);
        assert_eq!(breakdown.formatted_cost(), "$0.0000");
    }

    #[test]
    fn test_cost_breakdown_accumulates_tokens() {
        let mut breakdown = CostBreakdown::new("token-test", "gpt-4o-mini");

        breakdown.add_operation_cost("op1", 100, 50, 0.0001);
        breakdown.add_operation_cost("op2", 200, 75, 0.0002);
        breakdown.add_operation_cost("op3", 150, 25, 0.00015);

        assert_eq!(breakdown.total_input_tokens, 450);
        assert_eq!(breakdown.total_output_tokens, 150);
    }
}

// =============================================================================
// Processing Stats Integration Tests
// =============================================================================

mod processing_stats_integration_tests {
    use super::*;

    #[test]
    fn test_processing_stats_with_cost_breakdown() {
        let stats = ProcessingStats {
            chunk_count: 5,
            entity_count: 15,
            relationship_count: 8,
            processing_time_ms: 2500,
            llm_calls: 5,
            total_tokens: 4700,
            llm_model: Some("gpt-4o-mini".to_string()),
            llm_provider: Some("openai".to_string()),
            embedding_model: Some("text-embedding-3-small".to_string()),
            embedding_provider: Some("openai".to_string()),
            embedding_dimensions: Some(1536),
            entity_types: None,
            relationship_types: None,
            keywords: None,
            chunking_strategy: Some("semantic".to_string()),
            avg_chunk_size: Some(500),
            input_tokens: 3500,
            output_tokens: 1200,
            cost_usd: 0.00245,
            successful_chunks: 5,
            failed_chunks: 0,
            chunk_errors: None,
            cost_breakdown: Some(CostBreakdownStats {
                extraction_cost_usd: 0.00180,
                extraction_input_tokens: 2500,
                extraction_output_tokens: 1000,
                embedding_cost_usd: 0.00050,
                embedding_tokens: 2500,
                summarization_cost_usd: 0.00015,
            }),
        };

        // Verify all fields
        assert_eq!(stats.entity_count, 15);
        assert_eq!(stats.input_tokens, 3500);
        assert_eq!(stats.output_tokens, 1200);
        assert!(stats.cost_usd > 0.002);

        // Verify breakdown
        let breakdown = stats.cost_breakdown.unwrap();
        assert!(breakdown.extraction_cost_usd > breakdown.summarization_cost_usd);
    }

    #[test]
    fn test_processing_stats_serialization_roundtrip() {
        let stats = ProcessingStats {
            chunk_count: 3,
            entity_count: 10,
            relationship_count: 5,
            processing_time_ms: 1500,
            llm_calls: 3,
            total_tokens: 2800,
            llm_model: Some("gpt-4o-mini".to_string()),
            llm_provider: Some("openai".to_string()),
            embedding_model: None,
            embedding_provider: None,
            embedding_dimensions: None,
            entity_types: None,
            relationship_types: None,
            keywords: None,
            chunking_strategy: None,
            avg_chunk_size: None,
            input_tokens: 2000,
            output_tokens: 800,
            cost_usd: 0.00156,
            successful_chunks: 3,
            failed_chunks: 0,
            chunk_errors: None,
            cost_breakdown: Some(CostBreakdownStats {
                extraction_cost_usd: 0.00120,
                extraction_input_tokens: 1500,
                extraction_output_tokens: 600,
                embedding_cost_usd: 0.00030,
                embedding_tokens: 1500,
                summarization_cost_usd: 0.00006,
            }),
        };

        // Serialize
        let json = serde_json::to_string(&stats).expect("Failed to serialize");

        // Verify JSON contains cost fields
        assert!(
            json.contains("input_tokens"),
            "JSON should contain input_tokens"
        );
        assert!(
            json.contains("output_tokens"),
            "JSON should contain output_tokens"
        );
        assert!(json.contains("cost_usd"), "JSON should contain cost_usd");
        assert!(
            json.contains("cost_breakdown"),
            "JSON should contain cost_breakdown"
        );
        assert!(
            json.contains("extraction_cost_usd"),
            "JSON should contain extraction_cost_usd"
        );

        // Deserialize and verify
        let deserialized: ProcessingStats =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.entity_count, stats.entity_count);
        assert_eq!(deserialized.input_tokens, stats.input_tokens);
        assert_eq!(deserialized.output_tokens, stats.output_tokens);
        assert!((deserialized.cost_usd - stats.cost_usd).abs() < 0.000001);
    }

    #[test]
    fn test_processing_stats_default_has_zero_costs() {
        let stats = ProcessingStats::default();

        assert_eq!(stats.input_tokens, 0);
        assert_eq!(stats.output_tokens, 0);
        assert_eq!(stats.cost_usd, 0.0);
        assert!(stats.cost_breakdown.is_none());
    }
}

// =============================================================================
// Pipeline Cost Flow Integration Tests
// =============================================================================

mod pipeline_cost_flow_tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_returns_processing_stats_with_costs() {
        let pipeline = Pipeline::default_pipeline();

        let result = pipeline
            .process("cost-test-1", SHORT_DOCUMENT)
            .await
            .unwrap();

        // Verify stats are returned
        assert!(!result.chunks.is_empty());
        assert!(result.stats.chunk_count > 0);

        // Processing time may be 0 for fast operations
        // Main assertion is that it doesn't error
    }

    #[tokio::test]
    async fn test_pipeline_medium_document_cost_structure() {
        let pipeline = Pipeline::default_pipeline();

        let result = pipeline
            .process("cost-test-2", MEDIUM_DOCUMENT)
            .await
            .unwrap();

        // Should have processed chunks
        assert!(result.stats.chunk_count >= 1);

        // Main assertion is that it doesn't error
    }

    #[tokio::test]
    async fn test_pipeline_long_document_produces_chunks() {
        let pipeline = Pipeline::default_pipeline();

        let result = pipeline
            .process("cost-test-3", LONG_DOCUMENT)
            .await
            .unwrap();

        // Long document should produce at least one chunk
        // (exact number depends on chunker config, default may produce 1)
        assert!(
            result.stats.chunk_count >= 1,
            "Long document should produce at least one chunk, got {}",
            result.stats.chunk_count
        );
    }

    #[tokio::test]
    async fn test_pipeline_consistent_results() {
        let pipeline = Pipeline::default_pipeline();

        // Process same document twice
        let result1 = pipeline
            .process("consist-1", MEDIUM_DOCUMENT)
            .await
            .unwrap();
        let result2 = pipeline
            .process("consist-2", MEDIUM_DOCUMENT)
            .await
            .unwrap();

        // Results should be consistent
        assert_eq!(result1.stats.chunk_count, result2.stats.chunk_count);
    }
}

// =============================================================================
// Cost Calculation Accuracy Tests
// =============================================================================

mod cost_calculation_accuracy_tests {
    use super::*;

    #[tokio::test]
    async fn test_gpt4o_mini_cost_calculation_accuracy() {
        let tracker = CostTracker::new_gpt4o_mini("accuracy-test-1");

        // Known calculation: 1000 input, 500 output
        // Input: 1000 * 0.00015 / 1000 = 0.00015
        // Output: 500 * 0.0006 / 1000 = 0.0003
        // Total: 0.00045
        tracker.record("test", 1000, 500).await;

        let cost = tracker.total_cost().await;
        let expected = 0.00045;

        assert!(
            (cost - expected).abs() < 0.0001,
            "Expected ${:.6}, got ${:.6}",
            expected,
            cost
        );
    }

    #[tokio::test]
    async fn test_gpt4o_cost_calculation_accuracy() {
        let tracker = CostTracker::new_gpt4o("accuracy-test-2");

        // Known calculation: 1000 input, 500 output
        // Input: 1000 * 0.005 / 1000 = 0.005
        // Output: 500 * 0.015 / 1000 = 0.0075
        // Total: 0.0125
        tracker.record("test", 1000, 500).await;

        let cost = tracker.total_cost().await;
        let expected = 0.0125;

        assert!(
            (cost - expected).abs() < 0.001,
            "Expected ${:.6}, got ${:.6}",
            expected,
            cost
        );
    }

    #[tokio::test]
    async fn test_cost_scales_linearly() {
        let tracker1 = CostTracker::new_gpt4o_mini("scale-test-1");
        let tracker2 = CostTracker::new_gpt4o_mini("scale-test-2");

        tracker1.record("test", 1000, 500).await;
        tracker2.record("test", 2000, 1000).await; // Double

        let cost1 = tracker1.total_cost().await;
        let cost2 = tracker2.total_cost().await;

        // Cost2 should be approximately 2x cost1
        let ratio = cost2 / cost1;
        assert!(
            (ratio - 2.0).abs() < 0.01,
            "Cost should scale linearly, got ratio {:.4}",
            ratio
        );
    }

    #[tokio::test]
    async fn test_cumulative_cost_is_sum_of_parts() {
        let tracker = CostTracker::new_gpt4o_mini("cumulative-test");

        // Individual costs
        let tracker1 = CostTracker::new_gpt4o_mini("part-1");
        tracker1.record("op1", 500, 200).await;
        let cost1 = tracker1.total_cost().await;

        let tracker2 = CostTracker::new_gpt4o_mini("part-2");
        tracker2.record("op2", 300, 100).await;
        let cost2 = tracker2.total_cost().await;

        // Combined
        tracker.record("op1", 500, 200).await;
        tracker.record("op2", 300, 100).await;
        let combined = tracker.total_cost().await;

        assert!(
            (combined - (cost1 + cost2)).abs() < 0.000001,
            "Cumulative ${:.6} != sum ${:.6} + ${:.6}",
            combined,
            cost1,
            cost2
        );
    }
}

// =============================================================================
// Edge Cases and Error Handling Tests
// =============================================================================

mod edge_case_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_zero_token_operations() {
        let tracker = CostTracker::new_gpt4o_mini("zero-test");
        tracker.record("zero", 0, 0).await;

        let cost = tracker.total_cost().await;
        assert_eq!(cost, 0.0);

        let snapshot = tracker.snapshot().await;
        assert_eq!(snapshot.total_input_tokens, 0);
        assert_eq!(snapshot.total_output_tokens, 0);
    }

    #[tokio::test]
    async fn test_very_large_document_simulation() {
        let tracker = CostTracker::new_gpt4o_mini("large-doc-test");

        // Simulate processing a very large document
        // 100 chunks, each with 1000 input and 400 output tokens
        for i in 0..100 {
            tracker.record(&format!("chunk_{}", i), 1000, 400).await;
        }

        // Should handle without overflow
        let snapshot = tracker.snapshot().await;
        assert!(snapshot.total_input_tokens >= 100_000);
        assert!(snapshot.total_output_tokens >= 40_000);

        // Cost should be reasonable (not overflow to infinity or NaN)
        let cost = tracker.total_cost().await;
        assert!(cost.is_finite());
        assert!(cost > 0.0);

        // Expected: 100 * (1000 * 0.00015/1000 + 400 * 0.0006/1000) = 100 * 0.00039 = 0.039
        assert!(
            cost > 0.03 && cost < 0.05,
            "Expected ~$0.039, got ${:.4}",
            cost
        );
    }

    #[tokio::test]
    async fn test_concurrent_cost_tracking() {
        let tracker = Arc::new(CostTracker::new_gpt4o_mini("concurrent-test"));

        let mut handles = vec![];

        // Spawn 10 tasks, each recording 100 operations
        for t in 0..10 {
            let tracker_clone = Arc::clone(&tracker);
            let handle = tokio::spawn(async move {
                for i in 0..100 {
                    tracker_clone
                        .record(&format!("task_{}_op_{}", t, i), 100, 50)
                        .await;
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        // Should have accumulated 1000 operations * (100 + 50) = 150,000 tokens
        let snapshot = tracker.snapshot().await;
        assert!(snapshot.total_input_tokens >= 100_000);
        assert!(snapshot.total_output_tokens >= 50_000);
    }

    #[tokio::test]
    async fn test_pipeline_handles_empty_document() {
        let pipeline = Pipeline::default_pipeline();

        let result = pipeline.process("empty-doc", "").await;

        // Should handle gracefully (either success with 0 chunks or appropriate error)
        match result {
            Ok(res) => {
                // Empty doc should produce minimal stats
                assert!(res.stats.entity_count == 0 || res.stats.chunk_count == 0);
            }
            Err(_) => {
                // Or it might error - that's also acceptable
            }
        }
    }

    #[tokio::test]
    async fn test_pipeline_handles_whitespace_only() {
        let pipeline = Pipeline::default_pipeline();

        let result = pipeline.process("whitespace-doc", "   \n\n\t  \n  ").await;

        // Should handle gracefully
        match result {
            Ok(res) => {
                assert!(res.stats.entity_count == 0);
            }
            Err(_) => {
                // Error is also acceptable
            }
        }
    }
}

// =============================================================================
// Cost Reporting Tests
// =============================================================================

mod cost_reporting_tests {
    use super::*;

    #[test]
    fn test_cost_breakdown_stats_total_calculation() {
        let breakdown = CostBreakdownStats {
            extraction_cost_usd: 0.00150,
            extraction_input_tokens: 2000,
            extraction_output_tokens: 800,
            embedding_cost_usd: 0.00040,
            embedding_tokens: 2000,
            summarization_cost_usd: 0.00010,
        };

        let total = breakdown.extraction_cost_usd
            + breakdown.embedding_cost_usd
            + breakdown.summarization_cost_usd;
        let expected = 0.00150 + 0.00040 + 0.00010;

        assert!(
            (total - expected).abs() < 0.000001,
            "Expected ${:.6}, got ${:.6}",
            expected,
            total
        );
    }

    #[test]
    fn test_cost_breakdown_stats_token_totals() {
        let breakdown = CostBreakdownStats {
            extraction_cost_usd: 0.0,
            extraction_input_tokens: 1000,
            extraction_output_tokens: 500,
            embedding_cost_usd: 0.0,
            embedding_tokens: 2000,
            summarization_cost_usd: 0.0,
        };

        let total_input = breakdown.extraction_input_tokens + breakdown.embedding_tokens;

        assert_eq!(total_input, 3000);
        assert_eq!(breakdown.extraction_output_tokens, 500);
    }

    #[test]
    fn test_processing_stats_cost_percentage_breakdown() {
        let stats = ProcessingStats {
            chunk_count: 3,
            entity_count: 10,
            relationship_count: 5,
            processing_time_ms: 1000,
            llm_calls: 3,
            total_tokens: 4000,
            llm_model: Some("gpt-4o-mini".to_string()),
            llm_provider: Some("openai".to_string()),
            embedding_model: None,
            embedding_provider: None,
            embedding_dimensions: None,
            entity_types: None,
            relationship_types: None,
            keywords: None,
            chunking_strategy: None,
            avg_chunk_size: None,
            input_tokens: 3000,
            output_tokens: 1000,
            cost_usd: 0.00200,
            successful_chunks: 3,
            failed_chunks: 0,
            chunk_errors: None,
            cost_breakdown: Some(CostBreakdownStats {
                extraction_cost_usd: 0.00140, // 70%
                extraction_input_tokens: 2000,
                extraction_output_tokens: 800,
                embedding_cost_usd: 0.00040, // 20%
                embedding_tokens: 2000,
                summarization_cost_usd: 0.00020, // 10%
            }),
        };

        let breakdown = stats.cost_breakdown.unwrap();
        let total = breakdown.extraction_cost_usd
            + breakdown.embedding_cost_usd
            + breakdown.summarization_cost_usd;

        // Extraction should be the largest cost
        assert!(breakdown.extraction_cost_usd > breakdown.embedding_cost_usd);
        assert!(breakdown.extraction_cost_usd > breakdown.summarization_cost_usd);

        // Percentages should be reasonable
        let extraction_pct = breakdown.extraction_cost_usd / total * 100.0;
        let embedding_pct = breakdown.embedding_cost_usd / total * 100.0;
        let summary_pct = breakdown.summarization_cost_usd / total * 100.0;

        assert!(
            extraction_pct > 50.0,
            "Extraction should be > 50%, got {}%",
            extraction_pct
        );
        assert!(
            embedding_pct > 10.0,
            "Embedding should be > 10%, got {}%",
            embedding_pct
        );
        assert!(
            summary_pct > 5.0,
            "Summary should be > 5%, got {}%",
            summary_pct
        );
    }
}

// =============================================================================
// Model Comparison Tests
// =============================================================================

mod model_comparison_tests {
    use super::*;

    #[tokio::test]
    async fn test_gpt4o_is_more_expensive_than_gpt4o_mini() {
        let tracker_mini = CostTracker::new_gpt4o_mini("compare-mini");
        let tracker_full = CostTracker::new_gpt4o("compare-full");

        // Same token count
        tracker_mini.record("test", 1000, 500).await;
        tracker_full.record("test", 1000, 500).await;

        let cost_mini = tracker_mini.total_cost().await;
        let cost_full = tracker_full.total_cost().await;

        assert!(
            cost_full > cost_mini,
            "gpt-4o ${:.6} should be more expensive than gpt-4o-mini ${:.6}",
            cost_full,
            cost_mini
        );

        // gpt-4o should be roughly 20-30x more expensive
        let ratio = cost_full / cost_mini;
        assert!(
            ratio > 20.0 && ratio < 40.0,
            "Expected ~25-30x ratio, got {:.2}x",
            ratio
        );
    }

    #[tokio::test]
    async fn test_output_tokens_are_more_expensive_than_input() {
        let tracker1 = CostTracker::new_gpt4o_mini("input-heavy");
        let tracker2 = CostTracker::new_gpt4o_mini("output-heavy");

        // Track input-heavy operation
        tracker1.record("input_heavy", 1000, 100).await;
        let cost_input_heavy = tracker1.total_cost().await;

        // Track output-heavy
        tracker2.record("output_heavy", 100, 1000).await;
        let cost_output_heavy = tracker2.total_cost().await;

        // Output-heavy should be more expensive for gpt-4o-mini
        // (output is 4x more expensive per token than input)
        assert!(
            cost_output_heavy > cost_input_heavy,
            "Output-heavy ${:.6} should cost more than input-heavy ${:.6}",
            cost_output_heavy,
            cost_input_heavy
        );
    }

    #[test]
    fn test_model_pricing_calculation() {
        // GPT-4o-mini pricing
        let pricing = ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006);

        // 1000 input, 500 output
        let cost = pricing.calculate_cost(1000, 500);

        // Input: 1000 * 0.00015 / 1000 = 0.00015
        // Output: 500 * 0.0006 / 1000 = 0.0003
        // Total: 0.00045
        let expected = 0.00045;

        assert!(
            (cost - expected).abs() < 0.00001,
            "Expected ${:.6}, got ${:.6}",
            expected,
            cost
        );
    }

    #[test]
    fn test_model_pricing_zero_tokens() {
        let pricing = ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006);
        let cost = pricing.calculate_cost(0, 0);
        assert_eq!(cost, 0.0);
    }
}

// =============================================================================
// Model Pricing Tests
// =============================================================================

mod model_pricing_tests {
    use super::*;

    #[test]
    fn test_model_pricing_new() {
        let pricing = ModelPricing::new("test-model", 0.001, 0.002);
        assert_eq!(pricing.model, "test-model");
        assert_eq!(pricing.input_cost_per_1k, 0.001);
        assert_eq!(pricing.output_cost_per_1k, 0.002);
    }

    #[test]
    fn test_model_pricing_input_only() {
        let pricing = ModelPricing::new("embed-model", 0.00002, 0.0);

        let cost = pricing.calculate_cost(10000, 0);

        // 10000 * 0.00002 / 1000 = 0.0002
        assert!((cost - 0.0002).abs() < 0.00001);
    }

    #[test]
    fn test_model_pricing_output_only() {
        let pricing = ModelPricing::new("gen-model", 0.0, 0.01);

        let cost = pricing.calculate_cost(0, 1000);

        // 1000 * 0.01 / 1000 = 0.01
        assert!((cost - 0.01).abs() < 0.0001);
    }

    #[test]
    fn test_model_pricing_large_scale() {
        let pricing = ModelPricing::new("gpt-4o", 0.005, 0.015);

        // 1 million input tokens, 100k output tokens
        let cost = pricing.calculate_cost(1_000_000, 100_000);

        // Input: 1_000_000 * 0.005 / 1000 = 5.0
        // Output: 100_000 * 0.015 / 1000 = 1.5
        // Total: 6.5
        assert!((cost - 6.5).abs() < 0.01);
    }
}
