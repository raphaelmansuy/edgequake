'use client';

import {
  ContextSelectorPopover,
  ContextTriggerChip,
} from '@/components/layout/context-selector';
import { CreateTenantWizard } from '@/components/onboarding/create-tenant-wizard';
import { CreateWorkspaceWizard } from '@/components/onboarding/create-workspace-wizard';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { getTenants, getWorkspaces } from '@/lib/api/edgequake';
import {
  applyCreatedTenantContext,
  applyCreatedWorkspaceContext,
  buildCreatedContextSearchParams,
} from '@/lib/onboarding/apply-created-workspace-context';
import {
  extrasInSameTenant,
  mergeEntitiesById,
} from '@/lib/tenant/merge-entities-by-id';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useRouter } from 'next/navigation';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface HeaderTenantSelectorProps {
  className?: string;
}

/**
 * Compact tenant/workspace selector for the header.
 * SPEC-101: Create flows use shared wizards (LAW-101-1);
 * context chip is dual-labeled (LAW-101-11).
 */
export function HeaderTenantSelector({ className }: HeaderTenantSelectorProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const router = useRouter();

  const syncCreatedContextUrl = useCallback(
    (
      workspace: { id: string; name: string; slug?: string | null },
      tenant?: { id: string; name: string; slug?: string | null },
    ) => {
      const search = buildCreatedContextSearchParams(
        typeof window !== 'undefined' ? window.location.search : '',
        workspace,
        tenant,
      );
      const path = typeof window !== 'undefined' ? window.location.pathname : '/';
      router.replace(`${path}?${search}`, { scroll: false });
    },
    [router],
  );

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
    isInitialized,
    setInitialized,
  } = useTenantStore();

  const [selectorOpen, setSelectorOpen] = useState(false);
  const [showCreateTenant, setShowCreateTenant] = useState(false);
  const [showCreateWorkspace, setShowCreateWorkspace] = useState(false);

  useEffect(() => {
    initializeFromStorage();
  }, [initializeFromStorage]);

  const { data: tenantsData, isLoading: isLoadingTenants } = useQuery({
    queryKey: ['tenants'],
    queryFn: getTenants,
    staleTime: 60000,
  });

  useEffect(() => {
    if (tenantsData) {
      setTenants(tenantsData);
      if (!selectedTenantId && tenantsData.length > 0) {
        selectTenant(tenantsData[0].id);
      }
      if (!isInitialized) {
        setInitialized(true);
      }
    }
  }, [
    tenantsData,
    setTenants,
    selectedTenantId,
    selectTenant,
    isInitialized,
    setInitialized,
  ]);

  const {
    data: workspacesData,
    isLoading: isLoadingWorkspaces,
    isFetching: isFetchingWorkspaces,
    isError: isWorkspacesError,
  } = useQuery({
    queryKey: ['workspaces', selectedTenantId],
    queryFn: () => (selectedTenantId ? getWorkspaces(selectedTenantId) : Promise.resolve([])),
    enabled: !!selectedTenantId,
    staleTime: 60000,
  });

  useEffect(() => {
    if (!workspacesData) return;

    // Merge server list with optimistic create entries so a just-created workspace
    // is not wiped when refetch returns a slightly stale list.
    const storeWorkspaces = extrasInSameTenant(
      useTenantStore.getState().workspaces,
      selectedTenantId,
    );
    const merged = mergeEntitiesById(workspacesData, storeWorkspaces);
    setWorkspaces(merged);

    const exists =
      selectedWorkspaceId && merged.some((w) => w.id === selectedWorkspaceId);

    if (!selectedWorkspaceId && merged.length > 0) {
      selectWorkspace(merged[0].id);
      if (isInitialized && !localStorage.getItem('edgequake-workspace-initialized')) {
        toast.success(
          t('workspace.autoSelected', `Workspace "${merged[0].name}" selected`),
          {
            description: t(
              'workspace.autoSelectedDesc',
              'You can change this anytime from the selector above',
            ),
          },
        );
        localStorage.setItem('edgequake-workspace-initialized', 'true');
      }
    } else if (selectedWorkspaceId && !exists && !isFetchingWorkspaces && merged.length > 0) {
      selectWorkspace(merged[0].id);
    }
  }, [
    workspacesData,
    setWorkspaces,
    selectedWorkspaceId,
    selectWorkspace,
    isInitialized,
    isFetchingWorkspaces,
    t,
    selectedTenantId,
  ]);

  const handleTenantSelect = useCallback(
    (tenantId: string) => {
      if (tenantId === selectedTenantId) return;
      selectTenant(tenantId);
      const tenant = tenants.find((te) => te.id === tenantId);
      if (tenant) {
        toast.info(t('tenant.switched', `Switched to tenant "{{name}}"`, { name: tenant.name }), {
          id: 'tenant-switch',
          duration: 2000,
        });
      }
      // LAW-101-11: keep popover open so Workspaces list stays reachable.
    },
    [selectTenant, selectedTenantId, tenants, t],
  );

  const handleWorkspaceSelect = useCallback(
    (workspaceId: string) => {
      const workspace = workspaces.find((w) => w.id === workspaceId);
      if (!workspace) {
        setSelectorOpen(false);
        return;
      }
      const changed = workspaceId !== selectedWorkspaceId;
      if (changed) {
        selectWorkspace(workspaceId);
        toast.info(
          t('workspace.switched', `Switched to workspace "{{name}}"`, { name: workspace.name }),
          { id: 'workspace-switch', duration: 2000 },
        );
      }
      // Always sync URL (auto-select after tenant switch may already match id).
      syncCreatedContextUrl(
        workspace,
        tenants.find((te) => te.id === selectedTenantId) ??
          useTenantStore.getState().tenants.find((te) => te.id === selectedTenantId),
      );
      setSelectorOpen(false);
    },
    [
      selectWorkspace,
      selectedWorkspaceId,
      workspaces,
      t,
      syncCreatedContextUrl,
      tenants,
      selectedTenantId,
    ],
  );

  const selectedTenant = tenants.find((te) => te.id === selectedTenantId);
  const selectedWorkspace = workspaces.find((w) => w.id === selectedWorkspaceId);
  const isLoading = isLoadingTenants || isLoadingWorkspaces;

  return (
    <>
      <Popover open={selectorOpen} onOpenChange={setSelectorOpen}>
        <PopoverTrigger asChild>
          <ContextTriggerChip
            className={className}
            tenantName={selectedTenant?.name}
            workspaceName={selectedWorkspace?.name}
            workspaceCount={workspaces.length}
            isLoading={isLoading}
            open={selectorOpen}
          />
        </PopoverTrigger>

        <PopoverContent align="start" className="w-80 p-0" sideOffset={6}>
          <ContextSelectorPopover
            tenants={tenants}
            workspaces={workspaces}
            selectedTenantId={selectedTenantId}
            selectedWorkspaceId={selectedWorkspaceId}
            selectedTenantName={selectedTenant?.name}
            selectedWorkspaceName={selectedWorkspace?.name}
            isLoadingWorkspaces={isLoadingWorkspaces}
            isWorkspacesError={isWorkspacesError}
            searchResetKey={selectorOpen ? 'open' : 'closed'}
            onTenantSelect={handleTenantSelect}
            onWorkspaceSelect={handleWorkspaceSelect}
            onCreateTenant={() => {
              setSelectorOpen(false);
              setShowCreateTenant(true);
            }}
            onCreateWorkspace={() => {
              setSelectorOpen(false);
              setShowCreateWorkspace(true);
            }}
          />
        </PopoverContent>
      </Popover>

      <CreateTenantWizard
        open={showCreateTenant}
        onOpenChange={setShowCreateTenant}
        onCreated={(tenant, workspace) => {
          applyCreatedTenantContext(tenant, workspace, syncCreatedContextUrl);
          void queryClient.invalidateQueries({ queryKey: ['tenants'] });
          void queryClient.invalidateQueries({ queryKey: ['workspaces', tenant.id] });
          toast.success(t('tenant.createSuccess', 'Tenant created successfully'), {
            action: {
              label: t('onboarding.uploadDocuments', 'Upload documents'),
              onClick: () => {
                window.location.href = '/documents';
              },
            },
          });
        }}
      />

      <CreateWorkspaceWizard
        open={showCreateWorkspace}
        onOpenChange={setShowCreateWorkspace}
        tenantId={selectedTenantId}
        onCreated={(workspace) => {
          applyCreatedWorkspaceContext(workspace, syncCreatedContextUrl);
          void queryClient.invalidateQueries({ queryKey: ['workspaces', selectedTenantId] });
          toast.success(t('workspace.createSuccess', 'Workspace created successfully'), {
            action: {
              label: t('onboarding.uploadDocuments', 'Upload documents'),
              onClick: () => {
                window.location.href = '/documents';
              },
            },
          });
        }}
      />
    </>
  );
}

export default HeaderTenantSelector;
