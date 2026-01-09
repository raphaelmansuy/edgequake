/**
 * @module use-backend-store
 * @description Zustand store for backend connection and pipeline status.
 * Tracks health check results and processing pipeline state.
 *
 * @implements FEAT0611 - Backend health monitoring
 * @implements FEAT0620 - Pipeline status tracking
 * @implements FEAT0621 - Connection error handling
 *
 * @enforces BR0608 - Health updates periodically
 * @enforces BR0613 - Error state clears on successful check
 *
 * @see {@link docs/features.md} FEAT0611, FEAT0620
 */
"use client";

import type { HealthResponse, PipelineStatus } from "@/types";
import { create } from "zustand";

interface BackendState {
  // Health
  health: HealthResponse | null;
  isHealthy: boolean;
  lastHealthCheck: number | null;

  // Pipeline
  pipelineStatus: PipelineStatus | null;

  // Loading states
  isCheckingHealth: boolean;
  isLoadingPipeline: boolean;

  // Error
  error: string | null;
}

interface BackendActions {
  // Health
  setHealth: (health: HealthResponse) => void;
  setHealthCheckFailed: () => void;
  setIsCheckingHealth: (checking: boolean) => void;

  // Pipeline
  setPipelineStatus: (status: PipelineStatus) => void;
  setIsLoadingPipeline: (loading: boolean) => void;

  // Error
  setError: (error: string | null) => void;

  // Reset
  reset: () => void;
}

type BackendStore = BackendState & BackendActions;

const initialState: BackendState = {
  health: null,
  isHealthy: false,
  lastHealthCheck: null,
  pipelineStatus: null,
  isCheckingHealth: false,
  isLoadingPipeline: false,
  error: null,
};

export const useBackendStore = create<BackendStore>()((set) => ({
  ...initialState,

  // Health
  setHealth: (health) =>
    set({
      health,
      isHealthy: health.status === "healthy",
      lastHealthCheck: Date.now(),
      isCheckingHealth: false,
      error: null,
    }),

  setHealthCheckFailed: () =>
    set({
      isHealthy: false,
      lastHealthCheck: Date.now(),
      isCheckingHealth: false,
    }),

  setIsCheckingHealth: (checking) => set({ isCheckingHealth: checking }),

  // Pipeline
  setPipelineStatus: (status) =>
    set({
      pipelineStatus: status,
      isLoadingPipeline: false,
    }),

  setIsLoadingPipeline: (loading) => set({ isLoadingPipeline: loading }),

  // Error
  setError: (error) => set({ error }),

  // Reset
  reset: () => set(initialState),
}));

// Selectors
export const useIsProcessing = () => {
  const { pipelineStatus } = useBackendStore();
  return pipelineStatus && pipelineStatus.running_tasks > 0;
};

export const useHasPendingTasks = () => {
  const { pipelineStatus } = useBackendStore();
  return (
    pipelineStatus &&
    (pipelineStatus.running_tasks > 0 || pipelineStatus.queued_tasks > 0)
  );
};

export default useBackendStore;
