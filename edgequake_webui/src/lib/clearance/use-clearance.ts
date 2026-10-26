"use client";

import { useQuery, type QueryClient } from "@tanstack/react-query";
import { clearanceApi } from "./api";

export function clearanceQueryKey(workspaceId: string | null | undefined) {
  return ["clearance", workspaceId] as const;
}

export function clearanceTagsKey(workspaceId: string | null | undefined) {
  return ["clearance-tags", workspaceId] as const;
}

export function clearanceRolesKey(workspaceId: string | null | undefined) {
  return ["clearance-roles", workspaceId] as const;
}

export function memberRolesKey(workspaceId: string | null | undefined, userId: string) {
  return ["clearance-member-roles", workspaceId, userId] as const;
}

export function useClearanceSettings(workspaceId: string | null | undefined) {
  return useQuery({
    queryKey: clearanceQueryKey(workspaceId),
    queryFn: () => clearanceApi.settings(workspaceId!),
    enabled: Boolean(workspaceId),
    retry: false,
  });
}

/** Vocabulary of departments and projects. Only for owners and admins. */
export function useClearanceTags(workspaceId: string | null | undefined, enabled: boolean) {
  return useQuery({
    queryKey: clearanceTagsKey(workspaceId),
    queryFn: () => clearanceApi.tags(workspaceId!),
    enabled: Boolean(workspaceId) && enabled,
    retry: false,
  });
}

/** Access profiles with their saved rules. Only for owners and admins. */
export function useClearanceRoles(workspaceId: string | null | undefined, enabled: boolean) {
  return useQuery({
    queryKey: clearanceRolesKey(workspaceId),
    queryFn: () => clearanceApi.roles(workspaceId!),
    enabled: Boolean(workspaceId) && enabled,
    retry: false,
  });
}

/**
 * Refresh every view that a label or profile change touches: settings, vocabulary,
 * profiles, the library, graph, and stats. Documents are refetched so a relabel shows
 * up in the list at once.
 */
export async function invalidateClearanceViews(queryClient: QueryClient, workspaceId: string) {
  const sensitive = ['document', 'graph', 'query-context', 'retrieval-context', 'query', 'chat', 'access-member-search', 'access-member-check'];
  await Promise.all(sensitive.map(key => queryClient.cancelQueries({ queryKey: [key] })));
  for (const key of sensitive) queryClient.removeQueries({ queryKey: [key] });
  await Promise.all([
    ...['document-access', 'document-access-options', 'document-access-tags', 'bulk-document-access', 'document-access-preview', 'bulk-access-preview', 'upload-access-preview'].map(key => queryClient.invalidateQueries({ queryKey: [key] })),
    queryClient.invalidateQueries({ queryKey: clearanceQueryKey(workspaceId) }),
    queryClient.invalidateQueries({ queryKey: clearanceTagsKey(workspaceId) }),
    queryClient.invalidateQueries({ queryKey: clearanceRolesKey(workspaceId) }),
    queryClient.invalidateQueries({ queryKey: ["clearance-member-roles", workspaceId] }),
    queryClient.invalidateQueries({ queryKey: ["documents"] }),
    queryClient.invalidateQueries({ queryKey: ["document"] }),
    queryClient.invalidateQueries({ queryKey: ["graph"] }),
    queryClient.invalidateQueries({ queryKey: ["workspace-stats"] }),
  ]);
}
