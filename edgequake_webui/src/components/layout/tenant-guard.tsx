'use client';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { getTenants, getWorkspaces, createTenant, createWorkspace } from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { FolderKanban, Building2, Loader2, Plus, AlertTriangle } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

interface TenantGuardProps {
  children: React.ReactNode;
}

/**
 * TenantGuard ensures a tenant and workspace are always selected.
 * If none exist, it prompts the user to create one.
 * If they exist but none are selected, it auto-selects them.
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

  // Dialog states
  const [showCreateTenant, setShowCreateTenant] = useState(false);
  const [showCreateWorkspace, setShowCreateWorkspace] = useState(false);
  const [newTenantName, setNewTenantName] = useState('EdgeQuake');
  const [newWorkspaceName, setNewWorkspaceName] = useState('Default Workspace');

  // Initialize from localStorage on mount
  useEffect(() => {
    initializeFromStorage();
  }, [initializeFromStorage]);

  // Fetch tenants
  const { data: tenantsData, isLoading: isLoadingTenants, error: tenantsError } = useQuery({
    queryKey: ['tenants'],
    queryFn: getTenants,
    staleTime: 60000,
  });

  // Fetch workspaces (only if tenant selected)
  const { data: workspacesData, isLoading: isLoadingWorkspaces } = useQuery({
    queryKey: ['workspaces', selectedTenantId],
    queryFn: () => selectedTenantId ? getWorkspaces(selectedTenantId) : Promise.resolve([]),
    enabled: !!selectedTenantId,
    staleTime: 60000,
  });

  // Auto-select tenant
  useEffect(() => {
    if (tenantsData && tenantsData.length > 0) {
      setTenants(tenantsData);
      if (!selectedTenantId) {
        selectTenant(tenantsData[0].id);
      }
    }
  }, [tenantsData, setTenants, selectedTenantId, selectTenant]);

  // Auto-select workspace
  useEffect(() => {
    if (workspacesData && workspacesData.length > 0) {
      setWorkspaces(workspacesData);
      if (!selectedWorkspaceId) {
        selectWorkspace(workspacesData[0].id);
      }
    }
  }, [workspacesData, setWorkspaces, selectedWorkspaceId, selectWorkspace]);

  // Create tenant mutation
  const createTenantMutation = useMutation({
    mutationFn: (data: { name: string }) => createTenant(data),
    onSuccess: (newTenant) => {
      toast.success(t('tenant.createSuccess', 'Tenant created'));
      queryClient.invalidateQueries({ queryKey: ['tenants'] });
      selectTenant(newTenant.id);
      setShowCreateTenant(false);
    },
    onError: (error) => {
      toast.error(t('tenant.createFailed', 'Failed to create tenant'), {
        description: error instanceof Error ? error.message : 'Unknown error',
      });
    },
  });

  // Create workspace mutation
  const createWorkspaceMutation = useMutation({
    mutationFn: (data: { name: string }) =>
      selectedTenantId
        ? createWorkspace(selectedTenantId, data)
        : Promise.reject(new Error('No tenant selected')),
    onSuccess: (newWorkspace) => {
      toast.success(t('workspace.createSuccess', 'Workspace created'));
      queryClient.invalidateQueries({ queryKey: ['workspaces', selectedTenantId] });
      selectWorkspace(newWorkspace.id);
      setShowCreateWorkspace(false);
    },
    onError: (error) => {
      toast.error(t('workspace.createFailed', 'Failed to create workspace'), {
        description: error instanceof Error ? error.message : 'Unknown error',
      });
    },
  });

  const isLoading = isLoadingTenants || (selectedTenantId && isLoadingWorkspaces);

  // Loading state
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <Loader2 className="h-8 w-8 animate-spin mx-auto text-muted-foreground mb-3" />
          <p className="text-sm text-muted-foreground">
            {t('tenant.loading', 'Loading workspace...')}
          </p>
        </div>
      </div>
    );
  }

  // Error state
  if (tenantsError) {
    return (
      <div className="flex items-center justify-center h-full p-4">
        <Card className="max-w-md w-full">
          <CardHeader className="text-center pb-2">
            <div className="mx-auto w-12 h-12 rounded-full bg-red-100 dark:bg-red-900/30 flex items-center justify-center mb-3">
              <AlertTriangle className="h-6 w-6 text-red-600 dark:text-red-400" />
            </div>
            <CardTitle>{t('tenant.connectionError', 'Connection Error')}</CardTitle>
            <CardDescription>
              {t('tenant.connectionErrorDesc', 'Unable to connect to the server. Please check your connection and try again.')}
            </CardDescription>
          </CardHeader>
          <CardContent className="text-center">
            <Button onClick={() => queryClient.invalidateQueries({ queryKey: ['tenants'] })}>
              {t('common.retry', 'Retry')}
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  // No tenants exist - prompt to create one
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
                {t('tenant.createFirstTenant', 'Create your first tenant to get started. A tenant represents an organization or project.')}
              </CardDescription>
            </CardHeader>
            <CardContent className="text-center">
              <Button onClick={() => setShowCreateTenant(true)}>
                <Plus className="h-4 w-4 mr-2" />
                {t('tenant.createTenant', 'Create Tenant')}
              </Button>
            </CardContent>
          </Card>
        </div>

        <Dialog open={showCreateTenant} onOpenChange={setShowCreateTenant}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>{t('tenant.createNew', 'Create Tenant')}</DialogTitle>
              <DialogDescription>
                {t('tenant.createNewDesc', 'Enter a name for your new tenant.')}
              </DialogDescription>
            </DialogHeader>
            <div className="grid gap-4 py-4">
              <div className="grid gap-2">
                <Label htmlFor="tenant-name">{t('common.name', 'Name')}</Label>
                <Input
                  id="tenant-name"
                  value={newTenantName}
                  onChange={(e) => setNewTenantName(e.target.value)}
                  placeholder="My Organization"
                />
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setShowCreateTenant(false)}>
                {t('common.cancel', 'Cancel')}
              </Button>
              <Button
                onClick={() => createTenantMutation.mutate({ name: newTenantName })}
                disabled={!newTenantName.trim() || createTenantMutation.isPending}
              >
                {createTenantMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                {t('common.create', 'Create')}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </>
    );
  }

  // Tenant selected but no workspaces exist - prompt to create one
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
                {t('workspace.createFirstDesc', 'Create your first workspace to start uploading documents and building your knowledge graph.')}
              </CardDescription>
            </CardHeader>
            <CardContent className="text-center">
              <Button onClick={() => setShowCreateWorkspace(true)}>
                <Plus className="h-4 w-4 mr-2" />
                {t('workspace.createWorkspace', 'Create Workspace')}
              </Button>
            </CardContent>
          </Card>
        </div>

        <Dialog open={showCreateWorkspace} onOpenChange={setShowCreateWorkspace}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>{t('workspace.createNew', 'Create Workspace')}</DialogTitle>
              <DialogDescription>
                {t('workspace.createNewDesc', 'Enter a name for your new workspace.')}
              </DialogDescription>
            </DialogHeader>
            <div className="grid gap-4 py-4">
              <div className="grid gap-2">
                <Label htmlFor="workspace-name">{t('common.name', 'Name')}</Label>
                <Input
                  id="workspace-name"
                  value={newWorkspaceName}
                  onChange={(e) => setNewWorkspaceName(e.target.value)}
                  placeholder="My Project"
                />
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setShowCreateWorkspace(false)}>
                {t('common.cancel', 'Cancel')}
              </Button>
              <Button
                onClick={() => createWorkspaceMutation.mutate({ name: newWorkspaceName })}
                disabled={!newWorkspaceName.trim() || createWorkspaceMutation.isPending}
              >
                {createWorkspaceMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                {t('common.create', 'Create')}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </>
    );
  }

  // Context not yet selected (should auto-select, but guard anyway)
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

  // All good - render children
  return <>{children}</>;
}

export default TenantGuard;
