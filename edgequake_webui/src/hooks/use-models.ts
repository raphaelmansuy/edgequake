/**
 * useModels Hook
 *
 * @implements FEAT0470: Models Configuration API
 * @implements FEAT0471: Provider Capability Exposure
 *
 * React hook for fetching and managing model configuration.
 * Provides access to:
 * - All providers and their models
 * - LLM models with capabilities
 * - Embedding models with dimensions
 * - Provider health status
 */

import { useQuery } from "@tanstack/react-query";
import {
  EmbeddingModelsResponse,
  fetchEmbeddingModels,
  fetchLlmModels,
  fetchModelsConfig,
  fetchProvidersHealth,
  LlmModelsResponse,
  ModelsListResponse,
  ProviderResponse,
} from "../lib/api/models";

/**
 * Query keys for models-related queries.
 */
export const modelsQueryKeys = {
  all: ["models"] as const,
  config: ["models", "config"] as const,
  llm: ["models", "llm"] as const,
  embedding: ["models", "embedding"] as const,
  health: ["models", "health"] as const,
  provider: (name: string) => ["models", "provider", name] as const,
  model: (provider: string, model: string) =>
    ["models", provider, model] as const,
};

/**
 * Hook to fetch all models configuration.
 *
 * @returns Query result with all providers and models
 */
export function useModelsConfig() {
  return useQuery<ModelsListResponse>({
    queryKey: modelsQueryKeys.config,
    queryFn: fetchModelsConfig,
    staleTime: 5 * 60 * 1000, // 5 minutes
    refetchOnWindowFocus: false,
  });
}

/**
 * Hook to fetch LLM models only.
 *
 * @returns Query result with LLM models
 */
export function useLlmModels() {
  return useQuery<LlmModelsResponse>({
    queryKey: modelsQueryKeys.llm,
    queryFn: fetchLlmModels,
    staleTime: 5 * 60 * 1000,
    refetchOnWindowFocus: false,
  });
}

/**
 * Hook to fetch embedding models only.
 *
 * @returns Query result with embedding models
 */
export function useEmbeddingModels() {
  return useQuery<EmbeddingModelsResponse>({
    queryKey: modelsQueryKeys.embedding,
    queryFn: fetchEmbeddingModels,
    staleTime: 5 * 60 * 1000,
    refetchOnWindowFocus: false,
  });
}

/**
 * Hook to fetch provider health status.
 *
 * @param options - Optional query options
 * @returns Query result with provider health
 */
export function useProvidersHealth(options?: {
  enabled?: boolean;
  refetchInterval?: number;
}) {
  return useQuery<ProviderResponse[]>({
    queryKey: modelsQueryKeys.health,
    queryFn: fetchProvidersHealth,
    staleTime: 30 * 1000, // 30 seconds
    refetchInterval: options?.refetchInterval ?? 60 * 1000, // 1 minute
    enabled: options?.enabled ?? true,
    refetchOnWindowFocus: true,
  });
}

/**
 * Hook to get available LLM options for a dropdown.
 *
 * @returns Grouped LLM options by provider
 */
export function useLlmOptions() {
  const { data, isLoading, error } = useLlmModels();

  if (!data) {
    return {
      options: [],
      defaultProvider: "",
      defaultModel: "",
      isLoading,
      error,
    };
  }

  // Group models by provider
  const groupedOptions = data.models.reduce((acc, model) => {
    const group = acc.find((g) => g.provider === model.provider);
    if (group) {
      group.models.push(model);
    } else {
      acc.push({
        provider: model.provider,
        displayName: model.provider_display_name,
        models: [model],
      });
    }
    return acc;
  }, [] as Array<{ provider: string; displayName: string; models: typeof data.models }>);

  return {
    options: groupedOptions,
    defaultProvider: data.default_provider,
    defaultModel: data.default_model,
    isLoading,
    error,
  };
}

/**
 * Hook to get available embedding options for a dropdown.
 *
 * @returns Grouped embedding options by provider
 */
export function useEmbeddingOptions() {
  const { data, isLoading, error } = useEmbeddingModels();

  if (!data) {
    return {
      options: [],
      defaultProvider: "",
      defaultModel: "",
      isLoading,
      error,
    };
  }

  // Group models by provider
  const groupedOptions = data.models.reduce((acc, model) => {
    const group = acc.find((g) => g.provider === model.provider);
    if (group) {
      group.models.push(model);
    } else {
      acc.push({
        provider: model.provider,
        displayName: model.provider_display_name,
        models: [model],
      });
    }
    return acc;
  }, [] as Array<{ provider: string; displayName: string; models: typeof data.models }>);

  return {
    options: groupedOptions,
    defaultProvider: data.default_provider,
    defaultModel: data.default_model,
    isLoading,
    error,
  };
}
