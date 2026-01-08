//! Cost management DTO types.
//!
//! This module contains all Data Transfer Objects for the cost tracking and budget API.
//! Extracted from costs.rs for modularity and single responsibility.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ============================================================================
// Request DTOs
// ============================================================================

/// Cost estimation request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct EstimateCostRequest {
    /// Model to use for estimation.
    pub model: String,

    /// Estimated input tokens.
    pub input_tokens: usize,

    /// Estimated output tokens.
    pub output_tokens: usize,
}

/// Query parameters for cost history.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CostHistoryQuery {
    /// Start date (ISO 8601).
    pub start_date: Option<String>,

    /// End date (ISO 8601).
    pub end_date: Option<String>,

    /// Granularity: hour, day, week, month.
    pub granularity: Option<String>,
}

// ============================================================================
// Response DTOs
// ============================================================================

/// Model pricing information.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ModelPricingResponse {
    /// Model name.
    pub model: String,

    /// Cost per 1K input tokens (USD).
    pub input_cost_per_1k: f64,

    /// Cost per 1K output tokens (USD).
    pub output_cost_per_1k: f64,
}

/// Cost summary for the current session.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CostSummaryResponse {
    /// Total input tokens used.
    pub total_input_tokens: usize,

    /// Total output tokens used.
    pub total_output_tokens: usize,

    /// Total cost in USD.
    pub total_cost_usd: f64,

    /// Formatted cost string.
    pub formatted_cost: String,

    /// Per-operation breakdown.
    pub operations: Vec<OperationCostResponse>,
}

/// Cost for a single operation type.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OperationCostResponse {
    /// Operation name (extract, glean, summarize, embed).
    pub operation: String,

    /// Number of API calls.
    pub call_count: usize,

    /// Input tokens used.
    pub input_tokens: usize,

    /// Output tokens used.
    pub output_tokens: usize,

    /// Total cost (USD).
    pub cost_usd: f64,
}

/// Available model pricing configurations.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AvailablePricingResponse {
    /// List of available model pricing configs.
    pub models: Vec<ModelPricingResponse>,
}

/// Cost estimation response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EstimateCostResponse {
    /// Model used.
    pub model: String,

    /// Input tokens.
    pub input_tokens: usize,

    /// Output tokens.
    pub output_tokens: usize,

    /// Estimated cost in USD.
    pub estimated_cost_usd: f64,

    /// Formatted cost.
    pub formatted_cost: String,
}

/// Workspace cost summary response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WorkspaceCostSummaryResponse {
    /// Workspace ID.
    pub workspace_id: String,

    /// Total cost in USD.
    pub total_cost: f64,

    /// Total document count.
    pub document_count: usize,

    /// Total tokens used.
    pub total_tokens: usize,

    /// Average cost per document.
    pub average_cost_per_document: f64,

    /// Period start (ISO date).
    pub period_start: Option<String>,

    /// Period end (ISO date).
    pub period_end: Option<String>,

    /// Cost breakdown by operation.
    pub by_operation: Vec<OperationBreakdown>,

    /// Budget info if configured.
    pub budget: Option<BudgetInfo>,
}

/// Operation cost breakdown.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OperationBreakdown {
    /// Operation name.
    pub operation: String,

    /// Cost in USD.
    pub cost: f64,

    /// Percentage of total cost.
    pub percentage: f64,

    /// Input tokens for this operation.
    pub input_tokens: usize,

    /// Output tokens for this operation.
    pub output_tokens: usize,

    /// Total tokens for this operation.
    pub total_tokens: usize,

    /// Number of API calls for this operation.
    pub call_count: usize,
}

/// Budget information.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BudgetInfo {
    /// Monthly budget limit in USD.
    pub monthly_budget_usd: f64,

    /// Amount spent so far.
    pub spent_usd: f64,

    /// Remaining budget.
    pub remaining_usd: f64,

    /// Alert threshold percentage (0-100).
    pub alert_threshold: f64,

    /// Whether budget is exceeded.
    pub is_over_budget: bool,
}

/// Cost history data point.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CostHistoryPoint {
    /// Timestamp (ISO 8601).
    pub timestamp: String,

    /// Total cost in USD for this period.
    pub total_cost: f64,

    /// Total tokens for this period.
    pub total_tokens: usize,

    /// Document count for this period.
    pub document_count: usize,
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_cost_request() {
        let json = r#"{
            "model": "gpt-4o-mini",
            "input_tokens": 1000,
            "output_tokens": 500
        }"#;
        let req: EstimateCostRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.model, "gpt-4o-mini");
        assert_eq!(req.input_tokens, 1000);
        assert_eq!(req.output_tokens, 500);
    }

    #[test]
    fn test_cost_history_query_minimal() {
        let json = r#"{}"#;
        let query: CostHistoryQuery = serde_json::from_str(json).unwrap();
        assert!(query.start_date.is_none());
        assert!(query.granularity.is_none());
    }

    #[test]
    fn test_cost_history_query_full() {
        let json = r#"{
            "start_date": "2026-01-01T00:00:00Z",
            "end_date": "2026-01-07T23:59:59Z",
            "granularity": "day"
        }"#;
        let query: CostHistoryQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.start_date, Some("2026-01-01T00:00:00Z".to_string()));
        assert_eq!(query.granularity, Some("day".to_string()));
    }

    #[test]
    fn test_model_pricing_response() {
        let pricing = ModelPricingResponse {
            model: "gpt-4o-mini".to_string(),
            input_cost_per_1k: 0.15,
            output_cost_per_1k: 0.60,
        };
        let json = serde_json::to_value(&pricing).unwrap();
        assert_eq!(json["model"], "gpt-4o-mini");
        let input_cost = json["input_cost_per_1k"].as_f64().unwrap();
        assert!((input_cost - 0.15).abs() < 0.001);
    }

    #[test]
    fn test_cost_summary_response() {
        let summary = CostSummaryResponse {
            total_input_tokens: 10000,
            total_output_tokens: 5000,
            total_cost_usd: 2.25,
            formatted_cost: "$2.25".to_string(),
            operations: vec![],
        };
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["total_input_tokens"], 10000);
        assert_eq!(json["formatted_cost"], "$2.25");
    }

    #[test]
    fn test_operation_cost_response() {
        let op = OperationCostResponse {
            operation: "embed".to_string(),
            call_count: 100,
            input_tokens: 5000,
            output_tokens: 0,
            cost_usd: 0.75,
        };
        let json = serde_json::to_value(&op).unwrap();
        assert_eq!(json["operation"], "embed");
        assert_eq!(json["call_count"], 100);
    }

    #[test]
    fn test_budget_info() {
        let budget = BudgetInfo {
            monthly_budget_usd: 100.0,
            spent_usd: 75.50,
            remaining_usd: 24.50,
            alert_threshold: 80.0,
            is_over_budget: false,
        };
        let json = serde_json::to_value(&budget).unwrap();
        assert_eq!(json["is_over_budget"], false);
        let spent = json["spent_usd"].as_f64().unwrap();
        assert!((spent - 75.50).abs() < 0.001);
    }

    #[test]
    fn test_workspace_cost_summary() {
        let summary = WorkspaceCostSummaryResponse {
            workspace_id: "ws_123".to_string(),
            total_cost: 150.0,
            document_count: 100,
            total_tokens: 500000,
            average_cost_per_document: 1.50,
            period_start: Some("2026-01-01".to_string()),
            period_end: Some("2026-01-07".to_string()),
            by_operation: vec![],
            budget: None,
        };
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["workspace_id"], "ws_123");
        assert_eq!(json["document_count"], 100);
    }

    #[test]
    fn test_operation_breakdown() {
        let breakdown = OperationBreakdown {
            operation: "extract".to_string(),
            cost: 50.0,
            percentage: 33.33,
            input_tokens: 100000,
            output_tokens: 50000,
            total_tokens: 150000,
            call_count: 500,
        };
        let json = serde_json::to_value(&breakdown).unwrap();
        assert_eq!(json["operation"], "extract");
        assert_eq!(json["total_tokens"], 150000);
    }

    #[test]
    fn test_cost_history_point() {
        let point = CostHistoryPoint {
            timestamp: "2026-01-07T00:00:00Z".to_string(),
            total_cost: 25.50,
            total_tokens: 100000,
            document_count: 50,
        };
        let json = serde_json::to_value(&point).unwrap();
        assert_eq!(json["timestamp"], "2026-01-07T00:00:00Z");
        assert_eq!(json["document_count"], 50);
    }

    #[test]
    fn test_estimate_cost_response() {
        let response = EstimateCostResponse {
            model: "gpt-4o-mini".to_string(),
            input_tokens: 2000,
            output_tokens: 1000,
            estimated_cost_usd: 0.90,
            formatted_cost: "$0.90".to_string(),
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["model"], "gpt-4o-mini");
        assert_eq!(json["formatted_cost"], "$0.90");
    }
}
