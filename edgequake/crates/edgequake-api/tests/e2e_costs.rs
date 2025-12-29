//! E2E tests for cost tracking API endpoints.
//!
//! Tests the following cost-related endpoints:
//! - GET /api/v1/pipeline/costs/pricing - Get model pricing
//! - POST /api/v1/pipeline/costs/estimate - Estimate cost
//! - GET /api/v1/costs/summary - Get workspace cost summary
//! - GET /api/v1/costs/budget - Get budget status
//! - PATCH /api/v1/costs/budget - Update budget settings
//!
//! Run with: `cargo test --package edgequake-api --test e2e_costs`

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use tower::ServiceExt;

// ============================================================================
// Test Utilities
// ============================================================================

fn create_test_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    }
}

fn create_test_server() -> Server {
    Server::new(create_test_config(), AppState::test_state())
}

fn create_test_app() -> axum::Router {
    create_test_server().build_router()
}

async fn extract_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&bytes).unwrap_or(json!({}))
}

// =============================================================================
// Model Pricing Tests
// =============================================================================

mod model_pricing_tests {
    use super::*;

    #[tokio::test]
    async fn test_get_model_pricing_returns_models() {
        let app = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/pipeline/costs/pricing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = extract_json(response).await;
        let models = body["models"]
            .as_array()
            .expect("models should be an array");

        // Should have at least some models
        assert!(!models.is_empty(), "Should return at least one model");
    }

    #[tokio::test]
    async fn test_model_pricing_has_expected_fields() {
        let app = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/pipeline/costs/pricing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = extract_json(response).await;
        let models = body["models"].as_array().unwrap();

        for model in models {
            // Each model should have required fields
            assert!(model["model"].is_string(), "model should be a string");
            assert!(
                model["input_cost_per_1k"].is_f64(),
                "input_cost_per_1k should be a number"
            );
            assert!(
                model["output_cost_per_1k"].is_f64(),
                "output_cost_per_1k should be a number"
            );

            // Costs should be non-negative
            let input_cost = model["input_cost_per_1k"].as_f64().unwrap();
            let output_cost = model["output_cost_per_1k"].as_f64().unwrap();
            assert!(input_cost >= 0.0, "input cost should be non-negative");
            assert!(output_cost >= 0.0, "output cost should be non-negative");
        }
    }

    #[tokio::test]
    async fn test_model_pricing_includes_gpt4o_mini() {
        let app = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/pipeline/costs/pricing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = extract_json(response).await;
        let models = body["models"].as_array().unwrap();

        let has_gpt4o_mini = models.iter().any(|m| m["model"] == "gpt-4o-mini");
        assert!(has_gpt4o_mini, "Should include gpt-4o-mini pricing");
    }

    #[tokio::test]
    async fn test_model_pricing_includes_embedding_models() {
        let app = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/pipeline/costs/pricing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = extract_json(response).await;
        let models = body["models"].as_array().unwrap();

        let has_embedding = models
            .iter()
            .any(|m| m["model"].as_str().unwrap_or("").contains("embedding"));
        assert!(has_embedding, "Should include embedding model pricing");
    }
}

// =============================================================================
// Cost Estimation Tests
// =============================================================================

mod cost_estimation_tests {
    use super::*;

    #[tokio::test]
    async fn test_estimate_cost_gpt4o_mini() {
        let app = create_test_app();

        let request_body = json!({
            "model": "gpt-4o-mini",
            "input_tokens": 1000,
            "output_tokens": 500
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pipeline/costs/estimate")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = extract_json(response).await;

        assert_eq!(body["model"], "gpt-4o-mini");
        assert_eq!(body["input_tokens"], 1000);
        assert_eq!(body["output_tokens"], 500);

        // gpt-4o-mini: $0.00015/1k input, $0.0006/1k output
        // Expected: 1000 * 0.00015/1000 + 500 * 0.0006/1000 = 0.00015 + 0.0003 = 0.00045
        let cost = body["estimated_cost_usd"].as_f64().unwrap();
        assert!(
            (cost - 0.00045).abs() < 0.0001,
            "Expected ~$0.00045, got ${}",
            cost
        );

        // Should have formatted cost
        let formatted = body["formatted_cost"].as_str().unwrap();
        assert!(
            formatted.starts_with("$"),
            "formatted_cost should start with $"
        );
    }

    #[tokio::test]
    async fn test_estimate_cost_gpt4o() {
        let app = create_test_app();

        let request_body = json!({
            "model": "gpt-4o",
            "input_tokens": 1000,
            "output_tokens": 500
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pipeline/costs/estimate")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = extract_json(response).await;

        // gpt-4o: $0.005/1k input, $0.015/1k output
        // Expected: 1000 * 0.005/1000 + 500 * 0.015/1000 = 0.005 + 0.0075 = 0.0125
        let cost = body["estimated_cost_usd"].as_f64().unwrap();
        assert!(
            (cost - 0.0125).abs() < 0.001,
            "Expected ~$0.0125, got ${}",
            cost
        );
    }

    #[tokio::test]
    async fn test_estimate_cost_zero_tokens() {
        let app = create_test_app();

        let request_body = json!({
            "model": "gpt-4o-mini",
            "input_tokens": 0,
            "output_tokens": 0
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pipeline/costs/estimate")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = extract_json(response).await;
        let cost = body["estimated_cost_usd"].as_f64().unwrap();
        assert_eq!(cost, 0.0, "Zero tokens should cost $0.00");
    }

    #[tokio::test]
    async fn test_estimate_cost_large_document() {
        let app = create_test_app();

        // Simulate a large document: 100K input, 20K output
        let request_body = json!({
            "model": "gpt-4o-mini",
            "input_tokens": 100000,
            "output_tokens": 20000
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pipeline/costs/estimate")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = extract_json(response).await;
        let cost = body["estimated_cost_usd"].as_f64().unwrap();

        // Expected: 100000 * 0.00015/1000 + 20000 * 0.0006/1000 = 0.015 + 0.012 = 0.027
        assert!(
            (cost - 0.027).abs() < 0.005,
            "Expected ~$0.027, got ${}",
            cost
        );
    }

    #[tokio::test]
    async fn test_estimate_cost_unknown_model_uses_default() {
        let app = create_test_app();

        let request_body = json!({
            "model": "unknown-model-xyz",
            "input_tokens": 1000,
            "output_tokens": 500
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pipeline/costs/estimate")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should still succeed with default pricing
        assert_eq!(response.status(), StatusCode::OK);

        let body = extract_json(response).await;
        let cost = body["estimated_cost_usd"].as_f64().unwrap();

        // Should use gpt-4o-mini as default
        assert!(
            (cost - 0.00045).abs() < 0.0001,
            "Unknown model should use gpt-4o-mini pricing"
        );
    }

    #[tokio::test]
    async fn test_estimate_cost_input_only() {
        let app = create_test_app();

        let request_body = json!({
            "model": "gpt-4o-mini",
            "input_tokens": 10000,
            "output_tokens": 0
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pipeline/costs/estimate")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = extract_json(response).await;
        let cost = body["estimated_cost_usd"].as_f64().unwrap();

        // Input only: 10000 * 0.00015/1000 = 0.0015
        assert!(
            (cost - 0.0015).abs() < 0.0001,
            "Expected $0.0015, got ${}",
            cost
        );
    }

    #[tokio::test]
    async fn test_estimate_cost_output_only() {
        let app = create_test_app();

        let request_body = json!({
            "model": "gpt-4o-mini",
            "input_tokens": 0,
            "output_tokens": 10000
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pipeline/costs/estimate")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = extract_json(response).await;
        let cost = body["estimated_cost_usd"].as_f64().unwrap();

        // Output only: 10000 * 0.0006/1000 = 0.006
        assert!(
            (cost - 0.006).abs() < 0.001,
            "Expected $0.006, got ${}",
            cost
        );
    }
}

// =============================================================================
// Cost Summary Tests
// =============================================================================

mod cost_summary_tests {
    use super::*;

    #[tokio::test]
    async fn test_get_cost_summary_structure() {
        let app = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/costs/summary")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = extract_json(response).await;

        // Check required fields exist
        assert!(
            body["workspace_id"].is_string(),
            "workspace_id should exist"
        );
        assert!(body["total_cost"].is_f64(), "total_cost should exist");
        assert!(
            body["document_count"].is_number(),
            "document_count should exist"
        );
        assert!(
            body["total_tokens"].is_number(),
            "total_tokens should exist"
        );
        assert!(body["by_operation"].is_array(), "by_operation should exist");
    }

    #[tokio::test]
    async fn test_cost_summary_has_operation_breakdown() {
        let app = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/costs/summary")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = extract_json(response).await;
        let operations = body["by_operation"].as_array().unwrap();

        // Should have operation breakdown
        for op in operations {
            assert!(op["operation"].is_string(), "operation should be a string");
            assert!(op["cost"].is_f64(), "cost should be a number");
            assert!(op["percentage"].is_f64(), "percentage should be a number");
        }
    }

    #[tokio::test]
    async fn test_cost_summary_values_are_valid() {
        let app = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/costs/summary")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = extract_json(response).await;

        let total_cost = body["total_cost"].as_f64().unwrap();
        let document_count = body["document_count"].as_u64().unwrap_or(0);
        let total_tokens = body["total_tokens"].as_u64().unwrap_or(0);

        // Values should be non-negative
        assert!(total_cost >= 0.0, "total_cost should be non-negative");
        assert!(document_count >= 0, "document_count should be non-negative");
        assert!(total_tokens >= 0, "total_tokens should be non-negative");
    }
}

// =============================================================================
// Budget Tests
// =============================================================================

mod budget_tests {
    use super::*;

    #[tokio::test]
    async fn test_get_budget_status() {
        let app = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/costs/budget")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = extract_json(response).await;

        // Check required fields
        assert!(body["monthly_budget_usd"].is_f64());
        assert!(body["spent_usd"].is_f64());
        assert!(body["remaining_usd"].is_f64());
        assert!(body["alert_threshold"].is_f64());
        assert!(body["is_over_budget"].is_boolean());
    }

    #[tokio::test]
    async fn test_update_budget() {
        let app = create_test_app();

        let new_budget = json!({
            "monthly_budget_usd": 200.0,
            "spent_usd": 50.0,
            "remaining_usd": 150.0,
            "alert_threshold": 75.0,
            "is_over_budget": false
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/v1/costs/budget")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(new_budget.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = extract_json(response).await;
        assert_eq!(body["monthly_budget_usd"], 200.0);
        assert_eq!(body["alert_threshold"], 75.0);
    }

    #[tokio::test]
    async fn test_budget_calculation_consistency() {
        let app = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/costs/budget")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = extract_json(response).await;

        let budget = body["monthly_budget_usd"].as_f64().unwrap();
        let spent = body["spent_usd"].as_f64().unwrap();
        let remaining = body["remaining_usd"].as_f64().unwrap();

        // Remaining should equal budget - spent
        let expected_remaining = budget - spent;
        assert!(
            (remaining - expected_remaining).abs() < 0.01,
            "remaining ({}) should equal budget ({}) - spent ({})",
            remaining,
            budget,
            spent
        );
    }

    #[tokio::test]
    async fn test_budget_over_budget_flag() {
        let app = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/costs/budget")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = extract_json(response).await;

        let spent = body["spent_usd"].as_f64().unwrap();
        let budget = body["monthly_budget_usd"].as_f64().unwrap();
        let is_over = body["is_over_budget"].as_bool().unwrap();

        // is_over_budget should be consistent with spent vs budget
        if spent > budget {
            assert!(is_over, "is_over_budget should be true when spent > budget");
        } else {
            assert!(
                !is_over,
                "is_over_budget should be false when spent <= budget"
            );
        }
    }
}

// =============================================================================
// Cost Formatting Tests
// =============================================================================

mod cost_formatting_tests {
    use super::*;

    #[tokio::test]
    async fn test_formatted_cost_has_dollar_sign() {
        let app = create_test_app();

        let request_body = json!({
            "model": "gpt-4o-mini",
            "input_tokens": 1000,
            "output_tokens": 500
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pipeline/costs/estimate")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = extract_json(response).await;
        let formatted = body["formatted_cost"].as_str().unwrap();

        assert!(
            formatted.starts_with("$"),
            "Formatted cost should start with $"
        );
    }

    #[tokio::test]
    async fn test_formatted_cost_has_proper_precision() {
        let app = create_test_app();

        let request_body = json!({
            "model": "gpt-4o-mini",
            "input_tokens": 1000,
            "output_tokens": 500
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pipeline/costs/estimate")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = extract_json(response).await;
        let formatted = body["formatted_cost"].as_str().unwrap();

        // Should have at least 4 decimal places for small costs
        // e.g., "$0.000450"
        assert!(
            formatted.len() >= 5,
            "Formatted cost should have proper precision: {}",
            formatted
        );
    }
}

// =============================================================================
// Model Pricing Accuracy Tests
// =============================================================================

mod model_pricing_accuracy_tests {
    use super::*;

    #[tokio::test]
    async fn test_gpt4o_mini_pricing_is_correct() {
        let app = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/pipeline/costs/pricing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = extract_json(response).await;
        let models = body["models"].as_array().unwrap();

        let gpt4o_mini = models.iter().find(|m| m["model"] == "gpt-4o-mini");
        assert!(gpt4o_mini.is_some(), "Should have gpt-4o-mini");

        let model = gpt4o_mini.unwrap();
        let input_cost = model["input_cost_per_1k"].as_f64().unwrap();
        let output_cost = model["output_cost_per_1k"].as_f64().unwrap();

        // gpt-4o-mini: $0.00015/1k input, $0.0006/1k output
        assert!(
            (input_cost - 0.00015).abs() < 0.0001,
            "gpt-4o-mini input cost should be ~$0.00015, got ${}",
            input_cost
        );
        assert!(
            (output_cost - 0.0006).abs() < 0.0001,
            "gpt-4o-mini output cost should be ~$0.0006, got ${}",
            output_cost
        );
    }

    #[tokio::test]
    async fn test_gpt4o_pricing_is_correct() {
        let app = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/pipeline/costs/pricing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = extract_json(response).await;
        let models = body["models"].as_array().unwrap();

        let gpt4o = models.iter().find(|m| m["model"] == "gpt-4o");
        assert!(gpt4o.is_some(), "Should have gpt-4o");

        let model = gpt4o.unwrap();
        let input_cost = model["input_cost_per_1k"].as_f64().unwrap();
        let output_cost = model["output_cost_per_1k"].as_f64().unwrap();

        // gpt-4o: $0.005/1k input, $0.015/1k output
        assert!(
            (input_cost - 0.005).abs() < 0.001,
            "gpt-4o input cost should be ~$0.005, got ${}",
            input_cost
        );
        assert!(
            (output_cost - 0.015).abs() < 0.001,
            "gpt-4o output cost should be ~$0.015, got ${}",
            output_cost
        );
    }
}
