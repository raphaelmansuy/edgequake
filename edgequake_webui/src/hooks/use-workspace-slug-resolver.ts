"use client";

/**
 * @module use-workspace-slug-resolver
 * @description Reusable hook to resolve workspace slug to workspace context.
 * Eliminates duplicated workspace resolution logic across deeplink pages.
 *
 * @implements SPEC-032: Focus 6 - Deeplinks to workspace
 * @implements FEAT0653 - Workspace slug resolution hook
 * @implements FEAT0654 - Auto-tenant selection for workspace deeplinks
 *
 * @enforces BR0636 - Invalid slug handled gracefully
 * @enforces BR0504 - Tenant/workspace context set on resolution
 *
 * WHY: This hook was extracted (DRY) from 4+ deeplink pages that all
 * duplicated the same tenant auto-select → slug resolution → store sync logic.
 * Each page was ~100 lines of identical boilerplate. Now it's one shared hook.
 */

import {
  getTenants,
  getWorkspaceBySlug,
  getWorkspaces,
} from "@/lib/api/edgequake";
import { useTenantStore } from "@/stores/use-tenant-store";
import type { Workspace } from "@/types";
import { useQuery } from "@tanstack/react-query";
import { useEffect } from "react";

/** Resolution state returned by the hook. */
export interface WorkspaceSlugResolution {
  /** Resolved workspace object, or undefined if still loading / not found. */
  workspace: Workspace | undefined;
  /** True while any fetch is in-flight (tenants, workspace, workspaces list). */
  isLoading: boolean;
  /** Error from workspace-by-slug fetch, if any. */
  error: Error | null;
  /** True once the workspace is resolved AND synced to the tenant store. */
  isReady: boolean;
}

/**
 * Resolve a workspace slug to a full workspace context.
 *
 * Handles:
 * 1. Auto-selecting the first tenant if none is selected
 * 2. Fetching workspace by slug via API
 * 3. Fetching all workspaces to populate the store (prevents race conditions)
 * 4. Syncing the resolved workspace to the tenant store
 *
 * @param slug - Workspace slug to resolve (from URL path or query param)
 * @returns Resolution state with workspace, loading, error, and readiness
 *
 * @example
 * ```tsx
 * const { workspace, isLoading, error, isReady } = useWorkspaceSlugResolver(slug);
 * if (isLoading) return <LoadingSpinner />;
 * if (error || !workspace) return <NotFound slug={slug} />;
 * return <MyContent />;
 * ```
 */
export function useWorkspaceSlugResolver(
  slug: string | null | undefined,
): WorkspaceSlugResolution {
  const {
    selectedTenantId,
    selectTenant,
    selectWorkspace,
    selectedWorkspaceId,
    setWorkspaces,
  } = useTenantStore();

  // Step 1: Fetch tenants to auto-select if needed
  const { data: tenants } = useQuery({
    queryKey: ["tenants"],
    queryFn: getTenants,
    staleTime: 5 * 60 * 1000,
  });

  // Step 2: Auto-select tenant if only one exists or none selected
  useEffect(() => {
    if (!selectedTenantId && tenants && tenants.length > 0) {
      selectTenant(tenants[0].id);
    }
  }, [selectedTenantId, tenants, selectTenant]);

  // Step 3: Fetch workspace by slug
  const {
    data: workspace,
    isLoading: isLoadingWorkspace,
    error: workspaceError,
  } = useQuery({
    queryKey: ["workspace", "by-slug", selectedTenantId, slug],
    queryFn: () =>
      selectedTenantId && slug
        ? getWorkspaceBySlug(selectedTenantId, slug)
        : Promise.reject(new Error("No tenant or slug")),
    enabled: !!selectedTenantId && !!slug,
  });

  // Step 4: Fetch all workspaces to populate store
  // WHY: Prevents TenantGuard "no workspaces" race condition
  const { data: workspacesData } = useQuery({
    queryKey: ["workspaces", selectedTenantId],
    queryFn: () =>
      selectedTenantId ? getWorkspaces(selectedTenantId) : Promise.resolve([]),
    enabled: !!selectedTenantId,
    staleTime: 5 * 60 * 1000,
  });

  // Step 5: Update workspace list in store when fetched
  useEffect(() => {
    if (workspacesData && workspacesData.length > 0) {
      setWorkspaces(workspacesData);
    }
  }, [workspacesData, setWorkspaces]);

  // Step 6: Set workspace context when resolved
  useEffect(() => {
    if (workspace && workspace.id !== selectedWorkspaceId) {
      selectWorkspace(workspace.id);
    }
  }, [workspace, selectedWorkspaceId, selectWorkspace]);

  // Compute overall loading state
  const isLoading =
    isLoadingWorkspace || (!selectedTenantId && tenants === undefined);

  // Ready when workspace is resolved AND synced to store
  const isReady = !!workspace && workspace.id === selectedWorkspaceId;

  return {
    workspace,
    isLoading,
    error: workspaceError as Error | null,
    isReady,
  };
}

export default useWorkspaceSlugResolver;
