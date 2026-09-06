'use client';

/**
 * SPEC-146: whether document ABAC is enabled (from /health capabilities).
 */

import { checkHealth } from '@/lib/api/edgequake/health';
import { useQuery } from '@tanstack/react-query';

export function useDocAbacEnabled() {
  const query = useQuery({
    queryKey: ['health', 'doc_abac'],
    queryFn: async () => {
      const health = await checkHealth();
      return Boolean(health.capabilities?.doc_abac);
    },
    staleTime: 30_000,
    refetchOnWindowFocus: false,
  });

  return {
    docAbacEnabled: query.data === true,
    isLoading: query.isLoading,
    refetch: query.refetch,
  };
}
