# SOTA Cost of Ingestion Testing Plan

> Document ID: COST-SOTA-001
> Version: 1.0
> Created: 2024-12-29
> Status: IN PROGRESS

---

## Executive Summary

This document outlines the comprehensive testing strategy for making the EdgeQuake ingestion cost tracking system SOTA (State of the Art). The goal is to ensure:

1. **Complete cost visibility**: Both $ and tokens at every level (document, chunk, operation)
2. **Full stack coverage**: Backend API, Pipeline, Storage, and WebUI in sync
3. **SOTA test coverage**: Unit, Integration, and E2E tests with brutal honesty evaluation

---

## Current State Analysis

### What Exists ✅

| Component           | File                                                      | Status                   |
| ------------------- | --------------------------------------------------------- | ------------------------ |
| CostTracker         | `edgequake-pipeline/src/progress.rs`                      | ✅ Implemented           |
| ModelPricing        | `edgequake-pipeline/src/progress.rs`                      | ✅ Implemented           |
| ProcessingStats     | `edgequake-pipeline/src/pipeline.rs`                      | ⚠️ Has tokens, missing $ |
| Cost API Endpoints  | `edgequake-api/src/handlers/costs.rs`                     | ⚠️ Placeholder responses |
| Cost Types (WebUI)  | `edgequake_webui/src/types/cost.ts`                       | ✅ Complete              |
| Cost Hooks (WebUI)  | `edgequake_webui/src/hooks/use-cost.ts`                   | ✅ Implemented           |
| CostBadge Component | `edgequake_webui/src/components/documents/cost-badge.tsx` | ✅ Implemented           |

### What's Missing ❌

| Gap              | Description                              | Priority |
| ---------------- | ---------------------------------------- | -------- |
| **GAP-COST-001** | ProcessingStats missing `cost_usd` field | P0       |
| **GAP-COST-002** | Document response missing cost breakdown | P0       |
| **GAP-COST-003** | API endpoints return placeholder data    | P0       |
| **GAP-COST-004** | No cost persistence (database storage)   | P1       |
| **GAP-COST-005** | No WebSocket cost streaming              | P2       |
| **GAP-COST-006** | No budget enforcement                    | P2       |

---

## SOTA Test Categories

### 1. Unit Tests (edgequake-pipeline)

```
edgequake/crates/edgequake-pipeline/tests/
├── cost_tracker_tests.rs       ← NEW: CostTracker comprehensive tests
├── model_pricing_tests.rs      ← NEW: ModelPricing edge cases
└── processing_stats_tests.rs   ← NEW: ProcessingStats with $ metrics
```

**Tests to create:**

| Test ID     | Test Name                                   | Description                    |
| ----------- | ------------------------------------------- | ------------------------------ |
| UT-COST-001 | `test_model_pricing_gpt4o_mini`             | Verify pricing for gpt-4o-mini |
| UT-COST-002 | `test_model_pricing_gpt4o`                  | Verify pricing for gpt-4o      |
| UT-COST-003 | `test_model_pricing_embeddings`             | Verify embedding model pricing |
| UT-COST-004 | `test_cost_tracker_record_single`           | Record single operation        |
| UT-COST-005 | `test_cost_tracker_record_multiple`         | Record multiple operations     |
| UT-COST-006 | `test_cost_tracker_thread_safe`             | Concurrent access to tracker   |
| UT-COST-007 | `test_cost_breakdown_operations`            | Per-operation breakdown        |
| UT-COST-008 | `test_cost_breakdown_total`                 | Total accumulation             |
| UT-COST-009 | `test_processing_stats_cost_usd`            | ProcessingStats includes $     |
| UT-COST-010 | `test_processing_stats_input_output_tokens` | Separate token tracking        |

### 2. Integration Tests (edgequake-api)

```
edgequake/crates/edgequake-api/tests/
├── e2e_pipeline_cost_tests.rs  ← NEW: Cost integration tests
└── e2e_cost_api_tests.rs       ← NEW: Cost API endpoint tests
```

**Tests to create:**

| Test ID     | Test Name                              | Description                                 |
| ----------- | -------------------------------------- | ------------------------------------------- |
| IT-COST-001 | `test_pipeline_returns_cost_in_result` | Pipeline includes cost in response          |
| IT-COST-002 | `test_document_upload_returns_cost`    | Upload returns cost breakdown               |
| IT-COST-003 | `test_batch_upload_aggregates_cost`    | Batch sums costs correctly                  |
| IT-COST-004 | `test_cost_summary_endpoint`           | GET /api/v1/costs/summary returns real data |
| IT-COST-005 | `test_document_cost_endpoint`          | GET /api/v1/costs/documents/:id             |
| IT-COST-006 | `test_estimate_cost_endpoint`          | POST /api/v1/pipeline/costs/estimate        |
| IT-COST-007 | `test_model_pricing_endpoint`          | GET /api/v1/pipeline/costs/pricing          |
| IT-COST-008 | `test_budget_get_endpoint`             | GET /api/v1/costs/budget                    |
| IT-COST-009 | `test_budget_update_endpoint`          | PATCH /api/v1/costs/budget                  |
| IT-COST-010 | `test_cost_history_endpoint`           | GET /api/v1/costs/history                   |

### 3. E2E Tests (Full Stack)

```
edgequake_webui/e2e/
├── cost-tracking.spec.ts       ← NEW: Cost E2E tests
├── ingestion-cost.spec.ts      ← NEW: Ingestion cost flow
└── cost-display.spec.ts        ← NEW: WebUI cost display
```

**Tests to create:**

| Test ID      | Test Name                                 | Description                        |
| ------------ | ----------------------------------------- | ---------------------------------- |
| E2E-COST-001 | `test_upload_shows_cost_after_completion` | Document card shows cost           |
| E2E-COST-002 | `test_batch_shows_total_cost`             | Batch progress shows aggregate     |
| E2E-COST-003 | `test_document_detail_cost_tab`           | Detail panel Cost tab works        |
| E2E-COST-004 | `test_cost_breakdown_chart`               | CostBreakdownChart renders         |
| E2E-COST-005 | `test_cost_badge_formats`                 | CostBadge shows $0.0001+ correctly |
| E2E-COST-006 | `test_costs_page_summary`                 | /costs page shows summary          |
| E2E-COST-007 | `test_cost_in_tokens_and_dollars`         | Both metrics visible               |
| E2E-COST-008 | `test_budget_alert_display`               | Budget threshold triggers alert    |
| E2E-COST-009 | `test_cost_trend_indicator`               | Trend up/down shows correctly      |
| E2E-COST-010 | `test_cost_export`                        | CSV/JSON export works              |

---

## Implementation Plan

### Phase 1: Backend Cost Infrastructure (Priority P0)

#### 1.1 Enhance ProcessingStats

Add cost fields to `ProcessingStats`:

```rust
pub struct ProcessingStats {
    // ... existing fields ...

    /// Cost in USD for this processing run.
    pub cost_usd: f64,

    /// Input tokens used.
    pub input_tokens: usize,

    /// Output tokens used.
    pub output_tokens: usize,

    /// Cost breakdown by operation.
    pub cost_breakdown: Option<CostBreakdown>,
}
```

#### 1.2 Integrate CostTracker into Pipeline

```rust
impl Pipeline {
    pub async fn process_with_cost_tracking(
        &self,
        document_id: &str,
        content: &str,
        cost_tracker: &CostTracker,
    ) -> Result<ProcessingResult> {
        // ... process document ...

        // Record costs for each operation
        cost_tracker.record("extraction", input_tokens, output_tokens).await;
        cost_tracker.record("embedding", embed_tokens, 0).await;

        // Include cost in result
        let cost_breakdown = cost_tracker.snapshot().await;
        stats.cost_usd = cost_breakdown.total_cost_usd;
        stats.cost_breakdown = Some(cost_breakdown);

        // ... return result with cost ...
    }
}
```

#### 1.3 Update Document Response

Add cost to `UploadDocumentResponse`:

```rust
pub struct UploadDocumentResponse {
    // ... existing fields ...

    /// Cost breakdown for this document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<DocumentCostResponse>,
}

pub struct DocumentCostResponse {
    pub total_cost_usd: f64,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub model: String,
}
```

### Phase 2: Unit Tests (Priority P0)

Create `edgequake/crates/edgequake-pipeline/tests/cost_tracking_tests.rs`:

```rust
//! Cost tracking unit tests.

use edgequake_pipeline::{
    CostBreakdown, CostTracker, ModelPricing,
    OperationCost, default_model_pricing
};

#[test]
fn test_model_pricing_gpt4o_mini() {
    let pricing = ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006);
    let cost = pricing.calculate_cost(1000, 500);
    // Input: 1000 * 0.00015/1000 = 0.00015
    // Output: 500 * 0.0006/1000 = 0.0003
    // Total: 0.00045
    assert!((cost - 0.00045).abs() < 0.000001);
}

#[test]
fn test_model_pricing_large_scale() {
    let pricing = ModelPricing::new("gpt-4o", 0.005, 0.015);
    // 1M input, 500K output
    let cost = pricing.calculate_cost(1_000_000, 500_000);
    // Input: 1M * 0.005/1000 = $5.00
    // Output: 500K * 0.015/1000 = $7.50
    // Total: $12.50
    assert!((cost - 12.50).abs() < 0.01);
}

#[tokio::test]
async fn test_cost_tracker_accumulation() {
    let tracker = CostTracker::new_gpt4o_mini("job-1");

    tracker.record("extract", 1000, 500).await;
    tracker.record("extract", 2000, 1000).await;
    tracker.record("embed", 5000, 0).await;

    let breakdown = tracker.snapshot().await;

    assert_eq!(breakdown.total_input_tokens, 8000);
    assert_eq!(breakdown.total_output_tokens, 1500);
    assert_eq!(breakdown.operations.len(), 2);
    assert!(breakdown.total_cost_usd > 0.0);
}

// ... more tests ...
```

### Phase 3: Integration Tests (Priority P0)

Create `edgequake/crates/edgequake-api/tests/e2e_cost_tracking.rs`:

```rust
//! Cost tracking E2E tests.

use axum_test::TestServer;
// ... imports ...

#[tokio::test]
async fn test_document_upload_returns_cost() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/api/v1/documents")
        .json(&json!({
            "content": "Dr. Sarah Chen works at Stanford.",
            "title": "Test Doc",
            "async_processing": false
        }))
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();

    // Verify cost is included
    assert!(body.get("cost").is_some());
    let cost = body["cost"]["total_cost_usd"].as_f64().unwrap();
    assert!(cost > 0.0);

    // Verify tokens are included
    let input_tokens = body["cost"]["input_tokens"].as_u64().unwrap();
    let output_tokens = body["cost"]["output_tokens"].as_u64().unwrap();
    assert!(input_tokens > 0);
    assert!(output_tokens > 0);
}

#[tokio::test]
async fn test_cost_summary_returns_workspace_total() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Upload multiple documents
    for i in 0..3 {
        server
            .post("/api/v1/documents")
            .json(&json!({
                "content": format!("Document {} content.", i),
                "async_processing": false
            }))
            .await;
    }

    // Get cost summary
    let response = server.get("/api/v1/costs/summary").await;
    response.assert_status_ok();

    let summary: serde_json::Value = response.json();

    assert!(summary["total_cost"].as_f64().unwrap() > 0.0);
    assert_eq!(summary["document_count"].as_u64().unwrap(), 3);
}

// ... more tests ...
```

### Phase 4: WebUI E2E Tests (Priority P1)

Create `edgequake_webui/e2e/cost-tracking.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";

test.describe("Cost Tracking E2E", () => {
  test("document shows cost after upload", async ({ page }) => {
    await page.goto("/documents");

    // Upload a document
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles("./test-data/sample.txt");

    // Wait for processing
    await expect(page.locator('[data-testid="document-status"]')).toHaveText(
      "completed",
      { timeout: 30000 }
    );

    // Check cost badge is visible
    const costBadge = page.locator('[data-testid="cost-badge"]');
    await expect(costBadge).toBeVisible();

    // Verify cost format
    const costText = await costBadge.textContent();
    expect(costText).toMatch(/\$\d+\.\d+/);
  });

  test("cost tab shows breakdown chart", async ({ page }) => {
    await page.goto("/documents");

    // Click on a completed document
    await page.locator('[data-testid="document-row"]').first().click();

    // Navigate to Cost tab
    await page.locator('[data-testid="tab-cost"]').click();

    // Verify breakdown chart renders
    await expect(
      page.locator('[data-testid="cost-breakdown-chart"]')
    ).toBeVisible();

    // Verify token counts are shown
    await expect(page.getByText(/input tokens/i)).toBeVisible();
    await expect(page.getByText(/output tokens/i)).toBeVisible();
  });

  test("costs page shows summary with $ and tokens", async ({ page }) => {
    await page.goto("/costs");

    // Wait for data to load
    await expect(page.locator('[data-testid="cost-summary"]')).toBeVisible({
      timeout: 10000,
    });

    // Verify $ amount
    const totalCost = page.locator('[data-testid="total-cost"]');
    await expect(totalCost).toHaveText(/\$/);

    // Verify token count
    const totalTokens = page.locator('[data-testid="total-tokens"]');
    await expect(totalTokens).toBeVisible();

    // Verify breakdown by operation
    await expect(page.getByText(/extraction/i)).toBeVisible();
    await expect(page.getByText(/embedding/i)).toBeVisible();
  });
});
```

---

## Success Criteria

### SOTA Evaluation Rubric

| Category          | Weight | Criteria                        | Target |
| ----------------- | ------ | ------------------------------- | ------ |
| **Test Coverage** | 30%    | Line coverage of cost code      | >90%   |
| **Accuracy**      | 25%    | Cost calculations match pricing | 100%   |
| **Completeness**  | 20%    | All $ and token metrics tracked | 100%   |
| **WebUI Parity**  | 15%    | WebUI shows all backend data    | 100%   |
| **Performance**   | 10%    | No cost tracking overhead       | <5ms   |

### Definition of Done

- [ ] All unit tests pass (30+ tests)
- [ ] All integration tests pass (20+ tests)
- [ ] All E2E tests pass (10+ tests)
- [ ] ProcessingStats includes `cost_usd`, `input_tokens`, `output_tokens`
- [ ] Document upload response includes cost breakdown
- [ ] Cost summary API returns real data (not placeholders)
- [ ] WebUI CostBadge displays correctly on document rows
- [ ] WebUI Cost tab in detail panel works
- [ ] WebUI /costs page shows summary with $ and tokens
- [ ] No test failures or flakiness

---

## Brutal Honesty Evaluation Criteria

At the end of implementation, evaluate against:

1. **Are costs accurate?** Compare calculated costs against manual token counting
2. **Is coverage complete?** Run with OPENAI_API_KEY and verify real costs match
3. **Are edge cases handled?** 0 tokens, huge documents, concurrent uploads
4. **Is WebUI useful?** Can a user understand their costs at a glance?
5. **Is it SOTA?** Compare against LightRAG, OpenAI usage dashboards

---

## Appendix: Model Pricing Reference

| Model                  | Input $/1K | Output $/1K | Notes                  |
| ---------------------- | ---------- | ----------- | ---------------------- |
| gpt-4o-mini            | $0.00015   | $0.0006     | Default for extraction |
| gpt-4o                 | $0.005     | $0.015      | Premium model          |
| gpt-4-turbo            | $0.01      | $0.03       | Legacy                 |
| gpt-3.5-turbo          | $0.0005    | $0.0015     | Budget option          |
| text-embedding-3-small | $0.00002   | -           | 1536 dims              |
| text-embedding-3-large | $0.00013   | -           | 3072 dims              |
| claude-3-haiku         | $0.00025   | $0.00125    | Anthropic budget       |
| claude-3-sonnet        | $0.003     | $0.015      | Anthropic mid-tier     |
| claude-3-opus          | $0.015     | $0.075      | Anthropic premium      |

---

## Timeline

| Phase | Task                        | Duration | Status         |
| ----- | --------------------------- | -------- | -------------- |
| 1     | Backend Cost Infrastructure | 2h       | 🔄 In Progress |
| 2     | Unit Tests                  | 1h       | ⏳ Pending     |
| 3     | Integration Tests           | 2h       | ⏳ Pending     |
| 4     | WebUI E2E Tests             | 1h       | ⏳ Pending     |
| 5     | Fix Issues & Iterate        | 2h       | ⏳ Pending     |
| 6     | SOTA Evaluation             | 1h       | ⏳ Pending     |

**Total Estimated: 9 hours**
