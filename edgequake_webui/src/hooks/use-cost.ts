/**
 * @module use-cost
 * @description React Query hooks for cost data fetching.
 * Based on WebUI Specification Document WEBUI-007 (16-webui-cost-monitoring.md)
 *
 * @implements UC0801 - User monitors LLM usage costs
 * @implements UC0802 - User sets budget alerts
 * @implements FEAT0850 - Per-document cost tracking
 * @implements FEAT0852 - Workspace cost summary
 * @implements FEAT0610 - Cost history timeline
 *
 * @enforces BR0801 - Costs update in real-time
 * @enforces BR0802 - Budget alerts trigger at thresholds
 *
 * @see {@link specs/WEBUI-007.md} for specification
 */

import {
  getBudgetStatus,
  getCostHistory,
  getDocumentCost,
  getIngestionCost,
  getWorkspaceCostSummary,
  updateBudget,
  type CostHistoryParams,
} from "@/lib/api/edgequake";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

/**
 * Query keys for cost data.
 */
export const costKeys = {
  all: ["cost"] as const,
  summary: () => [...costKeys.all, "summary"] as const,
  document: (documentId: string) =>
    [...costKeys.all, "document", documentId] as const,
  ingestion: (trackId: string) =>
    [...costKeys.all, "ingestion", trackId] as const,
  budget: () => [...costKeys.all, "budget"] as const,
  history: (params?: CostHistoryParams) =>
    [...costKeys.all, "history", params] as const,
};

/**
 * Hook to fetch workspace cost summary.
 */
export function useWorkspaceCostSummary() {
  return useQuery({
    queryKey: costKeys.summary(),
    queryFn: getWorkspaceCostSummary,
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}

/**
 * Hook to fetch document cost.
 */
export function useDocumentCost(documentId: string | null) {
  return useQuery({
    queryKey: costKeys.document(documentId ?? ""),
    queryFn: () => getDocumentCost(documentId!),
    enabled: !!documentId,
    staleTime: 10 * 60 * 1000, // 10 minutes
  });
}

/**
 * Hook to fetch ingestion cost.
 */
export function useIngestionCost(trackId: string | null) {
  return useQuery({
    queryKey: costKeys.ingestion(trackId ?? ""),
    queryFn: () => getIngestionCost(trackId!),
    enabled: !!trackId,
    staleTime: 30 * 1000, // 30 seconds - updates frequently during ingestion
  });
}

/**
 * Hook to fetch budget status.
 */
export function useBudgetStatus() {
  return useQuery({
    queryKey: costKeys.budget(),
    queryFn: getBudgetStatus,
    staleTime: 60 * 1000, // 1 minute
  });
}

/**
 * Hook to update budget.
 */
export function useUpdateBudget() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: updateBudget,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: costKeys.budget() });
    },
  });
}

/**
 * Hook to fetch cost history.
 */
export function useCostHistory(params?: CostHistoryParams) {
  return useQuery({
    queryKey: costKeys.history(params),
    queryFn: () => getCostHistory(params),
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}
