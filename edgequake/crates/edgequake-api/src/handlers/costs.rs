//! Cost tracking API handlers (Phase 5).
//!
//! Provides endpoints for querying LLM API costs and token usage.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ApiResult;
use crate::state::AppState;

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

/// Get available model pricing configurations.
#[utoipa::path(
    get,
    path = "/api/v1/pipeline/costs/pricing",
    tag = "Pipeline",
    responses(
        (status = 200, description = "Available model pricing", body = AvailablePricingResponse)
    )
)]
pub async fn get_model_pricing(
    State(_state): State<AppState>,
) -> ApiResult<Json<AvailablePricingResponse>> {
    let pricing = edgequake_pipeline::default_model_pricing();

    let models: Vec<ModelPricingResponse> = pricing
        .values()
        .map(|p| ModelPricingResponse {
            model: p.model.clone(),
            input_cost_per_1k: p.input_cost_per_1k,
            output_cost_per_1k: p.output_cost_per_1k,
        })
        .collect();

    Ok(Json(AvailablePricingResponse { models }))
}

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

/// Estimate cost for token usage.
#[utoipa::path(
    post,
    path = "/api/v1/pipeline/costs/estimate",
    tag = "Pipeline",
    request_body = EstimateCostRequest,
    responses(
        (status = 200, description = "Cost estimate", body = EstimateCostResponse),
        (status = 400, description = "Unknown model")
    )
)]
pub async fn estimate_cost(
    State(_state): State<AppState>,
    Json(request): Json<EstimateCostRequest>,
) -> ApiResult<Json<EstimateCostResponse>> {
    let pricing = edgequake_pipeline::default_model_pricing();

    let model_pricing = pricing.get(&request.model).cloned().unwrap_or_else(|| {
        // Default to gpt-4o-mini pricing if unknown
        edgequake_pipeline::ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006)
    });

    let cost = model_pricing.calculate_cost(request.input_tokens, request.output_tokens);

    Ok(Json(EstimateCostResponse {
        model: request.model,
        input_tokens: request.input_tokens,
        output_tokens: request.output_tokens,
        estimated_cost_usd: cost,
        formatted_cost: format!("${:.6}", cost),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_pricing_response_serialization() {
        let response = ModelPricingResponse {
            model: "gpt-4o-mini".to_string(),
            input_cost_per_1k: 0.00015,
            output_cost_per_1k: 0.0006,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("gpt-4o-mini"));
        assert!(json.contains("0.00015"));
    }

    #[test]
    fn test_cost_summary_response_serialization() {
        let response = CostSummaryResponse {
            total_input_tokens: 1000,
            total_output_tokens: 500,
            total_cost_usd: 0.00045,
            formatted_cost: "$0.0005".to_string(),
            operations: vec![OperationCostResponse {
                operation: "extract".to_string(),
                call_count: 5,
                input_tokens: 1000,
                output_tokens: 500,
                cost_usd: 0.00045,
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total_input_tokens\":1000"));
        assert!(json.contains("extract"));
    }

    #[test]
    fn test_estimate_cost_request_deserialization() {
        let json = r#"{"model": "gpt-4o-mini", "input_tokens": 1000, "output_tokens": 500}"#;
        let request: EstimateCostRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.model, "gpt-4o-mini");
        assert_eq!(request.input_tokens, 1000);
        assert_eq!(request.output_tokens, 500);
    }
}
