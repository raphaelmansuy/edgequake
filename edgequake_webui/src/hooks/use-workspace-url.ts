"use client";

/**
 * @module use-workspace-url
 * @description Hook to synchronize workspace context with URL.
 *
 * Supports two URL formats:
 * 1. Query parameter: /query?workspace=my-project
 * 2. Path segment: /w/my-project/query (future enhancement)
 *
 * @implements UC0508 - User shares workspace-scoped URL
 * @implements FEAT0651 - Workspace slug in URL
 * @implements FEAT0652 - URL-driven workspace selection
 *
 * @enforces BR0636 - Invalid slug redirects to default workspace
 * @enforces BR0637 - Workspace change updates URL
 */

import { getWorkspaceBySlug } from "@/lib/api/edgequake";
import { useTenantStore } from "@/stores/use-tenant-store";
import type { Workspace } from "@/types";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useCallback, useEffect, useRef } from "react";
/**
 * Hook to synchronize workspace context with URL.
 *
 * Supports two URL formats:
 * 1. Query parameter: /query?workspace=my-project
 * 2. Path segment: /w/my-project/query (future enhancement)
 *
 * This hook:
 * - Reads workspace slug from URL on mount
 * - Resolves slug to workspace ID
 * - Updates URL when workspace changes
 * - Handles edge cases like invalid slugs, stale URLs
 */
export function useWorkspaceUrl() {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();

  const {
    selectedTenantId,
    selectedWorkspaceId,
    workspaces,
    tenants,
    selectWorkspace,
    selectTenant,
    setWorkspaces,
  } = useTenantStore();

  // Track if we've initialized from URL
  const hasInitializedRef = useRef(false);
  // Track last known slug to avoid redundant updates
  const lastSlugRef = useRef<string | null>(null);

  // Get current workspace and tenant objects
  const selectedWorkspace = workspaces.find(
    (w) => w.id === selectedWorkspaceId
  );
  const selectedTenant = tenants.find((t) => t.id === selectedTenantId);

  /**
   * Resolve workspace slug to workspace ID
   */
  const resolveSlugToWorkspace = useCallback(
    async (tenantId: string, slug: string): Promise<Workspace | null> => {
      try {
        // First check if we already have this workspace in the store
        const existingWorkspace = workspaces.find((w) => w.slug === slug);
        if (existingWorkspace) {
          return existingWorkspace;
        }

        // Otherwise fetch from API
        const workspace = await getWorkspaceBySlug(tenantId, slug);
        return workspace;
      } catch (error) {
        console.warn(`Failed to resolve workspace slug "${slug}":`, error);
        return null;
      }
    },
    [workspaces]
  );

  /**
   * Update URL to reflect current workspace and tenant.
   * Writes ?tenant=<slug>&workspace=<slug> for deeplink support (audit F-WS-02).
   */
  const updateUrlWithWorkspace = useCallback(
    (workspace: Workspace | null) => {
      if (!workspace) return;

      const slug = workspace.slug;
      if (!slug || slug === lastSlugRef.current) return; // No change needed

      lastSlugRef.current = slug;

      // Create new URL with workspace (and optionally tenant) params
      const params = new URLSearchParams(searchParams.toString());
      params.set("workspace", slug);

      // Add tenant slug for full deeplink support
      const tenantSlug = selectedTenant?.slug;
      if (tenantSlug) {
        params.set("tenant", tenantSlug);
      } else if (selectedTenant?.name) {
        // Derive a slug from name if no explicit slug exists
        params.set(
          "tenant",
          selectedTenant.name.toLowerCase().replace(/[^a-z0-9]+/g, "-")
        );
      }

      // Use replaceState to avoid adding history entries for every workspace switch
      const newUrl = `${pathname}?${params.toString()}`;
      router.replace(newUrl, { scroll: false });
    },
    [pathname, searchParams, router]
  );

  /**
   * Initialize from URL on first load.
   * Reads ?tenant= and ?workspace= params to restore context from a shared deeplink.
   */
  useEffect(() => {
    if (hasInitializedRef.current) return;
    if (!selectedTenantId) return;

    hasInitializedRef.current = true;

    // Resolve tenant from URL (F-WS-02: deeplink support)
    const tenantSlugFromUrl = searchParams.get("tenant");
    if (tenantSlugFromUrl && tenants.length > 0) {
      const matchingTenant = tenants.find(
        (t) =>
          t.slug === tenantSlugFromUrl ||
          t.name.toLowerCase().replace(/[^a-z0-9]+/g, "-") === tenantSlugFromUrl
      );
      if (matchingTenant && matchingTenant.id !== selectedTenantId) {
        selectTenant(matchingTenant.id);
      }
    }

    const workspaceSlug = searchParams.get("workspace");
    if (!workspaceSlug) {
      return;
    }

    // Resolve slug to workspace and select it
    (async () => {
      const workspace = await resolveSlugToWorkspace(
        selectedTenantId,
        workspaceSlug
      );
      if (workspace) {
        selectWorkspace(workspace.id);
        // Ensure workspace is in the store
        if (!workspaces.find((w) => w.id === workspace.id)) {
          setWorkspaces([...workspaces, workspace]);
        }
      } else {
        // Invalid slug - remove from URL
        const params = new URLSearchParams(searchParams.toString());
        params.delete("workspace");
        const newUrl = params.toString()
          ? `${pathname}?${params.toString()}`
          : pathname;
        router.replace(newUrl, { scroll: false });
      }
    })();
  }, [
    selectedTenantId,
    tenants,
    searchParams,
    pathname,
    router,
    resolveSlugToWorkspace,
    selectWorkspace,
    selectTenant,
    workspaces,
    setWorkspaces,
  ]);

  /**
   * Update URL when workspace or tenant changes (after initial load)
   */
  useEffect(() => {
    // Skip if we're still initializing from URL
    if (!hasInitializedRef.current) return;
    if (!selectedWorkspace) return;

    updateUrlWithWorkspace(selectedWorkspace);
  }, [selectedWorkspace, selectedTenant, updateUrlWithWorkspace]);

  return {
    currentWorkspaceSlug: selectedWorkspace?.slug ?? null,
    updateUrlWithWorkspace,
    resolveSlugToWorkspace,
  };
}

/**
 * Hook to read workspace from URL and ensure it's selected.
 * Use this in layout components that need workspace context from URL.
 */
export function useWorkspaceFromUrl() {
  const searchParams = useSearchParams();
  const workspaceSlug = searchParams.get("workspace");

  return {
    workspaceSlug,
    hasWorkspaceInUrl: !!workspaceSlug,
  };
}

export default useWorkspaceUrl;
