'use client';

import { CreateOrganizationDialog } from '@/components/admin/create-organization-dialog';
import { CreateWorkspaceDialog } from '@/components/admin/create-workspace-dialog';
import { useAccess } from '@/lib/access/use-access';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { getTenants, getWorkspaces } from '@/lib/api/edgequake';
import {
  extrasInSameTenant,
  mergeEntitiesById,
} from '@/lib/tenant/merge-entities-by-id';
import { useAuthStore } from '@/stores/use-auth-store';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, Building2, FolderKanban, Loader2, Plus } from 'lucide-react';
import Link from 'next/link';
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

  // SPEC-181: one access read decides what each empty state offers.
  const { access } = useAccess();
  const user = useAuthStore((s) => s.user);
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
    // SPEC-181 LAW-181-12: empty is a drawn state, and it differs by what the caller may do.
    // A member of an organization whose grant was removed must not be offered a create button
    // that answers 403, and must not be told the product is empty when it is full.
    const canCreateOrg = access.canCreate.organization;
    return (
      <>
        <div className="flex h-full items-center justify-center p-4">
          <Card className="w-full max-w-md">
            <CardHeader className="pb-2 text-center">
              <div className="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-full bg-muted">
                <Building2 className="h-6 w-6 text-muted-foreground" aria-hidden />
              </div>
              <CardTitle>
                {canCreateOrg
                  ? t('tenant.welcome', "Create your organization")
                  : t('tenant.noOrgTitle', "You're not in an organization yet")}
              </CardTitle>
              <CardDescription>
                {canCreateOrg
                  ? t(
                      'tenant.createFirstTenant',
                      'An organization holds your workspaces, your people and your plan. You will be its owner.',
                    )
                  : t(
                      'tenant.noOrgBody',
                      "Ask an owner of the organization you work with to add you, or to send you an invitation link.",
                    )}
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3 text-center">
              {canCreateOrg ? (
                <Button onClick={() => setShowCreateTenant(true)} data-testid="guard-create-tenant">
                  <Plus className="mr-2 h-4 w-4" aria-hidden />
                  {t('admin.create.org.title', 'Create organization')}
                </Button>
              ) : (
                <>
                  <Button
                    variant="outline"
                    onClick={() => {
                      const who = user?.email ?? 'me';
                      void navigator.clipboard?.writeText(
                        `Hi — please add ${who} to our EdgeQuake organization. Thanks.`,
                      );
                      toast.success(t('tenant.copied', 'Request copied'));
                    }}
                    data-testid="guard-copy-request"
                  >
                    {t('tenant.copyRequest', 'Copy request message')}
                  </Button>
                  <p className="text-xs text-muted-foreground" data-testid="guard-signed-in-as">
                    {user?.email
                      ? t('tenant.signedInAs', 'Signed in as {{email}}', { email: user.email })
                      : t('tenant.askAdmin', 'Ask an admin to add you.')}
                  </p>
                </>
              )}
            </CardContent>
          </Card>
        </div>
        <CreateOrganizationDialog
          open={showCreateTenant}
          onOpenChange={setShowCreateTenant}
          onCreated={() => {
            setIsSettingUpContext(true);
            void queryClient.invalidateQueries({ queryKey: ['tenants'] });
          }}
        />
      </>
    );
  }

  if (selectedTenantId && workspacesData && workspacesData.length === 0) {
    const canCreateWs = access.canCreate.workspace;
    const quotaBlocked = access.reason.workspace === 'workspace_quota_reached';
    return (
      <>
        <div className="flex h-full items-center justify-center p-4">
          <Card className="w-full max-w-md">
            <CardHeader className="pb-2 text-center">
              <div className="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-full bg-muted">
                <FolderKanban className="h-6 w-6 text-muted-foreground" aria-hidden />
              </div>
              <CardTitle>
                {canCreateWs
                  ? t('workspace.createFirst', 'Create a workspace')
                  : t('workspace.noAccessTitle', 'No workspace yet')}
              </CardTitle>
              <CardDescription>
                {canCreateWs
                  ? t(
                      'workspace.createFirstDesc',
                      'A workspace holds your documents and the knowledge extracted from them.',
                    )
                  : quotaBlocked
                    ? t(
                        'workspace.quotaBlocked',
                        'Your plan allows {{limit}} workspaces. Archive one to free a slot.',
                        { limit: access.quota.workspaces.limit },
                      )
                    : t(
                        'workspace.noAccessBody',
                        "You don't have access to a workspace in this organization yet. Ask an owner to add you.",
                      )}
              </CardDescription>
            </CardHeader>
            <CardContent className="text-center">
              {canCreateWs ? (
                <Button
                  onClick={() => setShowCreateWorkspace(true)}
                  data-testid="guard-create-workspace"
                >
                  <Plus className="mr-2 h-4 w-4" aria-hidden />
                  {t('workspace.createWorkspace', 'Create workspace')}
                </Button>
              ) : quotaBlocked ? (
                <Button variant="outline" asChild data-testid="guard-archive-workspace">
                  <Link href="/settings/organization/workspaces?tab=archived">
                    {t('workspace.archiveOne', 'Archive a workspace')}
                  </Link>
                </Button>
              ) : null}
            </CardContent>
          </Card>
        </div>
        <CreateWorkspaceDialog
          open={showCreateWorkspace}
          onOpenChange={setShowCreateWorkspace}
          tenantId={selectedTenantId}
          onCreated={() => {
            setIsSettingUpContext(true);
            void queryClient.invalidateQueries({
              queryKey: ['workspaces', selectedTenantId],
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
