'use client';

import { CreateTenantWizard } from '@/components/onboarding/create-tenant-wizard';
import { CreateWorkspaceWizard } from '@/components/onboarding/create-workspace-wizard';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { getTenants, getWorkspaces } from '@/lib/api/edgequake';
import {
  applyCreatedTenantContext,
  applyCreatedWorkspaceContext,
} from '@/lib/onboarding/apply-created-workspace-context';
import {
  extrasInSameTenant,
  mergeEntitiesById,
} from '@/lib/tenant/merge-entities-by-id';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, Building2, FolderKanban, Loader2, Plus } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface TenantGuardProps {
  children: React.ReactNode;
}

/**
 * Ensures a tenant and workspace are selected.
 * SPEC-101: empty states open shared create wizards (LAW-101-1).
 */
export function TenantGuard({ children }: TenantGuardProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const {
    selectedTenantId,
    selectedWorkspaceId,
    setTenants,
    setWorkspaces,
    selectTenant,
    selectWorkspace,
    initializeFromStorage,
  } = useTenantStore();

  const [showCreateTenant, setShowCreateTenant] = useState(false);
  const [showCreateWorkspace, setShowCreateWorkspace] = useState(false);
  const [isSettingUpContext, setIsSettingUpContext] = useState(false);
  const [loadingTimedOut, setLoadingTimedOut] = useState(false);

  useEffect(() => {
    initializeFromStorage();
  }, [initializeFromStorage]);

  const {
    data: tenantsData,
    isLoading: isLoadingTenants,
    error: tenantsError,
  } = useQuery({
    queryKey: ['tenants'],
    queryFn: getTenants,
    staleTime: 60000,
  });

  const {
    data: workspacesData,
    isLoading: isLoadingWorkspaces,
    isFetching: isFetchingWorkspaces,
    error: workspacesError,
  } = useQuery({
    queryKey: ['workspaces', selectedTenantId],
    queryFn: () => (selectedTenantId ? getWorkspaces(selectedTenantId) : Promise.resolve([])),
    enabled: !!selectedTenantId,
    staleTime: 60000,
  });

  const isLoading =
    isLoadingTenants || (!!selectedTenantId && isLoadingWorkspaces) || isSettingUpContext;

  useEffect(() => {
    if (!isLoading) {
      setLoadingTimedOut(false);
      return;
    }
    const timer = window.setTimeout(() => setLoadingTimedOut(true), 8000);
    return () => window.clearTimeout(timer);
  }, [isLoading]);

  useEffect(() => {
    if (tenantsData && tenantsData.length > 0) {
      setTenants(tenantsData);
      const tenantExists =
        selectedTenantId && tenantsData.some((te) => te.id === selectedTenantId);
      if (!selectedTenantId || !tenantExists) {
        selectTenant(tenantsData[0].id);
      }
    }
  }, [tenantsData, setTenants, selectedTenantId, selectTenant]);

  useEffect(() => {
    if (!workspacesData || workspacesData.length === 0) return;

    // Merge server list with any optimistic entries (just-created workspace) so we
    // do not drop a selection that is not yet in the refetch payload.
    const storeWorkspaces = extrasInSameTenant(
      useTenantStore.getState().workspaces,
      selectedTenantId,
    );
    const merged = mergeEntitiesById(workspacesData, storeWorkspaces);
    setWorkspaces(merged);

    const workspaceExists =
      selectedWorkspaceId && merged.some((w) => w.id === selectedWorkspaceId);

    // While refetching after create, do not snap back to workspacesData[0].
    if (!selectedWorkspaceId) {
      selectWorkspace(merged[0].id);
    } else if (!workspaceExists && !isFetchingWorkspaces) {
      selectWorkspace(merged[0].id);
    }
    // eslint-disable-next-line react-hooks/set-state-in-effect -- context ready after heal
    setIsSettingUpContext(false);
  }, [
    workspacesData,
    setWorkspaces,
    selectedWorkspaceId,
    selectWorkspace,
    isFetchingWorkspaces,
    selectedTenantId,
  ]);

  if (tenantsError || workspacesError || (isLoading && loadingTimedOut && !isSettingUpContext)) {
    const errorDescription = workspacesError
      ? t(
          'tenant.workspaceLoadError',
          'Could not load workspaces for this organization. The session may still be bound to another organization, or this account cannot access it.',
        )
      : loadingTimedOut && !tenantsError
        ? t(
            'tenant.connectionSlowDesc',
            'The backend is taking too long to respond. It may be busy with ingestion — check the status banner above, then retry.',
          )
        : t(
            'tenant.connectionErrorDesc',
            'Unable to connect to the server. Please check your connection and try again.',
          );
    return (
      <div className="flex items-center justify-center h-full p-4">
        <Card className="max-w-md w-full">
          <CardHeader className="text-center pb-2">
            <div className="mx-auto w-12 h-12 rounded-full bg-red-100 dark:bg-red-900/30 flex items-center justify-center mb-3">
              <AlertTriangle className="h-6 w-6 text-red-600 dark:text-red-400" />
            </div>
            <CardTitle>{t('tenant.connectionError', 'Connection Error')}</CardTitle>
            <CardDescription>{errorDescription}</CardDescription>
          </CardHeader>
          <CardContent className="text-center">
            <Button
              onClick={() => {
                setLoadingTimedOut(false);
                void queryClient.invalidateQueries({ queryKey: ['tenants'] });
                if (selectedTenantId) {
                  void queryClient.invalidateQueries({
                    queryKey: ['workspaces', selectedTenantId],
                  });
                }
              }}
            >
              {t('common.retry', 'Retry')}
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  if (isLoading) {
    const loadingLabel = isSettingUpContext
      ? t('tenant.settingUp', 'Setting up your workspace...')
      : t('tenant.loading', 'Loading workspace...');
    if (selectedTenantId && selectedWorkspaceId && !isSettingUpContext) {
      return (
        <div className="relative h-full min-h-0" data-testid="tenant-guard-overlay">
          {children}
          <div
            className="absolute inset-0 z-20 flex items-center justify-center bg-background/60"
            role="status"
            aria-busy="true"
            aria-label={loadingLabel}
          >
            <div className="text-center">
              <Loader2 className="mx-auto mb-3 h-8 w-8 animate-spin text-muted-foreground" />
              <p className="text-sm text-muted-foreground">{loadingLabel}</p>
            </div>
          </div>
        </div>
      );
    }
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-center">
          <Loader2 className="mx-auto mb-3 h-8 w-8 animate-spin text-muted-foreground" />
          <p className="text-sm text-muted-foreground">{loadingLabel}</p>
        </div>
      </div>
    );
  }

  if (tenantsData && tenantsData.length === 0) {
    return (
      <>
        <div className="flex items-center justify-center h-full p-4">
          <Card className="max-w-md w-full">
            <CardHeader className="text-center pb-2">
              <div className="mx-auto w-12 h-12 rounded-full bg-primary/10 flex items-center justify-center mb-3">
                <Building2 className="h-6 w-6 text-primary" />
              </div>
              <CardTitle>{t('tenant.welcome', 'Welcome to EdgeQuake')}</CardTitle>
              <CardDescription>
                {t(
                  'tenant.createFirstTenant',
                  'Create your first tenant to get started. A tenant represents an organization or project.',
                )}
              </CardDescription>
            </CardHeader>
            <CardContent className="text-center">
              <Button onClick={() => setShowCreateTenant(true)} data-testid="guard-create-tenant">
                <Plus className="h-4 w-4 mr-2" />
                {t('tenant.createTenant', 'Create Tenant')}
              </Button>
            </CardContent>
          </Card>
        </div>
        <CreateTenantWizard
          open={showCreateTenant}
          onOpenChange={setShowCreateTenant}
          onCreated={(tenant, workspace) => {
            setIsSettingUpContext(true);
            applyCreatedTenantContext(tenant, workspace);
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
      </>
    );
  }

  if (selectedTenantId && workspacesData && workspacesData.length === 0) {
    return (
      <>
        <div className="flex items-center justify-center h-full p-4">
          <Card className="max-w-md w-full">
            <CardHeader className="text-center pb-2">
              <div className="mx-auto w-12 h-12 rounded-full bg-primary/10 flex items-center justify-center mb-3">
                <FolderKanban className="h-6 w-6 text-primary" />
              </div>
              <CardTitle>{t('workspace.createFirst', 'Create a Workspace')}</CardTitle>
              <CardDescription>
                {t(
                  'workspace.createFirstDesc',
                  'Create your first workspace to start uploading documents and building your knowledge graph.',
                )}
              </CardDescription>
            </CardHeader>
            <CardContent className="text-center">
              <Button
                onClick={() => setShowCreateWorkspace(true)}
                data-testid="guard-create-workspace"
              >
                <Plus className="h-4 w-4 mr-2" />
                {t('workspace.createWorkspace', 'Create Workspace')}
              </Button>
            </CardContent>
          </Card>
        </div>
        <CreateWorkspaceWizard
          open={showCreateWorkspace}
          onOpenChange={setShowCreateWorkspace}
          tenantId={selectedTenantId}
          onCreated={(workspace) => {
            setIsSettingUpContext(true);
            applyCreatedWorkspaceContext(workspace);
            void queryClient.invalidateQueries({
              queryKey: ['workspaces', selectedTenantId],
            });
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

  if (!selectedTenantId || !selectedWorkspaceId) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <Loader2 className="h-8 w-8 animate-spin mx-auto text-muted-foreground mb-3" />
          <p className="text-sm text-muted-foreground">
            {t('tenant.selectingWorkspace', 'Selecting workspace...')}
          </p>
        </div>
      </div>
    );
  }

  return <>{children}</>;
}

export default TenantGuard;
