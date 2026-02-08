//! Cost tracking API handlers (Phase 5).
//!
//! Provides endpoints for querying LLM API costs and token usage.
//!
//! ## Implements
//!
//! - **FEAT0510**: Model pricing retrieval endpoint
//! - **FEAT0511**: Cost estimation for token usage
//! - **FEAT0512**: Workspace cost summary aggregation
//! - **FEAT0513**: Operation-level cost breakdown
//!
//! ## Use Cases
//!
//! - **UC2110**: User views available model pricing information
//! - **UC2111**: User estimates cost before running expensive operations
//! - **UC2112**: Admin reviews workspace cost summary
//! - **UC2113**: User tracks cost trends over time via history endpoint
//!
//! ## Enforces
//!
//! - **BR0510**: Costs must be calculated using official pricing data
//! - **BR0511**: Unknown models fallback to gpt-4o-mini pricing
//! - **BR0512**: Cost history must respect workspace isolation

use axum::{
    extract::{Query, State},
    Json,
};

use crate::error::ApiResult;
use crate::state::AppState;

// Re-export DTOs for backward compatibility
pub use crate::handlers::costs_types::{
    AvailablePricingResponse, BudgetInfo, CostHistoryPoint, CostHistoryQuery, CostSummaryResponse,
    EstimateCostRequest, EstimateCostResponse, ModelPricingResponse, OperationBreakdown,
    OperationCostResponse, WorkspaceCostSummaryResponse,
};

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
        // Default to gpt-4.1-nano pricing if unknown
        edgequake_pipeline::ModelPricing::new("gpt-4.1-nano", 0.00015, 0.0006)
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

// ============================================================================
// Cost Summary Endpoint (WebUI Spec WEBUI-007)
// ============================================================================

/// Get workspace cost summary.
#[utoipa::path(
    get,
    path = "/api/v1/costs/summary",
    tag = "Costs",
    responses(
        (status = 200, description = "Workspace cost summary", body = WorkspaceCostSummaryResponse)
    )
)]
pub async fn get_cost_summary(
    State(state): State<AppState>,
    tenant_ctx: crate::middleware::TenantContext,
) -> ApiResult<Json<WorkspaceCostSummaryResponse>> {
    use tracing::{debug, warn};

    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Getting cost summary with tenant context"
    );

    // SECURITY: Enforce strict tenant context requirement
    if tenant_ctx.tenant_id.is_none() || tenant_ctx.workspace_id.is_none() {
        warn!(
            tenant_id = ?tenant_ctx.tenant_id,
            workspace_id = ?tenant_ctx.workspace_id,
            "Tenant context missing - returning empty cost summary for security"
        );
        return Ok(Json(WorkspaceCostSummaryResponse {
            workspace_id: "unknown".to_string(),
            total_cost: 0.0,
            document_count: 0,
            total_tokens: 0,
            average_cost_per_document: 0.0,
            period_start: None,
            period_end: None,
            by_operation: vec![],
            budget: None,
        }));
    }
    // Query all document metadata to aggregate costs
    let keys = state.kv_storage.keys().await?;

    // Find all metadata keys
    let metadata_keys: Vec<String> = keys
        .iter()
        .filter(|k| k.ends_with("-metadata"))
        .cloned()
        .collect();

    let mut total_cost = 0.0;
    let mut total_input_tokens = 0usize;
    let mut total_output_tokens = 0usize;
    let mut document_count = 0usize;
    let mut extraction_cost = 0.0;
    let mut embedding_cost = 0.0;

    if !metadata_keys.is_empty() {
        let values = state.kv_storage.get_by_ids(&metadata_keys).await?;

        for value in values {
            if let Some(obj) = value.as_object() {
                // SECURITY: Filter by tenant context
                let doc_tenant_id = obj.get("tenant_id").and_then(|v| v.as_str());
                let doc_workspace_id = obj.get("workspace_id").and_then(|v| v.as_str());

                // Only process documents matching BOTH tenant_id AND workspace_id
                if tenant_ctx.tenant_id.as_deref() != doc_tenant_id {
                    continue; // Skip document from other tenant
                }
                if tenant_ctx.workspace_id.as_deref() != doc_workspace_id {
                    continue; // Skip document from other workspace
                }

                // Only count completed documents
                let status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
                if status == "completed" || status == "indexed" {
                    document_count += 1;

                    // Aggregate cost
                    if let Some(cost) = obj.get("cost_usd").and_then(|v| v.as_f64()) {
                        total_cost += cost;
                        // For now, assume extraction is ~90% of cost
                        extraction_cost += cost * 0.9;
                        embedding_cost += cost * 0.1;
                    }

                    // Aggregate tokens
                    if let Some(input) = obj.get("input_tokens").and_then(|v| v.as_u64()) {
                        total_input_tokens += input as usize;
                    }
                    if let Some(output) = obj.get("output_tokens").and_then(|v| v.as_u64()) {
                        total_output_tokens += output as usize;
                    }
                }
            }
        }
    }

    let total_tokens = total_input_tokens + total_output_tokens;
    let average_cost = if document_count > 0 {
        total_cost / document_count as f64
    } else {
        0.0
    };

    // Calculate percentages
    let extraction_percentage = if total_cost > 0.0 {
        (extraction_cost / total_cost) * 100.0
    } else {
        0.0
    };
    let embedding_percentage = if total_cost > 0.0 {
        (embedding_cost / total_cost) * 100.0
    } else {
        0.0
    };

    Ok(Json(WorkspaceCostSummaryResponse {
        workspace_id: tenant_ctx
            .workspace_id
            .unwrap_or_else(|| "default".to_string()),
        total_cost,
        document_count,
        total_tokens,
        average_cost_per_document: average_cost,
        period_start: None,
        period_end: None,
        by_operation: vec![
            OperationBreakdown {
                operation: "extraction".to_string(),
                cost: extraction_cost,
                percentage: extraction_percentage,
                input_tokens: total_input_tokens,
                output_tokens: total_output_tokens,
                total_tokens: total_input_tokens + total_output_tokens,
                call_count: document_count,
            },
            OperationBreakdown {
                operation: "embedding".to_string(),
                cost: embedding_cost,
                percentage: embedding_percentage,
                input_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
                call_count: document_count,
            },
        ],
        budget: None,
    }))
}

/// Get budget status.
#[utoipa::path(
    get,
    path = "/api/v1/costs/budget",
    tag = "Costs",
    responses(
        (status = 200, description = "Budget status", body = BudgetInfo)
    )
)]
pub async fn get_budget_status(
    State(_state): State<AppState>,
    tenant_ctx: crate::middleware::TenantContext,
) -> ApiResult<Json<BudgetInfo>> {
    use tracing::{debug, warn};

    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Getting budget status with tenant context"
    );

    // SECURITY: Enforce strict tenant context requirement
    if tenant_ctx.tenant_id.is_none() || tenant_ctx.workspace_id.is_none() {
        warn!(
            tenant_id = ?tenant_ctx.tenant_id,
            workspace_id = ?tenant_ctx.workspace_id,
            "Tenant context missing - returning default budget for security"
        );
    }

    // Return default budget info (no budget configured by default)
    // TODO: In production, fetch tenant-specific budget from database
    Ok(Json(BudgetInfo {
        monthly_budget_usd: 100.0,
        spent_usd: 0.0,
        remaining_usd: 100.0,
        alert_threshold: 80.0,
        is_over_budget: false,
    }))
}

/// Update budget settings.
#[utoipa::path(
    patch,
    path = "/api/v1/costs/budget",
    tag = "Costs",
    request_body = BudgetInfo,
    responses(
        (status = 200, description = "Updated budget", body = BudgetInfo)
    )
)]
pub async fn update_budget(
    State(_state): State<AppState>,
    tenant_ctx: crate::middleware::TenantContext,
    Json(budget): Json<BudgetInfo>,
) -> ApiResult<Json<BudgetInfo>> {
    use tracing::{debug, warn};

    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Updating budget with tenant context"
    );

    // SECURITY: Enforce strict tenant context requirement
    if tenant_ctx.tenant_id.is_none() || tenant_ctx.workspace_id.is_none() {
        warn!(
            tenant_id = ?tenant_ctx.tenant_id,
            workspace_id = ?tenant_ctx.workspace_id,
            "Tenant context missing - rejecting budget update"
        );
        return Err(crate::error::ApiError::BadRequest(
            "Tenant context required for budget updates".to_string(),
        ));
    }

    // In production, this would persist budget settings per tenant/workspace
    Ok(Json(budget))
}

// ============================================================================
// Cost History Endpoint (WebUI Spec WEBUI-007)
// ============================================================================

/// Get cost history over time.
#[utoipa::path(
    get,
    path = "/api/v1/costs/history",
    tag = "Costs",
    params(
        ("start_date" = Option<String>, Query, description = "Start date (ISO 8601)"),
        ("end_date" = Option<String>, Query, description = "End date (ISO 8601)"),
        ("granularity" = Option<String>, Query, description = "Granularity: hour, day, week, month")
    ),
    responses(
        (status = 200, description = "Cost history", body = Vec<CostHistoryPoint>)
    )
)]
pub async fn get_cost_history(
    State(state): State<AppState>,
    tenant_ctx: crate::middleware::TenantContext,
    Query(params): Query<CostHistoryQuery>,
) -> ApiResult<Json<Vec<CostHistoryPoint>>> {
    use chrono::{DateTime, Datelike, Duration, Utc};
    use std::collections::BTreeMap;
    use tracing::{debug, warn};

    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Getting cost history with tenant context"
    );

    // SECURITY: Enforce strict tenant context requirement
    if tenant_ctx.tenant_id.is_none() || tenant_ctx.workspace_id.is_none() {
        warn!(
            tenant_id = ?tenant_ctx.tenant_id,
            workspace_id = ?tenant_ctx.workspace_id,
            "Tenant context missing - returning empty cost history for security"
        );
        return Ok(Json(vec![]));
    }

    let granularity = params.granularity.as_deref().unwrap_or("day");

    // Query all document metadata
    let keys = state.kv_storage.keys().await?;
    let metadata_keys: Vec<String> = keys
        .iter()
        .filter(|k| k.ends_with("-metadata"))
        .cloned()
        .collect();

    // Group costs by time period
    let mut period_data: BTreeMap<String, (f64, usize, usize)> = BTreeMap::new();

    if !metadata_keys.is_empty() {
        let values = state.kv_storage.get_by_ids(&metadata_keys).await?;

        for value in values {
            if let Some(obj) = value.as_object() {
                // SECURITY: Filter by tenant context
                let doc_tenant_id = obj.get("tenant_id").and_then(|v| v.as_str());
                let doc_workspace_id = obj.get("workspace_id").and_then(|v| v.as_str());

                // Only process documents matching BOTH tenant_id AND workspace_id
                if tenant_ctx.tenant_id.as_deref() != doc_tenant_id {
                    continue; // Skip document from other tenant
                }
                if tenant_ctx.workspace_id.as_deref() != doc_workspace_id {
                    continue; // Skip document from other workspace
                }

                // Only count completed documents
                let status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
                if status != "completed" && status != "indexed" {
                    continue;
                }

                // Get processed_at or created_at timestamp
                let timestamp_str = obj
                    .get("processed_at")
                    .or_else(|| obj.get("created_at"))
                    .and_then(|v| v.as_str());

                if let Some(ts) = timestamp_str {
                    // Parse timestamp and truncate to period
                    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
                        let dt_utc = dt.with_timezone(&Utc);
                        let period_key = match granularity {
                            "hour" => dt_utc.format("%Y-%m-%dT%H:00:00Z").to_string(),
                            "week" => {
                                let week_start = dt_utc
                                    - Duration::days(dt_utc.weekday().num_days_from_monday() as i64);
                                week_start.format("%Y-%m-%dT00:00:00Z").to_string()
                            }
                            "month" => dt_utc.format("%Y-%m-01T00:00:00Z").to_string(),
                            _ => dt_utc.format("%Y-%m-%dT00:00:00Z").to_string(), // day
                        };

                        let entry = period_data.entry(period_key).or_insert((0.0, 0, 0));

                        // Add cost
                        if let Some(cost) = obj.get("cost_usd").and_then(|v| v.as_f64()) {
                            entry.0 += cost;
                        }

                        // Add tokens
                        let input = obj
                            .get("input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let output = obj
                            .get("output_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        entry.1 += (input + output) as usize;

                        // Increment document count
                        entry.2 += 1;
                    }
                }
            }
        }
    }

    // Convert to response
    let history: Vec<CostHistoryPoint> = period_data
        .into_iter()
        .map(|(timestamp, (cost, tokens, count))| CostHistoryPoint {
            timestamp,
            total_cost: cost,
            total_tokens: tokens,
            document_count: count,
        })
        .collect();

    Ok(Json(history))
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

    #[test]
    fn test_estimate_cost_response_serialization() {
        let response = EstimateCostResponse {
            model: "gpt-4o".to_string(),
            input_tokens: 5000,
            output_tokens: 2000,
            estimated_cost_usd: 0.025,
            formatted_cost: "$0.025000".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"model\":\"gpt-4o\""));
        assert!(json.contains("\"input_tokens\":5000"));
        assert!(json.contains("\"formatted_cost\":\"$0.025000\""));
    }

    #[test]
    fn test_operation_cost_response_serialization() {
        let response = OperationCostResponse {
            operation: "summarize".to_string(),
            call_count: 10,
            input_tokens: 5000,
            output_tokens: 1000,
            cost_usd: 0.00135,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"operation\":\"summarize\""));
        assert!(json.contains("\"call_count\":10"));
    }

    #[test]
    fn test_available_pricing_response_empty() {
        let response = AvailablePricingResponse { models: vec![] };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"models\":[]"));
    }
}
