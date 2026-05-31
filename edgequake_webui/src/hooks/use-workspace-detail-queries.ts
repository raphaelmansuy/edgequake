"use client";

import {
  getWorkspace,
  getWorkspaceStats,
} from "@/lib/api/edgequake";
import { fetchProvidersHealth } from "@/lib/api/models";
import { useQuery } from "@tanstack/react-query";

export interface UseWorkspaceDetailQueriesOptions {
  /** When false, all queries are disabled. */
  enabled?: boolean;
}

/**
 * Shared workspace detail data fetching for dashboard and deeplink routes.
 * @implements SPEC-017 UI-P3-001
 */
export function useWorkspaceDetailQueries(
  tenantId: string | null | undefined,
  workspaceId: string | null | undefined,
  options: UseWorkspaceDetailQueriesOptions = {},
) {
  const enabled =
    options.enabled !== false && !!tenantId && !!workspaceId;

  const workspaceQuery = useQuery({
    queryKey: ["workspace", tenantId, workspaceId],
    queryFn: () => getWorkspace(tenantId!, workspaceId!),
    enabled,
    staleTime: 30_000,
  });

  const statsQuery = useQuery({
    queryKey: ["workspaceStats", workspaceId],
    queryFn: () => getWorkspaceStats(workspaceId!),
    enabled: enabled && !!workspaceId,
    staleTime: 0,
    refetchOnMount: "always",
  });

  const providerHealthQuery = useQuery({
    queryKey: ["providersHealth"],
    queryFn: fetchProvidersHealth,
    enabled,
    staleTime: 60_000,
    retry: 1,
  });

  return {
    workspace: workspaceQuery.data,
    stats: statsQuery.data,
    providerHealth: providerHealthQuery.data,
    isLoadingWorkspace: workspaceQuery.isLoading,
    isLoadingStats: statsQuery.isLoading,
    isLoadingHealth: providerHealthQuery.isLoading,
    refetchWorkspace: workspaceQuery.refetch,
  };
}
