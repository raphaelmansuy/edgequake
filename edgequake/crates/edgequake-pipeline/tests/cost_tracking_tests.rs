//! Cost Tracking Unit Tests
//!
//! Comprehensive tests for cost tracking in the ingestion pipeline.
//! Tests cover:
//! - ModelPricing calculations
//! - CostTracker operations
//! - CostBreakdown aggregation
//! - ProcessingStats cost fields
//! - Edge cases and thread safety

use edgequake_pipeline::{
    default_model_pricing, CostBreakdown, CostBreakdownStats, CostTracker, ModelPricing,
    OperationCost, ProcessingStats,
};

// ============================================================================
// ModelPricing Tests
// ============================================================================

mod model_pricing_tests {
    use super::*;

    #[test]
    fn test_model_pricing_gpt4o_mini() {
        let pricing = ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006);

        // 1K input + 500 output
        let cost = pricing.calculate_cost(1000, 500);
        // Input: 1000 * 0.00015 / 1000 = $0.00015
        // Output: 500 * 0.0006 / 1000 = $0.0003
        // Total: $0.00045
        assert!(
            (cost - 0.00045).abs() < 0.000001,
            "Expected $0.00045, got ${}",
            cost
        );
    }

    #[test]
    fn test_model_pricing_gpt4o() {
        let pricing = ModelPricing::new("gpt-4o", 0.005, 0.015);

        // 1K input + 500 output
        let cost = pricing.calculate_cost(1000, 500);
        // Input: 1000 * 0.005 / 1000 = $0.005
        // Output: 500 * 0.015 / 1000 = $0.0075
        // Total: $0.0125
        assert!(
            (cost - 0.0125).abs() < 0.0001,
            "Expected $0.0125, got ${}",
            cost
        );
    }

    #[test]
    fn test_model_pricing_large_scale() {
        let pricing = ModelPricing::new("gpt-4o", 0.005, 0.015);

        // 1M input + 500K output
        let cost = pricing.calculate_cost(1_000_000, 500_000);
        // Input: 1M * 0.005 / 1000 = $5.00
        // Output: 500K * 0.015 / 1000 = $7.50
        // Total: $12.50
        assert!(
            (cost - 12.50).abs() < 0.01,
            "Expected $12.50, got ${}",
            cost
        );
    }

    #[test]
    fn test_model_pricing_zero_tokens() {
        let pricing = ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006);
        let cost = pricing.calculate_cost(0, 0);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_model_pricing_input_only() {
        let pricing = ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006);
        let cost = pricing.calculate_cost(1000, 0);
        assert!((cost - 0.00015).abs() < 0.000001);
    }

    #[test]
    fn test_model_pricing_output_only() {
        let pricing = ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006);
        let cost = pricing.calculate_cost(0, 1000);
        assert!((cost - 0.0006).abs() < 0.000001);
    }

    #[test]
    fn test_embedding_model_pricing() {
        let pricing = ModelPricing::new("text-embedding-3-small", 0.00002, 0.0);

        // 10K tokens for embedding
        let cost = pricing.calculate_cost(10_000, 0);
        // 10000 * 0.00002 / 1000 = $0.0002
        assert!(
            (cost - 0.0002).abs() < 0.00001,
            "Expected $0.0002, got ${}",
            cost
        );
    }

    #[test]
    fn test_default_model_pricing_contains_expected_models() {
        let pricing = default_model_pricing();

        // OpenAI models
        assert!(pricing.contains_key("gpt-4o-mini"), "Missing gpt-4o-mini");
        assert!(pricing.contains_key("gpt-4o"), "Missing gpt-4o");
        assert!(pricing.contains_key("gpt-4-turbo"), "Missing gpt-4-turbo");
        assert!(
            pricing.contains_key("gpt-3.5-turbo"),
            "Missing gpt-3.5-turbo"
        );

        // Anthropic models
        assert!(
            pricing.contains_key("claude-3-haiku"),
            "Missing claude-3-haiku"
        );
        assert!(
            pricing.contains_key("claude-3-sonnet"),
            "Missing claude-3-sonnet"
        );
        assert!(
            pricing.contains_key("claude-3-opus"),
            "Missing claude-3-opus"
        );

        // Embedding models
        assert!(
            pricing.contains_key("text-embedding-3-small"),
            "Missing text-embedding-3-small"
        );
        assert!(
            pricing.contains_key("text-embedding-3-large"),
            "Missing text-embedding-3-large"
        );
    }

    #[test]
    fn test_model_pricing_values_reasonable() {
        let pricing = default_model_pricing();

        // gpt-4o-mini should be cheapest
        let mini = pricing.get("gpt-4o-mini").unwrap();
        assert!(
            mini.input_cost_per_1k < 0.001,
            "gpt-4o-mini input should be < $0.001/1K"
        );

        // claude-3-opus should be most expensive
        let opus = pricing.get("claude-3-opus").unwrap();
        assert!(
            opus.input_cost_per_1k > 0.01,
            "claude-3-opus input should be > $0.01/1K"
        );

        // Embeddings should be cheaper than LLM
        let embed = pricing.get("text-embedding-3-small").unwrap();
        assert!(
            embed.input_cost_per_1k < mini.input_cost_per_1k,
            "Embedding should be cheaper than LLM"
        );
    }
}

// ============================================================================
// CostTracker Tests
// ============================================================================

mod cost_tracker_tests {
    use super::*;

    #[tokio::test]
    async fn test_cost_tracker_new_gpt4o_mini() {
        let tracker = CostTracker::new_gpt4o_mini("job-1");
        let snapshot = tracker.snapshot().await;

        assert_eq!(snapshot.job_id, "job-1");
        assert_eq!(snapshot.model, "gpt-4o-mini");
        assert_eq!(snapshot.total_cost_usd, 0.0);
        assert_eq!(snapshot.total_input_tokens, 0);
        assert_eq!(snapshot.total_output_tokens, 0);
    }

    #[tokio::test]
    async fn test_cost_tracker_record_single() {
        let tracker = CostTracker::new_gpt4o_mini("job-1");

        tracker.record("extract", 1000, 500).await;

        let snapshot = tracker.snapshot().await;
        assert_eq!(snapshot.total_input_tokens, 1000);
        assert_eq!(snapshot.total_output_tokens, 500);
        assert!(snapshot.total_cost_usd > 0.0);
        assert_eq!(snapshot.operations.len(), 1);
        assert!(snapshot.operations.contains_key("extract"));
    }

    #[tokio::test]
    async fn test_cost_tracker_record_multiple_same_operation() {
        let tracker = CostTracker::new_gpt4o_mini("job-1");

        tracker.record("extract", 1000, 500).await;
        tracker.record("extract", 2000, 1000).await;
        tracker.record("extract", 500, 250).await;

        let snapshot = tracker.snapshot().await;
        assert_eq!(snapshot.total_input_tokens, 3500);
        assert_eq!(snapshot.total_output_tokens, 1750);
        assert_eq!(snapshot.operations.len(), 1);
        assert_eq!(snapshot.operations["extract"].call_count, 3);
    }

    #[tokio::test]
    async fn test_cost_tracker_record_different_operations() {
        let tracker = CostTracker::new_gpt4o_mini("job-1");

        tracker.record("extract", 1000, 500).await;
        tracker.record("glean", 800, 400).await;
        tracker.record("summarize", 500, 200).await;

        let snapshot = tracker.snapshot().await;
        assert_eq!(snapshot.total_input_tokens, 2300);
        assert_eq!(snapshot.total_output_tokens, 1100);
        assert_eq!(snapshot.operations.len(), 3);
        assert!(snapshot.operations.contains_key("extract"));
        assert!(snapshot.operations.contains_key("glean"));
        assert!(snapshot.operations.contains_key("summarize"));
    }

    #[tokio::test]
    async fn test_cost_tracker_total_cost() {
        let tracker = CostTracker::new_gpt4o_mini("job-1");

        tracker.record("extract", 1000, 500).await;

        let total = tracker.total_cost().await;
        assert!(total > 0.0);

        // Verify it matches snapshot
        let snapshot = tracker.snapshot().await;
        assert_eq!(total, snapshot.total_cost_usd);
    }

    #[tokio::test]
    async fn test_cost_tracker_gpt4o() {
        let tracker = CostTracker::new_gpt4o("job-1");

        tracker.record("extract", 1000, 500).await;

        let snapshot = tracker.snapshot().await;
        assert_eq!(snapshot.model, "gpt-4o");

        // gpt-4o should be more expensive than gpt-4o-mini
        let mini_tracker = CostTracker::new_gpt4o_mini("job-2");
        mini_tracker.record("extract", 1000, 500).await;
        let mini_snapshot = mini_tracker.snapshot().await;

        assert!(snapshot.total_cost_usd > mini_snapshot.total_cost_usd);
    }

    #[tokio::test]
    async fn test_cost_tracker_thread_safety() {
        use std::sync::Arc;

        let tracker = Arc::new(CostTracker::new_gpt4o_mini("job-1"));
        let mut handles = vec![];

        // Spawn 10 concurrent tasks
        for i in 0..10 {
            let tracker = Arc::clone(&tracker);
            let handle = tokio::spawn(async move {
                tracker.record(&format!("op-{}", i), 100, 50).await;
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        let snapshot = tracker.snapshot().await;
        assert_eq!(snapshot.total_input_tokens, 1000);
        assert_eq!(snapshot.total_output_tokens, 500);
        assert_eq!(snapshot.operations.len(), 10);
    }

    #[tokio::test]
    async fn test_cost_tracker_clone() {
        let tracker1 = CostTracker::new_gpt4o_mini("job-1");
        tracker1.record("extract", 1000, 500).await;

        let tracker2 = tracker1.clone();
        tracker2.record("glean", 500, 250).await;

        // Both should see the same data (shared state)
        let snapshot1 = tracker1.snapshot().await;
        let snapshot2 = tracker2.snapshot().await;

        assert_eq!(snapshot1.total_input_tokens, snapshot2.total_input_tokens);
    }
}

// ============================================================================
// OperationCost Tests
// ============================================================================

mod operation_cost_tests {
    use super::*;

    #[test]
    fn test_operation_cost_new() {
        let op = OperationCost::new("extract");

        assert_eq!(op.operation, "extract");
        assert_eq!(op.call_count, 0);
        assert_eq!(op.input_tokens, 0);
        assert_eq!(op.output_tokens, 0);
        assert_eq!(op.total_cost_usd, 0.0);
    }

    #[test]
    fn test_operation_cost_add() {
        let mut op = OperationCost::new("extract");

        op.add(1000, 500, 0.00045);

        assert_eq!(op.call_count, 1);
        assert_eq!(op.input_tokens, 1000);
        assert_eq!(op.output_tokens, 500);
        assert!((op.total_cost_usd - 0.00045).abs() < 0.000001);
    }

    #[test]
    fn test_operation_cost_add_multiple() {
        let mut op = OperationCost::new("extract");

        op.add(1000, 500, 0.00045);
        op.add(2000, 1000, 0.0009);
        op.add(500, 250, 0.000225);

        assert_eq!(op.call_count, 3);
        assert_eq!(op.input_tokens, 3500);
        assert_eq!(op.output_tokens, 1750);
        assert!((op.total_cost_usd - 0.001575).abs() < 0.000001);
    }
}

// ============================================================================
// CostBreakdown Tests
// ============================================================================

mod cost_breakdown_tests {
    use super::*;

    #[test]
    fn test_cost_breakdown_new() {
        let breakdown = CostBreakdown::new("job-1", "gpt-4o-mini");

        assert_eq!(breakdown.job_id, "job-1");
        assert_eq!(breakdown.model, "gpt-4o-mini");
        assert_eq!(breakdown.total_cost_usd, 0.0);
        assert!(breakdown.operations.is_empty());
    }

    #[test]
    fn test_cost_breakdown_add_operation_cost() {
        let mut breakdown = CostBreakdown::new("job-1", "gpt-4o-mini");

        breakdown.add_operation_cost("extract", 1000, 500, 0.00045);

        assert_eq!(breakdown.total_input_tokens, 1000);
        assert_eq!(breakdown.total_output_tokens, 500);
        assert!((breakdown.total_cost_usd - 0.00045).abs() < 0.000001);
        assert!(breakdown.operations.contains_key("extract"));
    }

    #[test]
    fn test_cost_breakdown_formatted_cost() {
        let mut breakdown = CostBreakdown::new("job-1", "gpt-4o-mini");
        breakdown.add_operation_cost("extract", 1000, 500, 0.00045);

        let formatted = breakdown.formatted_cost();
        assert!(formatted.starts_with("$"));
        // The formatted string should contain the cost value
        assert!(
            formatted.contains("0.0004") || formatted.contains("0.00045"),
            "Expected cost around $0.0004, got {}",
            formatted
        );
    }

    #[test]
    fn test_cost_breakdown_multiple_operations() {
        let mut breakdown = CostBreakdown::new("job-1", "gpt-4o-mini");

        breakdown.add_operation_cost("extract", 1000, 500, 0.00045);
        breakdown.add_operation_cost("glean", 800, 400, 0.00036);
        breakdown.add_operation_cost("embed", 5000, 0, 0.0001);

        assert_eq!(breakdown.operations.len(), 3);
        assert_eq!(breakdown.total_input_tokens, 6800);
        assert_eq!(breakdown.total_output_tokens, 900);

        let expected_total = 0.00045 + 0.00036 + 0.0001;
        assert!((breakdown.total_cost_usd - expected_total).abs() < 0.000001);
    }
}

// ============================================================================
// ProcessingStats Tests
// ============================================================================

mod processing_stats_tests {
    use super::*;

    #[test]
    fn test_processing_stats_default() {
        let stats = ProcessingStats::default();

        assert_eq!(stats.chunk_count, 0);
        assert_eq!(stats.entity_count, 0);
        assert_eq!(stats.relationship_count, 0);
        assert_eq!(stats.llm_calls, 0);
        assert_eq!(stats.total_tokens, 0);
        assert_eq!(stats.input_tokens, 0);
        assert_eq!(stats.output_tokens, 0);
        assert_eq!(stats.cost_usd, 0.0);
        assert!(stats.cost_breakdown.is_none());
    }

    #[test]
    fn test_processing_stats_cost_fields() {
        let mut stats = ProcessingStats::default();

        stats.input_tokens = 1000;
        stats.output_tokens = 500;
        stats.cost_usd = 0.00045;

        assert_eq!(stats.input_tokens, 1000);
        assert_eq!(stats.output_tokens, 500);
        assert_eq!(stats.cost_usd, 0.00045);
    }

    #[test]
    fn test_processing_stats_cost_breakdown() {
        let mut stats = ProcessingStats::default();

        let mut breakdown = CostBreakdownStats::default();
        breakdown.extraction_cost_usd = 0.00045;
        breakdown.embedding_cost_usd = 0.0001;
        breakdown.extraction_input_tokens = 1000;
        breakdown.extraction_output_tokens = 500;
        breakdown.embedding_tokens = 5000;

        stats.cost_breakdown = Some(breakdown);
        stats.cost_usd = 0.00045 + 0.0001;

        let breakdown = stats.cost_breakdown.as_ref().unwrap();
        assert!((breakdown.extraction_cost_usd - 0.00045).abs() < 0.000001);
        assert!((breakdown.embedding_cost_usd - 0.0001).abs() < 0.000001);
    }

    #[test]
    fn test_processing_stats_serialization() {
        let mut stats = ProcessingStats::default();
        stats.chunk_count = 5;
        stats.entity_count = 10;
        stats.input_tokens = 1000;
        stats.output_tokens = 500;
        stats.cost_usd = 0.00045;

        let json = serde_json::to_string(&stats).unwrap();

        assert!(json.contains("\"chunk_count\":5"));
        assert!(json.contains("\"input_tokens\":1000"));
        assert!(json.contains("\"output_tokens\":500"));
        assert!(json.contains("\"cost_usd\":0.00045"));
    }

    #[test]
    fn test_processing_stats_deserialization() {
        let json = r#"{
            "chunk_count": 5,
            "entity_count": 10,
            "input_tokens": 1000,
            "output_tokens": 500,
            "cost_usd": 0.00045,
            "llm_calls": 3,
            "total_tokens": 1500,
            "relationship_count": 8,
            "processing_time_ms": 2500
        }"#;

        let stats: ProcessingStats = serde_json::from_str(json).unwrap();

        assert_eq!(stats.chunk_count, 5);
        assert_eq!(stats.entity_count, 10);
        assert_eq!(stats.input_tokens, 1000);
        assert_eq!(stats.output_tokens, 500);
        assert!((stats.cost_usd - 0.00045).abs() < 0.000001);
    }

    #[test]
    fn test_processing_stats_backward_compatible() {
        // Test that old JSON without cost fields still deserializes
        let json = r#"{
            "chunk_count": 5,
            "entity_count": 10,
            "llm_calls": 3,
            "total_tokens": 1500,
            "relationship_count": 8,
            "processing_time_ms": 2500
        }"#;

        let stats: ProcessingStats = serde_json::from_str(json).unwrap();

        assert_eq!(stats.chunk_count, 5);
        // New fields should default to 0
        assert_eq!(stats.input_tokens, 0);
        assert_eq!(stats.output_tokens, 0);
        assert_eq!(stats.cost_usd, 0.0);
    }
}

// ============================================================================
// CostBreakdownStats Tests
// ============================================================================

mod cost_breakdown_stats_tests {
    use super::*;

    #[test]
    fn test_cost_breakdown_stats_default() {
        let breakdown = CostBreakdownStats::default();

        assert_eq!(breakdown.extraction_cost_usd, 0.0);
        assert_eq!(breakdown.embedding_cost_usd, 0.0);
        assert_eq!(breakdown.summarization_cost_usd, 0.0);
        assert_eq!(breakdown.extraction_input_tokens, 0);
        assert_eq!(breakdown.extraction_output_tokens, 0);
        assert_eq!(breakdown.embedding_tokens, 0);
    }

    #[test]
    fn test_cost_breakdown_stats_serialization() {
        let mut breakdown = CostBreakdownStats::default();
        breakdown.extraction_cost_usd = 0.00045;
        breakdown.embedding_cost_usd = 0.0001;
        breakdown.extraction_input_tokens = 1000;
        breakdown.extraction_output_tokens = 500;
        breakdown.embedding_tokens = 5000;

        let json = serde_json::to_string(&breakdown).unwrap();

        assert!(json.contains("extraction_cost_usd"));
        assert!(json.contains("embedding_cost_usd"));
        assert!(json.contains("0.00045"));
    }

    #[test]
    fn test_cost_breakdown_stats_total_calculation() {
        let mut breakdown = CostBreakdownStats::default();
        breakdown.extraction_cost_usd = 0.00045;
        breakdown.embedding_cost_usd = 0.0001;
        breakdown.summarization_cost_usd = 0.00005;

        let total = breakdown.extraction_cost_usd
            + breakdown.embedding_cost_usd
            + breakdown.summarization_cost_usd;

        assert!((total - 0.0006).abs() < 0.000001);
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_very_large_token_counts() {
        let pricing = ModelPricing::new("gpt-4o", 0.005, 0.015);

        // 100M tokens (unrealistic but should not overflow)
        let cost = pricing.calculate_cost(100_000_000, 50_000_000);

        // Should produce reasonable result without overflow
        assert!(cost > 0.0);
        assert!(cost < 10_000_000.0); // Should be reasonable
    }

    #[test]
    fn test_very_small_costs() {
        let pricing = ModelPricing::new("text-embedding-3-small", 0.00002, 0.0);

        // Just 10 tokens
        let cost = pricing.calculate_cost(10, 0);

        // Should be a very small but non-zero number
        assert!(cost > 0.0);
        assert!(cost < 0.000001);
    }

    #[test]
    fn test_cost_precision() {
        let pricing = ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006);

        // Test various token counts
        let test_cases = [
            (1, 0, 0.00015 / 1000.0),
            (10, 5, (10.0 * 0.00015 + 5.0 * 0.0006) / 1000.0),
            (100, 50, (100.0 * 0.00015 + 50.0 * 0.0006) / 1000.0),
        ];

        for (input, output, expected) in test_cases {
            let cost = pricing.calculate_cost(input, output);
            assert!(
                (cost - expected).abs() < 0.0000001,
                "For ({}, {}): expected {}, got {}",
                input,
                output,
                expected,
                cost
            );
        }
    }
}
