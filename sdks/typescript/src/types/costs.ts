/**
 * Cost tracking types.
 *
 * @module types/costs
 * @see edgequake/crates/edgequake-api/src/handlers/costs_types.rs
 */

import type { Timestamp } from "./common.js";

export interface CostSummary {
  total_cost: number;
  currency: string;
  period: string;
  breakdown: Array<{
    category: string;
    cost: number;
    count: number;
  }>;
}

export interface CostHistoryQuery {
  from?: string;
  to?: string;
  granularity?: "hourly" | "daily" | "weekly";
}

export interface CostHistory {
  data_points: Array<{
    timestamp: Timestamp;
    cost: number;
    operations: number;
  }>;
}

export interface BudgetStatus {
  monthly_budget?: number;
  current_spend: number;
  remaining?: number;
  percentage_used?: number;
  alert_threshold?: number;
}

export interface UpdateBudgetRequest {
  monthly_budget?: number;
  alert_threshold?: number;
}
