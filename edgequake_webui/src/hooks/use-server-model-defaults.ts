'use client';

import { fetchModelsConfig } from '@/lib/api/models';
import { useQuery } from '@tanstack/react-query';

/**
 * Server-configured default models from /api/v1/models (env / models.toml).
 * @implements SPEC-013 / GitHub #233
 */
export function useServerModelDefaults() {
  const { data, isLoading } = useQuery({
    queryKey: ['models', 'defaults'],
    queryFn: fetchModelsConfig,
    staleTime: 5 * 60_000,
  });

  const hasConfiguredDefaults = Boolean(
    data?.default_llm_model &&
      data?.default_embedding_model &&
      data.default_llm_model.length > 0 &&
      data.default_embedding_model.length > 0
  );

  return {
    isLoading,
    hasConfiguredDefaults,
    defaultLlmProvider: data?.default_llm_provider,
    defaultLlmModel: data?.default_llm_model,
    defaultEmbeddingProvider: data?.default_embedding_provider,
    defaultEmbeddingModel: data?.default_embedding_model,
  };
}
