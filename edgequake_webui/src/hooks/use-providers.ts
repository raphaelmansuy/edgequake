/**
 * @module useProviders
 * @description React hooks for fetching and managing LLM/embedding providers.
 *
 * @implements SPEC-032: Ollama/LM Studio provider support - WebUI hooks
 * @iteration OODA #17 - Provider selector implementation
 * @iteration OODA #54 - Multi-model support per provider
 */
"use client";

import { isUiVisibleProviderId } from "@/lib/provider-visibility";
import { getProviderDisplayName as getCentralProviderDisplayName } from "@/lib/provider-display";
import { getProviderIconColorClass } from "@/components/providers/provider-icon";
import { apiClient } from "@/lib/api/client";
import type {
    EmbeddingModelsResponse,
    LlmModelsResponse,
} from "@/lib/api/models";
import { refreshModelDiscovery } from "@/lib/api/models";
import { getAutomationAwareRefetchInterval } from "@/lib/runtime/browser-detection";
import type {
    AvailableProvidersResponse,
    ProviderStatusResponse,
} from "@/types/provider";
import { useQuery } from "@tanstack/react-query";

/**
 * Fetch current provider status.
 */
async function fetchProviderStatus(): Promise<ProviderStatusResponse> {
  return apiClient<ProviderStatusResponse>("/settings/provider/status");
}

/**
 * Fetch available providers.
 */
async function fetchAvailableProviders(): Promise<AvailableProvidersResponse> {
  return apiClient<AvailableProvidersResponse>("/settings/providers");
}

/**
 * Fetch LLM models from all providers.
 * @implements SPEC-032: Multi-model support per provider (Focus 7)
 */
async function fetchLlmModels(): Promise<LlmModelsResponse> {
  const response = await apiClient<LlmModelsResponse>("/models/llm");
  return {
    ...response,
    models: response.models.filter((m) => isUiVisibleProviderId(m.provider)),
  };
}

/**
 * Fetch embedding models from all providers.
 * @implements SPEC-032: Multi-model support per provider (Focus 7)
 */
async function fetchEmbeddingModels(): Promise<EmbeddingModelsResponse> {
  const response = await apiClient<EmbeddingModelsResponse>("/models/embedding");
  return {
    ...response,
    models: response.models.filter((m) => isUiVisibleProviderId(m.provider)),
  };
}

/**
 * Hook to get current provider status with auto-refresh.
 */
export function useProviderStatus(refreshInterval = 30000) {
  return useQuery({
    queryKey: ["provider-status"],
    queryFn: fetchProviderStatus,
    refetchInterval: getAutomationAwareRefetchInterval(refreshInterval),
    staleTime: 10000,
  });
}

/**
 * Hook to get available providers.
 */
export function useAvailableProviders() {
  return useQuery({
    queryKey: ["available-providers"],
    queryFn: fetchAvailableProviders,
    staleTime: 60000, // Cache for 1 minute
  });
}

/**
 * Hook to get all LLM models across all providers.
 * @implements SPEC-032: Multi-model support per provider (Focus 7)
 */
export function useLlmModels() {
  return useQuery({
    queryKey: ["llm-models"],
    queryFn: fetchLlmModels,
    staleTime: 30_000,
  });
}

/**
 * Hook to get all embedding models across all providers.
 * @implements SPEC-032: Multi-model support per provider (Focus 7)
 */
export function useEmbeddingModels() {
  return useQuery({
    queryKey: ["embedding-models"],
    queryFn: fetchEmbeddingModels,
    staleTime: 30_000,
  });
}

/** Invalidate discovery cache on backend and refetch model catalogs. */
export async function refreshDynamicModels(queryClient: {
  invalidateQueries: (opts: { queryKey: string[] }) => void;
}) {
  await refreshModelDiscovery();
  queryClient.invalidateQueries({ queryKey: ["llm-models"] });
  queryClient.invalidateQueries({ queryKey: ["embedding-models"] });
  queryClient.invalidateQueries({ queryKey: ["provider-health"] });
}

/**
 * Get display name for a provider.
 */
export function getProviderDisplayName(providerId: string): string {
  return getCentralProviderDisplayName(providerId);
}

/**
 * Get provider icon class based on provider ID.
 * @deprecated Prefer `getProviderIconColorClass` from `@/components/providers/provider-icon`.
 */
export function getProviderIconClass(providerId: string): string {
  return getProviderIconColorClass(providerId);
}
