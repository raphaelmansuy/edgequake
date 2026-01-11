/**
 * @module useProviders
 * @description React hooks for fetching and managing LLM/embedding providers.
 * 
 * @implements SPEC-032: Ollama/LM Studio provider support - WebUI hooks
 * @iteration OODA #17 - Provider selector implementation
 */
'use client';

import { SERVER_BASE_URL } from '@/lib/api/client';
import type { AvailableProvidersResponse, ProviderStatusResponse } from '@/types/provider';
import { useQuery } from '@tanstack/react-query';

const getApiUrl = () => SERVER_BASE_URL || 'http://localhost:8080';

/**
 * Fetch current provider status.
 */
async function fetchProviderStatus(): Promise<ProviderStatusResponse> {
  const response = await fetch(`${getApiUrl()}/api/v1/settings/provider/status`);
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
  }
  return response.json();
}

/**
 * Fetch available providers.
 */
async function fetchAvailableProviders(): Promise<AvailableProvidersResponse> {
  const response = await fetch(`${getApiUrl()}/api/v1/settings/providers`);
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
  }
  return response.json();
}

/**
 * Hook to get current provider status with auto-refresh.
 */
export function useProviderStatus(refreshInterval = 30000) {
  return useQuery({
    queryKey: ['provider-status'],
    queryFn: fetchProviderStatus,
    refetchInterval: refreshInterval,
    staleTime: 10000,
  });
}

/**
 * Hook to get available providers.
 */
export function useAvailableProviders() {
  return useQuery({
    queryKey: ['available-providers'],
    queryFn: fetchAvailableProviders,
    staleTime: 60000, // Cache for 1 minute
  });
}

/**
 * Get display name for a provider.
 */
export function getProviderDisplayName(providerId: string): string {
  const names: Record<string, string> = {
    openai: 'OpenAI',
    ollama: 'Ollama',
    lmstudio: 'LM Studio',
    mock: 'Mock (Dev)',
  };
  return names[providerId.toLowerCase()] || providerId;
}

/**
 * Get provider icon class based on provider ID.
 */
export function getProviderIconClass(providerId: string): string {
  const icons: Record<string, string> = {
    openai: 'text-green-600',
    ollama: 'text-blue-600',
    lmstudio: 'text-purple-600',
    mock: 'text-gray-500',
  };
  return icons[providerId.toLowerCase()] || 'text-gray-500';
}
