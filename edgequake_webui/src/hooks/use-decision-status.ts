'use client';

import type { DecisionModels, DecisionStatus } from '@/constants/extraction-mode';
import { getDecisionModels, getDecisionStatus } from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import { keepPreviousData, useQuery } from '@tanstack/react-query';

/** Short enough that "I just pulled the model" shows up without a reload. */
const DECISION_STATUS_STALE_MS = 15_000;

/**
 * SPEC-160 — can this workspace run decision extraction now?
 *
 * The cache key includes the workspace id, so switching workspace never shows
 * another workspace's activation. One extra key per model tag, so the card
 * (its own model) and the upload select (server default) do not overwrite.
 */
export function useDecisionStatus(
  model?: string | null,
  options?: { enabled?: boolean },
) {
  const workspaceId = useTenantStore((s) => s.selectedWorkspaceId);
  const tag = model?.trim() || null;
  return useQuery<DecisionStatus>({
    queryKey: ['decision', 'status', workspaceId, tag],
    queryFn: () => getDecisionStatus(tag),
    enabled: options?.enabled ?? true,
    staleTime: DECISION_STATUS_STALE_MS,
    refetchOnWindowFocus: true,
    placeholderData: keepPreviousData,
    retry: 1,
  });
}

export function useDecisionModels(options?: { enabled?: boolean }) {
  const workspaceId = useTenantStore((s) => s.selectedWorkspaceId);
  return useQuery<DecisionModels>({
    queryKey: ['decision', 'models', workspaceId],
    queryFn: () => getDecisionModels(),
    enabled: options?.enabled ?? true,
    staleTime: DECISION_STATUS_STALE_MS,
    refetchOnWindowFocus: true,
    placeholderData: keepPreviousData,
    retry: 1,
  });
}
