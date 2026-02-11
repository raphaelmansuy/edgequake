/**
 * Costs resource — cost tracking, history, and budgets.
 *
 * @module resources/costs
 * @see edgequake/crates/edgequake-api/src/handlers/costs.rs
 */

import type {
  BudgetStatus,
  CostHistory,
  CostSummary,
  UpdateBudgetRequest,
} from "../types/costs.js";
import { Resource } from "./base.js";

export class CostsResource extends Resource {
  /** Get cost summary for the workspace. */
  async summary(): Promise<CostSummary> {
    return this._get("/api/v1/costs/summary");
  }

  /** Get cost history over time. */
  async history(): Promise<CostHistory> {
    return this._get("/api/v1/costs/history");
  }

  /** Get current budget status. */
  async budget(): Promise<BudgetStatus> {
    return this._get("/api/v1/costs/budget");
  }

  /** Update budget settings. */
  async updateBudget(request: UpdateBudgetRequest): Promise<BudgetStatus> {
    return this._patch("/api/v1/costs/budget", request);
  }
}
