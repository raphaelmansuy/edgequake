"use client";

/**
 * @module use-tenant-context
 * @description Hook to manage tenant and workspace context.
 * Provides queries for fetching tenants/workspaces and
 * mutations for creating new ones.
 *
 * @implements UC0506 - User selects tenant from available list
 * @implements UC0507 - User selects workspace within tenant
 * @implements FEAT0861 - Multi-tenancy with workspace isolation
 * @implements FEAT0629 - Tenant/workspace switching
 *
 * @enforces BR0504 - All API calls include tenant/workspace headers
 * @enforces BR0618 - Workspace switch clears cached data
 */

import {
  createTenant,
  createWorkspace,
  getTenants,
  getWorkspaces,
} from "@/lib/api/edgequake";
import { useTenantStore } from "@/stores/use-tenant-store";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect } from "react";

/**
 * Hook to manage tenant and workspace context.
 * Provides queries for fetching tenants/workspaces and
 * mutations for creating new ones.
 */
export function useTenantContext() {
  const queryClient = useQueryClient();

  const {
    tenants,
    workspaces,
    selectedTenantId,
    selectedWorkspaceId,
    setTenants,
    setWorkspaces,
    selectTenant,
    selectWorkspace,
    initializeFromStorage,
  } = useTenantStore();

  // Initialize from storage on mount
  useEffect(() => {
    initializeFromStorage();
  }, [initializeFromStorage]);

  // Fetch tenants
  const tenantsQuery = useQuery({
    queryKey: ["tenants"],
    queryFn: getTenants,
    staleTime: 60000, // Cache for 1 minute
  });

  // Update store when tenants are fetched
  useEffect(() => {
    if (tenantsQuery.data) {
      setTenants(tenantsQuery.data);
      // Auto-select first tenant if none selected
      if (!selectedTenantId && tenantsQuery.data.length > 0) {
        selectTenant(tenantsQuery.data[0].id);
      }
    }
  }, [tenantsQuery.data, setTenants, selectedTenantId, selectTenant]);

  // Fetch workspaces for selected tenant
  const workspacesQuery = useQuery({
    queryKey: ["workspaces", selectedTenantId],
    queryFn: () =>
      selectedTenantId ? getWorkspaces(selectedTenantId) : Promise.resolve([]),
    enabled: !!selectedTenantId,
    staleTime: 60000,
  });

  // Update store when workspaces are fetched
  useEffect(() => {
    if (workspacesQuery.data) {
      setWorkspaces(workspacesQuery.data);
      // Auto-select first workspace if none selected
      if (!selectedWorkspaceId && workspacesQuery.data.length > 0) {
        selectWorkspace(workspacesQuery.data[0].id);
      }
    }
  }, [
    workspacesQuery.data,
    setWorkspaces,
    selectedWorkspaceId,
    selectWorkspace,
  ]);

  // Create tenant mutation
  const createTenantMutation = useMutation({
    mutationFn: (data: { name: string; description?: string }) =>
      createTenant(data),
    onSuccess: (newTenant) => {
      // WHY: Immediately add to store so any consumer of this hook sees the
      // new tenant selected right away without waiting for the query refetch.
      const currentTenants = useTenantStore.getState().tenants;
      setTenants([...currentTenants, newTenant]);
      queryClient.invalidateQueries({ queryKey: ["tenants"] });
      selectTenant(newTenant.id);
    },
  });

  // Create workspace mutation
  const createWorkspaceMutation = useMutation({
    mutationFn: (data: { name: string; description?: string }) =>
      selectedTenantId
        ? createWorkspace(selectedTenantId, data)
        : Promise.reject(new Error("No tenant selected")),
    onSuccess: (newWorkspace) => {
      // WHY: Immediately add to store so any consumer of this hook sees the
      // new workspace selected right away without waiting for the query refetch.
      const currentWorkspaces = useTenantStore.getState().workspaces;
      setWorkspaces([...currentWorkspaces, newWorkspace]);
      queryClient.invalidateQueries({
        queryKey: ["workspaces", selectedTenantId],
      });
      selectWorkspace(newWorkspace.id);
    },
  });

  // Callbacks
  const handleTenantSelect = useCallback(
    (tenantId: string) => {
      selectTenant(tenantId);
    },
    [selectTenant],
  );

  const handleWorkspaceSelect = useCallback(
    (workspaceId: string) => {
      selectWorkspace(workspaceId);
    },
    [selectWorkspace],
  );

  const refetchAll = useCallback(() => {
    tenantsQuery.refetch();
    if (selectedTenantId) {
      workspacesQuery.refetch();
    }
  }, [tenantsQuery, workspacesQuery, selectedTenantId]);

  // Derived values
  const selectedTenant = tenants.find((t) => t.id === selectedTenantId) || null;
  const selectedWorkspace =
    workspaces.find((w) => w.id === selectedWorkspaceId) || null;
  const isLoading = tenantsQuery.isLoading || workspacesQuery.isLoading;
  const hasContext = !!selectedTenantId && !!selectedWorkspaceId;

  return {
    // State
    tenants,
    workspaces,
    selectedTenantId,
    selectedWorkspaceId,
    selectedTenant,
    selectedWorkspace,
    isLoading,
    hasContext,

    // Queries
    tenantsQuery,
    workspacesQuery,

    // Mutations
    createTenantMutation,
    createWorkspaceMutation,

    // Actions
    selectTenant: handleTenantSelect,
    selectWorkspace: handleWorkspaceSelect,
    refetchAll,
  };
}

/**
 * Simple hook to check if tenant context is ready.
 * Useful for guarding API calls that require tenant/workspace context.
 */
export function useTenantContextReady(): boolean {
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();
  return !!selectedTenantId && !!selectedWorkspaceId;
}

/**
 * Hook to get the current tenant context headers.
 * These are automatically set by the API client, but this hook
 * can be useful for debugging or manual API calls.
 */
export function useTenantHeaders(): Record<string, string> {
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();

  const headers: Record<string, string> = {};

  if (selectedTenantId) {
    headers["X-Tenant-ID"] = selectedTenantId;
  }
  if (selectedWorkspaceId) {
    headers["X-Workspace-ID"] = selectedWorkspaceId;
  }

  return headers;
}

export default useTenantContext;
