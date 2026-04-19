/**
 * @module useProviders
 * @description React hooks for fetching and managing LLM/embedding providers.
 *
 * @implements SPEC-032: Ollama/LM Studio provider support - WebUI hooks
 * @iteration OODA #17 - Provider selector implementation
 * @iteration OODA #54 - Multi-model support per provider
 */
"use client";

import { apiClient } from "@/lib/api/client";
import type {
    EmbeddingModelsResponse,
    LlmModelsResponse,
} from "@/lib/api/models";
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
  return apiClient<LlmModelsResponse>("/models/llm");
}

/**
 * Fetch embedding models from all providers.
 * @implements SPEC-032: Multi-model support per provider (Focus 7)
 */
async function fetchEmbeddingModels(): Promise<EmbeddingModelsResponse> {
  return apiClient<EmbeddingModelsResponse>("/models/embedding");
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
    staleTime: 60000, // Cache for 1 minute
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
    staleTime: 60000, // Cache for 1 minute
  });
}

/**
 * Get display name for a provider.
 */
export function getProviderDisplayName(providerId: string): string {
  const names: Record<string, string> = {
    openai: "OpenAI",
    ollama: "Ollama",
    lmstudio: "LM Studio",
    anthropic: "Anthropic",
    gemini: "Google Gemini",
    xai: "xAI",
    openrouter: "OpenRouter",
    azure: "Azure OpenAI",
    minimax: "MiniMax",
    mock: "Mock (Dev)",
  };
  return names[providerId.toLowerCase()] || providerId;
}

/**
 * Get provider icon class based on provider ID.
 */
export function getProviderIconClass(providerId: string): string {
  const icons: Record<string, string> = {
    openai: "text-green-600",
    ollama: "text-blue-600",
    lmstudio: "text-purple-600",
    anthropic: "text-orange-600",
    gemini: "text-blue-500",
    xai: "text-slate-700",
    openrouter: "text-indigo-600",
    azure: "text-sky-600",
    minimax: "text-teal-600",
    mock: "text-gray-500",
  };
  return icons[providerId.toLowerCase()] || "text-gray-500";
}
