/**
 * Costs resource — cost tracking, history, budgets, and estimation.
 *
 * WHY: Updated to match Rust costs_types.rs exactly.
 * Adds pricing, estimation, and workspace cost summary endpoints.
 *
 * @module resources/costs
 * @see edgequake/crates/edgequake-api/src/handlers/costs.rs
 */

import type {
  AvailablePricingResponse,
  BudgetInfo,
  CostHistoryQuery,
  CostHistoryResponse,
  CostSummaryResponse,
  EstimateCostRequest,
  EstimateCostResponse,
  UpdateBudgetRequest,
  WorkspaceCostSummaryResponse,
} from "../types/costs.js";
import { Resource } from "./base.js";

export class CostsResource extends Resource {
  /** Get cost summary for the workspace. */
  async summary(): Promise<CostSummaryResponse> {
    return this._get("/api/v1/costs/summary");
  }

  /** Get cost history over time. */
  async history(query?: CostHistoryQuery): Promise<CostHistoryResponse> {
    const params = new URLSearchParams();
    if (query?.start_date) params.set("start_date", query.start_date);
    if (query?.end_date) params.set("end_date", query.end_date);
    if (query?.granularity) params.set("granularity", query.granularity);
    const qs = params.toString();
    return this._get(`/api/v1/costs/history${qs ? `?${qs}` : ""}`);
  }

  /** Get current budget status. */
  async budget(): Promise<BudgetInfo> {
    return this._get("/api/v1/costs/budget");
  }

  /** Update budget settings. */
  async updateBudget(request: UpdateBudgetRequest): Promise<BudgetInfo> {
    return this._patch("/api/v1/costs/budget", request);
  }

  /** Get available model pricing configurations. */
  async pricing(): Promise<AvailablePricingResponse> {
    return this._get("/api/v1/costs/pricing");
  }

  /** Estimate cost for a given model and token counts. */
  async estimate(request: EstimateCostRequest): Promise<EstimateCostResponse> {
    return this._post("/api/v1/costs/estimate", request);
  }

  /** Get workspace cost summary (includes per-operation breakdown). */
  async workspaceSummary(): Promise<WorkspaceCostSummaryResponse> {
    return this._get("/api/v1/costs/workspace");
  }
}
