/**
 * @module cost-types
 * @description Types for LLM cost monitoring and budget management.
 * Based on WebUI Specification Document WEBUI-007 (16-webui-cost-monitoring.md)
 *
 * @implements UC0801 - Monitor LLM usage costs
 * @implements UC0802 - Set and manage budgets
 * @implements FEAT0850 - Per-document cost tracking
 * @implements FEAT0853 - Token usage breakdown
 *
 * @enforces BR0801 - Costs tracked per operation
 * @enforces BR0804 - Budget limits enforceable
 *
 * @see {@link specs/WEBUI-007.md} for specification
 */

import type { IngestionStage } from "./ingestion";

// ============================================================================
// Token Usage Types
// ============================================================================

export interface TokenUsage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  embedding_tokens: number;
}

export interface OperationCost {
  operation: string;
  api_calls: number;
  input_tokens: number;
  output_tokens: number;
  cost_usd: number;
  model: string;
  duration_ms?: number;
  cached_calls?: number;
}

// ============================================================================
// Cost Breakdown Types
// ============================================================================

export interface CostBreakdown {
  extraction?: OperationCost;
  gleaning?: OperationCost;
  summarization?: OperationCost;
  embedding?: OperationCost;
  // Alternative flat format for breakdown display
  total_cost: number;
  by_stage?: Array<{
    stage: string;
    cost: number;
    tokens?: {
      input: number;
      output: number;
      total: number;
    };
    call_count?: number;
    cached_calls?: number;
  }>;
  tokens?: {
    input: number;
    output: number;
    total: number;
  };
}

export interface StageCostBreakdown {
  stage: IngestionStage | string;
  cost: number;
  cost_usd?: number;
  token_usage?: TokenUsage;
  tokens?: {
    input: number;
    output: number;
    total: number;
  };
  call_count?: number;
  model?: string;
  duration_ms?: number;
  cached_calls?: number;
}

// ============================================================================
// Document Cost Types
// ============================================================================

export interface DocumentCostBreakdown {
  document_id: string;
  document_name: string;
  total_cost_usd: number;
  token_usage: TokenUsage;
  stages: StageCostBreakdown[];
  estimated_cost_usd?: number;
  savings_from_cache_usd: number;
  ingested_at: string;
}

export interface IngestionCost {
  track_id: string;
  document_id: string;
  total_cost_usd: number;
  breakdown: CostBreakdown;
  token_usage: TokenUsage;
  calculated_at: string;
}

// ============================================================================
// Cost Summary Types
// ============================================================================

export interface PeriodCost {
  period: string; // ISO date
  cost_usd: number;
  documents: number;
}

export interface ModelCost {
  model: string;
  cost_usd: number;
  token_count: number;
  call_count: number;
}

export interface CostSummary {
  workspace_id?: string;
  // Flat format for components
  total_cost: number;
  document_count: number;
  total_tokens: number;
  average_cost_per_document: number;
  period_start?: string;
  period_end?: string;
  by_operation?: Array<{
    operation: string;
    cost: number;
    percentage: number;
    input_tokens?: number;
    output_tokens?: number;
    total_tokens?: number;
    call_count?: number;
  }>;
  by_model?: ModelCost[];
  budget?: BudgetInfo | null;
  // Legacy nested format
  period?: {
    start: string;
    end: string;
  };
  summary?: {
    total_cost_usd: number;
    total_documents: number;
    total_tokens: number;
    average_cost_per_document: number;
  };
  breakdown_by_operation?: Record<string, number>;
  breakdown_by_period?: PeriodCost[];
}

// ============================================================================
// Budget Types
// ============================================================================

export interface BudgetInfo {
  monthly_budget_usd: number;
  spent_usd: number;
  remaining_usd: number;
  alert_threshold: number; // Percentage (e.g., 80)
  is_over_budget: boolean;
}

export interface BudgetConfig {
  enabled: boolean;
  daily_limit_usd?: number;
  monthly_limit_usd?: number;
  alert_threshold_percent: number;
}

export interface BudgetStatus {
  current_usage_usd: number;
  limit_usd: number;
  percentage_used: number;
  period: "daily" | "monthly";
  reset_at: string;
  alert_triggered: boolean;
}

export interface BudgetAlert {
  id: string;
  type: "warning" | "critical" | "exceeded";
  message: string;
  percentage_used: number;
  created_at: string;
  acknowledged: boolean;
}

// ============================================================================
// WebSocket Cost Events
// ============================================================================

export interface CostUpdateEvent {
  type: "cost_update";
  track_id: string;
  stage: IngestionStage;
  operation: string;
  cost_usd: number;
  tokens_used?: {
    input: number;
    output: number;
  };
  cumulative_cost_usd: number;
}

// ============================================================================
// API Response Types
// ============================================================================

export interface CostSummaryResponse {
  workspace_id: string;
  period: { start: string; end: string };
  summary: {
    total_cost_usd: number;
    total_documents: number;
    total_tokens: number;
    average_cost_per_document: number;
  };
  breakdown_by_operation: Record<string, number>;
  breakdown_by_period: PeriodCost[];
  budget: BudgetInfo | null;
}

export interface DocumentCostResponse {
  document_id: string;
  document_name: string;
  total_cost_usd: number;
  token_usage: TokenUsage;
  stages: StageCostBreakdown[];
  ingested_at: string;
}
