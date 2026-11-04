'use client';

/**
 * @module useEntityTypeColors
 * @description Workspace-scoped entity-type color overrides (SPEC-102).
 *
 * Reads from the selected workspace in the tenant store and exposes a stable
 * map for `resolveEntityTypeColor`. Mutations update the workspace via API and
 * patch the local store so graph surfaces recolor without a full reload.
 *
 * @implements SPEC-102 LAW-102-2 / LAW-102-6
 */

import { updateWorkspace } from '@/lib/api/edgequake';
import {
  mergeEntityTypeColorMap,
  resolveEntityTypeColor,
  stripDefaultOverrides,
} from '@/lib/graph/entity-type-colors';
import {
  useSelectedWorkspace,
  useTenantStore,
} from '@/stores/use-tenant-store';
import { useCallback, useMemo } from 'react';
import { toast } from 'sonner';

export function useEntityTypeColors() {
  const workspace = useSelectedWorkspace();
  const selectedTenantId = useTenantStore((s) => s.selectedTenantId);
  const setWorkspaces = useTenantStore((s) => s.setWorkspaces);

  // WHY the intermediate binding: `useMemo` over `workspace?.entity_type_colors` reads an optional
  // member expression as its dependency, which the React Compiler cannot prove stable, so it skips
  // compiling the whole hook ("Existing memoization could not be preserved"). Naming the value
  // first gives the dependency a plain identifier and the hook compiles again.
  const workspaceColors = workspace?.entity_type_colors;
  const colors = useMemo(
    () => mergeEntityTypeColorMap(workspaceColors),
    [workspaceColors],
  );

  const colorFor = useCallback(
    (entityType: string | undefined) => resolveEntityTypeColor(entityType, colors),
    [colors],
  );

  // Same reason as `workspaceColors` above: a dependency on an optional chain (`workspace?.id`)
  // prevents the compiler from preserving the memoization, so the value is named first.
  const workspaceId = workspace?.id;
  const persistColors = useCallback(
    async (next: Record<string, string>) => {
      if (!workspaceId || !selectedTenantId) {
        toast.error('No workspace selected');
        return;
      }
      const stripped = stripDefaultOverrides(next);
      const wsId = workspaceId;
      const previous = useTenantStore.getState().workspaces;
      setWorkspaces(
        previous.map((w) =>
          w.id === wsId
            ? {
                ...w,
                entity_type_colors:
                  Object.keys(stripped).length > 0 ? stripped : undefined,
              }
            : w,
        ),
      );
      try {
        const updated = await updateWorkspace(selectedTenantId, wsId, {
          entity_type_colors: stripped,
        });
        const latest = useTenantStore.getState().workspaces;
        setWorkspaces(
          latest.map((w) =>
            w.id === wsId
              ? {
                  ...w,
                  ...updated,
                  entity_type_colors:
                    updated.entity_type_colors ??
                    (Object.keys(stripped).length > 0 ? stripped : undefined),
                }
              : w,
          ),
        );
      } catch (err) {
        setWorkspaces(previous);
        toast.error(
          err instanceof Error ? err.message : 'Failed to save entity colors',
        );
      }
    },
    [workspaceId, selectedTenantId, setWorkspaces],
  );

  const setTypeColor = useCallback(
    async (entityType: string, hex: string) => {
      const next = { ...colors, [entityType.toUpperCase()]: hex };
      await persistColors(next);
    },
    [colors, persistColors],
  );

  const resetTypeColor = useCallback(
    async (entityType: string) => {
      const key = entityType.toUpperCase();
      const next = { ...colors };
      delete next[key];
      await persistColors(next);
    },
    [colors, persistColors],
  );

  return {
    colors,
    colorFor,
    setTypeColor,
    resetTypeColor,
    persistColors,
    workspaceId: workspace?.id ?? null,
  };
}
