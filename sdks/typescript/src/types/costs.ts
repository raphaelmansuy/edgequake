/**
 * Cost tracking types.
 *
 * WHY: Rewritten to match Rust costs_types.rs exactly.
 * Rust uses detailed per-operation breakdowns, workspace-scoped summaries,
 * and budget info with is_over_budget flag.
 *
 * @module types/costs
 * @see edgequake/crates/edgequake-api/src/handlers/costs_types.rs
 */

// ── Request DTOs ──────────────────────────────────────────────

/** Cost estimation request. */
export interface EstimateCostRequest {
  /** Model to use for estimation. */
  model: string;
  /** Estimated input tokens. */
  input_tokens: number;
  /** Estimated output tokens. */
  output_tokens: number;
}

/** Query parameters for cost history. */
export interface CostHistoryQuery {
  /** Start date (ISO 8601). */
  start_date?: string;
  /** End date (ISO 8601). */
  end_date?: string;
  /** Granularity: hour, day, week, month. */
  granularity?: string;
}

/** Update budget settings. */
export interface UpdateBudgetRequest {
  monthly_budget_usd?: number;
  alert_threshold?: number;
}

// ── Response DTOs ─────────────────────────────────────────────

/** Model pricing information. */
export interface ModelPricingResponse {
  /** Model name. */
  model: string;
  /** Cost per 1K input tokens (USD). */
  input_cost_per_1k: number;
  /** Cost per 1K output tokens (USD). */
  output_cost_per_1k: number;
}

/** Cost summary for the current session/workspace. */
export interface CostSummaryResponse {
  /** Total input tokens used. */
  total_input_tokens: number;
  /** Total output tokens used. */
  total_output_tokens: number;
  /** Total cost in USD. */
  total_cost_usd: number;
  /** Formatted cost string (e.g., "$2.25"). */
  formatted_cost: string;
  /** Per-operation breakdown. */
  operations: OperationCostResponse[];
}

/** Cost for a single operation type. */
export interface OperationCostResponse {
  /** Operation name (extract, glean, summarize, embed). */
  operation: string;
  /** Number of API calls. */
  call_count: number;
  /** Input tokens used. */
  input_tokens: number;
  /** Output tokens used. */
  output_tokens: number;
  /** Total cost (USD). */
  cost_usd: number;
}

/** Available model pricing configurations. */
export interface AvailablePricingResponse {
  /** List of available model pricing configs. */
  models: ModelPricingResponse[];
}

/** Cost estimation response. */
export interface EstimateCostResponse {
  /** Model used. */
  model: string;
  /** Input tokens. */
  input_tokens: number;
  /** Output tokens. */
  output_tokens: number;
  /** Estimated cost in USD. */
  estimated_cost_usd: number;
  /** Formatted cost. */
  formatted_cost: string;
}

/** Workspace cost summary response. */
export interface WorkspaceCostSummaryResponse {
  /** Workspace ID. */
  workspace_id: string;
  /** Total cost in USD. */
  total_cost: number;
  /** Total document count. */
  document_count: number;
  /** Total tokens used. */
  total_tokens: number;
  /** Average cost per document. */
  average_cost_per_document: number;
  /** Period start (ISO date). */
  period_start?: string;
  /** Period end (ISO date). */
  period_end?: string;
  /** Cost breakdown by operation. */
  by_operation: OperationBreakdown[];
  /** Budget info if configured. */
  budget?: BudgetInfo;
}

/** Operation cost breakdown within workspace summary. */
export interface OperationBreakdown {
  /** Operation name. */
  operation: string;
  /** Cost in USD. */
  cost: number;
  /** Percentage of total cost. */
  percentage: number;
  /** Input tokens for this operation. */
  input_tokens: number;
  /** Output tokens for this operation. */
  output_tokens: number;
  /** Total tokens for this operation. */
  total_tokens: number;
  /** Number of API calls for this operation. */
  call_count: number;
}

/** Budget information. */
export interface BudgetInfo {
  /** Monthly budget limit in USD. */
  monthly_budget_usd: number;
  /** Amount spent so far. */
  spent_usd: number;
  /** Remaining budget. */
  remaining_usd: number;
  /** Alert threshold percentage (0-100). */
  alert_threshold: number;
  /** Whether budget is exceeded. */
  is_over_budget: boolean;
}

/** Cost history data point. */
export interface CostHistoryPoint {
  /** Timestamp (ISO 8601). */
  timestamp: string;
  /** Total cost in USD for this period. */
  total_cost: number;
  /** Total tokens for this period. */
  total_tokens: number;
  /** Document count for this period. */
  document_count: number;
}

/** Cost history response. */
export interface CostHistoryResponse {
  /** History data points. */
  data_points: CostHistoryPoint[];
}

// ── Legacy aliases (backward compat) ─────────────────────────
// WHY: Keep old names as aliases so existing user code doesn't break.
/** @deprecated Use CostSummaryResponse */
export type CostSummary = CostSummaryResponse;
/** @deprecated Use CostHistoryResponse */
export type CostHistory = CostHistoryResponse;
/** @deprecated Use BudgetInfo */
export type BudgetStatus = BudgetInfo;
