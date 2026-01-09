/**
 * @module use-cost-store
 * @description Zustand store for LLM cost tracking and budget management.
 * Based on WebUI Specification Document WEBUI-007 (16-webui-cost-monitoring.md)
 *
 * @implements UC0801 - User monitors LLM usage costs
 * @implements UC0802 - User sets budget alerts
 * @implements FEAT0850 - Per-document cost tracking
 * @implements FEAT0851 - Real-time ingestion cost updates
 * @implements FEAT0852 - Workspace cost summary
 *
 * @enforces BR0801 - Costs update in real-time during ingestion
 * @enforces BR0802 - Budget alerts trigger at thresholds
 * @enforces BR0803 - Cost breakdown available per document
 *
 * @see {@link specs/WEBUI-007.md} for specification
 */

import { STORE_VERSIONS, ZUSTAND_STORAGE_KEYS } from "@/lib/storage-keys";
import type {
  BudgetAlert,
  BudgetStatus,
  CostSummary,
  DocumentCostBreakdown,
} from "@/types/cost";
import { create } from "zustand";
import { devtools, persist } from "zustand/middleware";

// ============================================================================
// Store Types
// ============================================================================

interface CostState {
  // Real-time tracking during ingestion
  activeIngestionCosts: Map<string, number>; // trackId -> cumulative cost

  // Document costs
  documentCosts: Map<string, DocumentCostBreakdown>;

  // Workspace summary
  workspaceSummary: CostSummary | null;

  // Budget status
  budgetStatus: BudgetStatus | null;

  // Budget alerts
  budgetAlerts: BudgetAlert[];

  // Last refresh timestamp
  lastRefresh: string | null;
}

interface CostActions {
  // Ingestion cost tracking
  updateIngestionCost: (trackId: string, cost: number) => void;
  clearIngestionCost: (trackId: string) => void;
  clearAllIngestionCosts: () => void;

  // Document costs
  setDocumentCost: (documentId: string, cost: DocumentCostBreakdown) => void;
  clearDocumentCost: (documentId: string) => void;

  // Workspace summary
  setWorkspaceSummary: (summary: CostSummary) => void;
  clearWorkspaceSummary: () => void;

  // Budget status
  setBudgetStatus: (status: BudgetStatus) => void;
  clearBudgetStatus: () => void;

  // Budget alerts
  addBudgetAlert: (alert: BudgetAlert) => void;
  acknowledgeBudgetAlert: (alertId: string) => void;
  clearBudgetAlerts: () => void;

  // Getters
  getIngestionCost: (trackId: string) => number;
  getTotalActiveCost: () => number;
  getDocumentCost: (documentId: string) => DocumentCostBreakdown | undefined;
  getUnacknowledgedAlerts: () => BudgetAlert[];
}

type CostStore = CostState & CostActions;

// ============================================================================
// Store Definition
// ============================================================================

export const useCostStore = create<CostStore>()(
  devtools(
    persist(
      (set, get) => ({
        // Initial state
        activeIngestionCosts: new Map(),
        documentCosts: new Map(),
        workspaceSummary: null,
        budgetStatus: null,
        budgetAlerts: [],
        lastRefresh: null,

        // Ingestion cost tracking
        updateIngestionCost: (trackId, cost) => {
          set((state) => {
            const activeIngestionCosts = new Map(state.activeIngestionCosts);
            activeIngestionCosts.set(trackId, cost);
            return { activeIngestionCosts };
          });
        },

        clearIngestionCost: (trackId) => {
          set((state) => {
            const activeIngestionCosts = new Map(state.activeIngestionCosts);
            activeIngestionCosts.delete(trackId);
            return { activeIngestionCosts };
          });
        },

        clearAllIngestionCosts: () => {
          set({ activeIngestionCosts: new Map() });
        },

        // Document costs
        setDocumentCost: (documentId, cost) => {
          set((state) => {
            const documentCosts = new Map(state.documentCosts);
            documentCosts.set(documentId, cost);
            return { documentCosts };
          });
        },

        clearDocumentCost: (documentId) => {
          set((state) => {
            const documentCosts = new Map(state.documentCosts);
            documentCosts.delete(documentId);
            return { documentCosts };
          });
        },

        // Workspace summary
        setWorkspaceSummary: (summary) => {
          set({
            workspaceSummary: summary,
            lastRefresh: new Date().toISOString(),
          });
        },

        clearWorkspaceSummary: () => {
          set({ workspaceSummary: null });
        },

        // Budget status
        setBudgetStatus: (status) => {
          const prevStatus = get().budgetStatus;
          set({ budgetStatus: status });

          // Check if we need to create an alert
          if (
            status.alert_triggered &&
            (!prevStatus || !prevStatus.alert_triggered)
          ) {
            const alertType =
              status.percentage_used >= 100
                ? "exceeded"
                : status.percentage_used >= 90
                ? "critical"
                : "warning";

            get().addBudgetAlert({
              id: `budget-alert-${Date.now()}`,
              type: alertType,
              message:
                alertType === "exceeded"
                  ? `Budget exceeded! Currently at ${status.percentage_used.toFixed(
                      1
                    )}% of ${status.period} limit.`
                  : `Budget ${alertType}: ${status.percentage_used.toFixed(
                      1
                    )}% of ${status.period} limit used.`,
              percentage_used: status.percentage_used,
              created_at: new Date().toISOString(),
              acknowledged: false,
            });
          }
        },

        clearBudgetStatus: () => {
          set({ budgetStatus: null });
        },

        // Budget alerts
        addBudgetAlert: (alert) => {
          set((state) => ({
            budgetAlerts: [alert, ...state.budgetAlerts.slice(0, 49)], // Keep last 50 alerts
          }));
        },

        acknowledgeBudgetAlert: (alertId) => {
          set((state) => ({
            budgetAlerts: state.budgetAlerts.map((alert) =>
              alert.id === alertId ? { ...alert, acknowledged: true } : alert
            ),
          }));
        },

        clearBudgetAlerts: () => {
          set({ budgetAlerts: [] });
        },

        // Getters
        getIngestionCost: (trackId) => {
          return get().activeIngestionCosts.get(trackId) ?? 0;
        },

        getTotalActiveCost: () => {
          let total = 0;
          get().activeIngestionCosts.forEach((cost) => {
            total += cost;
          });
          return total;
        },

        getDocumentCost: (documentId) => {
          return get().documentCosts.get(documentId);
        },

        getUnacknowledgedAlerts: () => {
          return get().budgetAlerts.filter((alert) => !alert.acknowledged);
        },
      }),
      {
        name: ZUSTAND_STORAGE_KEYS.COST_STORE,
        version: STORE_VERSIONS[ZUSTAND_STORAGE_KEYS.COST_STORE],
        // Only persist certain fields (Map types are NOT persisted as they don't serialize)
        partialize: (state) => ({
          budgetAlerts: state.budgetAlerts,
        }),
      }
    ),
    { name: "cost-store" }
  )
);

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Format cost as USD string with appropriate precision.
 */
export function formatCost(usd: number): string {
  if (usd < 0.01) {
    return `$${usd.toFixed(4)}`;
  } else if (usd < 1) {
    return `$${usd.toFixed(3)}`;
  } else {
    return `$${usd.toFixed(2)}`;
  }
}

/**
 * Format token count with K/M suffix.
 */
export function formatTokenCount(tokens: number): string {
  if (tokens >= 1000000) {
    return `${(tokens / 1000000).toFixed(1)}M`;
  } else if (tokens >= 1000) {
    return `${(tokens / 1000).toFixed(1)}K`;
  } else {
    return tokens.toString();
  }
}
