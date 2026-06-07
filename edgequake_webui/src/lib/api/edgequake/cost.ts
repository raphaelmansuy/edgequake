/**
 * Domain API module — split from edgequake.ts (SPEC-017 UI-DRY-001).
 */

import { api } from "../client";

/**
 * Get cost summary for the current workspace.
 */
export async function getWorkspaceCostSummary(): Promise<
  import("@/types/cost").CostSummary
> {
  return api.get<import("@/types/cost").CostSummary>("/costs/summary");
}

/**
 * Get detailed cost breakdown for a specific document.
 */
export async function getDocumentCost(
  documentId: string,
): Promise<import("@/types/cost").CostBreakdown> {
  return api.get<import("@/types/cost").CostBreakdown>(
    `/documents/${documentId}/cost`,
  );
}

/**
 * Get cost breakdown for a specific ingestion track.
 */
export async function getIngestionCost(
  trackId: string,
): Promise<import("@/types/cost").CostBreakdown> {
  return api.get<import("@/types/cost").CostBreakdown>(
    `/ingestion/${trackId}/cost`,
  );
}

/**
 * Get budget status and limits.
 */
export async function getBudgetStatus(): Promise<
  import("@/types/cost").BudgetInfo
> {
  return api.get<import("@/types/cost").BudgetInfo>("/costs/budget");
}

/**
 * Update budget limits.
 */
export async function updateBudget(
  budget: Partial<import("@/types/cost").BudgetInfo>,
): Promise<import("@/types/cost").BudgetInfo> {
  return api.patch<import("@/types/cost").BudgetInfo>("/costs/budget", budget);
}

/**
 * Get cost history for a time period.
 */
export interface CostHistoryParams {
  start_date?: string;
  end_date?: string;
  granularity?: "hour" | "day" | "week" | "month";
}

export interface CostHistoryPoint {
  timestamp: string;
  total_cost: number;
  total_tokens: number;
  document_count: number;
}

export async function getCostHistory(
  params?: CostHistoryParams,
): Promise<CostHistoryPoint[]> {
  const searchParams = new URLSearchParams();
  if (params?.start_date) searchParams.set("start_date", params.start_date);
  if (params?.end_date) searchParams.set("end_date", params.end_date);
  if (params?.granularity) searchParams.set("granularity", params.granularity);

  const query = searchParams.toString();
  return api.get<CostHistoryPoint[]>(
    `/costs/history${query ? `?${query}` : ""}`,
  );
}
