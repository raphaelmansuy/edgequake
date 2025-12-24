'use client';

import { Button } from '@/components/ui/button';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuGroup,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import {
    createTenant,
    createWorkspace,
    getTenants,
    getWorkspaces,
} from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import type { Tenant, Workspace } from '@/types';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
    Building2,
    Check,
    FolderKanban,
    Loader2,
    Plus,
    RefreshCw
} from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface TenantWorkspaceSelectorProps {
  /**
   * Whether to show in compact mode (icon only)
   */
  compact?: boolean;
  /**
   * Callback when tenant changes
   */
  onTenantChange?: (tenant: Tenant) => void;
  /**
   * Callback when workspace changes
   */
  onWorkspaceChange?: (workspace: Workspace) => void;
}

export function TenantWorkspaceSelector({
  compact = false,
  onTenantChange,
  onWorkspaceChange,
}: TenantWorkspaceSelectorProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  // Store state
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

  // Dialog states
  const [showCreateTenant, setShowCreateTenant] = useState(false);
  const [showCreateWorkspace, setShowCreateWorkspace] = useState(false);
  const [newTenantName, setNewTenantName] = useState('');
  const [newTenantDescription, setNewTenantDescription] = useState('');
  const [newWorkspaceName, setNewWorkspaceName] = useState('');
  const [newWorkspaceDescription, setNewWorkspaceDescription] = useState('');

  // Initialize from storage on mount
  useEffect(() => {
    initializeFromStorage();
  }, [initializeFromStorage]);

  // Fetch tenants
  const {
    data: tenantsData,
    isLoading: isLoadingTenants,
    refetch: refetchTenants,
  } = useQuery({
    queryKey: ['tenants'],
    queryFn: getTenants,
    staleTime: 60000, // Cache for 1 minute
  });

  // Update store when tenants are fetched
  useEffect(() => {
    if (tenantsData) {
      setTenants(tenantsData);
      // Auto-select first tenant if none selected
      if (!selectedTenantId && tenantsData.length > 0) {
        selectTenant(tenantsData[0].id);
      }
    }
  }, [tenantsData, setTenants, selectedTenantId, selectTenant]);

  // Fetch workspaces for selected tenant
  const {
    data: workspacesData,
    isLoading: isLoadingWorkspaces,
    refetch: refetchWorkspaces,
  } = useQuery({
    queryKey: ['workspaces', selectedTenantId],
    queryFn: () =>
      selectedTenantId ? getWorkspaces(selectedTenantId) : Promise.resolve([]),
    enabled: !!selectedTenantId,
    staleTime: 60000,
  });

  // Update store when workspaces are fetched
  useEffect(() => {
    if (workspacesData) {
      setWorkspaces(workspacesData);
      // Auto-select first workspace if none selected
      if (!selectedWorkspaceId && workspacesData.length > 0) {
        selectWorkspace(workspacesData[0].id);
      }
    }
  }, [workspacesData, setWorkspaces, selectedWorkspaceId, selectWorkspace]);

  // Create tenant mutation
  const createTenantMutation = useMutation({
    mutationFn: (data: { name: string; description?: string }) =>
      createTenant(data),
    onSuccess: (newTenant) => {
      toast.success(t('tenant.createSuccess', 'Tenant created successfully'));
      queryClient.invalidateQueries({ queryKey: ['tenants'] });
      selectTenant(newTenant.id);
      setShowCreateTenant(false);
      setNewTenantName('');
      setNewTenantDescription('');
    },
    onError: (error) => {
      toast.error(
        t('tenant.createFailed', 'Failed to create tenant'),
        {
          description:
            error instanceof Error ? error.message : 'Unknown error',
        }
      );
    },
  });

  // Create workspace mutation
  const createWorkspaceMutation = useMutation({
    mutationFn: (data: { name: string; description?: string }) =>
      selectedTenantId
        ? createWorkspace(selectedTenantId, data)
        : Promise.reject(new Error('No tenant selected')),
    onSuccess: (newWorkspace) => {
      toast.success(
        t('workspace.createSuccess', 'Workspace created successfully')
      );
      queryClient.invalidateQueries({
        queryKey: ['workspaces', selectedTenantId],
      });
      selectWorkspace(newWorkspace.id);
      setShowCreateWorkspace(false);
      setNewWorkspaceName('');
      setNewWorkspaceDescription('');
    },
    onError: (error) => {
      toast.error(
        t('workspace.createFailed', 'Failed to create workspace'),
        {
          description:
            error instanceof Error ? error.message : 'Unknown error',
        }
      );
    },
  });

  const handleTenantSelect = useCallback(
    (tenantId: string) => {
      selectTenant(tenantId);
      const tenant = tenants.find((t) => t.id === tenantId);
      if (tenant) {
        onTenantChange?.(tenant);
      }
    },
    [selectTenant, tenants, onTenantChange]
  );

  const handleWorkspaceSelect = useCallback(
    (workspaceId: string) => {
      selectWorkspace(workspaceId);
      const workspace = workspaces.find((w) => w.id === workspaceId);
      if (workspace) {
        onWorkspaceChange?.(workspace);
      }
    },
    [selectWorkspace, workspaces, onWorkspaceChange]
  );

  const selectedTenant = tenants.find((t) => t.id === selectedTenantId);
  const selectedWorkspace = workspaces.find(
    (w) => w.id === selectedWorkspaceId
  );

  const isLoading = isLoadingTenants || isLoadingWorkspaces;

  // Compact mode - just show icon with tooltip
  if (compact) {
    return (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon" className="relative">
                  {isLoading ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Building2 className="h-4 w-4" />
                  )}
                  {selectedTenant && selectedWorkspace && (
                    <span className="absolute -top-1 -right-1 h-2 w-2 rounded-full bg-green-500" />
                  )}
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-64">
                <DropdownMenuLabel>
                  {t('tenant.selectContext', 'Select Context')}
                </DropdownMenuLabel>
                <DropdownMenuSeparator />
                <DropdownMenuGroup>
                  <DropdownMenuLabel className="text-xs text-muted-foreground">
                    {t('tenant.tenant', 'Tenant')}
                  </DropdownMenuLabel>
                  {tenants.map((tenant) => (
                    <DropdownMenuItem
                      key={tenant.id}
                      onClick={() => handleTenantSelect(tenant.id)}
                    >
                      <Building2 className="mr-2 h-4 w-4" />
                      <span className="flex-1 truncate">{tenant.name}</span>
                      {tenant.id === selectedTenantId && (
                        <Check className="ml-2 h-4 w-4" />
                      )}
                    </DropdownMenuItem>
                  ))}
                  <DropdownMenuItem onClick={() => setShowCreateTenant(true)}>
                    <Plus className="mr-2 h-4 w-4" />
                    {t('tenant.createNew', 'Create New Tenant')}
                  </DropdownMenuItem>
                </DropdownMenuGroup>
                {selectedTenantId && (
                  <>
                    <DropdownMenuSeparator />
                    <DropdownMenuGroup>
                      <DropdownMenuLabel className="text-xs text-muted-foreground">
                        {t('workspace.workspace', 'Workspace')}
                      </DropdownMenuLabel>
                      {workspaces.map((workspace) => (
                        <DropdownMenuItem
                          key={workspace.id}
                          onClick={() => handleWorkspaceSelect(workspace.id)}
                        >
                          <FolderKanban className="mr-2 h-4 w-4" />
                          <span className="flex-1 truncate">
                            {workspace.name}
                          </span>
                          <span className="text-xs text-muted-foreground ml-2">
                            {workspace.document_count ?? 0} docs
                          </span>
                          {workspace.id === selectedWorkspaceId && (
                            <Check className="ml-2 h-4 w-4" />
                          )}
                        </DropdownMenuItem>
                      ))}
                      <DropdownMenuItem
                        onClick={() => setShowCreateWorkspace(true)}
                      >
                        <Plus className="mr-2 h-4 w-4" />
                        {t('workspace.createNew', 'Create New Workspace')}
                      </DropdownMenuItem>
                    </DropdownMenuGroup>
                  </>
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          </TooltipTrigger>
          <TooltipContent>
            {selectedTenant && selectedWorkspace ? (
              <div className="text-xs">
                <div className="font-medium">{selectedTenant.name}</div>
                <div className="text-muted-foreground">
                  {selectedWorkspace.name}
                </div>
              </div>
            ) : (
              t('tenant.selectContext', 'Select Context')
            )}
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  // Full mode - show selectors stacked vertically for sidebar
  return (
    <>
      <div className="flex flex-col gap-3 p-3 bg-muted/50 rounded-lg border border-border/50">
        {/* Tenant Selector */}
        <div className="flex flex-col gap-1.5">
          <Label className="text-xs font-semibold text-muted-foreground">
            {t('tenant.tenant', 'Tenant')}
          </Label>
          <div className="flex gap-1.5 items-center">
            {isLoadingTenants ? (
              <Skeleton className="h-8 flex-1" />
            ) : (
              <Select
                value={selectedTenantId || ''}
                onValueChange={handleTenantSelect}
              >
                <SelectTrigger className="h-8 text-xs flex-1">
                  <SelectValue
                    placeholder={t('tenant.selectTenant', 'Select tenant...')}
                  />
                </SelectTrigger>
                <SelectContent>
                  {tenants.map((tenant) => (
                    <SelectItem key={tenant.id} value={tenant.id}>
                      <div className="flex items-center gap-2">
                        <Building2 className="h-3 w-3 text-muted-foreground" />
                        {tenant.name}
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
            <Button
              size="sm"
              variant="ghost"
              className="h-8 w-8 p-0 shrink-0"
              onClick={() => setShowCreateTenant(true)}
              title={t('tenant.createNew', 'Create New Tenant')}
            >
              <Plus className="h-4 w-4" />
            </Button>
            <Button
              size="sm"
              variant="ghost"
              className="h-8 w-8 p-0 shrink-0"
              onClick={() => refetchTenants()}
              title={t('common.refresh', 'Refresh')}
            >
              <RefreshCw className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* Workspace Selector - Always show, even if tenant not selected */}
        <div className="flex flex-col gap-1.5">
          <Label className="text-xs font-semibold text-muted-foreground">
            {t('workspace.workspace', 'Workspace')}
          </Label>
          <div className="flex gap-1.5 items-center">
            {isLoadingWorkspaces ? (
              <Skeleton className="h-8 flex-1" />
            ) : (
              <Select
                value={selectedWorkspaceId || ''}
                onValueChange={handleWorkspaceSelect}
                disabled={!selectedTenantId}
              >
                <SelectTrigger className="h-8 text-xs flex-1">
                  <SelectValue
                    placeholder={
                      selectedTenantId
                        ? t('workspace.selectWorkspace', 'Select workspace...')
                        : t('workspace.selectTenantFirst', 'Select tenant first')
                    }
                  />
                </SelectTrigger>
                <SelectContent>
                  {workspaces.map((workspace) => (
                    <SelectItem key={workspace.id} value={workspace.id}>
                      <div className="flex items-center gap-2">
                        <FolderKanban className="h-3 w-3 text-muted-foreground" />
                        {workspace.name}
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
            <Button
              size="sm"
              variant="ghost"
              className="h-8 w-8 p-0 shrink-0"
              onClick={() => setShowCreateWorkspace(true)}
              disabled={!selectedTenantId}
              title={t('workspace.createNew', 'Create New Workspace')}
            >
              <Plus className="h-4 w-4" />
            </Button>
            <Button
              size="sm"
              variant="ghost"
              className="h-8 w-8 p-0 shrink-0"
              onClick={() => refetchWorkspaces()}
              disabled={!selectedTenantId}
              title={t('common.refresh', 'Refresh')}
            >
              <RefreshCw className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </div>

      {/* Create Tenant Dialog */}
      <Dialog open={showCreateTenant} onOpenChange={setShowCreateTenant}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t('tenant.createNew', 'Create New Tenant')}
            </DialogTitle>
            <DialogDescription>
              {t(
                'tenant.createDescription',
                'Create a new tenant to organize your workspaces and data.'
              )}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="tenant-name">
                {t('tenant.name', 'Tenant Name')}
              </Label>
              <Input
                id="tenant-name"
                value={newTenantName}
                onChange={(e) => setNewTenantName(e.target.value)}
                placeholder={t('tenant.namePlaceholder', 'My Organization')}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="tenant-description">
                {t('tenant.description', 'Description')} ({t('common.optional', 'Optional')})
              </Label>
              <Input
                id="tenant-description"
                value={newTenantDescription}
                onChange={(e) => setNewTenantDescription(e.target.value)}
                placeholder={t(
                  'tenant.descriptionPlaceholder',
                  'A brief description...'
                )}
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowCreateTenant(false)}
            >
              {t('common.cancel', 'Cancel')}
            </Button>
            <Button
              onClick={() =>
                createTenantMutation.mutate({
                  name: newTenantName,
                  description: newTenantDescription || undefined,
                })
              }
              disabled={!newTenantName.trim() || createTenantMutation.isPending}
            >
              {createTenantMutation.isPending && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              {t('common.create', 'Create')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Create Workspace Dialog */}
      <Dialog open={showCreateWorkspace} onOpenChange={setShowCreateWorkspace}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t('workspace.createNew', 'Create New Workspace')}
            </DialogTitle>
            <DialogDescription>
              {t(
                'workspace.createDescription',
                'Create a new workspace within the current tenant to organize your documents.'
              )}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="workspace-name">
                {t('workspace.name', 'Workspace Name')}
              </Label>
              <Input
                id="workspace-name"
                value={newWorkspaceName}
                onChange={(e) => setNewWorkspaceName(e.target.value)}
                placeholder={t('workspace.namePlaceholder', 'Project Alpha')}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="workspace-description">
                {t('workspace.description', 'Description')} ({t('common.optional', 'Optional')})
              </Label>
              <Input
                id="workspace-description"
                value={newWorkspaceDescription}
                onChange={(e) => setNewWorkspaceDescription(e.target.value)}
                placeholder={t(
                  'workspace.descriptionPlaceholder',
                  'A brief description...'
                )}
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowCreateWorkspace(false)}
            >
              {t('common.cancel', 'Cancel')}
            </Button>
            <Button
              onClick={() =>
                createWorkspaceMutation.mutate({
                  name: newWorkspaceName,
                  description: newWorkspaceDescription || undefined,
                })
              }
              disabled={
                !newWorkspaceName.trim() || createWorkspaceMutation.isPending
              }
            >
              {createWorkspaceMutation.isPending && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              {t('common.create', 'Create')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

export default TenantWorkspaceSelector;
