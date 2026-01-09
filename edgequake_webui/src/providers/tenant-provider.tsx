/**
 * @module TenantProvider
 * @description Provider for tenant and workspace context initialization.
 * Auto-selects first available tenant/workspace on fresh start.
 *
 * @implements FEAT0861 - Multi-tenancy with workspace isolation
 * @implements FEAT0868 - Auto-tenant selection
 *
 * @enforces BR0504 - All API calls include tenant/workspace context
 * @enforces BR0868 - Default to first tenant if none selected
 */
'use client';

import { getTenants, getWorkspaces } from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery } from '@tanstack/react-query';
import { type ReactNode, useEffect } from 'react';

interface TenantProviderProps {
  children: ReactNode;
}

/**
 * Provider that ensures tenant and workspace context is initialized early.
 * This provider auto-selects the first available tenant and workspace
 * if none is currently selected (fresh start scenario).
 */
export function TenantProvider({ children }: TenantProviderProps) {
  const {
    selectedTenantId,
    selectedWorkspaceId,
    setTenants,
    setWorkspaces,
    selectTenant,
    selectWorkspace,
    initializeFromStorage,
    isInitialized,
    setInitialized,
  } = useTenantStore();

  // Initialize from localStorage on mount
  useEffect(() => {
    initializeFromStorage();
  }, [initializeFromStorage]);

  // Fetch tenants
  const { data: tenantsData } = useQuery({
    queryKey: ['tenants'],
    queryFn: getTenants,
    staleTime: 60000,
  });

  // Auto-select tenant when data is available
  useEffect(() => {
    if (tenantsData) {
      setTenants(tenantsData);
      
      // Auto-select first tenant if none selected
      if (!selectedTenantId && tenantsData.length > 0) {
        selectTenant(tenantsData[0].id);
      }
      
      // Mark as initialized once we have tenant data
      if (!isInitialized) {
        setInitialized(true);
      }
    }
  }, [tenantsData, setTenants, selectedTenantId, selectTenant, isInitialized, setInitialized]);

  // Fetch workspaces for selected tenant
  const { data: workspacesData } = useQuery({
    queryKey: ['workspaces', selectedTenantId],
    queryFn: () => selectedTenantId ? getWorkspaces(selectedTenantId) : Promise.resolve([]),
    enabled: !!selectedTenantId,
    staleTime: 60000,
  });

  // Auto-select workspace when data is available
  useEffect(() => {
    if (workspacesData) {
      setWorkspaces(workspacesData);
      
      // Auto-select first workspace if none selected
      if (!selectedWorkspaceId && workspacesData.length > 0) {
        selectWorkspace(workspacesData[0].id);
      }
    }
  }, [workspacesData, setWorkspaces, selectedWorkspaceId, selectWorkspace]);

  return <>{children}</>;
}

export default TenantProvider;
