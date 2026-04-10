//! Cost tracking and estimation.
//!
//! Provides LLM API cost estimation and tracking based on token usage
//! with configurable pricing for different models.
//!
//! @implements FEAT0013 (Cost Tracking)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Model pricing information (per 1K tokens).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Model name.
    pub model: String,
    /// Cost per 1K input tokens (USD).
    pub input_cost_per_1k: f64,
    /// Cost per 1K output tokens (USD).
    pub output_cost_per_1k: f64,
}

impl ModelPricing {
    /// Create new pricing config.
    pub fn new(model: impl Into<String>, input_cost: f64, output_cost: f64) -> Self {
        Self {
            model: model.into(),
            input_cost_per_1k: input_cost,
            output_cost_per_1k: output_cost,
        }
    }

    /// Calculate cost for token usage.
    pub fn calculate_cost(&self, input_tokens: usize, output_tokens: usize) -> f64 {
        let input_cost = (input_tokens as f64 / 1000.0) * self.input_cost_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * self.output_cost_per_1k;
        input_cost + output_cost
    }
}

/// Cost for a single operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperationCost {
    /// Operation type (extract, glean, summarize, embed).
    pub operation: String,
    /// Number of calls.
    pub call_count: usize,
    /// Total input tokens.
    pub input_tokens: usize,
    /// Total output tokens.
    pub output_tokens: usize,
    /// Total cost (USD).
    pub total_cost_usd: f64,
}

impl OperationCost {
    /// Create new operation cost tracker.
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            ..Default::default()
        }
    }

    /// Add usage to this operation.
    pub fn add(&mut self, input: usize, output: usize, cost: f64) {
        self.call_count += 1;
        self.input_tokens += input;
        self.output_tokens += output;
        self.total_cost_usd += cost;
    }
}

/// Complete cost breakdown for an ingestion job.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostBreakdown {
    /// Job ID.
    pub job_id: String,
    /// Model used.
    pub model: String,
    /// Per-operation costs.
    pub operations: HashMap<String, OperationCost>,
    /// Total input tokens.
    pub total_input_tokens: usize,
    /// Total output tokens.
    pub total_output_tokens: usize,
    /// Total cost (USD).
    pub total_cost_usd: f64,
}

impl CostBreakdown {
    /// Create new cost breakdown.
    pub fn new(job_id: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            job_id: job_id.into(),
            model: model.into(),
            ..Default::default()
        }
    }

    /// Add cost for an operation.
    pub fn add_operation_cost(&mut self, operation: &str, input: usize, output: usize, cost: f64) {
        let op = self
            .operations
            .entry(operation.to_string())
            .or_insert_with(|| OperationCost::new(operation));

        op.add(input, output, cost);
        self.total_input_tokens += input;
        self.total_output_tokens += output;
        self.total_cost_usd += cost;
    }

    /// Get formatted cost string.
    pub fn formatted_cost(&self) -> String {
        format!("${:.4}", self.total_cost_usd)
    }
}

/// Thread-safe cost tracker.
#[derive(Debug)]
pub struct CostTracker {
    inner: Arc<RwLock<CostBreakdown>>,
    pricing: ModelPricing,
}

impl CostTracker {
    /// Create new cost tracker.
    pub fn new(job_id: impl Into<String>, model: impl Into<String>, pricing: ModelPricing) -> Self {
        let model_str = model.into();
        Self {
            inner: Arc::new(RwLock::new(CostBreakdown::new(job_id, &model_str))),
            pricing,
        }
    }

    /// Create with default gpt-4.1-nano pricing (recommended cost-effective model).
    pub fn new_gpt5_nano(job_id: impl Into<String>) -> Self {
        let pricing = ModelPricing::new("gpt-4.1-nano", 0.0001, 0.0004);
        Self::new(job_id, "gpt-4.1-nano", pricing)
    }

    /// Create with gpt-4o-mini pricing (legacy, prefer gpt-4.1-nano).
    ///
    /// # Deprecation Notice
    /// This function is deprecated. Use `new_gpt5_nano()` instead for better
    /// cost efficiency and availability. gpt-4o-mini quotas may be exceeded.
    #[deprecated(
        since = "0.1.0",
        note = "Use new_gpt5_nano() for better cost efficiency"
    )]
    pub fn new_gpt4o_mini(job_id: impl Into<String>) -> Self {
        let pricing = ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006);
        Self::new(job_id, "gpt-4o-mini", pricing)
    }

    /// Create with gpt-4o pricing.
    pub fn new_gpt4o(job_id: impl Into<String>) -> Self {
        let pricing = ModelPricing::new("gpt-4o", 0.005, 0.015);
        Self::new(job_id, "gpt-4o", pricing)
    }

    /// Record token usage for an operation.
    pub async fn record(&self, operation: &str, input_tokens: usize, output_tokens: usize) {
        let cost = self.pricing.calculate_cost(input_tokens, output_tokens);
        let mut breakdown = self.inner.write().await;
        breakdown.add_operation_cost(operation, input_tokens, output_tokens, cost);
    }

    /// Get current cost breakdown.
    pub async fn snapshot(&self) -> CostBreakdown {
        self.inner.read().await.clone()
    }

    /// Get total cost so far.
    pub async fn total_cost(&self) -> f64 {
        self.inner.read().await.total_cost_usd
    }
}

impl Clone for CostTracker {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            pricing: self.pricing.clone(),
        }
    }
}

/// Default model pricing configurations.
pub fn default_model_pricing() -> HashMap<String, ModelPricing> {
    let mut pricing = HashMap::new();

    // === OpenAI Models ===
    pricing.insert(
        "gpt-4.1-nano".to_string(),
        ModelPricing::new("gpt-4.1-nano", 0.0001, 0.0004),
    );
    pricing.insert(
        "gpt-4.1-mini".to_string(),
        ModelPricing::new("gpt-4.1-mini", 0.0004, 0.0016),
    );
    pricing.insert(
        "gpt-4.1".to_string(),
        ModelPricing::new("gpt-4.1", 0.002, 0.008),
    );
    pricing.insert(
        "gpt-4o-mini".to_string(),
        ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006),
    );
    pricing.insert(
        "gpt-4o".to_string(),
        ModelPricing::new("gpt-4o", 0.0025, 0.01),
    );
    pricing.insert(
        "o4-mini".to_string(),
        ModelPricing::new("o4-mini", 0.0011, 0.0044),
    );
    pricing.insert(
        "gpt-4-turbo".to_string(),
        ModelPricing::new("gpt-4-turbo", 0.01, 0.03),
    );
    pricing.insert(
        "gpt-3.5-turbo".to_string(),
        ModelPricing::new("gpt-3.5-turbo", 0.0005, 0.0015),
    );

    // === Anthropic Models ===
    pricing.insert(
        "claude-opus-4-6".to_string(),
        ModelPricing::new("claude-opus-4-6", 0.005, 0.025),
    );
    pricing.insert(
        "claude-sonnet-4-5-20250929".to_string(),
        ModelPricing::new("claude-sonnet-4-5-20250929", 0.003, 0.015),
    );
    pricing.insert(
        "claude-haiku-4-5-20251001".to_string(),
        ModelPricing::new("claude-haiku-4-5-20251001", 0.001, 0.005),
    );

    // === Google Gemini Models ===
    pricing.insert(
        "gemini-2.5-pro".to_string(),
        ModelPricing::new("gemini-2.5-pro", 0.00125, 0.01),
    );
    pricing.insert(
        "gemini-2.5-flash".to_string(),
        ModelPricing::new("gemini-2.5-flash", 0.00015, 0.0006),
    );
    pricing.insert(
        "gemini-2.5-flash-lite".to_string(),
        ModelPricing::new("gemini-2.5-flash-lite", 0.0001, 0.0004),
    );
    pricing.insert(
        "gemini-2.0-flash".to_string(),
        ModelPricing::new("gemini-2.0-flash", 0.0001, 0.0004),
    );

    // === xAI Models ===
    pricing.insert(
        "grok-4-1-fast".to_string(),
        ModelPricing::new("grok-4-1-fast", 0.0002, 0.0005),
    );
    pricing.insert(
        "grok-4-0709".to_string(),
        ModelPricing::new("grok-4-0709", 0.003, 0.015),
    );
    pricing.insert(
        "grok-3".to_string(),
        ModelPricing::new("grok-3", 0.003, 0.015),
    );
    pricing.insert(
        "grok-3-mini".to_string(),
        ModelPricing::new("grok-3-mini", 0.0003, 0.0005),
    );

    // === Embedding Models ===
    pricing.insert(
        "text-embedding-3-small".to_string(),
        ModelPricing::new("text-embedding-3-small", 0.00002, 0.0),
    );
    pricing.insert(
        "text-embedding-3-large".to_string(),
        ModelPricing::new("text-embedding-3-large", 0.00013, 0.0),
    );
    pricing.insert(
        "gemini-embedding-001".to_string(),
        ModelPricing::new("gemini-embedding-001", 0.00015, 0.0),
    );

    // === Legacy compatibility aliases ===
    pricing.insert(
        "claude-3-haiku".to_string(),
        ModelPricing::new("claude-3-haiku", 0.00025, 0.00125),
    );
    pricing.insert(
        "claude-3-sonnet".to_string(),
        ModelPricing::new("claude-3-sonnet", 0.003, 0.015),
    );
    pricing.insert(
        "claude-3-opus".to_string(),
        ModelPricing::new("claude-3-opus", 0.015, 0.075),
    );

    pricing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_pricing() {
        let pricing = ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006);

        let cost = pricing.calculate_cost(1000, 500);
        // 1000 input = $0.00015, 500 output = $0.0003
        assert!((cost - 0.00045).abs() < 0.00001);
    }

    #[tokio::test]
    async fn test_cost_tracker() {
        let tracker = CostTracker::new_gpt5_nano("job-1");

        tracker.record("extract", 1000, 500).await;
        tracker.record("extract", 2000, 1000).await;

        let breakdown = tracker.snapshot().await;
        assert_eq!(breakdown.operations.len(), 1);
        assert_eq!(breakdown.operations["extract"].call_count, 2);
        assert_eq!(breakdown.total_input_tokens, 3000);
        assert_eq!(breakdown.total_output_tokens, 1500);
    }

    #[test]
    fn test_default_model_pricing() {
        let pricing = default_model_pricing();
        assert!(pricing.contains_key("gpt-4o-mini"));
        assert!(pricing.contains_key("claude-3-haiku"));
        assert!(pricing.contains_key("text-embedding-3-small"));
    }

    #[test]
    fn test_model_pricing_zero_tokens() {
        let pricing = ModelPricing::new("test", 0.001, 0.002);
        assert_eq!(pricing.calculate_cost(0, 0), 0.0);
    }

    #[test]
    fn test_model_pricing_only_input() {
        let pricing = ModelPricing::new("test", 1.0, 2.0);
        // 2000 input tokens = 2.0 * (2000/1000) = 2.0
        let cost = pricing.calculate_cost(2000, 0);
        assert!((cost - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_model_pricing_only_output() {
        let pricing = ModelPricing::new("test", 1.0, 2.0);
        // 500 output tokens = 2.0 * (500/1000) = 1.0
        let cost = pricing.calculate_cost(0, 500);
        assert!((cost - 1.0).abs() < 1e-10);
    }

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
    fn test_operation_cost_add_accumulates() {
        let mut op = OperationCost::new("embed");
        op.add(100, 50, 0.01);
        op.add(200, 100, 0.02);
        assert_eq!(op.call_count, 2);
        assert_eq!(op.input_tokens, 300);
        assert_eq!(op.output_tokens, 150);
        assert!((op.total_cost_usd - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_cost_breakdown_new() {
        let cb = CostBreakdown::new("job-1", "gpt-4o");
        assert_eq!(cb.job_id, "job-1");
        assert_eq!(cb.model, "gpt-4o");
        assert!(cb.operations.is_empty());
        assert_eq!(cb.total_cost_usd, 0.0);
    }

    #[test]
    fn test_cost_breakdown_add_operation_cost() {
        let mut cb = CostBreakdown::new("j", "m");
        cb.add_operation_cost("extract", 1000, 500, 0.05);
        cb.add_operation_cost("extract", 2000, 1000, 0.10);
        cb.add_operation_cost("embed", 500, 0, 0.01);

        assert_eq!(cb.operations.len(), 2);
        assert_eq!(cb.operations["extract"].call_count, 2);
        assert_eq!(cb.operations["embed"].call_count, 1);
        assert_eq!(cb.total_input_tokens, 3500);
        assert_eq!(cb.total_output_tokens, 1500);
        assert!((cb.total_cost_usd - 0.16).abs() < 1e-10);
    }

    #[test]
    fn test_cost_breakdown_formatted_cost() {
        let mut cb = CostBreakdown::new("j", "m");
        cb.total_cost_usd = 0.1234;
        assert_eq!(cb.formatted_cost(), "$0.1234");
    }

    #[test]
    fn test_cost_breakdown_formatted_cost_zero() {
        let cb = CostBreakdown::new("j", "m");
        assert_eq!(cb.formatted_cost(), "$0.0000");
    }
}
